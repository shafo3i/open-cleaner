use std::path::{Path, PathBuf};
use std::time::Instant;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use super::CleanerModule;
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct RegistryAnalyzer {
    root_prefix: Option<String>,
}

impl RegistryAnalyzer {
    pub fn new() -> Self {
        Self { root_prefix: None }
    }

    #[allow(dead_code)]
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            root_prefix: Some(prefix.into()),
        }
    }

    fn key_path(&self, rel: &str) -> String {
        match &self.root_prefix {
            Some(prefix) => format!(r"{prefix}\{rel}"),
            None => rel.to_string(),
        }
    }

    /// Extracts clean executable path from a registry command string (stripping quotes and args)
    fn extract_file_path(raw: &str) -> Option<PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Handle quoted paths: "C:\Program Files\App\app.exe" --arg
        if let Some(stripped) = trimmed.strip_prefix('"')
            && let Some(end_quote) = stripped.find('"')
        {
            let candidate = &stripped[..end_quote];
            let path = Path::new(candidate);
            if path.is_absolute() {
                return Some(path.to_path_buf());
            }
        }

        // Handle unquoted paths: split by space and search for existing executable prefix
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let mut accumulated = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                accumulated.push(' ');
            }
            accumulated.push_str(part);
            let path = Path::new(&accumulated);
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| {
                    ext.eq_ignore_ascii_case("exe")
                        || ext.eq_ignore_ascii_case("dll")
                        || ext.eq_ignore_ascii_case("bat")
                        || ext.eq_ignore_ascii_case("cmd")
                })
                .unwrap_or(false)
            {
                return Some(path.to_path_buf());
            }
        }

        let first = parts.first().copied().unwrap_or(trimmed);
        let path = Path::new(first);
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }

    /// Checks whether a local drive letter exists before flagging a path as orphaned
    /// (prevents false positives for disconnected USB/network drives)
    fn is_fixed_drive_present(path: &Path) -> bool {
        if let Some(path_str) = path.to_str() {
            let bytes = path_str.as_bytes();
            if bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
                let root = &path_str[..3];
                return Path::new(root).exists();
            }
        }
        false
    }

    /// 1. Audits App Paths in HKCU for uninstalled executables
    pub fn audit_app_paths(&self, mode: CleanMode, report: &mut ModuleReport) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let app_paths_subkey =
            self.key_path(r"Software\Microsoft\Windows\CurrentVersion\App Paths");
        let app_paths_key = match hkcu.open_subkey_with_flags(
            &app_paths_subkey,
            KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
        ) {
            Ok(k) => k,
            Err(_) => return,
        };

        let subkeys: Vec<String> = app_paths_key.enum_keys().filter_map(|k| k.ok()).collect();

        for subkey_name in subkeys {
            let default_val = app_paths_key
                .open_subkey(&subkey_name)
                .ok()
                .and_then(|sub| sub.get_value::<String, _>("").ok());

            if let Some(default_val) = default_val
                && let Some(exe_path) = Self::extract_file_path(&default_val)
                && Self::is_fixed_drive_present(&exe_path)
                && !exe_path.exists()
            {
                let desc = format!(
                    "Broken App Path: {subkey_name} -> Missing: {}",
                    exe_path.display()
                );
                if mode.is_dry_run() {
                    report.record_scanned(PathBuf::from(&default_val), 1, desc);
                } else {
                    match app_paths_key.delete_subkey_all(&subkey_name) {
                        Ok(()) => report.record_cleaned(PathBuf::from(&default_val), 1, desc),
                        Err(error) => report.record_failed(
                            PathBuf::from(&default_val),
                            1,
                            error.to_string(),
                            desc,
                        ),
                    }
                }
            }
        }
    }

    /// 2. Audits MuiCache entries in HKCU for deleted binaries
    pub fn audit_mui_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let mui_path = self
            .key_path(r"Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache");
        let mui_key = match hkcu.open_subkey_with_flags(
            &mui_path,
            KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
        ) {
            Ok(k) => k,
            Err(_) => return,
        };

        let values: Vec<(String, String)> = mui_key
            .enum_values()
            .filter_map(|v| v.ok())
            .filter_map(|(name, _val)| {
                let raw_name = name.clone();
                let clean_name = if let Some(idx) = raw_name.find(".ApplicationCompany") {
                    &raw_name[..idx]
                } else if let Some(idx) = raw_name.find(".FriendlyAppName") {
                    &raw_name[..idx]
                } else {
                    &raw_name
                };

                let lower = clean_name.to_ascii_lowercase();
                if lower.ends_with(".exe") || lower.ends_with(".dll") {
                    Some((name, clean_name.to_string()))
                } else {
                    None
                }
            })
            .collect();

        for (val_name, exe_str) in values {
            let path = Path::new(&exe_str);
            if path.is_absolute() && Self::is_fixed_drive_present(path) && !path.exists() {
                let desc = format!("Orphaned MUI Cache Entry: {}", path.display());
                if mode.is_dry_run() {
                    report.record_scanned(path.to_path_buf(), 1, desc);
                } else {
                    match mui_key.delete_value(&val_name) {
                        Ok(()) => report.record_cleaned(path.to_path_buf(), 1, desc),
                        Err(error) => {
                            report.record_failed(path.to_path_buf(), 1, error.to_string(), desc)
                        }
                    }
                }
            }
        }
    }

    /// 3. Audits Run & RunOnce Startup keys in HKCU for missing program targets
    pub fn audit_startup_run(&self, mode: CleanMode, report: &mut ModuleReport) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for rel_path in [
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
        ] {
            let run_path = self.key_path(rel_path);
            let Ok(run_key) = hkcu.open_subkey_with_flags(
                &run_path,
                KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
            ) else {
                continue;
            };

            let values: Vec<(String, String)> = run_key
                .enum_values()
                .filter_map(|v| v.ok())
                .map(|(name, val)| (name, val.to_string()))
                .collect();

            for (val_name, cmd) in values {
                if let Some(exe_path) = Self::extract_file_path(&cmd)
                    && Self::is_fixed_drive_present(&exe_path)
                    && !exe_path.exists()
                {
                    let desc = format!(
                        "Broken Startup Item: '{val_name}' -> Missing: {}",
                        exe_path.display()
                    );
                    if mode.is_dry_run() {
                        report.record_scanned(exe_path, 1, desc);
                    } else {
                        match run_key.delete_value(&val_name) {
                            Ok(()) => report.record_cleaned(exe_path, 1, desc),
                            Err(error) => {
                                report.record_failed(exe_path, 1, error.to_string(), desc)
                            }
                        }
                    }
                }
            }
        }
    }

    /// 4. Audits Program Compatibility Assistant (AppCompatFlags Store & Layers) for deleted executables
    pub fn audit_appcompat_store(&self, mode: CleanMode, report: &mut ModuleReport) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for rel_path in [
            r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Compatibility Assistant\Store",
            r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers",
        ] {
            let subkey_path = self.key_path(rel_path);
            let Ok(compat_key) = hkcu.open_subkey_with_flags(
                &subkey_path,
                KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
            ) else {
                continue;
            };

            let val_names: Vec<String> = compat_key
                .enum_values()
                .filter_map(|v| v.ok().map(|(name, _)| name))
                .collect();

            for val_name in val_names {
                let path = Path::new(&val_name);
                if path.is_absolute() && Self::is_fixed_drive_present(path) && !path.exists() {
                    let desc = format!("Orphaned AppCompat Record: {}", path.display());
                    if mode.is_dry_run() {
                        report.record_scanned(path.to_path_buf(), 1, desc);
                    } else {
                        match compat_key.delete_value(&val_name) {
                            Ok(()) => report.record_cleaned(path.to_path_buf(), 1, desc),
                            Err(error) => {
                                report.record_failed(path.to_path_buf(), 1, error.to_string(), desc)
                            }
                        }
                    }
                }
            }
        }
    }

    /// 5. Audits Windows Run Dialog (Win+R RunMRU) & Explorer TypedPaths history
    pub fn audit_run_mru(&self, mode: CleanMode, report: &mut ModuleReport) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for (rel_path, label) in [
            (
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\RunMRU",
                "Explorer RunMRU Command Entry",
            ),
            (
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\TypedPaths",
                "Explorer TypedPaths Address Entry",
            ),
        ] {
            let subkey_path = self.key_path(rel_path);
            let Ok(mru_key) = hkcu.open_subkey_with_flags(
                &subkey_path,
                KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
            ) else {
                continue;
            };

            let val_names: Vec<String> = mru_key
                .enum_values()
                .filter_map(|v| v.ok().map(|(name, _)| name))
                .filter(|name| !name.is_empty())
                .collect();

            for val_name in val_names {
                let key_display = PathBuf::from(format!("HKCU\\{subkey_path}\\{val_name}"));
                if mode.is_dry_run() {
                    report.record_scanned(key_display, 1, label);
                } else {
                    match mru_key.delete_value(&val_name) {
                        Ok(()) => report.record_cleaned(key_display, 1, label),
                        Err(error) => {
                            report.record_failed(key_display, 1, error.to_string(), label)
                        }
                    }
                }
            }
        }
    }

    /// 6. Audits Common Dialog OpenSavePidlMRU & LastVisitedPidlMRU history
    pub fn audit_opensave_mru(&self, mode: CleanMode, report: &mut ModuleReport) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // LastVisitedPidlMRU has direct values
        let last_visited_path = self.key_path(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\ComDlg32\LastVisitedPidlMRU",
        );
        if let Ok(lv_key) = hkcu.open_subkey_with_flags(
            &last_visited_path,
            KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
        ) {
            let val_names: Vec<String> = lv_key
                .enum_values()
                .filter_map(|v| v.ok().map(|(name, _)| name))
                .filter(|name| !name.is_empty())
                .collect();

            for val_name in val_names {
                let key_display = PathBuf::from(format!("HKCU\\{last_visited_path}\\{val_name}"));
                let desc = "ComDlg32 LastVisitedPidlMRU Entry";
                if mode.is_dry_run() {
                    report.record_scanned(key_display, 1, desc);
                } else {
                    match lv_key.delete_value(&val_name) {
                        Ok(()) => report.record_cleaned(key_display, 1, desc),
                        Err(error) => report.record_failed(key_display, 1, error.to_string(), desc),
                    }
                }
            }
        }

        // OpenSavePidlMRU organizes values under extension subkeys (*, pdf, zip, etc.)
        let opensave_path = self.key_path(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\ComDlg32\OpenSavePidlMRU",
        );
        if let Ok(os_key) = hkcu.open_subkey_with_flags(
            &opensave_path,
            KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
        ) {
            let ext_subkeys: Vec<String> = os_key.enum_keys().filter_map(|k| k.ok()).collect();
            for ext in ext_subkeys {
                let Ok(sub) = os_key.open_subkey_with_flags(
                    &ext,
                    KEY_READ | if !mode.is_dry_run() { KEY_WRITE } else { 0 },
                ) else {
                    continue;
                };
                let val_names: Vec<String> = sub
                    .enum_values()
                    .filter_map(|v| v.ok().map(|(name, _)| name))
                    .filter(|name| !name.is_empty())
                    .collect();
                for val_name in val_names {
                    let key_display =
                        PathBuf::from(format!("HKCU\\{opensave_path}\\{ext}\\{val_name}"));
                    let desc = "ComDlg32 OpenSavePidlMRU Entry";
                    if mode.is_dry_run() {
                        report.record_scanned(key_display, 1, desc);
                    } else {
                        match sub.delete_value(&val_name) {
                            Ok(()) => report.record_cleaned(key_display, 1, desc),
                            Err(error) => {
                                report.record_failed(key_display, 1, error.to_string(), desc)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn clean_target(
        &self,
        target_id: &str,
        mode: CleanMode,
        report: &mut ModuleReport,
    ) -> bool {
        match target_id {
            "reg_app_paths" => {
                self.audit_app_paths(mode, report);
                true
            }
            "reg_mui_cache" => {
                self.audit_mui_cache(mode, report);
                true
            }
            "reg_startup" => {
                self.audit_startup_run(mode, report);
                true
            }
            "reg_appcompat_store" => {
                self.audit_appcompat_store(mode, report);
                true
            }
            "reg_run_mru" => {
                self.audit_run_mru(mode, report);
                true
            }
            "reg_opensave_mru" => {
                self.audit_opensave_mru(mode, report);
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for RegistryAnalyzer {
    fn id(&self) -> u32 {
        3
    }

    fn name(&self) -> &'static str {
        "Registry Analyzer & Verifier"
    }

    fn description(&self) -> &'static str {
        "Strict verification of orphaned App Paths, MuiCache, Startup Run, AppCompat Store, and Explorer MRU keys"
    }

    fn targets_summary(&self) -> &'static str {
        "HKCU App Paths, MuiCache, Startup Run, AppCompat Store, RunMRU/TypedPaths, ComDlg32 OpenSavePidlMRU"
    }

    fn safety_level(&self) -> &'static str {
        "GUARDED // STRICT DISK PATH VERIFICATION"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let tasks = [
            "Auditing Application Paths (App Paths)",
            "Auditing Shell MuiCache Entries",
            "Auditing Startup Program Commands (Run / RunOnce)",
            "Auditing Program Compatibility Assistant Store",
            "Auditing Explorer RunMRU & TypedPaths History",
            "Auditing ComDlg32 OpenSave & LastVisited MRU",
        ];

        let total = tasks.len();

        render_progress(tasks[0], 1, total);
        self.audit_app_paths(mode, &mut report);

        render_progress(tasks[1], 2, total);
        self.audit_mui_cache(mode, &mut report);

        render_progress(tasks[2], 3, total);
        self.audit_startup_run(mode, &mut report);

        render_progress(tasks[3], 4, total);
        self.audit_appcompat_store(mode, &mut report);

        render_progress(tasks[4], 5, total);
        self.audit_run_mru(mode, &mut report);

        render_progress(tasks[5], 6, total);
        self.audit_opensave_mru(mode, &mut report);

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
