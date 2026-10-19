use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::CleanerModule;
use crate::core::cleaner::{CleanResult, clean_directory_contents, secure_delete_file};
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct FileSystemCleaner;

impl FileSystemCleaner {
    pub fn new() -> Self {
        Self
    }

    /// Resolves canonical environment paths safely
    fn get_env_path(var_name: &str) -> Option<PathBuf> {
        env::var_os(var_name).map(PathBuf::from)
    }

    pub fn clean_user_temp(&self, mode: CleanMode, report: &mut ModuleReport) {
        let user_temp = env::temp_dir();
        if user_temp.exists() {
            clean_directory_contents(
                &user_temp,
                true,
                true, // Keep root Temp directory
                &|_| true,
                mode,
                report,
                "User Temporary Files",
            );
        }
    }

    pub fn clean_win_temp(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(windir) = Self::get_env_path("WINDIR") {
            let win_temp = windir.join("Temp");
            clean_directory_contents(
                &win_temp,
                true,
                true,
                &|_| true,
                mode,
                report,
                "Windows System Temp",
            );
        }
    }

    pub fn clean_logs(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(windir) = Self::get_env_path("WINDIR") {
            let win_logs = windir.join("Logs");
            clean_directory_contents(
                &win_logs,
                true,
                true,
                &|p| {
                    p.extension()
                        .map(|ext| ext == "log" || ext == "bak")
                        .unwrap_or(false)
                },
                mode,
                report,
                "Windows System Logs",
            );
        }

        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let ms_win = local_appdata.join("Microsoft").join("Windows");
            clean_directory_contents(
                &ms_win,
                false,
                true,
                &|p| p.extension().map(|ext| ext == "log").unwrap_or(false),
                mode,
                report,
                "Local Windows Logs",
            );
        }
    }

    pub fn clean_crash_dumps(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let crash_dumps = local_appdata.join("CrashDumps");
            clean_directory_contents(
                &crash_dumps,
                true,
                true,
                &|p| p.extension().map(|ext| ext == "dmp").unwrap_or(false),
                mode,
                report,
                "User Crash Dumps",
            );
        }

        if let Some(windir) = Self::get_env_path("WINDIR") {
            let memory_dmp = windir.join("MEMORY.DMP");
            if memory_dmp.exists() {
                match secure_delete_file(&memory_dmp, mode) {
                    CleanResult::Cleaned(s) => {
                        report.record_cleaned(memory_dmp, s, "Kernel Memory Dump")
                    }
                    CleanResult::DryRunFound(s) => {
                        report.record_scanned(memory_dmp, s, "Kernel Memory Dump")
                    }
                    CleanResult::SkippedLocked(s) => {
                        report.record_skipped_locked(memory_dmp, s, "Kernel Memory Dump")
                    }
                    CleanResult::SkippedPermission(s) => {
                        report.record_skipped_permission(memory_dmp, s, "Kernel Memory Dump")
                    }
                    CleanResult::Failed(s, e) => {
                        report.record_failed(memory_dmp, s, e, "Kernel Memory Dump")
                    }
                }
            }

            let minidump_dir = windir.join("Minidump");
            clean_directory_contents(
                &minidump_dir,
                true,
                true,
                &|p| p.extension().map(|ext| ext == "dmp").unwrap_or(false),
                mode,
                report,
                "Kernel Minidumps",
            );
        }
    }

    pub fn clean_wer(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            for sub in ["ReportArchive", "ReportQueue", "Temp"] {
                let wer_path = local_appdata
                    .join("Microsoft")
                    .join("Windows")
                    .join("WER")
                    .join(sub);
                clean_directory_contents(
                    &wer_path,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Windows Error Reporting (User)",
                );
            }
        }

        if let Some(prog_data) = Self::get_env_path("PROGRAMDATA") {
            for sub in ["ReportArchive", "ReportQueue", "Temp"] {
                let wer_path = prog_data
                    .join("Microsoft")
                    .join("Windows")
                    .join("WER")
                    .join(sub);
                clean_directory_contents(
                    &wer_path,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Windows Error Reporting (Machine)",
                );
            }
        }
    }

    pub fn clean_delivery_opt(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(windir) = Self::get_env_path("WINDIR") {
            let do_cache = windir
                .join("SoftwareDistribution")
                .join("DeliveryOptimization")
                .join("Cache");
            clean_directory_contents(
                &do_cache,
                true,
                true,
                &|_| true,
                mode,
                report,
                "Delivery Optimization Cache",
            );
        }
    }

    /// Cleans Recycle Bin files on fixed local drives and notifies Windows Shell
    pub fn clean_recycle_bin(&self, mode: CleanMode, report: &mut ModuleReport) {
        #[cfg(windows)]
        let drives: Vec<String> = {
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GetLogicalDrives() -> u32;
                fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
            }
            const DRIVE_FIXED: u32 = 3;
            let mask = unsafe { GetLogicalDrives() };
            let mut list = Vec::new();
            for i in 0..26u32 {
                if (mask & (1 << i)) != 0 {
                    let letter = (b'A' + i as u8) as char;
                    let wide: [u16; 4] = [letter as u16, b':' as u16, b'\\' as u16, 0];
                    if unsafe { GetDriveTypeW(wide.as_ptr()) } == DRIVE_FIXED {
                        list.push(format!("{letter}:\\"));
                    }
                }
            }
            list
        };
        #[cfg(not(windows))]
        let drives: Vec<String> = vec!["C:\\".to_string()];

        for drive in drives {
            let bin_path = Path::new(&drive).join("$Recycle.Bin");
            let Ok(entries) = std::fs::read_dir(&bin_path) else {
                continue;
            };
            for entry in entries.flatten() {
                let sid_path = entry.path();
                // Only clean SID folders accessible to the current user (skip SYSTEM S-1-5-18)
                if sid_path.is_dir() && std::fs::read_dir(&sid_path).is_ok() {
                    clean_directory_contents(
                        &sid_path,
                        true,
                        true, // Preserve user's SID container folder
                        &|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| !name.eq_ignore_ascii_case("desktop.ini"))
                                .unwrap_or(true)
                        },
                        mode,
                        report,
                        "Windows Recycle Bin Storage",
                    );
                }
            }
        }

        #[cfg(windows)]
        if !mode.is_dry_run() {
            #[link(name = "shell32")]
            unsafe extern "system" {
                fn SHEmptyRecycleBinW(hwnd: usize, pszRootPath: *const u16, dwFlags: u32) -> i32;
            }
            const SHERB_NOCONFIRMATION: u32 = 0x0000_0001;
            const SHERB_NOPROGRESSUI: u32 = 0x0000_0002;
            const SHERB_NOSOUND: u32 = 0x0000_0004;

            unsafe {
                let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
                let _ = SHEmptyRecycleBinW(0, std::ptr::null(), flags);
            }
        }
    }

    pub fn clean_event_traces(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(windir) = Self::get_env_path("WINDIR") {
            let win_logs = windir.join("Logs");
            clean_directory_contents(
                &win_logs,
                true,
                true,
                &|p| {
                    p.extension()
                        .map(|ext| ext == "etl" || ext == "ETL")
                        .unwrap_or(false)
                },
                mode,
                report,
                "Windows System Event Traces",
            );
        }

        if let Some(progdata) = Self::get_env_path("PROGRAMDATA") {
            let uso_logs = progdata.join("USOShared").join("Logs");
            if uso_logs.exists() {
                clean_directory_contents(
                    &uso_logs,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .map(|ext| ext == "etl" || ext == "ETL")
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Update Orchestrator Event Traces",
                );
            }
        }
    }

    pub fn clean_client_cbs_temp(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let cbs_temp = local_appdata
                .join("Packages")
                .join("MicrosoftWindows.Client.CBS_cw5n1h2txyewy")
                .join("TempState");
            if cbs_temp.exists() {
                clean_directory_contents(
                    &cbs_temp,
                    true,
                    true, // Preserve root TempState directory
                    &|_| true,
                    mode,
                    report,
                    "Windows Client CBS Temp Files",
                );
            }
        }
    }

    pub fn clean_wu_download(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(windir) = Self::get_env_path("WINDIR") {
            let wu_download = windir.join("SoftwareDistribution").join("Download");
            if wu_download.exists() {
                clean_directory_contents(
                    &wu_download,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Windows Update Download Cache",
                );
            }

            let wu_datastore_logs = windir
                .join("SoftwareDistribution")
                .join("DataStore")
                .join("Logs");
            if wu_datastore_logs.exists() {
                clean_directory_contents(
                    &wu_datastore_logs,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                ext.eq_ignore_ascii_case("log") || ext.eq_ignore_ascii_case("jrs")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Update DataStore Logs",
                );
            }
        }
    }

    pub fn clean_setup_clr_logs(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(windir) = Self::get_env_path("WINDIR") {
            let inf_dir = windir.join("inf");
            if inf_dir.exists() {
                clean_directory_contents(
                    &inf_dir,
                    false, // Non-recursive in %WINDIR%\inf
                    true,
                    &|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                let lower = name.to_ascii_lowercase();
                                lower.starts_with("setupapi") && lower.ends_with(".log")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Driver SetupAPI Logs",
                );
            }

            let panther_dir = windir.join("Panther");
            if panther_dir.exists() {
                clean_directory_contents(
                    &panther_dir,
                    true,
                    true,
                    &|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("log"))
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Panther Setup Logs",
                );
            }
        }

        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let ms_dir = local_appdata.join("Microsoft");
            for clr_folder in ["CLR_v4.0", "CLR_v4.0_32", "CLR_v2.0", "CLR_v2.0_32"] {
                let clr_path = ms_dir.join(clr_folder);
                if clr_path.exists() {
                    clean_directory_contents(
                        &clr_path,
                        true,
                        true,
                        &|p| {
                            p.extension()
                                .and_then(|e| e.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("log"))
                                .unwrap_or(false)
                        },
                        mode,
                        report,
                        ".NET CLR Usage Logs",
                    );
                }
            }
        }
    }

    pub fn clean_inet_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let inet_root = local_appdata
                .join("Microsoft")
                .join("Windows")
                .join("INetCache");
            for sub in ["IE", "Content.IE5"] {
                let target_dir = inet_root.join(sub);
                if target_dir.exists() {
                    clean_directory_contents(
                        &target_dir,
                        true,
                        true,
                        &|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| {
                                    !name.eq_ignore_ascii_case("container.dat")
                                        && !name.eq_ignore_ascii_case("desktop.ini")
                                })
                                .unwrap_or(true)
                        },
                        mode,
                        report,
                        "Windows WinINet Cache",
                    );
                }
            }
        }
    }

    pub fn clean_cryptnet_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        let mut roots = Vec::new();
        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            roots.push(
                userprofile
                    .join("AppData")
                    .join("LocalLow")
                    .join("Microsoft")
                    .join("CryptnetUrlCache"),
            );
        }
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            roots.push(local_appdata.join("Microsoft").join("CryptnetUrlCache"));
        }

        for root in roots {
            for sub in ["Content", "MetaData"] {
                let dir = root.join(sub);
                if dir.exists() {
                    clean_directory_contents(
                        &dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Windows Cryptnet URL Cache",
                    );
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
            "fs_user_temp" => {
                self.clean_user_temp(mode, report);
                true
            }
            "fs_win_temp" => {
                self.clean_win_temp(mode, report);
                true
            }
            "fs_recycle" => {
                self.clean_recycle_bin(mode, report);
                true
            }
            "fs_crash_dumps" => {
                self.clean_crash_dumps(mode, report);
                true
            }
            "fs_wer" => {
                self.clean_wer(mode, report);
                true
            }
            "fs_logs" => {
                self.clean_logs(mode, report);
                true
            }
            "fs_delivery_opt" => {
                self.clean_delivery_opt(mode, report);
                true
            }
            "fs_event_traces" => {
                self.clean_event_traces(mode, report);
                true
            }
            "fs_client_cbs_temp" => {
                self.clean_client_cbs_temp(mode, report);
                true
            }
            "fs_wu_download" => {
                self.clean_wu_download(mode, report);
                true
            }
            "fs_setup_clr_logs" => {
                self.clean_setup_clr_logs(mode, report);
                true
            }
            "fs_inet_cache" => {
                self.clean_inet_cache(mode, report);
                true
            }
            "fs_cryptnet_cache" => {
                self.clean_cryptnet_cache(mode, report);
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for FileSystemCleaner {
    fn id(&self) -> u32 {
        1
    }

    fn name(&self) -> &'static str {
        "File System Purifier"
    }

    fn description(&self) -> &'static str {
        "Cleans temporary files, logs, error reports, crash dumps, and Recycle Bin"
    }

    fn targets_summary(&self) -> &'static str {
        "%TEMP%, %WINDIR%\\Temp, WU Download, Setup/CLR Logs, WinINet, Cryptnet, Event Traces, WER, $Recycle.Bin"
    }

    fn safety_level(&self) -> &'static str {
        "SAFE // VERIFIED KERNEL / USER DISK CACHES"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let tasks = [
            "User Temporary Directory",
            "Windows System Temp",
            "Windows System Logs (*.log)",
            "System Crash Dumps & MiniDumps",
            "Windows Error Reporting (WER)",
            "Windows Delivery Optimization",
            "Windows Recycle Bin",
            "Windows Event Trace Logs (*.etl)",
            "Windows Client CBS Temp Files",
            "Windows Update Download Cache",
            "Driver SetupAPI, Panther & CLR Logs",
            "Windows WinINet Cache",
            "Windows Cryptnet URL Cache",
        ];

        let total_tasks = tasks.len();

        render_progress(tasks[0], 1, total_tasks);
        self.clean_user_temp(mode, &mut report);

        render_progress(tasks[1], 2, total_tasks);
        self.clean_win_temp(mode, &mut report);

        render_progress(tasks[2], 3, total_tasks);
        self.clean_logs(mode, &mut report);

        render_progress(tasks[3], 4, total_tasks);
        self.clean_crash_dumps(mode, &mut report);

        render_progress(tasks[4], 5, total_tasks);
        self.clean_wer(mode, &mut report);

        render_progress(tasks[5], 6, total_tasks);
        self.clean_delivery_opt(mode, &mut report);

        render_progress(tasks[6], 7, total_tasks);
        self.clean_recycle_bin(mode, &mut report);

        render_progress(tasks[7], 8, total_tasks);
        self.clean_event_traces(mode, &mut report);

        render_progress(tasks[8], 9, total_tasks);
        self.clean_client_cbs_temp(mode, &mut report);

        render_progress(tasks[9], 10, total_tasks);
        self.clean_wu_download(mode, &mut report);

        render_progress(tasks[10], 11, total_tasks);
        self.clean_setup_clr_logs(mode, &mut report);

        render_progress(tasks[11], 12, total_tasks);
        self.clean_inet_cache(mode, &mut report);

        render_progress(tasks[12], 13, total_tasks);
        self.clean_cryptnet_cache(mode, &mut report);

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
