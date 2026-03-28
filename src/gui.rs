//! Cross-platform GUI (Windows + macOS) using egui/eframe.
//! Provides SRN input, credentials, and a one-click download & push pipeline.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use eframe::egui;

use crate::eudamed_api::EudamedClient;
use crate::swissdamed_api::SwissdamedClient;
use crate::version_db::VersionDb;

const DATA_DIR: &str = "eudamed_json";
const DB_PATH: &str = "db/version_tracking.db";

/// Messages from the worker thread to the GUI.
enum WorkerMsg {
    Log(String),
    Progress { step: String, detail: String },
    Done { ok: bool, summary: String },
}

/// Persistent state saved between sessions.
#[derive(Default, Clone)]
struct Settings {
    srns: String,
    limit: String,
    client_id: String,
    client_secret: String,
    base_url: String,
    dry_run: bool,
}

pub struct App {
    settings: Settings,
    log_lines: Vec<String>,
    running: bool,
    rx: Option<mpsc::Receiver<WorkerMsg>>,
    show_credentials: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Pre-fill from env vars if available
        let client_id = std::env::var("SWISSDAMED_CLIENT_ID").unwrap_or_default();
        let client_secret = std::env::var("SWISSDAMED_CLIENT_SECRET").unwrap_or_default();
        let base_url = std::env::var("SWISSDAMED_BASE_URL")
            .unwrap_or_else(|_| "https://playground.swissdamed.ch".to_string());

        App {
            settings: Settings {
                client_id,
                client_secret,
                base_url,
                ..Default::default()
            },
            log_lines: Vec::new(),
            running: false,
            rx: None,
            show_credentials: false,
        }
    }

    fn start_pipeline(&mut self, ctx: egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.running = true;
        self.log_lines.clear();

        let settings = self.settings.clone();

        thread::spawn(move || {
            run_pipeline(settings, tx, ctx);
        });
    }
}

impl eframe::App for App {
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
                        }
                        self.running = false;
                    }
                }
            }
            // Keep repainting while running
            if self.running {
                ctx.request_repaint();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eudamed2swissdamed");
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

            // --- Action button ---
            let button_text = if self.running {
                "Running..."
            } else if self.settings.dry_run {
                "Download & Preview"
            } else {
                "Download & Push"
            };

            let can_start = !self.running && !self.settings.srns.trim().is_empty();

            if ui
                .add_enabled(can_start, egui::Button::new(button_text).min_size(egui::vec2(200.0, 36.0)))
                .clicked()
            {
                self.start_pipeline(ctx.clone());
            }

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
    }
}

/// Run the full download → push pipeline in a background thread.
fn run_pipeline(settings: Settings, tx: mpsc::Sender<WorkerMsg>, ctx: egui::Context) {
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

    // Load config
    let config = match load_gui_config(&settings) {
        Ok(c) => c,
        Err(e) => {
            done(false, &format!("Config error: {}", e));
            return;
        }
    };

    // --- Step 1: Download listings ---
    progress("Download", "Fetching listings from EUDAMED...");
    let client = EudamedClient::new(
        &config.eudamed_base_url,
        &config.eudamed_basic_url,
        config.parallel,
        &PathBuf::from(DATA_DIR),
    );

    let uuids = match client.download_listing(&srns, limit) {
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

    // --- Step 5: Update version DB ---
    progress("Version DB", "Tracking changes...");
    if let Err(e) = std::fs::create_dir_all("db") {
        log(&format!("Warning: could not create db dir: {}", e));
    }
    match VersionDb::open(&PathBuf::from(DB_PATH)) {
        Ok(db) => {
            let mut updated = 0;
            for uuid in &uuids {
                let detail_path = client.detail_dir().join(format!("{}.json", uuid));
                let basic_path = client.basic_dir().join(format!("{}.json", uuid));
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

    let detail_dir = PathBuf::from(DATA_DIR).join("detail");
    let basic_dir = PathBuf::from(DATA_DIR).join("basic");

    match sd_client.push_all(&detail_dir, &basic_dir, false) {
        Ok(summary) => {
            log(&format!(
                "Submitted: {}, Failed: {}, Skipped: {}",
                summary.submitted, summary.failed, summary.skipped
            ));
            done(
                summary.failed == 0,
                &format!(
                    "{} submitted, {} failed, {} skipped",
                    summary.submitted, summary.failed, summary.skipped
                ),
            );
        }
        Err(e) => done(false, &format!("Push error: {}", e)),
    }
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

    if let Ok(content) = std::fs::read_to_string("config.toml") {
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

/// Launch the GUI application.
pub fn run_gui() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("eudamed2swissdamed")
            .with_inner_size([700.0, 600.0])
            .with_min_inner_size([500.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "eudamed2swissdamed",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
