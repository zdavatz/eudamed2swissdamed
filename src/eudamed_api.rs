//! EUDAMED public API client — download listing, detail, and Basic UDI-DI data.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::api_json::ListingResponse;

const USER_AGENT: &str = "Mozilla/5.0 (compatible; eudamed2swissdamed/1.0)";
const PAGE_SIZE: usize = 300;

pub struct EudamedClient {
    base_url: String,
    basic_udi_base_url: String,
    parallel: usize,
    detail_dir: PathBuf,
    basic_dir: PathBuf,
    log_dir: PathBuf,
}

impl EudamedClient {
    pub fn new(
        base_url: &str,
        basic_udi_base_url: &str,
        parallel: usize,
        data_dir: &Path,
    ) -> Result<Self> {
        let detail_dir = data_dir.join("detail");
        let basic_dir = data_dir.join("basic");
        let log_dir = data_dir.join("log");
        fs::create_dir_all(&detail_dir)
            .with_context(|| format!("Failed to create detail dir {}", detail_dir.display()))?;
        fs::create_dir_all(&basic_dir)
            .with_context(|| format!("Failed to create basic dir {}", basic_dir.display()))?;
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log dir {}", log_dir.display()))?;

        Ok(EudamedClient {
            base_url: base_url.to_string(),
            basic_udi_base_url: basic_udi_base_url.to_string(),
            parallel,
            detail_dir,
            basic_dir,
            log_dir,
        })
    }

    pub fn detail_dir(&self) -> &Path {
        &self.detail_dir
    }

    pub fn basic_dir(&self) -> &Path {
        &self.basic_dir
    }

    /// Step 1: Download paginated listing, optionally filtered by SRN(s).
    /// Returns list of UUIDs extracted from the listing.
    pub fn download_listing(&self, srns: &[String], limit: Option<usize>) -> Result<Vec<String>> {
        let mut uuids = Vec::new();

        if srns.is_empty() {
            let total = limit.unwrap_or(PAGE_SIZE);
            let pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
            let mut collected = 0;

            for p in 0..pages {
                let remaining = total - collected;
                let fetch_size = remaining.min(PAGE_SIZE);
                eprint!("  Page {} (fetch {}) ... ", p, fetch_size);

                match self.fetch_listing_page(p, fetch_size, None) {
                    Ok(response) => {
                        let count = response.content.len();
                        for device in &response.content {
                            if let Some(ref uuid) = device.uuid {
                                uuids.push(uuid.clone());
                            }
                        }
                        collected += count;
                        eprintln!("{} records (total: {})", count, collected);
                        if count == 0 || collected >= total {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("FAILED: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        } else {
            for srn in srns {
                eprintln!("  --- SRN: {} ---", srn);
                let mut srn_collected = 0;
                let mut p = 0;

                loop {
                    let fetch_size = if let Some(total) = limit {
                        let remaining = total - srn_collected;
                        remaining.min(PAGE_SIZE)
                    } else {
                        PAGE_SIZE
                    };

                    eprint!("  Page {} (fetch {}) ... ", p, fetch_size);

                    match self.fetch_listing_page(p, fetch_size, Some(srn)) {
                        Ok(response) => {
                            let count = response.content.len();
                            if count == 0 {
                                eprintln!("empty page, done");
                                break;
                            }
                            for device in &response.content {
                                if let Some(ref uuid) = device.uuid {
                                    uuids.push(uuid.clone());
                                }
                            }
                            srn_collected += count;
                            eprintln!("{} records (SRN total: {})", count, srn_collected);

                            if let Some(total) = limit {
                                if srn_collected >= total {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("FAILED: {}", e);
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                    p += 1;
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            }
        }

        eprintln!("  Listings: {} UUIDs extracted", uuids.len());
        Ok(uuids)
    }

    fn fetch_listing_page(
        &self,
        page: usize,
        page_size: usize,
        srn: Option<&str>,
    ) -> Result<ListingResponse> {
        let mut url = format!(
            "{}?page={}&pageSize={}&size={}&iso2Code=en&languageIso2Code=en",
            self.base_url, page, page_size, page_size
        );
        if let Some(srn) = srn {
            url.push_str(&format!("&srn={}", srn));
        }

        let body: String = ureq::get(&url)
            .header("User-Agent", USER_AGENT)
            .call()
            .context("Listing request failed")?
            .into_body()
            .read_to_string()
            .context("Failed to read listing response")?;

        let response: ListingResponse =
            serde_json::from_str(&body).context("Failed to parse listing JSON")?;
        Ok(response)
    }

    /// Step 2: Download detail JSON for each UUID (parallel, with resume support).
    pub fn download_details(&self, uuids: &[String]) -> Result<DownloadStats> {
        let need: Vec<&String> = uuids
            .iter()
            .filter(|uuid| {
                let path = self.detail_dir.join(format!("{}.json", uuid));
                !path.exists() || fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true)
            })
            .collect();

        let have = uuids.len() - need.len();
        if have > 0 {
            eprintln!(
                "  Resuming: {} already downloaded, {} remaining",
                have,
                need.len()
            );
        }

        let downloaded = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.parallel)
            .build()?;

        let detail_dir = self.detail_dir.clone();
        let base_url = self.base_url.clone();
        let log_dir = self.log_dir.clone();
        let dl = downloaded.clone();
        let fl = failed.clone();
        let total = need.len();

        pool.install(|| {
            need.par_iter().for_each(|uuid| {
                match Self::fetch_detail(&base_url, uuid, &detail_dir, &log_dir) {
                    Ok(()) => {
                        let n = dl.fetch_add(1, Ordering::Relaxed) + 1;
                        if n % 50 == 0 {
                            eprintln!("  Detail progress: {} / {}", have + n, have + total);
                        }
                    }
                    Err(e) => {
                        fl.fetch_add(1, Ordering::Relaxed);
                        eprintln!("  FAIL detail {}: {}", uuid, e);
                    }
                }
            });
        });

        Ok(DownloadStats {
            total: uuids.len(),
            downloaded: downloaded.load(Ordering::Relaxed),
            already_had: have,
            failed: failed.load(Ordering::Relaxed),
        })
    }

    fn fetch_detail(
        base_url: &str,
        uuid: &str,
        detail_dir: &Path,
        log_dir: &Path,
    ) -> Result<()> {
        let url = format!("{}/{}?languageIso2Code=en", base_url, uuid);

        let body = retry_fetch(&url, 3)?;

        // Pretty-print the JSON
        let value: serde_json::Value =
            serde_json::from_str(&body).context("Invalid detail JSON")?;
        let pretty = serde_json::to_string_pretty(&value)?;
        fs::write(detail_dir.join(format!("{}.json", uuid)), pretty)?;

        // Log
        let log_path = log_dir.join("download.log");
        let entry = format!(
            "{} detail {}.json\n",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            uuid
        );
        append_download_log(&log_path, &entry)?;

        Ok(())
    }

    /// Step 3: Download Basic UDI-DI data for each UUID (parallel, with resume).
    pub fn download_basic_udi(&self, uuids: &[String]) -> Result<DownloadStats> {
        let need: Vec<&String> = uuids
            .iter()
            .filter(|uuid| {
                let path = self.basic_dir.join(format!("{}.json", uuid));
                !path.exists() || fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true)
            })
            .collect();

        let have = uuids.len() - need.len();
        if have > 0 {
            eprintln!(
                "  Basic UDI-DI: {} cached, {} remaining",
                have,
                need.len()
            );
        }

        let downloaded = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.parallel)
            .build()?;

        let basic_dir = self.basic_dir.clone();
        let base_url = self.basic_udi_base_url.clone();
        let log_dir = self.log_dir.clone();
        let dl = downloaded.clone();
        let fl = failed.clone();
        let total = need.len();

        pool.install(|| {
            need.par_iter().for_each(|uuid| {
                match Self::fetch_basic(&base_url, uuid, &basic_dir, &log_dir) {
                    Ok(()) => {
                        let n = dl.fetch_add(1, Ordering::Relaxed) + 1;
                        if n % 50 == 0 {
                            eprintln!("  Basic UDI-DI progress: {} / {}", have + n, have + total);
                        }
                    }
                    Err(e) => {
                        fl.fetch_add(1, Ordering::Relaxed);
                        eprintln!("  FAIL basic {}: {}", uuid, e);
                    }
                }
            });
        });

        Ok(DownloadStats {
            total: uuids.len(),
            downloaded: downloaded.load(Ordering::Relaxed),
            already_had: have,
            failed: failed.load(Ordering::Relaxed),
        })
    }

    fn fetch_basic(
        base_url: &str,
        uuid: &str,
        basic_dir: &Path,
        log_dir: &Path,
    ) -> Result<()> {
        let url = format!("{}/{}?languageIso2Code=en", base_url, uuid);
        let body = retry_fetch(&url, 3)?;

        fs::write(basic_dir.join(format!("{}.json", uuid)), &body)?;

        let log_path = log_dir.join("download.log");
        let entry = format!(
            "{} basic {}.json\n",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            uuid
        );
        append_download_log(&log_path, &entry)?;

        Ok(())
    }

    /// Retry missing downloads (detail + basic) with more attempts.
    pub fn retry_missing(&self, uuids: &[String]) -> Result<(usize, usize)> {
        let mut missing_detail = 0;
        let mut missing_basic = 0;

        for uuid in uuids {
            let detail_path = self.detail_dir.join(format!("{}.json", uuid));
            if !detail_path.exists() || fs::metadata(&detail_path).map(|m| m.len() == 0).unwrap_or(true) {
                let url = format!("{}/{}?languageIso2Code=en", self.base_url, uuid);
                match retry_fetch(&url, 5) {
                    Ok(body) => {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                            let pretty = serde_json::to_string_pretty(&value).unwrap_or(body);
                            fs::write(&detail_path, pretty).with_context(|| {
                                format!("Failed to write retried detail file {}", detail_path.display())
                            })?;
                        }
                    }
                    Err(_) => missing_detail += 1,
                }
            }

            let basic_path = self.basic_dir.join(format!("{}.json", uuid));
            if !basic_path.exists() || fs::metadata(&basic_path).map(|m| m.len() == 0).unwrap_or(true) {
                let url = format!("{}/{}?languageIso2Code=en", self.basic_udi_base_url, uuid);
                match retry_fetch(&url, 5) {
                    Ok(body) => {
                        fs::write(&basic_path, body).with_context(|| {
                            format!("Failed to write retried basic file {}", basic_path.display())
                        })?;
                    }
                    Err(_) => missing_basic += 1,
                }
            }
        }

        Ok((missing_detail, missing_basic))
    }
}

fn append_download_log(log_path: &Path, entry: &str) -> Result<()> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("Failed to open download log {}", log_path.display()))?;
    file.write_all(entry.as_bytes())
        .with_context(|| format!("Failed to append to download log {}", log_path.display()))?;
    Ok(())
}

fn retry_fetch(url: &str, max_attempts: u32) -> Result<String> {
    let mut last_err = None;
    for attempt in 1..=max_attempts {
        match ureq::get(url)
            .header("User-Agent", USER_AGENT)
            .call()
        {
            Ok(resp) => {
                let body: String = resp
                    .into_body()
                    .read_to_string()
                    .context("Failed to read response body")?;
                return Ok(body);
            }
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(std::time::Duration::from_secs(attempt as u64 * 2));
            }
        }
    }
    Err(last_err
        .map(|e| anyhow::anyhow!("{}", e))
        .unwrap_or_else(|| anyhow::anyhow!("fetch failed after {} attempts", max_attempts)))
}

pub struct DownloadStats {
    pub total: usize,
    pub downloaded: usize,
    pub already_had: usize,
    pub failed: usize,
}

impl std::fmt::Display for DownloadStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "total={}, downloaded={}, cached={}, failed={}",
            self.total, self.downloaded, self.already_had, self.failed
        )
    }
}
