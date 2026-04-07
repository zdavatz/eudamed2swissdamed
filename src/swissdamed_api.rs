//! Swissdamed M2M API client — OAuth2 auth, submit, status check, market status.

use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

use crate::api_detail::{parse_api_detail, parse_basic_udi_di, BasicUdiDiData};
use crate::swissdamed::{legislation_endpoint, to_swissdamed_json};

pub struct SwissdamedClient {
    base_url: String,
    client_id: String,
    client_secret: String,
    token: Option<String>,
    verbose: bool,
}

fn post_request(url: &str, content_type: &str, auth: Option<&str>, body: &str) -> Result<(u16, String)> {
    let mut req = ureq::post(url)
        .header("Content-Type", content_type);
    if let Some(auth) = auth {
        req = req.header("Authorization", auth);
    }

    match req.send(body) {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let resp_body: String = resp.into_body().read_to_string().unwrap_or_default();
            Ok((status, resp_body))
        }
        Err(ureq::Error::StatusCode(code)) => {
            Ok((code, format!("HTTP {}", code)))
        }
        Err(e) => Err(anyhow::anyhow!("{}", e)),
    }
}

impl SwissdamedClient {
    pub fn new(base_url: &str, client_id: &str, client_secret: &str, verbose: bool) -> Self {
        SwissdamedClient {
            base_url: base_url.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            token: None,
            verbose,
        }
    }

    /// Get OAuth2 access token via client credentials flow (CIAM endpoint)
    pub fn authenticate(&mut self) -> Result<()> {
        let token_url = "https://3a5c95df-c59f-418a-96fc-b8531bf24be8.ciamlogin.com/3a5c95df-c59f-418a-96fc-b8531bf24be8/oauth2/v2.0/token";
        let scope = "8d64e26d-ea71-4ab8-90d6-2acd795eb668/.default";
        if self.verbose {
            eprintln!("  POST {}", token_url);
        }

        let body = format!(
            "grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
            self.client_id, self.client_secret, scope
        );

        let (status, response) = post_request(
            &token_url,
            "application/x-www-form-urlencoded",
            None,
            &body,
        ).context("OAuth2 token request failed")?;

        if status != 200 {
            anyhow::bail!("OAuth2 token request returned HTTP {}: {}", status, response);
        }

        let json: serde_json::Value =
            serde_json::from_str(&response).context("Invalid token JSON")?;
        let token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .context("No access_token in response")?;

        if token.len() < 20 {
            anyhow::bail!("Token too short ({} chars)", token.len());
        }
        eprintln!("Token obtained ({} chars)", token.len());
        self.token = Some(token);
        Ok(())
    }

    fn token(&self) -> Result<&str> {
        self.token
            .as_deref()
            .context("Not authenticated — call authenticate() first")
    }

    fn auth_header(&self) -> Result<String> {
        Ok(format!("Bearer {}", self.token()?))
    }

    /// Submit a single device to the appropriate Swissdamed endpoint.
    pub fn submit_device(
        &self,
        detail_json: &str,
        basic_udi_json: &str,
    ) -> Result<SubmitResult> {
        let device = parse_api_detail(detail_json)?;
        let basic_udi = parse_basic_udi_di(basic_udi_json)?;
        let endpoint = legislation_endpoint(&basic_udi);
        let payload = to_swissdamed_json(&device, &basic_udi)?;
        let uuid = device.uuid.clone().unwrap_or_default();

        self.submit_payload(&uuid, endpoint, &payload)
    }

    fn submit_payload(
        &self,
        uuid: &str,
        endpoint: &str,
        payload: &serde_json::Value,
    ) -> Result<SubmitResult> {
        let url = format!("{}{}", self.base_url, endpoint);
        let auth = self.auth_header()?;

        if self.verbose {
            eprintln!("  POST {} ({})", url, uuid);
        }

        let body = serde_json::to_string(payload)?;
        let (status, resp_body) = post_request(&url, "application/json", Some(&auth), &body)
            .context(format!("Submit failed for {}", uuid))?;

        Ok(SubmitResult {
            uuid: uuid.to_string(),
            endpoint: endpoint.to_string(),
            http_status: status,
            accepted: status == 202,
            body: resp_body,
        })
    }

    /// Push all devices from detail_dir + basic_dir to Swissdamed.
    pub fn push_all(
        &self,
        detail_dir: &Path,
        basic_dir: &Path,
        dry_run: bool,
    ) -> Result<PushSummary> {
        let mut summary = PushSummary::default();

        let entries: Vec<_> = fs::read_dir(detail_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
            .collect();

        summary.total = entries.len();
        eprintln!("Found {} detail files", summary.total);

        if dry_run {
            eprintln!("[DRY RUN] Would push {} files to {}", summary.total, self.base_url);
            return Ok(summary);
        }

        for entry in &entries {
            let path = entry.path();
            let uuid = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let basic_path = basic_dir.join(format!("{}.json", uuid));
            if !basic_path.exists() {
                summary.skipped += 1;
                continue;
            }

            let detail_json = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => {
                    summary.skipped += 1;
                    continue;
                }
            };
            let basic_json = match fs::read_to_string(&basic_path) {
                Ok(s) => s,
                Err(_) => {
                    summary.skipped += 1;
                    continue;
                }
            };

            match self.submit_device(&detail_json, &basic_json) {
                Ok(result) => {
                    if result.accepted {
                        summary.submitted += 1;
                        if self.verbose {
                            eprintln!("    202 Accepted ({})", uuid);
                        }
                    } else if result.http_status == 429 {
                        eprintln!("    429 Rate limited — waiting 60s");
                        std::thread::sleep(std::time::Duration::from_secs(60));
                        // Retry once
                        match self.submit_device(&detail_json, &basic_json) {
                            Ok(r2) if r2.accepted => summary.submitted += 1,
                            _ => {
                                summary.failed += 1;
                                eprintln!("    FAIL ({}): HTTP {}", uuid, result.http_status);
                            }
                        }
                    } else {
                        summary.failed += 1;
                        let preview: String = result.body.chars().take(200).collect();
                        eprintln!(
                            "  FAIL ({}): HTTP {} — {}",
                            uuid, result.http_status, preview
                        );
                    }
                }
                Err(e) => {
                    summary.failed += 1;
                    eprintln!("  ERROR ({}): {}", uuid, e);
                }
            }

            // Progress
            let done = summary.submitted + summary.failed + summary.skipped;
            if done % 100 == 0 {
                eprintln!(
                    "  Progress: {}/{} (submitted={} failed={} skipped={})",
                    done, summary.total, summary.submitted, summary.failed, summary.skipped
                );
            }

            // Throttle
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        Ok(summary)
    }

    /// Check submission status for a correlation ID.
    pub fn check_status(&self, correlation_ids: &[String]) -> Result<serde_json::Value> {
        let url = format!("{}/v1/m2m/udi/data/udi-di-request-status", self.base_url);
        let auth = self.auth_header()?;
        let body = serde_json::to_string(&serde_json::json!({
            "correlationIds": correlation_ids,
        }))?;

        let (status, resp_body) = post_request(&url, "application/json", Some(&auth), &body)
            .context("Status request failed")?;

        if status != 200 {
            anyhow::bail!("Status check returned HTTP {}: {}", status, resp_body);
        }

        let json: serde_json::Value = serde_json::from_str(&resp_body)?;
        Ok(json)
    }

    /// Set market status for submitted devices.
    pub fn set_market_status(
        &self,
        payload: &serde_json::Value,
    ) -> Result<(u16, String)> {
        let url = format!("{}/v1/m2m/udi/data/market-status", self.base_url);
        let auth = self.auth_header()?;
        let body = serde_json::to_string(payload)?;

        post_request(&url, "application/json", Some(&auth), &body)
            .context("Market status request failed")
    }
}

/// Convert a single device from files to Swissdamed JSON (for preview/dry-run).
pub fn convert_device(
    detail_path: &Path,
    basic_path: &Path,
) -> Result<(String, serde_json::Value)> {
    let detail_json = fs::read_to_string(detail_path)?;
    let basic_json = fs::read_to_string(basic_path)?;
    let device = parse_api_detail(&detail_json)?;
    let basic_udi = parse_basic_udi_di(&basic_json)?;
    let endpoint = legislation_endpoint(&basic_udi);
    let payload = to_swissdamed_json(&device, &basic_udi)?;
    Ok((endpoint.to_string(), payload))
}

/// Determine endpoint for a device from its Basic UDI-DI file.
pub fn device_endpoint(basic_path: &Path) -> Result<String> {
    let basic_json = fs::read_to_string(basic_path)?;
    let basic_udi: BasicUdiDiData = serde_json::from_str(&basic_json)?;
    Ok(legislation_endpoint(&basic_udi).to_string())
}

pub struct SubmitResult {
    pub uuid: String,
    pub endpoint: String,
    pub http_status: u16,
    pub accepted: bool,
    pub body: String,
}

#[derive(Default)]
pub struct PushSummary {
    pub total: usize,
    pub submitted: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl std::fmt::Display for PushSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Swissdamed Push Summary ===")?;
        writeln!(f, "Total files:  {}", self.total)?;
        writeln!(f, "Submitted:    {}", self.submitted)?;
        writeln!(f, "Failed:       {}", self.failed)?;
        write!(f, "Skipped:      {}", self.skipped)
    }
}
