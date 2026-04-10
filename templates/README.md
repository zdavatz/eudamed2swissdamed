# Release Workflow Template

Reusable template for Rust/egui desktop apps with CI/CD for macOS, Windows, and Linux,
including automatic Microsoft Store publishing.

## Quick Start

1. Copy `release.yml.template` to `.github/workflows/release.yml`
2. Search & replace all `__PLACEHOLDERS__` with your app's values (see table below)
3. Set up GitHub Secrets and Variables (see below)
4. Generate screenshots: `pip install Pillow && python generate_screenshots_windows.py`
5. Tag and push: `git tag v1.0.0 && git push origin v1.0.0`

## Placeholders

| Placeholder | Example | Description |
|-------------|---------|-------------|
| `__APP_NAME__` | `eudamed2swissdamed` | Binary/package name (lowercase, no spaces) |
| `__APP_DISPLAY_NAME__` | `Eudamed2Swissdamed` | Store display title |
| `__BUNDLE_ID__` | `com.ywesee.eudamed2swissdamed` | macOS bundle identifier |
| `__MSSTORE_APP_ID__` | `9NH43R1CMKFN` | Microsoft Store app ID |
| `__CATEGORY_MACOS__` | `public.app-category.business` | macOS app category |
| `__CATEGORY_MSSTORE__` | `BooksAndReference_EReader` | MS Store category |
| `__DESCRIPTION_DE__` | German store description | Single-line string |
| `__DESCRIPTION_EN__` | English store description | Single-line string |
| `__KEYWORDS_DE__` | `"EUDAMED", "UDI"` | Comma-separated quoted strings |
| `__KEYWORDS_EN__` | `"EUDAMED", "UDI"` | Comma-separated quoted strings |
| `__CERT_NOTES__` | Notes for MS Store testers | What the app does, how to test |
| `__GITHUB_URL__` | `https://github.com/zdavatz/...` | Repo URL (privacy policy, website) |
| `__CONTACT_EMAIL__` | `zdavatz@ywesee.com` | Support contact email |
| `__COPYRIGHT__` | `Copyright 2026 ywesee GmbH` | Copyright notice |
| `__LINUX_COMMENT__` | `Medical device data converter` | .desktop file comment |
| `__SCREENSHOT_FILES__` | `"screenshot_1.png", ...` | Comma-separated quoted filenames |

## Required Repo Structure

```
your-app/
  Cargo.toml                          # version auto-synced from git tag
  assets/
    icon.icns                         # macOS icon
    icon.ico                          # Windows icon
    icon_256x256.png                  # Linux/AppImage icon
  windows/
    AppxManifest.xml                  # MSIX manifest (Version="1.0.0.0")
    assets/                           # Store tiles
  screenshots/
    windows/                          # 5x 3840x2160 PNG screenshots
  entitlements.plist                  # macOS DMG entitlements
  entitlements-appstore.plist         # macOS App Store entitlements
  generate_screenshots_windows.py     # Screenshot generator
```

## GitHub Setup

### Secrets (Settings > Secrets and variables > Actions > Secrets)

**Apple (macOS signing + notarization):**
- `APPLE_API_KEY_P8`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID`
- `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PASSWORD`
- `MACOS_INSTALLER_CERTIFICATE`, `MACOS_INSTALLER_CERTIFICATE_PASSWORD`
- `MACOS_DEVELOPER_ID_CERTIFICATE`, `MACOS_DEVELOPER_ID_CERTIFICATE_PASSWORD`
- `MACOS_PROVISIONING_PROFILE`

**Microsoft Store:**
- `MSSTORE_TENANT_ID`, `MSSTORE_CLIENT_ID`, `MSSTORE_CLIENT_SECRET`

**Windows signing (optional):**
- `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`

### Variables (Settings > Secrets and variables > Actions > Variables)

- `MSSTORE_ENABLED` = `true` (enables Microsoft Store publishing job)

## Features

- **Auto version sync:** `Cargo.toml` version is set from git tag before build
- **3-platform build:** macOS (universal binary), Windows (MSIX + ZIP), Linux (tar.gz + AppImage)
- **macOS:** Code signing, notarization, DMG + App Store pkg
- **Microsoft Store:** Automatic submission with screenshots, questionnaire fields, bilingual listings
- **GitHub Release:** Automatic with release notes
