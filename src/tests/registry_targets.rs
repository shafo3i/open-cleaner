use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use crate::core::{CleanMode, ModuleReport};
use crate::modules::registry::RegistryAnalyzer;

struct RegistrySandboxGuard {
    prefix: String,
}

impl Drop for RegistrySandboxGuard {
    fn drop(&mut self) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(&self.prefix);
    }
}

#[test]
fn registry_targets_scan_and_purge_broken_keys_while_preserving_valid_keys() {
    let prefix = format!(r"Software\OpenCleanerTest_{}", std::process::id());
    let _guard = RegistrySandboxGuard {
        prefix: prefix.clone(),
    };
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(&prefix);

    let valid_exe = r"C:\Windows\System32\cmd.exe";
    let missing_exe = r"C:\__open_cleaner_nonexistent_dir_999__\missing_app.exe";

    // 1. reg_app_paths: 1 broken subkey, 1 valid subkey
    let (broken_app, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\App Paths\broken.exe"
        ))
        .expect("create broken App Path");
    broken_app
        .set_value("", &missing_exe)
        .expect("set broken App Path value");
    drop(broken_app);

    let (valid_app, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\App Paths\valid.exe"
        ))
        .expect("create valid App Path");
    valid_app
        .set_value("", &valid_exe)
        .expect("set valid App Path value");
    drop(valid_app);

    // 2. reg_mui_cache: 1 broken entry, 1 valid entry
    let (mui_key, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache"
        ))
        .expect("create MuiCache key");
    mui_key
        .set_value(
            format!("{missing_exe}.FriendlyAppName"),
            &"Missing Application",
        )
        .expect("set broken MuiCache");
    mui_key
        .set_value(format!("{valid_exe}.FriendlyAppName"), &"Command Prompt")
        .expect("set valid MuiCache");
    drop(mui_key);

    // 3. reg_startup: 1 broken startup value, 1 valid startup value
    let (run_key, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\Run"
        ))
        .expect("create Run key");
    run_key
        .set_value("BrokenLauncher", &format!("\"{missing_exe}\" --minimized"))
        .expect("set broken Run");
    run_key
        .set_value("ValidLauncher", &format!("\"{valid_exe}\" /c echo ok"))
        .expect("set valid Run");
    drop(run_key);

    // 4. reg_appcompat_store: 1 broken value, 1 valid value
    let (compat_key, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers"
        ))
        .expect("create Layers key");
    compat_key
        .set_value(missing_exe, &"~ RUNASADMIN")
        .expect("set broken AppCompat");
    compat_key
        .set_value(valid_exe, &"~ RUNASADMIN")
        .expect("set valid AppCompat");
    drop(compat_key);

    // 5. reg_run_mru: 2 MRU entries
    let (run_mru, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\Explorer\RunMRU"
        ))
        .expect("create RunMRU key");
    run_mru.set_value("a", &"dxdiag\\1").expect("set RunMRU a");
    run_mru.set_value("MRUList", &"a").expect("set RunMRU list");
    drop(run_mru);

    // 6. reg_opensave_mru: 1 LastVisited value + 1 OpenSavePidlMRU subkey value
    let (lv_key, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\Explorer\ComDlg32\LastVisitedPidlMRU"
        ))
        .expect("create LastVisitedPidlMRU");
    lv_key
        .set_value("MRUListEx", &0u32)
        .expect("set LastVisitedPidlMRU value");
    drop(lv_key);

    let (os_pdf_key, _) = hkcu
        .create_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\Explorer\ComDlg32\OpenSavePidlMRU\pdf"
        ))
        .expect("create OpenSavePidlMRU\\pdf");
    os_pdf_key
        .set_value("0", &1u32)
        .expect("set OpenSavePidlMRU\\pdf value");
    drop(os_pdf_key);

    let analyzer = RegistryAnalyzer::with_prefix(&prefix);
    let targets = [
        ("reg_app_paths", 1usize),
        ("reg_mui_cache", 1usize),
        ("reg_startup", 1usize),
        ("reg_appcompat_store", 1usize),
        ("reg_run_mru", 2usize),
        ("reg_opensave_mru", 2usize),
    ];

    for (tid, expected_count) in targets {
        // Step 1: DryRun Pre-Scan
        let mut dry_report = ModuleReport::new(tid);
        assert!(analyzer.clean_target(tid, CleanMode::DryRun, &mut dry_report));
        assert_eq!(
            dry_report.items_scanned, expected_count,
            "DryRun scan count mismatch for {tid}"
        );
        assert_eq!(dry_report.items_cleaned, 0);

        // Step 2: Standard Purge
        let mut clean_report = ModuleReport::new(tid);
        assert!(analyzer.clean_target(tid, CleanMode::Standard, &mut clean_report));
        assert_eq!(
            clean_report.items_cleaned, expected_count,
            "Standard purge count mismatch for {tid}"
        );
        assert_eq!(clean_report.items_failed, 0, "Purge had failures for {tid}");

        // Step 3: Post-Purge DryRun Verification (must be 0 remaining)
        let mut post_report = ModuleReport::new(tid);
        assert!(analyzer.clean_target(tid, CleanMode::DryRun, &mut post_report));
        assert_eq!(
            post_report.items_scanned, 0,
            "Post-purge scan should find 0 remaining broken entries for {tid}"
        );
    }

    // Step 4: Verify valid executables were preserved untouched
    assert!(
        hkcu.open_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\App Paths\valid.exe"
        ))
        .is_ok(),
        "Valid App Path key should be preserved"
    );
    let run_check = hkcu
        .open_subkey(format!(
            r"{prefix}\Software\Microsoft\Windows\CurrentVersion\Run"
        ))
        .expect("open Run key");
    assert!(
        run_check.get_value::<String, _>("ValidLauncher").is_ok(),
        "Valid Startup Run entry should be preserved"
    );
}
