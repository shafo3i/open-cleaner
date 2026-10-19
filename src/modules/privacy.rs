use std::env;
use std::path::PathBuf;
use std::time::Instant;

use super::CleanerModule;
use crate::core::cleaner::clean_directory_contents;
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct PrivacyCleaner;

impl PrivacyCleaner {
    pub fn new() -> Self {
        Self
    }

    fn get_env_path(var_name: &str) -> Option<PathBuf> {
        env::var_os(var_name).map(PathBuf::from)
    }

    /// 1. Recent Documents & Taskbar Jump Lists
    pub fn clean_recent_docs(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(appdata) = Self::get_env_path("APPDATA") {
            let recent_dir = appdata.join("Microsoft").join("Windows").join("Recent");
            if recent_dir.exists() {
                clean_directory_contents(
                    &recent_dir,
                    false,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("lnk"))
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Recent Document Shortcuts (*.lnk)",
                );

                let auto_dest = recent_dir.join("AutomaticDestinations");
                if auto_dest.exists() {
                    clean_directory_contents(
                        &auto_dest,
                        false,
                        true,
                        &|p| {
                            p.extension()
                                .and_then(|e| e.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("automaticdestinations-ms"))
                                .unwrap_or(false)
                        },
                        mode,
                        report,
                        "Taskbar Jump Lists (AutomaticDestinations)",
                    );
                }

                let custom_dest = recent_dir.join("CustomDestinations");
                if custom_dest.exists() {
                    clean_directory_contents(
                        &custom_dest,
                        false,
                        true,
                        &|p| {
                            p.extension()
                                .and_then(|e| e.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("customdestinations-ms"))
                                .unwrap_or(false)
                        },
                        mode,
                        report,
                        "Taskbar Jump Lists (CustomDestinations)",
                    );
                }
            }
        }

        #[cfg(windows)]
        if !mode.is_dry_run() {
            #[link(name = "shell32")]
            unsafe extern "system" {
                fn SHAddToRecentDocs(uFlags: u32, pv: *const std::ffi::c_void);
            }
            unsafe {
                SHAddToRecentDocs(0, std::ptr::null());
            }
        }
    }

    /// 2. Windows Notification & Action Center Cache
    pub fn clean_notifications(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let win_dir = local_appdata.join("Microsoft").join("Windows");

            let wpn_dir = win_dir.join("Notifications").join("wpnidm");
            if wpn_dir.exists() {
                clean_directory_contents(
                    &wpn_dir,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                ext.eq_ignore_ascii_case("jpg")
                                    || ext.eq_ignore_ascii_case("png")
                                    || ext.eq_ignore_ascii_case("webp")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Notification Toast Cache",
                );
            }

            let action_center = win_dir.join("ActionCenterCache");
            if action_center.exists() {
                clean_directory_contents(
                    &action_center,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                ext.eq_ignore_ascii_case("png")
                                    || ext.eq_ignore_ascii_case("jpg")
                                    || ext.eq_ignore_ascii_case("tmp")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Action Center Cache",
                );
            }
        }
    }

    /// 3. Windows Activity History & Connected Devices Platform Timeline Cache
    pub fn clean_activity_history(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let cdp_dir = local_appdata.join("ConnectedDevicesPlatform");
            if cdp_dir.exists() {
                clean_directory_contents(
                    &cdp_dir,
                    true,
                    true,
                    &|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                let lower = name.to_ascii_lowercase();
                                lower.ends_with(".db")
                                    || lower.ends_with(".db-wal")
                                    || lower.ends_with(".db-shm")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Activity History Timeline DB",
                );
            }
        }
    }

    /// 4. Remote Desktop (RDP) Client Bitmap Cache
    pub fn clean_rdp_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let rdp_cache = local_appdata
                .join("Microsoft")
                .join("Terminal Server Client")
                .join("Cache");
            if rdp_cache.exists() {
                clean_directory_contents(
                    &rdp_cache,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                ext.eq_ignore_ascii_case("bmc") || ext.eq_ignore_ascii_case("bin")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Remote Desktop Bitmap Cache",
                );
            }
        }
    }

    /// 5. Windows Explorer History, ConnectedSearch & WebCache Transaction Logs
    pub fn clean_explorer_history(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let win_dir = local_appdata.join("Microsoft").join("Windows");

            let history_ie5 = win_dir.join("History").join("History.IE5");
            if history_ie5.exists() {
                clean_directory_contents(
                    &history_ie5,
                    true,
                    true,
                    &|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                !name.eq_ignore_ascii_case("desktop.ini")
                                    && !name.eq_ignore_ascii_case("container.dat")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Explorer History",
                );
            }

            let connected_search = win_dir.join("ConnectedSearch").join("History");
            if connected_search.exists() {
                clean_directory_contents(
                    &connected_search,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("lnk"))
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows ConnectedSearch History",
                );
            }

            let webcache = win_dir.join("WebCache");
            if webcache.exists() {
                clean_directory_contents(
                    &webcache,
                    false,
                    true,
                    &|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                let lower = name.to_ascii_lowercase();
                                lower.starts_with("v01") && lower.ends_with(".log")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows WebCache Transaction Logs",
                );
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
            "priv_recent_docs" | "fs_recent_docs" => {
                self.clean_recent_docs(mode, report);
                true
            }
            "priv_notifications" | "fs_notifications" => {
                self.clean_notifications(mode, report);
                true
            }
            "priv_activity_history" => {
                self.clean_activity_history(mode, report);
                true
            }
            "priv_rdp_cache" => {
                self.clean_rdp_cache(mode, report);
                true
            }
            "priv_explorer_history" => {
                self.clean_explorer_history(mode, report);
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for PrivacyCleaner {
    fn id(&self) -> u32 {
        3
    }

    fn name(&self) -> &'static str {
        "Windows Privacy & Activity Eraser"
    }

    fn description(&self) -> &'static str {
        "Purges Recent Docs, Jump Lists, Notification/Action Center caches, Activity History, RDP Bitmap Cache, and Explorer traces"
    }

    fn targets_summary(&self) -> &'static str {
        "Recent Docs, Jump Lists, Notifications, ActionCenterCache, ConnectedDevicesPlatform, RDP Bitmap Cache, WebCache Logs"
    }

    fn safety_level(&self) -> &'static str {
        "SAFE // USER ACTIVITY & SHELL PRIVACY TRACES"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let tasks = [
            "Recent Documents & Taskbar Jump Lists",
            "Notification & Action Center Cache",
            "Windows Activity History Timeline DB",
            "Remote Desktop (RDP) Bitmap Cache",
            "Explorer History & WebCache Logs",
        ];
        let total = tasks.len();

        render_progress(tasks[0], 1, total);
        self.clean_recent_docs(mode, &mut report);

        render_progress(tasks[1], 2, total);
        self.clean_notifications(mode, &mut report);

        render_progress(tasks[2], 3, total);
        self.clean_activity_history(mode, &mut report);

        render_progress(tasks[3], 4, total);
        self.clean_rdp_cache(mode, &mut report);

        render_progress(tasks[4], 5, total);
        self.clean_explorer_history(mode, &mut report);

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
