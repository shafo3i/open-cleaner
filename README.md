# Open Cleaner (`open-cleaner`)

> Open-source Windows system cleaner built natively in **Rust** — cache & privacy purging, guarded registry verification, complete application uninstallation, and risk-tiered bloatware removal — with a frameless, hard-edged egui interface.

Open Cleaner ships as a single portable executable with **no runtime dependencies**. Fonts are embedded at compile time, and all cleaning runs against verified Windows paths with its own secure-deletion and permission-recovery engine. No telemetry, no accounts.

---

## Highlights

A native GUI with six modules in the sidebar:

| Module | What it does |
|---|---|
| **Cleaner** | Disk, browser, app, developer, GPU and privacy caches — audited with live reclaim metrics |
| **Registry** | Guarded integrity verification of App Paths, MUI cache, startup entries and more |
| **Uninstaller** | Complete removal of Win32 + AppX/MSIX apps with deep remnant purging |
| **Utilities** | Software uninstaller, startup optimizer, and a `winget`-based updater |
| **Bloatware** | Preinstalled apps, telemetry/advertising policies, and optional features — each labeled **Safe / Careful / Expert** with explicit opt-in and a consent flow |
| **Audit Logs** | Session events and a running history of every debloat pass |

Other key capabilities:

- **Secure deletion** — POSIX unlink fallback for shared-open files, read-only attribute recovery, lock/access-denied handling, and optional `0x00` zero-fill wipe (`--secure`).
- **Background workers** — scanning, purging, uninstalling and debloating run off the UI thread with live progress and audit reporting.
- **Self-updater** — checks GitHub Releases on launch and can download + replace the running binary, then restart.
- **CLI mode** — opt-in via the `cli` feature for scripting/automation.

---

## Build

Prerequisites: stable Rust (edition 2024) on Windows.

```powershell
cargo build --release
# → target\release\open-cleaner.exe
```

Run the GUI:

```powershell
.\target\release\open-cleaner.exe
```

The release build is a **windows-subsystem** binary, so no console window appears. The terminal/CLI mode is available behind the `cli` feature:

```powershell
cargo run --features cli -- --help
```

---

## Usage

### GUI

Launch `open-cleaner.exe` and switch modules from the left sidebar. Use the header controls to toggle **GRID / TABLE** views, light/dark theme, and refresh. The footer `GITHUB ↗` link opens this repository.

### CLI

```powershell
# Safe preview scan (audits without deleting)
open-cleaner.exe --scan

# Standard purge
open-cleaner.exe --clean

# Best-effort zero-fill purge
open-cleaner.exe --secure

# Target a specific module
open-cleaner.exe --module filesystem --clean
open-cleaner.exe --module browser --clean
open-cleaner.exe --module registry --scan

# Interactive terminal menu
open-cleaner.exe --cli

# Verbose output (shows touched paths)
open-cleaner.exe --scan --verbose
```

> The CLI requires building with `--features cli`; the default GUI build hides the console.

---

## Administrator rights

Most cleanup works without elevation. The following **require** running as administrator (the app shows a privilege notice when it is not elevated):

- Windows **restore point** creation before a debloat pass
- **De-provisioning** preinstalled Appx packages (so they don't return for new users)
- **Optional feature** removal via DISM (`Remove-Capability`)
- **Machine-wide** policies (e.g. telemetry level)

---

## Bloatware risk model

Every bloat item is classified and individually opt-in — nothing is auto-selected:

- **Safe** — removable & re-installable; no system function is affected.
- **Careful** — removes a first-party feature; reversible from the Microsoft Store.
- **Expert** — touches system servicing/policies; may affect Windows Update and is not guaranteed reversible.

A confirmation dialog summarizes the selected items by tier, warns that removal can break Windows features, and offers a restore point before continuing.

---

## Packaging & releases

`cargo-dist` is configured in `dist-workspace.toml`:

```powershell
cargo install cargo-dist
cargo dist init      # generate the GitHub Actions release workflow
cargo dist build     # build the exe + MSI/NSIS installers locally
```

Pushing a `v*` tag (e.g. `v0.2.0`) triggers a release build on GitHub Actions and publishes the binary + installers to GitHub Releases.

---

## Updates

On startup, the app polls the latest GitHub Release for this repository and compares it with the running version. If a newer release exists, a toast appears with **DISMISS** / **UPDATE NOW**. `UPDATE NOW` downloads the release archive, replaces the running executable (`self_update`), and restarts the app.

The update source is configured in `src/updater.rs` (`REPO_OWNER` / `REPO_NAME`).

---

## Project layout

```
open-cleaner/
├── Cargo.toml
├── dist-workspace.toml      # cargo-dist release config
├── assets/
│   └── fonts/               # Archivo, GeistMono, Inter (embedded)
└── src/
    ├── main.rs              # entry point + CLI dispatcher
    ├── core/                # CleanMode, secure deletion engine, terminal UI
    ├── modules/             # cleaner modules:
    │   ├── filesystem.rs    #   file system purifier
    │   ├── browser.rs       #   browser privacy eraser
    │   ├── registry.rs      #   registry analyzer & verifier
    │   ├── apps.rs          #   desktop/UWP app caches
    │   ├── devtools.rs      #   developer & AI framework caches
    │   ├── graphics.rs      #   GPU shader & 3D caches
    │   ├── privacy.rs       #   user activity & shell traces
    │   └── rules.rs         #   safe-rule catalog
    ├── uninstaller/         # app discovery, native uninstallers, remnant purging
    ├── bloat/               # bloatware model, discovery, and executor
    ├── updater.rs           # GitHub Releases check + self-update
    └── ui/                  # egui application (app, sidebar, header, views, theme, widgets)
```

---

## Safety

- **Symlink/junction protection** — traversal never follows links out of the target boundary.
- **Verified paths** — only whitelisted locations are touched; registry entries are verified against the filesystem before removal.
- **No data resurrection guarantee** — zero-fill is best-effort and is not a secure-erasure guarantee on SSDs, copy-on-write filesystems, or cloud-synced storage.
- **Use at your own risk** — debloating can break Windows Update or features; create a restore point first.

