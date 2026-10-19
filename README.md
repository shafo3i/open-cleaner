# SYSTEM CLEANER (`open-cleaner`) ◈

> High-performance open-source system purifier, browser privacy eraser, and guarded registry integrity analyzer built 100% natively in **Rust** with an integrated, hard-edged minimalist GUI.

---

## ⚡ Architecture Overview

`open-cleaner` constructs its cleaning logic natively without proprietary loops or telemetry. It implements a secure file deletion engine with graceful permission recovery, file-lock handling, and strict disk path verification.

```
open-cleaner/
├── Cargo.toml
├── assets/
│   └── fonts/             # Archivo, GeistMono, and Inter variable font binaries
└── src/
    ├── main.rs            # Application entry point & CLI dispatcher
    ├── core/
    │   ├── mod.rs         # CleanMode, ItemStatus, and Report aggregates
    │   ├── cleaner.rs     # Secure deletion algorithm & permission recovery
    │   └── ui.rs          # Terminal formatting, progress indicators, and report summaries
    ├── modules/
    │   ├── mod.rs         # CleanerModule trait
    │   ├── filesystem.rs  # Module 1: File System Purifier
    │   ├── browser.rs     # Module 2: Browser Privacy Eraser
    │   └── registry.rs    # Module 3: Registry Analyzer & Verifier
    └── ui/
        ├── mod.rs         # GUI initialization & frameless viewport configuration
        ├── app.rs         # SystemCleanerApp state machine & async background workers
        ├── header.rs      # 48px integrated titlebar with window controls and view mode toggles
        ├── toolbar.rs     # 4-category filter cells, live search, and action triggers
        ├── banner.rs      # Audit hero metrics strip & best-effort zero-overwrite checkbox
        ├── table_view.rs  # Technical tabular target view
        ├── grid_view.rs   # Card-based target view
        ├── model.rs       # 18-target clean database with category indices
        ├── theme.rs       # Exact color tokens, typography scales, and 0px corner radii
        └── widgets.rs     # Modular HUD labels, badges, buttons, and drag handles
```

---

## 🛡️ The Three Priority Modules

### 1. File System Purifier (Safest Core)
Targets standard verified cache and temporary paths:
- **User Temporary Files**: `%TEMP%` (`%LOCALAPPDATA%\Temp\*`)
- **Windows System Temp**: `%WINDIR%\Temp\*`
- **System Logs & Traces**: `%WINDIR%\Logs\*.log`, `*.etl`, `%LOCALAPPDATA%\Microsoft\Windows\*.log`
- **Crash Dumps & MiniDumps**: `%LOCALAPPDATA%\CrashDumps\*.dmp`, `%WINDIR%\Minidump\*.dmp`, `%WINDIR%\MEMORY.DMP`
- **Windows Error Reporting (WER)**: User and machine `ReportArchive`, `ReportQueue`, `Temp`
- **Thumbnail Cache**: `%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db`
- **Delivery Optimization Cache**: `%WINDIR%\SoftwareDistribution\DeliveryOptimization\Cache`
- **Recycle Bin**: Scans `$Recycle.Bin` across active drives and unlinks safely.

### 2. Browser Privacy Eraser
Audits and purges cookies, session caches, and navigation history for all major browsers:
- **Chromium Family (Google Chrome, Microsoft Edge, Brave Browser)**:
  - Cache: `Cache`, `Code Cache`, `GPUCache`, `DawnGraphiteCache`, `DawnWebGPUCache`, `Media Cache`
  - Cookies: SQLite databases and network state in `Network\Cookies`, `Cookies`
  - History: `History`, `Visited Links`, `Top Sites`, `Shortcuts`, `History Provider Cache`
  - Sessions: `Sessions\*`, `Session Storage\*`, `Current Session`, `Last Tabs`
- **Mozilla Firefox (Gecko)**:
  - Cache: `cache2\entries\*`, `startupCache\*`
  - Cookies: `cookies.sqlite*`
  - Places History: `places.sqlite*`, `favicons.sqlite*`
  - Session Store: `sessionstore-backups\*`

### 3. Registry Analyzer & Verifier (Guarded / Advanced)
Implements **strict disk path verification** before flagging or modifying any registry key:
- **App Paths**: Inspects `HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths` and verifies whether the registered executable actually exists on disk.
- **MUI Cache**: Inspects `HKCU\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache` and tests whether the referenced application binary is present on the filesystem.
- **Run at Startup**: Inspects `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` and checks whether the target program command path exists.
- **Safety Policy**: Runs in **Read-Only Verification mode** by default (`--scan`). Only modifies confirmed orphaned values when run with explicit clean parameters.

---

## 🔒 Secure Deletion & Permission Recovery Algorithm

1. **Junction / Symlink Protection**: Symlinks are never followed recursively to prevent escaping target boundaries.
2. **Read-Only Attribute Recovery**: On Windows, files with the `readonly` attribute have their permissions modified dynamically before unlinking.
3. **Graceful Sharing Violation Handling**: Files locked by running processes (`ERROR_SHARING_VIOLATION` / Error 32) are caught, flagged as `SkippedLocked`, and recorded without halting execution.
4. **Access Denied Recovery**: Permission errors (`ERROR_ACCESS_DENIED` / Error 5) are logged and bypassed cleanly.
5. **Best-effort Zero-Fill (`--secure`)**: The file is overwritten with `0x00` in 64 KB chunks, flushed with `sync_all()`, truncated to 0 bytes, and then unlinked. This is not a secure-erasure guarantee on SSDs, copy-on-write filesystems, or cloud-synced storage.

---

## 🚀 Usage

### Native Graphical Interface (Default)
Run without arguments or pass `--gui` to launch the native frameless interface:
```powershell
.\target\release\open-cleaner.exe
```

### CLI Command Options
```powershell
# Safe Preview Scan (Audits all modules without unlinking)
.\target\release\open-cleaner.exe --scan

# Standard Purge of all modules
.\target\release\open-cleaner.exe --clean

# Best-effort zero-overwrite purge
.\target\release\open-cleaner.exe --secure

# Target a specific module
.\target\release\open-cleaner.exe --module filesystem --clean
.\target\release\open-cleaner.exe --module browser --clean
.\target\release\open-cleaner.exe --module registry --scan

# Interactive Terminal Menu
.\target\release\open-cleaner.exe --cli

# Verbose Mode (Shows touched file paths and reasons)
.\target\release\open-cleaner.exe --scan --verbose
```
