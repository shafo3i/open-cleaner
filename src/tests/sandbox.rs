use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::CleanMode;
use crate::modules::{execute_target, try_execute_target};

/// Global mutex to serialize tests that override process-wide environment variables
static ENV_MUTEX: Mutex<()> = Mutex::new(());

pub struct EnvSandbox {
    _guard: MutexGuard<'static, ()>,
    pub root: PathBuf,
    saved_vars: Vec<(&'static str, Option<OsString>)>,
}

impl EnvSandbox {
    pub fn new(suite_name: &str) -> Self {
        let guard = ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = env::temp_dir().join(format!(
            "open_cleaner_test_{}_{}_{}",
            suite_name,
            std::process::id(),
            stamp
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create sandbox root");

        let keys = [
            "TEMP",
            "TMP",
            "LOCALAPPDATA",
            "APPDATA",
            "USERPROFILE",
            "WINDIR",
            "PROGRAMDATA",
            "ProgramFiles(x86)",
        ];
        let mut saved_vars = Vec::with_capacity(keys.len());
        for k in keys {
            saved_vars.push((k, env::var_os(k)));
        }

        let temp_dir = root.join("Temp");
        let local_appdata = root.join("AppData").join("Local");
        let roaming_appdata = root.join("AppData").join("Roaming");
        let userprofile = root.clone();
        let windir = root.join("Windows");
        let progdata = root.join("ProgramData");
        let prog_x86 = root.join("Program Files (x86)");

        for dir in [
            &temp_dir,
            &local_appdata,
            &roaming_appdata,
            &windir,
            &progdata,
            &prog_x86,
        ] {
            fs::create_dir_all(dir).expect("create sandbox directory");
        }

        unsafe {
            env::set_var("TEMP", &temp_dir);
            env::set_var("TMP", &temp_dir);
            env::set_var("LOCALAPPDATA", &local_appdata);
            env::set_var("APPDATA", &roaming_appdata);
            env::set_var("USERPROFILE", &userprofile);
            env::set_var("WINDIR", &windir);
            env::set_var("PROGRAMDATA", &progdata);
            env::set_var("ProgramFiles(x86)", &prog_x86);
        }

        Self {
            _guard: guard,
            root,
            saved_vars,
        }
    }

    pub fn write_mock(&self, rel: impl AsRef<Path>, size: usize) -> PathBuf {
        let full = self.root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create mock parent directories");
        }
        let payload = vec![b'X'; size.max(64)];
        fs::write(&full, payload).expect("write mock file");
        full
    }
}

impl Drop for EnvSandbox {
    fn drop(&mut self) {
        unsafe {
            for (k, val) in &self.saved_vars {
                match val {
                    Some(v) => env::set_var(k, v),
                    None => env::remove_var(k),
                }
            }
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Executes the mandatory 3-step verification cycle on a target ID:
/// 1. Pre-Scan (`DryRun`): Must find > 0 items and > 0 bytes without deleting anything.
/// 2. Purge (`Standard`): Must delete the exact number of scanned items with 0 failures.
/// 3. Post-Scan (`DryRun`): Must report 0 remaining items and 0 remaining bytes.
pub fn assert_target_purge_cycle(target_id: &str) {
    let pre_scan = try_execute_target(target_id, CleanMode::DryRun)
        .unwrap_or_else(|| panic!("Target '{target_id}' is not routed to any module"));

    assert!(
        pre_scan.items_scanned > 0,
        "Pre-Scan failed for '{target_id}': expected > 0 items scanned in sandbox, got 0"
    );
    assert!(
        pre_scan.bytes_reclaimed > 0,
        "Pre-Scan failed for '{target_id}': expected > 0 bytes found in sandbox, got 0"
    );
    assert_eq!(
        pre_scan.items_cleaned, 0,
        "DryRun must never delete files for '{target_id}'"
    );

    let purge = execute_target(target_id, CleanMode::Standard);
    assert_eq!(
        purge.items_cleaned, pre_scan.items_scanned,
        "Purge count mismatch for '{target_id}': DryRun scanned {} items, Standard cleaned {} items",
        pre_scan.items_scanned, purge.items_cleaned
    );
    assert_eq!(
        purge.items_failed, 0,
        "Purge encountered unexpected failures for '{target_id}'"
    );

    let post_scan = execute_target(target_id, CleanMode::DryRun);
    assert_eq!(
        post_scan.items_scanned, 0,
        "Post-Scan failed for '{target_id}': {} items remain after purge",
        post_scan.items_scanned
    );
    assert_eq!(
        post_scan.bytes_reclaimed, 0,
        "Post-Scan failed for '{target_id}': {} bytes remain after purge",
        post_scan.bytes_reclaimed
    );
}
