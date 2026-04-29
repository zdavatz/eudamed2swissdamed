mod api_detail;
mod api_json;
mod eudamed_api;
mod gui;
mod swissdamed;
mod swissdamed_api;
mod version_db;

use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};

use eudamed_api::EudamedClient;
use swissdamed_api::SwissdamedClient;
use version_db::VersionDb;

const APP_DIR_NAME: &str = "eudamed2swissdamed";
const DATA_DIR_NAME: &str = "eudamed_json";

/// Return the application data directory (`~/eudamed2swissdamed/`).
/// Under macOS App Sandbox, uses the container directory.
/// On Windows uses `%USERPROFILE%\eudamed2swissdamed\`.
/// Falls back to the current working directory.
pub fn app_data_dir() -> PathBuf {
    // macOS sandbox: APP_SANDBOX_CONTAINER_ID env var is set
    if let Ok(container) = std::env::var("APP_SANDBOX_CONTAINER_ID") {
        if !container.is_empty() {
            if let Some(home) = std::env::var_os("HOME") {
                let dir = PathBuf::from(home).join(APP_DIR_NAME);
                let _ = fs::create_dir_all(&dir);
                return dir;
            }
        }
    }

    // All platforms: ~/eudamed2swissdamed/
    #[cfg(target_os = "windows")]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var_os("HOME");

    if let Some(home) = home {
        let dir = PathBuf::from(home).join(APP_DIR_NAME);
        let _ = fs::create_dir_all(&dir);
        return dir;
    }

    // Fallback: current working directory
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        // No arguments → launch GUI
        gui::run_gui().map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;
        return Ok(());
    }

    match args[0].as_str() {
        "gui" => {
            gui::run_gui().map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;
            return Ok(());
        }
        "download" => cmd_download(&args[1..]),
        "convert" => cmd_convert(&args[1..]),
        "push" => cmd_push(&args[1..]),
        "status" => cmd_status(&args[1..]),
        "stats" => cmd_stats(),
        "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!(
        "eudamed2swissdamed — Download EUDAMED devices and push to Swissdamed M2M API

Usage:
  eudamed2swissdamed download [--N] [--srn <SRN> ...]   Download from EUDAMED
  eudamed2swissdamed convert [--uuid <UUID>]             Convert to Swissdamed JSON (preview)
  eudamed2swissdamed push [--dry-run] [-v]               Push to Swissdamed API
  eudamed2swissdamed status <correlationId>              Check submission status
  eudamed2swissdamed stats                               Show version DB statistics

Environment:
  SWISSDAMED_CLIENT_ID      OAuth2 client ID
  SWISSDAMED_CLIENT_SECRET  OAuth2 client secret
  SWISSDAMED_BASE_URL       Override API base URL (default: playground)"
    );
}

// --- download ---

fn cmd_download(args: &[String]) -> Result<()> {
    let mut srns = Vec::new();
    let mut limit: Option<usize> = None;
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--srn" {
            i += 1;
            while i < args.len() && !args[i].starts_with("--") {
                srns.push(args[i].clone());
                i += 1;
            }
        } else if args[i].starts_with("--") && args[i][2..].parse::<usize>().is_ok() {
            limit = Some(args[i][2..].parse()?);
            i += 1;
        } else {
            i += 1;
        }
    }

    if limit.is_none() && srns.is_empty() {
        eprintln!("Usage: download [--N] [--srn <SRN> ...]");
        eprintln!("  --N              Number of products (e.g. --10, --100)");
        eprintln!("  --srn <SRN> ...  Filter by manufacturer/AR SRN(s)");
        std::process::exit(1);
    }

    let config = load_config()?;
    let data_dir = app_data_dir().join(DATA_DIR_NAME);
    let client = EudamedClient::new(
        &config.eudamed_base_url,
        &config.eudamed_basic_url,
        config.parallel,
        &data_dir,
    )?;

    // Step 1: Download listings
    eprintln!("=== Step 1: Downloading listings ===");
    let uuids = client.download_listing(&srns, limit)?;
    if uuids.is_empty() {
        eprintln!("No products found. Exiting.");
        return Ok(());
    }

    // Step 2: Download details
    eprintln!("=== Step 2: Downloading details ({} parallel) ===", config.parallel);
    let detail_stats = client.download_details(&uuids)?;
    eprintln!("  Details: {}", detail_stats);

    // Step 3: Download Basic UDI-DI
    eprintln!("=== Step 3: Downloading Basic UDI-DI ===");
    let basic_stats = client.download_basic_udi(&uuids)?;
    eprintln!("  Basic UDI-DI: {}", basic_stats);

    // Step 4: Completeness check + retry
    eprintln!("=== Step 4: Completeness check ===");
    let (missing_detail, missing_basic) = client.retry_missing(&uuids)?;
    if missing_detail > 0 || missing_basic > 0 {
        eprintln!(
            "  Still missing: {} detail, {} basic",
            missing_detail, missing_basic
        );
    } else {
        eprintln!("  All {} devices complete", uuids.len());
    }

    // Step 5: Update version DB
    eprintln!("=== Step 5: Updating version DB ===");
    let db_dir = app_data_dir().join("db");
    fs::create_dir_all(&db_dir)?;
    let db_path = db_dir.join("version_tracking.db");
    let db = VersionDb::open(&db_path)?;
    let mut updated = 0;
    for uuid in &uuids {
        let detail_path = client.detail_dir().join(format!("{}.json", uuid));
        let basic_path = client.basic_dir().join(format!("{}.json", uuid));
        if detail_path.exists() && basic_path.exists() {
            let detail_json = fs::read_to_string(&detail_path)?;
            let basic_json = fs::read_to_string(&basic_path)?;
            if db.has_changed(uuid, &detail_json, &basic_json)? {
                db.upsert_version(uuid, &detail_json, &basic_json)?;
                updated += 1;
            }
        }
    }
    eprintln!("  {} new/changed devices tracked", updated);

    eprintln!("=== Done! ===");
    Ok(())
}

// --- convert ---

fn cmd_convert(args: &[String]) -> Result<()> {
    let data_dir = app_data_dir().join(DATA_DIR_NAME);
    let detail_dir = data_dir.join("detail");
    let basic_dir = data_dir.join("basic");

    let uuid = args.iter()
        .position(|a| a == "--uuid")
        .and_then(|i| args.get(i + 1))
        .cloned();

    if let Some(uuid) = uuid {
        // Convert a single device
        let detail_path = detail_dir.join(format!("{}.json", uuid));
        let basic_path = basic_dir.join(format!("{}.json", uuid));
        let (endpoint, payload) = swissdamed_api::convert_device(&detail_path, &basic_path)?;
        eprintln!("Endpoint: {}", endpoint);
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        // Convert all and show summary
        let entries: Vec<_> = fs::read_dir(&detail_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect();

        let mut mdr = 0;
        let mut spp = 0;
        let mut ivdr = 0;
        let mut other = 0;
        let mut errors = 0;

        for entry in &entries {
            let uuid = entry.path().file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let basic_path = basic_dir.join(format!("{}.json", uuid));
            if !basic_path.exists() {
                continue;
            }

            match swissdamed_api::device_endpoint(&basic_path) {
                Ok(ep) => {
                    if ep.ends_with("/mdr") || ep.ends_with("/mdd") || ep.ends_with("/aimdd") {
                        mdr += 1;
                    } else if ep.ends_with("/spp") {
                        spp += 1;
                    } else if ep.ends_with("/ivdr") || ep.ends_with("/ivdd") {
                        ivdr += 1;
                    } else {
                        other += 1;
                    }
                }
                Err(_) => errors += 1,
            }
        }

        eprintln!("Conversion summary ({} files):", entries.len());
        eprintln!("  MDR/MDD/AIMDD: {}", mdr);
        eprintln!("  SPP:           {}", spp);
        eprintln!("  IVDR/IVDD:     {}", ivdr);
        if other > 0 {
            eprintln!("  Other:         {}", other);
        }
        if errors > 0 {
            eprintln!("  Errors:        {}", errors);
        }
    }

    Ok(())
}

// --- push ---

fn cmd_push(args: &[String]) -> Result<()> {
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let only_changed = args.iter().any(|a| a == "--changed");

    let config = load_config()?;
    let client_id = std::env::var("SWISSDAMED_CLIENT_ID")
        .context("Set SWISSDAMED_CLIENT_ID env var")?;
    let client_secret = std::env::var("SWISSDAMED_CLIENT_SECRET")
        .context("Set SWISSDAMED_CLIENT_SECRET env var")?;

    let mut client = SwissdamedClient::new(
        &config.swissdamed_base_url,
        &client_id,
        &client_secret,
        verbose,
    );

    let data_dir = app_data_dir().join(DATA_DIR_NAME);
    let detail_dir = data_dir.join("detail");
    let basic_dir = data_dir.join("basic");

    if dry_run {
        let count = fs::read_dir(&detail_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .count();
        eprintln!("[DRY RUN] Would push {} files to {}", count, config.swissdamed_base_url);
        return Ok(());
    }

    // Authenticate
    eprintln!("Getting token...");
    client.authenticate()?;

    if only_changed {
        // Only push devices that have changed since last push
        let db_dir = app_data_dir().join("db");
        fs::create_dir_all(&db_dir)?;
        let db = VersionDb::open(&db_dir.join("version_tracking.db"))?;
        let mut submitted = 0;
        let mut failed = 0;

        let entries: Vec<_> = fs::read_dir(&detail_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect();

        for entry in &entries {
            let uuid = entry.path().file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let detail_path = entry.path();
            let basic_path = basic_dir.join(format!("{}.json", uuid));
            if !basic_path.exists() { continue; }

            let detail_json = fs::read_to_string(&detail_path)?;
            let basic_json = fs::read_to_string(&basic_path)?;

            if !db.has_changed(&uuid, &detail_json, &basic_json)? {
                continue;
            }

            match client.submit_device(&detail_json, &basic_json) {
                Ok(result) => {
                    let endpoint = result.endpoint.clone();
                    db.log_push(
                        &uuid,
                        &endpoint,
                        result.http_status,
                        result.accepted,
                        if result.accepted { None } else { Some(&result.body) },
                    )?;
                    if result.accepted {
                        db.upsert_version(&uuid, &detail_json, &basic_json)?;
                        submitted += 1;
                    } else {
                        failed += 1;
                        eprintln!("  FAIL ({}): HTTP {}", uuid, result.http_status);
                    }
                }
                Err(e) => {
                    failed += 1;
                    eprintln!("  ERROR ({}): {}", uuid, e);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        eprintln!("\nChanged-only push: submitted={}, failed={}", submitted, failed);
    } else {
        let summary = client.push_all(&detail_dir, &basic_dir, false)?;
        eprintln!("\n{}", summary);
    }

    eprintln!("\nNext: Wait 5+ minutes, then check status with:");
    eprintln!("  eudamed2swissdamed status <correlationId>");

    Ok(())
}

// --- status ---

fn cmd_status(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Usage: status <correlationId> [...]");
        std::process::exit(1);
    }

    let config = load_config()?;
    let client_id = std::env::var("SWISSDAMED_CLIENT_ID")
        .context("Set SWISSDAMED_CLIENT_ID env var")?;
    let client_secret = std::env::var("SWISSDAMED_CLIENT_SECRET")
        .context("Set SWISSDAMED_CLIENT_SECRET env var")?;

    let mut client = SwissdamedClient::new(
        &config.swissdamed_base_url,
        &client_id,
        &client_secret,
        false,
    );

    eprintln!("Getting token...");
    client.authenticate()?;

    let result = client.check_status(args)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

// --- stats ---

fn cmd_stats() -> Result<()> {
    let db_path = app_data_dir().join("db").join("version_tracking.db");
    if !db_path.exists() {
        eprintln!("No version DB found at {}", db_path.display());
        return Ok(());
    }

    let db = VersionDb::open(&db_path)?;
    eprintln!("Version DB: {}", db_path.display());
    eprintln!("  Tracked devices: {}", db.count()?);
    eprintln!("  Successfully pushed: {}", db.count_pushed()?);

    // Count files on disk
    let data_dir = app_data_dir().join(DATA_DIR_NAME);
    let detail_dir = data_dir.join("detail");
    let basic_dir = data_dir.join("basic");

    let detail_count = fs::read_dir(&detail_dir)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0);
    let basic_count = fs::read_dir(&basic_dir)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0);

    eprintln!("  Detail files: {}", detail_count);
    eprintln!("  Basic UDI-DI files: {}", basic_count);

    Ok(())
}

// --- Config ---

struct Config {
    eudamed_base_url: String,
    eudamed_basic_url: String,
    parallel: usize,
    swissdamed_base_url: String,
}

fn load_config() -> Result<Config> {
    let swissdamed_base = std::env::var("SWISSDAMED_BASE_URL")
        .unwrap_or_else(|_| "https://playground.swissdamed.ch".to_string());

    // Try to read config.toml for EUDAMED settings
    let config_path = app_data_dir().join("config.toml");
    if let Ok(content) = fs::read_to_string(&config_path) {
        let table: toml::Table = content.parse().unwrap_or_default();

        let eudamed = table.get("eudamed");
        let base_url = eudamed
            .and_then(|e| e.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://ec.europa.eu/tools/eudamed/api/devices/udiDiData")
            .to_string();
        let basic_url = eudamed
            .and_then(|e| e.get("basic_udi_base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://ec.europa.eu/tools/eudamed/api/devices/basicUdiData/udiDiData")
            .to_string();
        let parallel = eudamed
            .and_then(|e| e.get("parallel"))
            .and_then(|v| v.as_integer())
            .unwrap_or(10) as usize;

        let sd_base = table.get("swissdamed")
            .and_then(|s| s.get("base_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(swissdamed_base);

        Ok(Config {
            eudamed_base_url: base_url,
            eudamed_basic_url: basic_url,
            parallel,
            swissdamed_base_url: sd_base,
        })
    } else {
        Ok(Config {
            eudamed_base_url: "https://ec.europa.eu/tools/eudamed/api/devices/udiDiData"
                .to_string(),
            eudamed_basic_url:
                "https://ec.europa.eu/tools/eudamed/api/devices/basicUdiData/udiDiData"
                    .to_string(),
            parallel: 10,
            swissdamed_base_url: swissdamed_base,
        })
    }
}
