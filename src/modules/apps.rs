use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::CleanerModule;
use crate::core::cleaner::clean_directory_contents;
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct AppCacheCleaner;

impl AppCacheCleaner {
    pub fn new() -> Self {
        Self
    }

    fn get_env_path(var_name: &str) -> Option<PathBuf> {
        env::var_os(var_name).map(PathBuf::from)
    }

    /// Helper to clean standard Chromium/Electron/EBWebView cache directories inside a profile or root
    fn clean_chromium_cache_tree(
        root: &Path,
        mode: CleanMode,
        report: &mut ModuleReport,
        label: &str,
    ) {
        if !root.exists() {
            return;
        }

        // Direct cache directories (used by Electron apps like Discord, Slack, Steam htmlcache, Epic webcache)
        for sub in [
            "Cache",
            "Code Cache",
            "GPUCache",
            "DawnGraphiteCache",
            "DawnWebGPUCache",
            "GrShaderCache",
            "ShaderCache",
            "GraphiteDawnCache",
            "Crashpad\\reports",
            "BrowserMetrics",
            "Service Worker\\CacheStorage",
            "Service Worker\\ScriptCache",
        ] {
            let dir = root.join(sub);
            if dir.exists() {
                clean_directory_contents(&dir, true, true, &|_| true, mode, report, label);
            }
        }

        // Also check `Default` and `WV2Profile_*` subdirectories if this is an EBWebView root
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_dir() {
                    continue;
                }
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.eq_ignore_ascii_case("Default")
                    || name.starts_with("WV2Profile_")
                    || name.starts_with("Profile ")
                {
                    for sub in [
                        "Cache",
                        "Code Cache",
                        "GPUCache",
                        "DawnGraphiteCache",
                        "DawnWebGPUCache",
                        "Service Worker\\CacheStorage",
                        "Service Worker\\ScriptCache",
                    ] {
                        let dir = p.join(sub);
                        if dir.exists() {
                            clean_directory_contents(
                                &dir,
                                true,
                                true,
                                &|_| true,
                                mode,
                                report,
                                label,
                            );
                        }
                    }
                }
            }
        }
    }

    /// 1. Steam Client HTML Cache, HTTP Cache, Logs & Crash Dumps
    pub fn clean_steam_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let steam_htmlcache = local_appdata.join("Steam").join("htmlcache");
            Self::clean_chromium_cache_tree(
                &steam_htmlcache,
                mode,
                report,
                "Steam Client HTML & GPU Cache",
            );
        }

        let prog_x86 = Self::get_env_path("ProgramFiles(x86)")
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
        let steam_root = prog_x86.join("Steam");
        if steam_root.exists() {
            let http_cache = steam_root.join("appcache").join("httpcache");
            if http_cache.exists() {
                clean_directory_contents(
                    &http_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Steam AppCache HTTP Cache",
                );
            }

            let steam_logs = steam_root.join("logs");
            if steam_logs.exists() {
                clean_directory_contents(
                    &steam_logs,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                ext.eq_ignore_ascii_case("txt")
                                    || ext.eq_ignore_ascii_case("log")
                                    || ext.eq_ignore_ascii_case("old")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Steam Diagnostic Logs",
                );
            }

            let steam_dumps = steam_root.join("dumps");
            if steam_dumps.exists() {
                clean_directory_contents(
                    &steam_dumps,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("dmp"))
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Steam Crash Dumps",
                );
            }
        }
    }

    /// 2. Epic Games Launcher Webcache, Logs & Crash Reports
    pub fn clean_epic_games_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let epic_saved = local_appdata.join("EpicGamesLauncher").join("Saved");
            if !epic_saved.exists() {
                return;
            }

            if let Ok(entries) = fs::read_dir(&epic_saved) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if name.to_ascii_lowercase().starts_with("webcache") {
                        Self::clean_chromium_cache_tree(
                            &path,
                            mode,
                            report,
                            "Epic Games Launcher Webcache",
                        );
                    }
                }
            }

            let epic_logs = epic_saved.join("Logs");
            if epic_logs.exists() {
                clean_directory_contents(
                    &epic_logs,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                ext.eq_ignore_ascii_case("log")
                                    || ext.eq_ignore_ascii_case("txt")
                                    || ext.eq_ignore_ascii_case("bak")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Epic Games Launcher Logs",
                );
            }

            let epic_crashes = epic_saved.join("Crashes");
            if epic_crashes.exists() {
                clean_directory_contents(
                    &epic_crashes,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Epic Games Crash Reports",
                );
            }
        }
    }

    /// 3. Zoom Logs & WebView2 Cache
    pub fn clean_zoom_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(appdata) = Self::get_env_path("APPDATA") {
            let zoom_root = appdata.join("Zoom");
            if !zoom_root.exists() {
                return;
            }

            let zoom_logs = zoom_root.join("logs");
            if zoom_logs.exists() {
                clean_directory_contents(
                    &zoom_logs,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Zoom Diagnostic Logs & Crash Dumps",
                );
            }

            let webview_x64 = zoom_root.join("data").join("WebviewCacheX64");
            if webview_x64.exists()
                && let Ok(entries) = fs::read_dir(&webview_x64)
            {
                for entry in entries.flatten() {
                    let account_dir = entry.path();
                    if account_dir.is_dir() {
                        let ebwebview = account_dir.join("EBWebView");
                        if ebwebview.exists() {
                            Self::clean_chromium_cache_tree(
                                &ebwebview,
                                mode,
                                report,
                                "Zoom WebView2 Cache",
                            );
                        }
                    }
                }
            }
        }
    }

    /// 4. Microsoft OneDrive & Office Telemetry / WebService Cache
    pub fn clean_onedrive_office_logs(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let ms_root = local_appdata.join("Microsoft");
            for onedrive_sub in ["OneDrive\\logs", "OneDrive\\setup\\logs"] {
                let dir = ms_root.join(onedrive_sub);
                if dir.exists() {
                    clean_directory_contents(
                        &dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Microsoft OneDrive Diagnostic Logs",
                    );
                }
            }

            for office_sub in [
                "Office\\16.0\\Wef",
                "Office\\16.0\\WebServiceCache\\AllUsers",
            ] {
                let dir = ms_root.join(office_sub);
                if dir.exists() {
                    clean_directory_contents(
                        &dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Microsoft Office Web Extension & Service Cache",
                    );
                }
            }
        }
    }

    /// 5. Discord Desktop Client Cache
    pub fn clean_discord_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(appdata) = Self::get_env_path("APPDATA") {
            for variant in ["discord", "discordptb", "discordcanary"] {
                let discord_dir = appdata.join(variant);
                if discord_dir.exists() {
                    Self::clean_chromium_cache_tree(
                        &discord_dir,
                        mode,
                        report,
                        "Discord Client Cache",
                    );
                }
            }
        }
    }

    /// 6. Microsoft Teams (New & Classic) and Slack Cache
    pub fn clean_teams_slack_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let new_teams_eb = local_appdata
                .join("Packages")
                .join("MSTeams_8wekyb3d8bbwe")
                .join("LocalCache")
                .join("Microsoft")
                .join("MSTeams")
                .join("EBWebView");
            if new_teams_eb.exists() {
                Self::clean_chromium_cache_tree(
                    &new_teams_eb,
                    mode,
                    report,
                    "Microsoft Teams (New) WebView Cache",
                );
            }
        }

        if let Some(appdata) = Self::get_env_path("APPDATA") {
            let classic_teams = appdata.join("Microsoft").join("Teams");
            if classic_teams.exists() {
                Self::clean_chromium_cache_tree(
                    &classic_teams,
                    mode,
                    report,
                    "Microsoft Teams (Classic) Cache",
                );
            }

            let slack_dir = appdata.join("Slack");
            if slack_dir.exists() {
                Self::clean_chromium_cache_tree(&slack_dir, mode, report, "Slack Client Cache");
                let slack_logs = slack_dir.join("logs");
                if slack_logs.exists() {
                    clean_directory_contents(
                        &slack_logs,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Slack Client Logs",
                    );
                }
            }
        }
    }

    /// 7. Spotify Audio Data & Embedded Browser Cache
    pub fn clean_spotify_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let spotify_root = local_appdata.join("Spotify");
            if spotify_root.exists() {
                let data_dir = spotify_root.join("Data");
                if data_dir.exists() {
                    clean_directory_contents(
                        &data_dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Spotify Audio Stream Cache",
                    );
                }
                let browser_dir = spotify_root.join("Browser");
                if browser_dir.exists() {
                    Self::clean_chromium_cache_tree(
                        &browser_dir,
                        mode,
                        report,
                        "Spotify Embedded Browser Cache",
                    );
                }
            }

            let store_spotify = local_appdata
                .join("Packages")
                .join("SpotifyAB.SpotifyMusic_zpdnekdrzrea0")
                .join("LocalCache")
                .join("Spotify");
            if store_spotify.exists() {
                let data_dir = store_spotify.join("Data");
                if data_dir.exists() {
                    clean_directory_contents(
                        &data_dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Spotify (Store) Audio Stream Cache",
                    );
                }
                let browser_dir = store_spotify.join("Browser");
                if browser_dir.exists() {
                    Self::clean_chromium_cache_tree(
                        &browser_dir,
                        mode,
                        report,
                        "Spotify (Store) Browser Cache",
                    );
                }
            }
        }
    }

    /// 8. Windows Widgets (WebExperience) & Microsoft Store Cache
    pub fn clean_windows_widgets_store(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let pkgs = local_appdata.join("Packages");

            let widgets_eb = pkgs
                .join("MicrosoftWindows.Client.WebExperience_cw5n1h2txyewy")
                .join("LocalState")
                .join("EBWebView");
            if widgets_eb.exists() {
                Self::clean_chromium_cache_tree(
                    &widgets_eb,
                    mode,
                    report,
                    "Windows Widgets WebExperience Cache",
                );
            }

            let store_pkg = pkgs.join("Microsoft.WindowsStore_8wekyb3d8bbwe");
            if store_pkg.exists() {
                for sub in ["LocalCache", "TempState"] {
                    let dir = store_pkg.join(sub);
                    if dir.exists() {
                        clean_directory_contents(
                            &dir,
                            true,
                            true,
                            &|_| true,
                            mode,
                            report,
                            "Microsoft Store Cache",
                        );
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
            "app_steam_cache" => {
                self.clean_steam_cache(mode, report);
                true
            }
            "app_epic_games_cache" => {
                self.clean_epic_games_cache(mode, report);
                true
            }
            "app_zoom_cache" => {
                self.clean_zoom_cache(mode, report);
                true
            }
            "app_onedrive_office_logs" => {
                self.clean_onedrive_office_logs(mode, report);
                true
            }
            "app_discord_cache" => {
                self.clean_discord_cache(mode, report);
                true
            }
            "app_teams_slack_cache" => {
                self.clean_teams_slack_cache(mode, report);
                true
            }
            "app_spotify_cache" => {
                self.clean_spotify_cache(mode, report);
                true
            }
            "app_windows_widgets_store" | "rule_windows_widgets_cache" => {
                self.clean_windows_widgets_store(mode, report);
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for AppCacheCleaner {
    fn id(&self) -> u32 {
        5
    }

    fn name(&self) -> &'static str {
        "Desktop Application Cache Purifier"
    }

    fn description(&self) -> &'static str {
        "Purges caches and logs for Steam, Epic Games, Zoom, OneDrive/Office, Discord, Teams/Slack, Spotify, and Windows Widgets/Store"
    }

    fn targets_summary(&self) -> &'static str {
        "Steam, Epic Games, Zoom, OneDrive & Office, Discord, Teams & Slack, Spotify, Windows Widgets & Store"
    }

    fn safety_level(&self) -> &'static str {
        "SAFE // VERIFIED APPLICATION CACHES & LOGS (PRESERVES LOGINS)"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let tasks = [
            "Steam Client HTML, HTTP Cache & Logs",
            "Epic Games Launcher Webcache & Logs",
            "Zoom Diagnostic Logs & WebView2 Cache",
            "OneDrive & Office Telemetry / WebService Cache",
            "Discord Desktop Client Cache",
            "Microsoft Teams & Slack Client Cache",
            "Spotify Audio Stream & Browser Cache",
            "Windows Widgets & Microsoft Store Cache",
        ];
        let total = tasks.len();

        render_progress(tasks[0], 1, total);
        self.clean_steam_cache(mode, &mut report);

        render_progress(tasks[1], 2, total);
        self.clean_epic_games_cache(mode, &mut report);

        render_progress(tasks[2], 3, total);
        self.clean_zoom_cache(mode, &mut report);

        render_progress(tasks[3], 4, total);
        self.clean_onedrive_office_logs(mode, &mut report);

        render_progress(tasks[4], 5, total);
        self.clean_discord_cache(mode, &mut report);

        render_progress(tasks[5], 6, total);
        self.clean_teams_slack_cache(mode, &mut report);

        render_progress(tasks[6], 7, total);
        self.clean_spotify_cache(mode, &mut report);

        render_progress(tasks[7], 8, total);
        self.clean_windows_widgets_store(mode, &mut report);

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
