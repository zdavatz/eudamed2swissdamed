# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**eudamed2swissdamed** — Download medical device data from EUDAMED and push it to the Swissdamed M2M API.

- **Language:** Rust (2021 edition)
- **License:** GPLv3
- **Repository:** https://github.com/zdavatz/eudamed2swissdamed
- **Author:** Zeno R.R. Davatz (zdavatz@ywesee.com)

## Build & Run

```bash
cargo build --release
cargo run                                         # launch GUI (no args)
cargo run -- download --10                        # download first 10 devices
cargo run -- download --srn CH-MF-000023141       # download by manufacturer SRN
cargo run -- convert                              # show conversion summary
cargo run -- convert --uuid <UUID>                # preview single device as Swissdamed JSON
cargo run -- push --dry-run                       # dry run push
cargo run -- push -v                              # push to Swissdamed API (verbose)
cargo run -- push --changed                       # only push devices changed since last push
cargo run -- status <correlationId>               # check submission status
cargo run -- stats                                # show version DB statistics
```

**Environment variables** (required for push/status):
- `SWISSDAMED_CLIENT_ID` — OAuth2 client ID
- `SWISSDAMED_CLIENT_SECRET` — OAuth2 client secret
- `SWISSDAMED_BASE_URL` — override API base (default: playground)

## macOS App Bundle

```bash
./bundle_macos.sh          # creates target/release/eudamed2swissdamed.app
```

## CI/CD Release

Tag-push (`v*`) triggers `.github/workflows/release.yml` which builds:
- **macOS:** Universal binary → signed `.app` → notarized `.dmg` + App Store `.pkg`
- **Windows:** `.zip` + `.msix` → Microsoft Store submission (if `MSSTORE_ENABLED=true`)
- **Linux:** `.tar.gz` + `.AppImage`

### Required GitHub Secrets

| Secret | Purpose |
|--------|---------|
| `APPLE_API_KEY_P8` | App Store Connect API key (.p8, base64) |
| `APPLE_API_KEY_ID` | API key ID |
| `APPLE_API_ISSUER_ID` | App Store Connect Issuer ID |
| `APPLE_TEAM_ID` | Apple Developer Team ID |
| `MACOS_CERTIFICATE` | Mac App Distribution cert (.p12, base64) |
| `MACOS_CERTIFICATE_PASSWORD` | Password for above |
| `MACOS_INSTALLER_CERTIFICATE` | Mac Installer Distribution cert (.p12, base64) |
| `MACOS_INSTALLER_CERTIFICATE_PASSWORD` | Password for above |
| `MACOS_DEVELOPER_ID_CERTIFICATE` | Developer ID Application cert (.p12, base64) |
| `MACOS_DEVELOPER_ID_CERTIFICATE_PASSWORD` | Password for above |
| `MACOS_PROVISIONING_PROFILE` | Provisioning profile (base64) |
| `MSSTORE_TENANT_ID` | Azure AD Tenant ID |
| `MSSTORE_CLIENT_ID` | Azure AD App Client ID |
| `MSSTORE_CLIENT_SECRET` | Azure AD App Client Secret |

### Store IDs
- **Microsoft Store:** `9NH43R1CMKFN`
- **macOS Bundle ID:** `com.ywesee.eudamed2swissdamed`

## Windows Store Screenshots

```bash
pip install Pillow
python generate_screenshots_windows.py    # outputs to screenshots/windows/
```

Generates 5 screenshots at 3840x2160 (4K) PNG using Python/Pillow, matching the egui light theme. Used for Microsoft Store listing.

## Architecture

Data flow: `EUDAMED API → eudamed_json/{detail,basic}/*.json → Swissdamed DTOs → Swissdamed M2M API`

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point: launches GUI (no args) or CLI subcommands |
| `gui.rs` | Cross-platform GUI (egui/eframe): SRN input, credentials, download & push with progress log |
| `eudamed_api.rs` | EUDAMED API client: paginated listing, parallel detail/basic UDI-DI download with resume |
| `api_detail.rs` | EUDAMED detail API response types (serde deserialization) |
| `api_json.rs` | EUDAMED listing API response types |
| `swissdamed.rs` | Swissdamed DTOs (MdrDto, SppDto, IvdrDto) and EUDAMED→Swissdamed mapper |
| `swissdamed_api.rs` | Swissdamed API client: OAuth2, submit, status check, market status |
| `version_db.rs` | SQLite version tracking (SHA256 change detection, push audit log) |

Endpoint routing: `legislation_endpoint()` in `swissdamed.rs` determines which Swissdamed API endpoint to use based on SPP detection (multi_component criterion) and regulatory act (MDR/IVDR/MDD/AIMDD/IVDD).

## Relationship to eudamed2firstbase

Sibling project at `/home/zeno/.software/eudamed2firstbase` does the full pipeline including GS1 firstbase transformation. This project only handles EUDAMED download → Swissdamed push (no GS1/GDSN).

## Key domain concepts

- **UDI-DI:** Unique Device Identification - Device Identifier (per-packaging level)
- **Basic UDI-DI:** Device-level identifier grouping all UDI-DIs for a device model
- **SRN:** Single Registration Number (identifies manufacturers/authorized reps)
- **MDR/IVDR:** EU Medical Device Regulation / In Vitro Diagnostic Regulation
- **SPP:** System/Procedure Pack (criterion="SPP" in multi_component)
- **Risk classes:** MDR: CLASS_I/IIA/IIB/III; IVDR: CLASS_A/B/C/D

## EUDAMED API endpoints

- **Listing:** `GET /devices/udiDiData?page=N&pageSize=300[&srn=SRN]&languageIso2Code=en`
- **Detail:** `GET /devices/udiDiData/{uuid}?languageIso2Code=en`
- **Basic UDI-DI:** `GET /devices/basicUdiData/udiDiData/{uuid}?languageIso2Code=en`

## Swissdamed M2M API endpoints

- **Auth:** `POST {base}/oauth2/token` (client credentials)
- **Submit:** `POST {base}/v1/m2m/udi/data/{mdr|spp|ivdr|mdd|aimdd|ivdd}`
- **Status:** `POST {base}/v1/m2m/udi/data/udi-di-request-status`
- **Market status:** `POST {base}/v1/m2m/udi/data/market-status`
