//! Cross-platform GUI (Windows + macOS) using egui/eframe.
//! Provides SRN input, credentials, and a one-click download & push pipeline.

use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use eframe::egui;

use crate::eudamed_api::EudamedClient;
use crate::swissdamed_api::SwissdamedClient;
use crate::version_db::VersionDb;

fn data_dir() -> PathBuf { crate::app_data_dir().join("eudamed_json") }
fn db_path() -> PathBuf { crate::app_data_dir().join("db").join("version_tracking.db") }
fn settings_path() -> PathBuf { crate::app_data_dir().join("settings.json") }
fn logs_dir() -> PathBuf { crate::app_data_dir().join("logs") }

/// Messages from the worker thread to the GUI.
enum WorkerMsg {
    Log(String),
    Progress { step: String, detail: String },
    Done { ok: bool, summary: String },
}

struct DialogMessage {
    title: String,
    message: String,
}

/// Persistent state saved between sessions.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Settings {
    srns: String,
    limit: String,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    base_url: String,
    dry_run: bool,
}

impl Settings {
    fn load() -> Self {
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(settings_path(), json)?;
        Ok(())
    }
}

pub struct App {
    settings: Settings,
    prev_settings: String,
    log_lines: Vec<String>,
    running: bool,
    rx: Option<mpsc::Receiver<WorkerMsg>>,
    show_credentials: bool,
    icon_texture: Option<egui::TextureHandle>,
    dialog: Option<DialogMessage>,
    /// 0 = Download & Push, 1 = Convert & Push (existing files only)
    pipeline_mode: u8,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut settings = Settings::load();

        // Env vars override saved credentials
        if let Ok(v) = std::env::var("SWISSDAMED_CLIENT_ID") {
            if !v.is_empty() { settings.client_id = v; }
        }
        if let Ok(v) = std::env::var("SWISSDAMED_CLIENT_SECRET") {
            if !v.is_empty() { settings.client_secret = v; }
        }
        if let Ok(v) = std::env::var("SWISSDAMED_BASE_URL") {
            if !v.is_empty() { settings.base_url = v; }
        }
        if settings.base_url.is_empty() {
            settings.base_url = "https://playground.swissdamed.ch".to_string();
        }

        let prev_settings = serde_json::to_string(&settings).unwrap_or_default();
        App {
            settings,
            prev_settings,
            log_lines: Vec::new(),
            running: false,
            rx: None,
            show_credentials: false,
            icon_texture: None,
            dialog: None,
            pipeline_mode: 0,
        }
    }

    fn save_log(&self) -> anyhow::Result<()> {
        if self.log_lines.is_empty() {
            return Ok(());
        }
        let log_dir = logs_dir();
        std::fs::create_dir_all(&log_dir)?;
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H%M%S");
        let path = log_dir.join(format!("{}.log", timestamp));
        let mut f = std::fs::File::create(&path)?;
        for line in &self.log_lines {
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }

    fn start_pipeline(&mut self, ctx: egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.running = true;
        self.log_lines.clear();

        let settings = self.settings.clone();
        let mode = self.pipeline_mode;

        thread::spawn(move || {
            run_pipeline(settings, mode, tx, ctx);
        });
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.settings.save();
        let _ = self.save_log();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain messages from worker thread
        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WorkerMsg::Log(line) => self.log_lines.push(line),
                    WorkerMsg::Progress { step, detail } => {
                        self.log_lines.push(format!("[{}] {}", step, detail));
                    }
                    WorkerMsg::Done { ok, summary } => {
                        self.log_lines.push(String::new());
                        if ok {
                            self.log_lines.push(format!("=== DONE === {}", summary));
                        } else {
                            self.log_lines.push(format!("=== FAILED === {}", summary));
                            self.dialog = Some(DialogMessage {
                                title: "Pipeline Error".to_string(),
                                message: summary.clone(),
                            });
                        }
                        self.running = false;
                        if let Err(err) = self.save_log() {
                            self.dialog = Some(DialogMessage {
                                title: "Log Save Error".to_string(),
                                message: err.to_string(),
                            });
                        }
                        if let Err(err) = self.settings.save() {
                            self.dialog = Some(DialogMessage {
                                title: "Settings Save Error".to_string(),
                                message: err.to_string(),
                            });
                        }
                    }
                }
            }
            // Keep repainting while running
            if self.running {
                ctx.request_repaint();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Load icon texture once
            let icon_texture = self.icon_texture.get_or_insert_with(|| {
                let png_bytes = include_bytes!("../assets/icon_256x256.png");
                let img = image::load_from_memory(png_bytes).unwrap().into_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let pixels = img.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                ctx.load_texture("app-icon", color_image, egui::TextureOptions::LINEAR)
            });

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let icon_button = ui.add(
                        egui::ImageButton::new(egui::load::SizedTexture::new(icon_texture.id(), egui::vec2(48.0, 48.0)))
                            .frame(false),
                    ).on_hover_text("zdavatz@ywesee.com");
                    if icon_button.clicked() {
                        if let Err(err) = open::that("mailto:zdavatz@ywesee.com") {
                            self.dialog = Some(DialogMessage {
                                title: "Open Link Error".to_string(),
                                message: err.to_string(),
                            });
                        }
                    }
                });
            });
            ui.add_space(4.0);

            // --- SRN input ---
            ui.label("SRNs (one per line or space-separated):");
            ui.add(
                egui::TextEdit::multiline(&mut self.settings.srns)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("CH-MF-000023141\nCH-MF-000012345"),
            );

            ui.add_space(4.0);

            // --- Options row ---
            ui.horizontal(|ui| {
                ui.label("Limit per SRN:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings.limit)
                        .desired_width(80.0)
                        .hint_text("all"),
                );
                ui.add_space(16.0);
                ui.checkbox(&mut self.settings.dry_run, "Dry run (no push)");
            });

            ui.add_space(8.0);

            // --- Credentials (collapsible) ---
            ui.collapsing("Swissdamed Credentials", |ui| {
                self.show_credentials = true;
                ui.horizontal(|ui| {
                    ui.label("Client ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.client_id)
                            .desired_width(300.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Client Secret:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.client_secret)
                            .desired_width(300.0)
                            .password(true),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("API Base URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.base_url)
                            .desired_width(300.0),
                    );
                });
            });

            ui.add_space(8.0);

            // --- Action buttons ---
            let can_start = !self.running && !self.settings.srns.trim().is_empty();
            let can_convert = !self.running;

            ui.horizontal(|ui| {
                let dl_text = if self.running {
                    "Running...".to_string()
                } else if self.settings.dry_run {
                    "Download & Preview".to_string()
                } else {
                    "Download & Push".to_string()
                };
                if ui
                    .add_enabled(can_start, egui::Button::new(&dl_text).min_size(egui::vec2(160.0, 32.0)))
                    .clicked()
                {
                    self.pipeline_mode = 0;
                    self.start_pipeline(ctx.clone());
                }

                let convert_text = if self.settings.dry_run {
                    "Convert only (all)".to_string()
                } else {
                    "Convert & Push (all)".to_string()
                };
                if ui
                    .add_enabled(can_convert, egui::Button::new(&convert_text).min_size(egui::vec2(150.0, 32.0)))
                    .on_hover_text("Skip download, convert+push all existing files")
                    .clicked()
                {
                    self.pipeline_mode = 1;
                    self.start_pipeline(ctx.clone());
                }

                if ui
                    .add_enabled(can_convert, egui::Button::new("Open Log Folder").min_size(egui::vec2(120.0, 32.0)))
                    .on_hover_text("Open the HTML log folder")
                    .clicked()
                {
                    let log_dir = crate::app_data_dir().join("log");
                    match std::fs::create_dir_all(&log_dir) {
                        Ok(()) => {
                            if let Err(err) = open::that(&log_dir) {
                                self.dialog = Some(DialogMessage {
                                    title: "Open Log Folder Error".to_string(),
                                    message: err.to_string(),
                                });
                            }
                        }
                        Err(err) => {
                            self.dialog = Some(DialogMessage {
                                title: "Create Log Folder Error".to_string(),
                                message: err.to_string(),
                            });
                        }
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();

            // --- Log output ---
            ui.label("Log:");
            let text_height = ui.available_height();
            egui::ScrollArea::vertical()
                .max_height(text_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log_lines {
                        ui.label(egui::RichText::new(line).monospace().size(12.0));
                    }
                });
        });

        // Auto-save settings whenever they change
        let current = serde_json::to_string(&self.settings).unwrap_or_default();
        if current != self.prev_settings {
            if let Err(err) = self.settings.save() {
                self.dialog = Some(DialogMessage {
                    title: "Settings Save Error".to_string(),
                    message: err.to_string(),
                });
            }
            self.prev_settings = current;
        }

        if let Some(dialog) = &self.dialog {
            let title = dialog.title.clone();
            let message = dialog.message.clone();
            let mut close = false;
            egui::Window::new(title)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_min_width(420.0);
                    ui.label(message);
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            if close {
                self.dialog = None;
            }
        }
    }
}

/// Run the full download → push pipeline in a background thread.
/// mode: 0 = Download & Push, 1 = Convert & Push (existing files only)
fn run_pipeline(settings: Settings, mode: u8, tx: mpsc::Sender<WorkerMsg>, ctx: egui::Context) {
    let log = |msg: &str| {
        let _ = tx.send(WorkerMsg::Log(msg.to_string()));
        ctx.request_repaint();
    };
    let progress = |step: &str, detail: &str| {
        let _ = tx.send(WorkerMsg::Progress {
            step: step.to_string(),
            detail: detail.to_string(),
        });
        ctx.request_repaint();
    };
    let done = |ok: bool, summary: &str| {
        let _ = tx.send(WorkerMsg::Done {
            ok,
            summary: summary.to_string(),
        });
        ctx.request_repaint();
    };

    // Load config
    let config = match load_gui_config(&settings) {
        Ok(c) => c,
        Err(e) => {
            done(false, &format!("Config error: {}", e));
            return;
        }
    };

    let uuids: Vec<String>;

    if mode == 0 {
        // --- Mode 0: Download & Push ---
        // Parse SRNs
        let srns: Vec<String> = settings
            .srns
            .split(|c: char| c == '\n' || c == ' ' || c == ',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if srns.is_empty() {
            done(false, "No SRNs provided");
            return;
        }

        let limit: Option<usize> = settings.limit.trim().parse().ok();

        log(&format!(
            "Starting pipeline for {} SRN(s){}",
            srns.len(),
            limit.map(|l| format!(", limit {} per SRN", l)).unwrap_or_default()
        ));

        // --- Step 1: Download listings ---
        progress("Download", "Fetching listings from EUDAMED...");
        let client = match EudamedClient::new(
            &config.eudamed_base_url,
            &config.eudamed_basic_url,
            config.parallel,
            &data_dir(),
        ) {
            Ok(client) => client,
            Err(e) => {
                done(false, &format!("Failed to initialize EUDAMED client: {}", e));
                return;
            }
        };

        uuids = match client.download_listing(&srns, limit) {
            Ok(u) => u,
            Err(e) => {
                done(false, &format!("Listing download failed: {}", e));
                return;
            }
        };

        if uuids.is_empty() {
            done(false, "No devices found for the given SRN(s)");
            return;
        }

        log(&format!("{} UUIDs extracted from listings", uuids.len()));

        // --- Step 2: Download details ---
        progress("Download", &format!("Downloading {} detail files...", uuids.len()));
        match client.download_details(&uuids) {
            Ok(stats) => log(&format!("Details: {}", stats)),
            Err(e) => {
                done(false, &format!("Detail download failed: {}", e));
                return;
            }
        }

        // --- Step 3: Download Basic UDI-DI ---
        progress("Download", "Downloading Basic UDI-DI data...");
        match client.download_basic_udi(&uuids) {
            Ok(stats) => log(&format!("Basic UDI-DI: {}", stats)),
            Err(e) => {
                done(false, &format!("Basic UDI-DI download failed: {}", e));
                return;
            }
        }

        // --- Step 4: Completeness check ---
        progress("Download", "Completeness check + retry...");
        match client.retry_missing(&uuids) {
            Ok((md, mb)) => {
                if md > 0 || mb > 0 {
                    log(&format!("Still missing: {} detail, {} basic", md, mb));
                } else {
                    log(&format!("All {} devices complete", uuids.len()));
                }
            }
            Err(e) => log(&format!("Retry warning: {}", e)),
        }
    } else {
        // --- Mode 1: Convert & Push (existing files only) ---
        let detail_dir = data_dir().join("detail");
        if !detail_dir.exists() {
            done(false, "No downloaded files found. Run Download & Push first.");
            return;
        }
        uuids = match std::fs::read_dir(&detail_dir) {
            Ok(entries) => entries,
            Err(e) => {
                done(
                    false,
                    &format!("Failed to read detail directory {}: {}", detail_dir.display(), e),
                );
                return;
            }
        }
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .filter_map(|e| e.path().file_stem().and_then(|s| s.to_str().map(|s| s.to_string())))
            .collect();

        if uuids.is_empty() {
            done(false, "No detail files found in eudamed_json/detail/");
            return;
        }
        log(&format!("Found {} existing detail files", uuids.len()));
    }

    // --- Step 5: Update version DB ---
    progress("Version DB", "Tracking changes...");
    let db_dir = crate::app_data_dir().join("db");
    if let Err(e) = std::fs::create_dir_all(&db_dir) {
        log(&format!("Warning: could not create db dir: {}", e));
    }
    match VersionDb::open(&db_dir.join("version_tracking.db")) {
        Ok(db) => {
            let mut updated = 0;
            for uuid in &uuids {
                let detail_path = data_dir().join("detail").join(format!("{}.json", uuid));
                let basic_path = data_dir().join("basic").join(format!("{}.json", uuid));
                if detail_path.exists() && basic_path.exists() {
                    if let (Ok(dj), Ok(bj)) = (
                        std::fs::read_to_string(&detail_path),
                        std::fs::read_to_string(&basic_path),
                    ) {
                        if db.has_changed(uuid, &dj, &bj).unwrap_or(true) {
                            let _ = db.upsert_version(uuid, &dj, &bj);
                            updated += 1;
                        }
                    }
                }
            }
            log(&format!("{} new/changed devices tracked", updated));
        }
        Err(e) => log(&format!("Version DB warning: {}", e)),
    }

    // --- Step 6: Push to Swissdamed ---
    if settings.dry_run {
        log("");
        done(true, &format!(
            "Dry run complete. {} devices downloaded, ready to push.",
            uuids.len()
        ));
        return;
    }

    if settings.client_id.is_empty() || settings.client_secret.is_empty() {
        log("");
        done(false, "Cannot push: SWISSDAMED_CLIENT_ID or SWISSDAMED_CLIENT_SECRET not set");
        return;
    }

    progress("Push", "Authenticating with Swissdamed...");
    let mut sd_client = SwissdamedClient::new(
        &config.swissdamed_base_url,
        &settings.client_id,
        &settings.client_secret,
        true,
    );

    if let Err(e) = sd_client.authenticate() {
        done(false, &format!("Swissdamed auth failed: {}", e));
        return;
    }

    progress("Push", &format!("Pushing {} devices...", uuids.len()));

    let eudamed_dir = data_dir();
    let detail_dir = eudamed_dir.join("detail");
    let basic_dir = eudamed_dir.join("basic");

    // Collect pushable devices
    let mut pushable: Vec<(String, String, String)> = Vec::new(); // (uuid, detail_json, basic_json)
    for uuid in &uuids {
        let detail_path = detail_dir.join(format!("{}.json", uuid));
        let basic_path = basic_dir.join(format!("{}.json", uuid));
        if let (Ok(dj), Ok(bj)) = (
            std::fs::read_to_string(&detail_path),
            std::fs::read_to_string(&basic_path),
        ) {
            pushable.push((uuid.clone(), dj, bj));
        }
    }

    let total = pushable.len();
    let mut accepted = 0u32;
    let mut rejected = 0u32;
    let mut error_details: Vec<(String, String, String, String)> = Vec::new();
    let mut accepted_uuids: Vec<(String, String)> = Vec::new();
    let mut rejected_uuids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut raw_responses: Vec<String> = Vec::new();

    for (i, (uuid, detail_json, basic_json)) in pushable.iter().enumerate() {
        progress("Push", &format!("Submitting {}/{}: {}...", i + 1, total, &uuid[..8.min(uuid.len())]));
        match sd_client.submit_device(detail_json, basic_json) {
            Ok(result) => {
                raw_responses.push(result.body.clone());
                if result.accepted {
                    accepted += 1;
                    accepted_uuids.push((uuid.clone(), result.endpoint.clone()));
                    log(&format!("  202 Accepted ({}) → {}", uuid, result.endpoint));
                } else {
                    rejected += 1;
                    rejected_uuids.insert(uuid.clone());
                    let preview: String = result.body.chars().take(300).collect();
                    error_details.push((uuid.clone(), result.endpoint.clone(), result.http_status.to_string(), preview.clone()));
                    log(&format!("  FAIL ({}) HTTP {} → {}", uuid, result.http_status, preview));
                }
            }
            Err(e) => {
                rejected += 1;
                rejected_uuids.insert(uuid.clone());
                error_details.push((uuid.clone(), String::new(), "ERROR".to_string(), e.to_string()));
                log(&format!("  ERROR ({}): {}", uuid, e));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    log(&format!("Accepted: {}, Rejected: {}", accepted, rejected));

    // --- Store in DB and generate HTML log ---
    let db_dir = crate::app_data_dir().join("db");
    let _ = std::fs::create_dir_all(&db_dir);
    if let Ok(conn) = rusqlite::Connection::open(db_dir.join("version_tracking.db")) {
        let _ = conn.execute_batch("
            CREATE TABLE IF NOT EXISTS swissdamed_push_session (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_ts TEXT NOT NULL,
                version TEXT NOT NULL,
                base_url TEXT NOT NULL,
                total_pushable INTEGER NOT NULL DEFAULT 0,
                total_accepted INTEGER NOT NULL DEFAULT 0,
                total_rejected INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS swissdamed_push_error (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                uuid TEXT NOT NULL DEFAULT '',
                endpoint TEXT NOT NULL DEFAULT '',
                http_status TEXT NOT NULL DEFAULT '',
                error_description TEXT NOT NULL DEFAULT '',
                FOREIGN KEY (session_id) REFERENCES swissdamed_push_session(id)
            );
            CREATE TABLE IF NOT EXISTS swissdamed_push_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL, correlation_id TEXT, pushed_at TEXT NOT NULL,
                endpoint TEXT NOT NULL, status TEXT NOT NULL,
                error_code TEXT, error_msg TEXT
            );
        ");

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let _ = conn.execute(
            "INSERT INTO swissdamed_push_session (session_ts, version, base_url, total_pushable, total_accepted, total_rejected) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![now, env!("CARGO_PKG_VERSION"), config.swissdamed_base_url, total, accepted, rejected],
        );
        let session_id = conn.last_insert_rowid();

        for (uuid, endpoint, http_status, err_msg) in &error_details {
            let _ = conn.execute(
                "INSERT INTO swissdamed_push_error (session_id, uuid, endpoint, http_status, error_description) VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![session_id, uuid, endpoint, http_status, err_msg],
            );
        }

        for (uuid, endpoint) in &accepted_uuids {
            let _ = conn.execute(
                "INSERT INTO swissdamed_push_log (uuid, correlation_id, pushed_at, endpoint, status) VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![uuid, uuid, now, endpoint, "ACCEPTED"],
            );
        }
        for uuid in &rejected_uuids {
            let ep = error_details.iter().find(|(u, _, _, _)| u == uuid).map(|(_, e, _, _)| e.as_str()).unwrap_or("");
            let _ = conn.execute(
                "INSERT INTO swissdamed_push_log (uuid, correlation_id, pushed_at, endpoint, status) VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![uuid, uuid, now, ep, "REJECTED"],
            );
        }

        // Generate HTML log
        let log_dir_path = crate::app_data_dir().join("log");
        let _ = std::fs::create_dir_all(&log_dir_path);
        let log_file = log_dir_path.join(format!("swissdamed_{}.log.html", chrono::Local::now().format("%H.%M_%d.%m.%Y")));
        let html = generate_push_html(&conn, session_id, &raw_responses);
        let _ = std::fs::write(&log_file, &html);
        log(&format!("HTML log: {}", log_file.display()));
    }

    done(
        rejected == 0,
        &format!("{} accepted, {} rejected", accepted, rejected),
    );
}

struct GuiConfig {
    eudamed_base_url: String,
    eudamed_basic_url: String,
    parallel: usize,
    swissdamed_base_url: String,
}

fn load_gui_config(settings: &Settings) -> anyhow::Result<GuiConfig> {
    let swissdamed_base = if settings.base_url.is_empty() {
        "https://playground.swissdamed.ch".to_string()
    } else {
        settings.base_url.clone()
    };

    let mut eudamed_base = "https://ec.europa.eu/tools/eudamed/api/devices/udiDiData".to_string();
    let mut eudamed_basic =
        "https://ec.europa.eu/tools/eudamed/api/devices/basicUdiData/udiDiData".to_string();
    let mut parallel = 10;

    let config_path = crate::app_data_dir().join("config.toml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(table) = content.parse::<toml::Table>() {
            if let Some(eudamed) = table.get("eudamed") {
                if let Some(url) = eudamed.get("base_url").and_then(|v| v.as_str()) {
                    eudamed_base = url.to_string();
                }
                if let Some(url) = eudamed.get("basic_udi_base_url").and_then(|v| v.as_str()) {
                    eudamed_basic = url.to_string();
                }
                if let Some(p) = eudamed.get("parallel").and_then(|v| v.as_integer()) {
                    parallel = p as usize;
                }
            }
        }
    }

    Ok(GuiConfig {
        eudamed_base_url: eudamed_base,
        eudamed_basic_url: eudamed_basic,
        parallel,
        swissdamed_base_url: swissdamed_base,
    })
}

/// Load the embedded app icon as an `egui::IconData`.
fn generate_push_html(conn: &rusqlite::Connection, session_id: i64, raw_responses: &[String]) -> String {
    let (version, timestamp, base_url, total_pushable, total_accepted, total_rejected) = conn
        .query_row(
            "SELECT version, session_ts, base_url, total_pushable, total_accepted, total_rejected FROM swissdamed_push_session WHERE id=?1",
            rusqlite::params![session_id],
            |row| Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, i64>(3).unwrap_or_default(),
                row.get::<_, i64>(4).unwrap_or_default(),
                row.get::<_, i64>(5).unwrap_or_default(),
            )),
        )
        .unwrap_or_default();

    let mut html = format!(
        "<!DOCTYPE html><html><head><meta charset='utf-8'><title>Swissdamed Push Log</title>\
        <style>body{{font-family:monospace;margin:20px}}h1{{font-size:18px}}\
        table{{border-collapse:collapse;width:100%;margin:10px 0}}\
        th,td{{border:1px solid #ccc;padding:6px 10px;text-align:left}}\
        th{{background:#f0f0f0}}.ok{{color:green}}.err{{color:red}}\
        .summary{{background:#f8f8f8;padding:10px;margin:10px 0}}\
        pre{{background:#f4f4f4;padding:10px;overflow-x:auto;max-height:600px;font-size:12px}}\
        </style></head><body>\
        <h1>Swissdamed Push Log (v{version})</h1>\
        <div class='summary'>\
        <b>Version:</b> {version}<br>\
        <b>Timestamp:</b> {timestamp}<br>\
        <b>Base URL:</b> {base_url}<br>\
        <b>Accepted:</b> <span class='ok'>{accepted}</span> | \
        <b>Rejected:</b> <span class='err'>{rejected}</span><br>\
        <b>Total pushable:</b> {pushable}\
        </div>",
        version = version, timestamp = timestamp, base_url = base_url,
        accepted = total_accepted, rejected = total_rejected, pushable = total_pushable,
    );

    // Error summary by HTTP status
    if let Ok(mut stmt) = conn.prepare(
        "SELECT http_status, COUNT(*) as total, COUNT(DISTINCT uuid) as devices \
         FROM swissdamed_push_error WHERE session_id=?1 GROUP BY http_status ORDER BY total DESC"
    ) {
        let rows: Vec<(String, i64, i64)> = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }).unwrap().filter_map(|r| r.ok()).collect();

        if !rows.is_empty() {
            html.push_str("<h2 class='err'>Error Summary</h2><table><tr><th>HTTP Status</th><th>Errors</th><th>Devices</th></tr>");
            for (status, count, devices) in &rows {
                html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td></tr>", status, count, devices));
            }
            html.push_str("</table>");
        }
    }

    // Rejected devices
    if let Ok(mut stmt) = conn.prepare(
        "SELECT uuid, endpoint, http_status, error_description FROM swissdamed_push_error WHERE session_id=?1 LIMIT 500"
    ) {
        let rows: Vec<(String, String, String, String)> = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        }).unwrap().filter_map(|r| r.ok()).collect();

        if !rows.is_empty() {
            html.push_str(&format!("<h2 class='err'>Rejected Devices ({})</h2><table><tr><th>#</th><th>UUID</th><th>Endpoint</th><th>HTTP</th><th>Error</th></tr>", rows.len()));
            for (i, (uuid, ep, status, err)) in rows.iter().enumerate() {
                html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    i + 1, uuid, ep, status,
                    err.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")));
            }
            html.push_str("</table>");
        }
    }

    // Accepted list
    if let Ok(mut stmt) = conn.prepare(
        "SELECT uuid, endpoint FROM swissdamed_push_log WHERE pushed_at=(SELECT session_ts FROM swissdamed_push_session WHERE id=?1) AND status='ACCEPTED' ORDER BY uuid LIMIT 500"
    ) {
        let rows: Vec<(String, String)> = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).unwrap().filter_map(|r| r.ok()).collect();

        if !rows.is_empty() {
            html.push_str(&format!("<h2 class='ok'>Accepted ({})</h2><table><tr><th>#</th><th>UUID</th><th>Endpoint</th></tr>", rows.len()));
            for (i, (uuid, ep)) in rows.iter().enumerate() {
                html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td></tr>", i + 1, uuid, ep));
            }
            html.push_str("</table>");
        }
    }

    // Raw responses
    if !raw_responses.is_empty() {
        html.push_str("<h2>Raw API Responses</h2>");
        for (i, raw) in raw_responses.iter().enumerate() {
            html.push_str(&format!("<h3>Response {}</h3><pre>{}</pre>", i + 1,
                raw.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")));
        }
    }

    html.push_str("</body></html>");
    html
}

fn load_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!("../assets/icon_256x256.png");
    let img = image::load_from_memory(png_bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    })
}

/// Launch the GUI application.
pub fn run_gui() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!("eudamed2swissdamed v{}", env!("CARGO_PKG_VERSION")))
        .with_inner_size([700.0, 600.0])
        .with_min_inner_size([500.0, 400.0]);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "eudamed2swissdamed",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
