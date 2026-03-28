# eudamed2swissdamed

Download medical device data from [EUDAMED](https://ec.europa.eu/tools/eudamed/) and push it to the [Swissdamed](https://www.swissdamed.ch/) M2M API.

## Build

```bash
cargo build --release
```

## Usage

```bash
# Download from EUDAMED
eudamed2swissdamed download --10                        # first 10 devices
eudamed2swissdamed download --srn CH-MF-000023141       # all devices for a manufacturer SRN
eudamed2swissdamed download --srn SRN1 SRN2 --50        # multiple SRNs, limit 50 per SRN

# Preview conversion
eudamed2swissdamed convert                              # summary of all downloaded devices
eudamed2swissdamed convert --uuid <UUID>                # Swissdamed JSON for one device

# Push to Swissdamed
eudamed2swissdamed push --dry-run                       # show what would be pushed
eudamed2swissdamed push -v                              # push all devices (verbose)
eudamed2swissdamed push --changed                       # only push new/changed devices

# Check status
eudamed2swissdamed status <correlationId>               # check submission status

# Statistics
eudamed2swissdamed stats                                # version DB statistics
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `SWISSDAMED_CLIENT_ID` | For push/status | OAuth2 client ID |
| `SWISSDAMED_CLIENT_SECRET` | For push/status | OAuth2 client secret |
| `SWISSDAMED_BASE_URL` | No | Override API base URL (default: playground) |

## Data Flow

```
EUDAMED Public API
  GET /devices/udiDiData (listing, paginated)
  GET /devices/udiDiData/{uuid} (detail)
  GET /devices/basicUdiData/udiDiData/{uuid} (Basic UDI-DI)
        |
        v
  eudamed_json/detail/*.json  +  eudamed_json/basic/*.json
        |
        v
  EUDAMED -> Swissdamed DTO mapping (MDR/SPP/IVDR)
        |
        v
  Swissdamed M2M API
  POST /v1/m2m/udi/data/{mdr|spp|ivdr|mdd|aimdd|ivdd}
```

## License

[GPLv3](LICENSE)
