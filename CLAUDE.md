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

## Architecture

Data flow: `EUDAMED API → eudamed_json/{detail,basic}/*.json → Swissdamed DTOs → Swissdamed M2M API`

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry point: download, convert, push, status, stats subcommands |
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
