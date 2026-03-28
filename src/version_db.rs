//! SQLite version tracking for change detection.
//! Tracks per-device hashes to skip unchanged devices on re-push.

use std::path::Path;
use anyhow::{Context, Result};
use rusqlite::Connection;
use sha2::{Sha256, Digest};

pub struct VersionDb {
    conn: Connection,
}

impl VersionDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open version DB")?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;"
        )?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS device_versions (
                uuid TEXT PRIMARY KEY,
                detail_hash TEXT NOT NULL,
                basic_hash TEXT NOT NULL,
                last_pushed TEXT,
                push_status TEXT,
                push_error TEXT,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS push_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                http_status INTEGER,
                accepted INTEGER NOT NULL DEFAULT 0,
                error_msg TEXT,
                pushed_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_push_log_uuid ON push_log(uuid);"
        )?;
        Ok(VersionDb { conn })
    }

    /// Check if a device has changed since last push.
    /// Returns true if the device is new or has changed.
    pub fn has_changed(&self, uuid: &str, detail_json: &str, basic_json: &str) -> Result<bool> {
        let detail_hash = sha256_hex(detail_json);
        let basic_hash = sha256_hex(basic_json);

        let result: Option<(String, String)> = self.conn.query_row(
            "SELECT detail_hash, basic_hash FROM device_versions WHERE uuid = ?1",
            [uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;

        match result {
            None => Ok(true), // New device
            Some((old_detail, old_basic)) => {
                Ok(old_detail != detail_hash || old_basic != basic_hash)
            }
        }
    }

    /// Update the version record for a device after processing.
    pub fn upsert_version(
        &self,
        uuid: &str,
        detail_json: &str,
        basic_json: &str,
    ) -> Result<()> {
        let detail_hash = sha256_hex(detail_json);
        let basic_hash = sha256_hex(basic_json);
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        self.conn.execute(
            "INSERT INTO device_versions (uuid, detail_hash, basic_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(uuid) DO UPDATE SET
                detail_hash = excluded.detail_hash,
                basic_hash = excluded.basic_hash,
                updated_at = excluded.updated_at",
            rusqlite::params![uuid, detail_hash, basic_hash, now],
        )?;
        Ok(())
    }

    /// Record a push attempt in the log.
    pub fn log_push(
        &self,
        uuid: &str,
        endpoint: &str,
        http_status: u16,
        accepted: bool,
        error_msg: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        self.conn.execute(
            "INSERT INTO push_log (uuid, endpoint, http_status, accepted, error_msg, pushed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![uuid, endpoint, http_status, accepted as i32, error_msg, now],
        )?;

        // Update device_versions with last push info
        if accepted {
            self.conn.execute(
                "UPDATE device_versions SET last_pushed = ?1, push_status = 'OK', push_error = NULL
                 WHERE uuid = ?2",
                rusqlite::params![now, uuid],
            )?;
        } else {
            self.conn.execute(
                "UPDATE device_versions SET push_status = 'FAIL', push_error = ?1
                 WHERE uuid = ?2",
                rusqlite::params![error_msg, uuid],
            )?;
        }

        Ok(())
    }

    /// Count total tracked devices.
    pub fn count(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM device_versions",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Count devices that have been successfully pushed.
    pub fn count_pushed(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM device_versions WHERE push_status = 'OK'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get UUIDs that were pushed but have since changed.
    pub fn changed_since_push(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid FROM device_versions WHERE last_pushed IS NOT NULL"
        )?;
        let uuids: Vec<String> = stmt.query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(uuids)
    }
}

fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// rusqlite optional row helper
trait OptionalRow {
    fn optional(self) -> Result<Option<(String, String)>>;
}

impl OptionalRow for rusqlite::Result<(String, String)> {
    fn optional(self) -> Result<Option<(String, String)>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }
}
