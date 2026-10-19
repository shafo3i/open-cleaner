use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::CleanerModule;
use crate::core::cleaner::{CleanResult, clean_directory_contents, secure_delete_file};
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct BrowserPrivacyEraser;

impl BrowserPrivacyEraser {
    pub fn new() -> Self {
        Self
    }

    fn get_env_path(var_name: &str) -> Option<PathBuf> {
        env::var_os(var_name).map(PathBuf::from)
    }

    fn chromium_profiles(profile_root: &Path) -> Vec<PathBuf> {
        if !profile_root.exists() {
            return Vec::new();
        }
        let mut profiles = match fs::read_dir(profile_root) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                name == "Default"
                                    || name == "Guest Profile"
                                    || name == "System Profile"
                                    || name.starts_with("Profile ")
                            })
                            .unwrap_or(false)
                })
                .collect::<Vec<_>>(),
            Err(_) => vec![profile_root.join("Default")],
        };

        // Support flat profile roots (e.g., older Opera / Opera GX layouts)
        if profiles.is_empty()
            && (profile_root.join("Cache").exists()
                || profile_root.join("Network").exists()
                || profile_root.join("History").exists())
        {
            profiles.push(profile_root.to_path_buf());
        }
        profiles
    }

    pub fn clean_chromium_caches(
        &self,
        profile_root: &Path,
        browser_name: &str,
        mode: CleanMode,
        report: &mut ModuleReport,
    ) {
        if !profile_root.exists() {
            return;
        }

        // Root-level shared shader caches inside User Data / Opera Stable
        for root_cache in [
            "GrShaderCache",
            "ShaderCache",
            "GraphiteDawnCache",
            "DawnGraphiteCache",
            "DawnWebGPUCache",
        ] {
            let target = profile_root.join(root_cache);
            if target.exists() {
                clean_directory_contents(
                    &target,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    &format!("{browser_name} - {root_cache}"),
                );
            }
        }

        for profile in Self::chromium_profiles(profile_root) {
            let profile_name = profile
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Default");
            let desc_prefix = format!("{browser_name} ({profile_name})");
            let cache_dirs = [
                "Cache",
                "System Cache",
                "Code Cache",
                "GPUCache",
                "DawnGraphiteCache",
                "DawnWebGPUCache",
                "Media Cache",
                "GrShaderCache",
                "ShaderCache",
                "Service Worker\\CacheStorage",
                "Service Worker\\ScriptCache",
            ];
            for c_dir in cache_dirs {
                let target = profile.join(c_dir);
                if target.exists() {
                    clean_directory_contents(
                        &target,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        &format!("{desc_prefix} - {c_dir}"),
                    );
                }
            }
        }
    }

    pub fn clean_chromium_cookies(
        &self,
        profile_root: &Path,
        browser_name: &str,
        mode: CleanMode,
        report: &mut ModuleReport,
    ) {
        for profile in Self::chromium_profiles(profile_root) {
            let profile_name = profile
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Default");
            let desc_prefix = format!("{browser_name} ({profile_name})");
            let cookie_files = [
                profile.join("Network").join("Cookies"),
                profile.join("Network").join("Cookies-journal"),
                profile.join("Network").join("Trust Tokens"),
                profile.join("Network").join("Trust Tokens-journal"),
                profile.join("Cookies"),
                profile.join("Cookies-journal"),
            ];
            for f in cookie_files {
                if f.exists() {
                    match secure_delete_file(&f, mode) {
                        CleanResult::Cleaned(s) => {
                            report.record_cleaned(f, s, format!("{desc_prefix} - Cookies"))
                        }
                        CleanResult::DryRunFound(s) => {
                            report.record_scanned(f, s, format!("{desc_prefix} - Cookies"))
                        }
                        CleanResult::SkippedLocked(s) => report.record_skipped_locked(
                            f,
                            s,
                            format!("{desc_prefix} - Cookies (In Use)"),
                        ),
                        CleanResult::SkippedPermission(s) => report.record_skipped_permission(
                            f,
                            s,
                            format!("{desc_prefix} - Cookies"),
                        ),
                        CleanResult::Failed(s, e) => {
                            report.record_failed(f, s, e, format!("{desc_prefix} - Cookies"))
                        }
                    }
                }
            }
        }
    }

    pub fn clean_chromium_history(
        &self,
        profile_root: &Path,
        browser_name: &str,
        mode: CleanMode,
        report: &mut ModuleReport,
    ) {
        for profile in Self::chromium_profiles(profile_root) {
            let profile_name = profile
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Default");
            let desc_prefix = format!("{browser_name} ({profile_name})");
            let history_files = [
                profile.join("History"),
                profile.join("History-journal"),
                profile.join("Visited Links"),
                profile.join("Top Sites"),
                profile.join("Top Sites-journal"),
                profile.join("Shortcuts"),
                profile.join("Shortcuts-journal"),
                profile.join("History Provider Cache"),
                profile.join("Network Action Predictor"),
                profile.join("Network Action Predictor-journal"),
            ];
            for f in history_files {
                if f.exists() {
                    match secure_delete_file(&f, mode) {
                        CleanResult::Cleaned(s) => {
                            report.record_cleaned(f, s, format!("{desc_prefix} - History"))
                        }
                        CleanResult::DryRunFound(s) => {
                            report.record_scanned(f, s, format!("{desc_prefix} - History"))
                        }
                        CleanResult::SkippedLocked(s) => report.record_skipped_locked(
                            f,
                            s,
                            format!("{desc_prefix} - History (In Use)"),
                        ),
                        CleanResult::SkippedPermission(s) => report.record_skipped_permission(
                            f,
                            s,
                            format!("{desc_prefix} - History"),
                        ),
                        CleanResult::Failed(s, e) => {
                            report.record_failed(f, s, e, format!("{desc_prefix} - History"))
                        }
                    }
                }
            }
        }
    }

    fn opera_roots() -> Vec<(PathBuf, &'static str)> {
        let mut roots = Vec::new();
        for env_key in ["LOCALAPPDATA", "APPDATA"] {
            if let Some(base) = Self::get_env_path(env_key) {
                let op_base = base.join("Opera Software");
                let stable = op_base.join("Opera Stable");
                if stable.exists() {
                    roots.push((stable, "Opera Stable"));
                }
                let gx = op_base.join("Opera GX Stable");
                if gx.exists() {
                    roots.push((gx, "Opera GX"));
                }
            }
        }
        roots
    }

    pub fn clean_opera_caches(&self, mode: CleanMode, report: &mut ModuleReport) {
        for (root, label) in Self::opera_roots() {
            self.clean_chromium_caches(&root, label, mode, report);
        }
    }

    pub fn clean_opera_cookies(&self, mode: CleanMode, report: &mut ModuleReport) {
        for (root, label) in Self::opera_roots() {
            self.clean_chromium_cookies(&root, label, mode, report);
        }
    }

    pub fn clean_opera_history(&self, mode: CleanMode, report: &mut ModuleReport) {
        for (root, label) in Self::opera_roots() {
            self.clean_chromium_history(&root, label, mode, report);
        }
    }

    pub fn clean_firefox_caches(&self, mode: CleanMode, report: &mut ModuleReport) {
        let local_appdata = match Self::get_env_path("LOCALAPPDATA") {
            Some(p) => p,
            None => return,
        };
        let local_profiles = local_appdata
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles");
        if local_profiles.exists()
            && let Ok(entries) = fs::read_dir(&local_profiles)
        {
            for entry in entries.flatten() {
                let profile_dir = entry.path();
                if profile_dir.is_dir() {
                    let profile_name = profile_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Profile");
                    let cache2 = profile_dir.join("cache2");
                    clean_directory_contents(
                        &cache2,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        &format!("Firefox ({profile_name}) - Cache2"),
                    );

                    let startup_cache = profile_dir.join("startupCache");
                    clean_directory_contents(
                        &startup_cache,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        &format!("Firefox ({profile_name}) - Startup Cache"),
                    );
                }
            }
        }
    }

    pub fn clean_firefox_cookies(&self, mode: CleanMode, report: &mut ModuleReport) {
        let roaming_appdata = match Self::get_env_path("APPDATA") {
            Some(p) => p,
            None => return,
        };
        let roaming_profiles = roaming_appdata
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles");
        if roaming_profiles.exists()
            && let Ok(entries) = fs::read_dir(&roaming_profiles)
        {
            for entry in entries.flatten() {
                let profile_dir = entry.path();
                if profile_dir.is_dir() {
                    let profile_name = profile_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Profile");
                    let desc_prefix = format!("Firefox ({profile_name})");

                    for f in ["cookies.sqlite", "cookies.sqlite-wal", "cookies.sqlite-shm"] {
                        let target = profile_dir.join(f);
                        if target.exists() {
                            match secure_delete_file(&target, mode) {
                                CleanResult::Cleaned(s) => report.record_cleaned(
                                    target,
                                    s,
                                    format!("{desc_prefix} - Cookies DB"),
                                ),
                                CleanResult::DryRunFound(s) => report.record_scanned(
                                    target,
                                    s,
                                    format!("{desc_prefix} - Cookies DB"),
                                ),
                                CleanResult::SkippedLocked(s) => report.record_skipped_locked(
                                    target,
                                    s,
                                    format!("{desc_prefix} - Cookies DB (In Use)"),
                                ),
                                CleanResult::SkippedPermission(s) => report
                                    .record_skipped_permission(
                                        target,
                                        s,
                                        format!("{desc_prefix} - Cookies DB"),
                                    ),
                                CleanResult::Failed(s, e) => report.record_failed(
                                    target,
                                    s,
                                    e,
                                    format!("{desc_prefix} - Cookies DB"),
                                ),
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn clean_firefox_history_and_sessions(&self, mode: CleanMode, report: &mut ModuleReport) {
        let Some(roaming_appdata) = Self::get_env_path("APPDATA") else {
            return;
        };
        let profiles = roaming_appdata
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles");
        let Ok(entries) = fs::read_dir(profiles) else {
            return;
        };

        for entry in entries.flatten() {
            let profile = entry.path();
            if !profile.is_dir() {
                continue;
            }
            let profile_name = profile
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Profile");
            for name in [
                "places.sqlite",
                "places.sqlite-wal",
                "places.sqlite-shm",
                "favicons.sqlite",
                "favicons.sqlite-wal",
                "favicons.sqlite-shm",
            ] {
                let target = profile.join(name);
                if target.exists() {
                    Self::delete_browser_file(
                        target,
                        mode,
                        report,
                        &format!("Firefox ({profile_name}) - History"),
                    );
                }
            }
            let sessions = profile.join("sessionstore-backups");
            clean_directory_contents(
                &sessions,
                true,
                true,
                &|_| true,
                mode,
                report,
                &format!("Firefox ({profile_name}) - Sessions"),
            );
        }
    }

    fn delete_browser_file(
        path: PathBuf,
        mode: CleanMode,
        report: &mut ModuleReport,
        description: &str,
    ) {
        match secure_delete_file(&path, mode) {
            CleanResult::Cleaned(size) => report.record_cleaned(path, size, description),
            CleanResult::DryRunFound(size) => report.record_scanned(path, size, description),
            CleanResult::SkippedLocked(size) => {
                report.record_skipped_locked(path, size, format!("{description} (In Use)"))
            }
            CleanResult::SkippedPermission(size) => {
                report.record_skipped_permission(path, size, description)
            }
            CleanResult::Failed(size, error) => {
                report.record_failed(path, size, error, description)
            }
        }
    }

    pub fn clean_target(
        &self,
        target_id: &str,
        mode: CleanMode,
        report: &mut ModuleReport,
    ) -> bool {
        let local_appdata = match Self::get_env_path("LOCALAPPDATA") {
            Some(p) => p,
            None => return false,
        };
        let chrome_path = local_appdata
            .join("Google")
            .join("Chrome")
            .join("User Data");
        let edge_path = local_appdata
            .join("Microsoft")
            .join("Edge")
            .join("User Data");
        let brave_path = local_appdata
            .join("BraveSoftware")
            .join("Brave-Browser")
            .join("User Data");
        let vivaldi_path = local_appdata.join("Vivaldi").join("User Data");

        match target_id {
            "br_chrome_cache" => {
                if chrome_path.exists() {
                    self.clean_chromium_caches(&chrome_path, "Google Chrome", mode, report);
                }
                true
            }
            "br_chrome_cookies" => {
                if chrome_path.exists() {
                    self.clean_chromium_cookies(&chrome_path, "Google Chrome", mode, report);
                }
                true
            }
            "br_chrome_history" => {
                if chrome_path.exists() {
                    self.clean_chromium_history(&chrome_path, "Google Chrome", mode, report);
                }
                true
            }
            "br_edge_cache" => {
                if edge_path.exists() {
                    self.clean_chromium_caches(&edge_path, "Microsoft Edge", mode, report);
                }
                true
            }
            "br_edge_cookies" => {
                if edge_path.exists() {
                    self.clean_chromium_cookies(&edge_path, "Microsoft Edge", mode, report);
                }
                true
            }
            "br_edge_history" => {
                if edge_path.exists() {
                    self.clean_chromium_history(&edge_path, "Microsoft Edge", mode, report);
                }
                true
            }
            "br_firefox_cache" => {
                self.clean_firefox_caches(mode, report);
                true
            }
            "br_firefox_cookies" => {
                self.clean_firefox_cookies(mode, report);
                true
            }
            "br_firefox_history" => {
                self.clean_firefox_history_and_sessions(mode, report);
                true
            }
            "br_opera_cache" => {
                self.clean_opera_caches(mode, report);
                true
            }
            "br_opera_cookies" => {
                self.clean_opera_cookies(mode, report);
                true
            }
            "br_opera_history" => {
                self.clean_opera_history(mode, report);
                true
            }
            "br_brave_cache" => {
                if brave_path.exists() {
                    self.clean_chromium_caches(&brave_path, "Brave Browser", mode, report);
                }
                true
            }
            "br_brave_cookies" => {
                if brave_path.exists() {
                    self.clean_chromium_cookies(&brave_path, "Brave Browser", mode, report);
                }
                true
            }
            "br_brave_history" => {
                if brave_path.exists() {
                    self.clean_chromium_history(&brave_path, "Brave Browser", mode, report);
                }
                true
            }
            "br_vivaldi_cache" => {
                if vivaldi_path.exists() {
                    self.clean_chromium_caches(&vivaldi_path, "Vivaldi", mode, report);
                }
                true
            }
            "br_vivaldi_cookies" => {
                if vivaldi_path.exists() {
                    self.clean_chromium_cookies(&vivaldi_path, "Vivaldi", mode, report);
                }
                true
            }
            "br_vivaldi_history" => {
                if vivaldi_path.exists() {
                    self.clean_chromium_history(&vivaldi_path, "Vivaldi", mode, report);
                }
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for BrowserPrivacyEraser {
    fn id(&self) -> u32 {
        2
    }

    fn name(&self) -> &'static str {
        "Browser Privacy Eraser"
    }

    fn description(&self) -> &'static str {
        "Purges database cookies, disk caches, and session history for Chrome, Edge, Firefox, Opera, Brave, and Vivaldi"
    }

    fn targets_summary(&self) -> &'static str {
        "Chrome, Edge, Firefox, Opera/GX, Brave, Vivaldi (Cookies, Cache, Code Cache, Shaders, History, Sessions)"
    }

    fn safety_level(&self) -> &'static str {
        "HIGH // LOCK-SAFE PARALLEL PURGE"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let local_appdata = match Self::get_env_path("LOCALAPPDATA") {
            Some(p) => p,
            None => {
                report.duration_ms = start_time.elapsed().as_millis();
                return report;
            }
        };

        let chrome_path = local_appdata
            .join("Google")
            .join("Chrome")
            .join("User Data");
        let edge_path = local_appdata
            .join("Microsoft")
            .join("Edge")
            .join("User Data");
        let brave_path = local_appdata
            .join("BraveSoftware")
            .join("Brave-Browser")
            .join("User Data");
        let vivaldi_path = local_appdata.join("Vivaldi").join("User Data");

        render_progress("Auditing Google Chrome", 1, 6);
        if chrome_path.exists() {
            self.clean_chromium_caches(&chrome_path, "Google Chrome", mode, &mut report);
            self.clean_chromium_cookies(&chrome_path, "Google Chrome", mode, &mut report);
            self.clean_chromium_history(&chrome_path, "Google Chrome", mode, &mut report);
        }

        render_progress("Auditing Microsoft Edge", 2, 6);
        if edge_path.exists() {
            self.clean_chromium_caches(&edge_path, "Microsoft Edge", mode, &mut report);
            self.clean_chromium_cookies(&edge_path, "Microsoft Edge", mode, &mut report);
            self.clean_chromium_history(&edge_path, "Microsoft Edge", mode, &mut report);
        }

        render_progress("Auditing Mozilla Firefox", 3, 6);
        self.clean_firefox_caches(mode, &mut report);
        self.clean_firefox_cookies(mode, &mut report);
        self.clean_firefox_history_and_sessions(mode, &mut report);

        render_progress("Auditing Opera & Opera GX", 4, 6);
        self.clean_opera_caches(mode, &mut report);
        self.clean_opera_cookies(mode, &mut report);
        self.clean_opera_history(mode, &mut report);

        render_progress("Auditing Brave Browser", 5, 6);
        if brave_path.exists() {
            self.clean_chromium_caches(&brave_path, "Brave Browser", mode, &mut report);
            self.clean_chromium_cookies(&brave_path, "Brave Browser", mode, &mut report);
            self.clean_chromium_history(&brave_path, "Brave Browser", mode, &mut report);
        }

        render_progress("Auditing Vivaldi Browser", 6, 6);
        if vivaldi_path.exists() {
            self.clean_chromium_caches(&vivaldi_path, "Vivaldi", mode, &mut report);
            self.clean_chromium_cookies(&vivaldi_path, "Vivaldi", mode, &mut report);
            self.clean_chromium_history(&vivaldi_path, "Vivaldi", mode, &mut report);
        }

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
