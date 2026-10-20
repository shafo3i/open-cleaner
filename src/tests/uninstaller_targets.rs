use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use crate::core::CleanMode;
use crate::tests::sandbox::EnvSandbox;
use crate::uninstaller::{
    AppSource, InstalledApp, InstallerKind, UninstallOptions, detect_installer_kind,
    normalize_uninstall_command, scan_app_remnants_with_prefix, scan_installed_apps,
    scan_installed_apps_with_prefix, split_command_line, uninstall_application_with_prefix,
};

static UNINSTALL_TEST_SEQ: AtomicU64 = AtomicU64::new(1);

struct RegPrefixGuard {
    prefix: String,
}

impl RegPrefixGuard {
    fn new(tag: &str) -> Self {
        let seq = UNINSTALL_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let prefix = format!("Software\\OpenCleanerUninstallTest_{pid}_{tag}_{seq}");
        Self { prefix }
    }
}

impl Drop for RegPrefixGuard {
    fn drop(&mut self) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(&self.prefix);
    }
}

#[test]
fn uninstaller_command_parsing_and_normalization_works() {
    // 1. Quoted executable with arguments
    let (prog, args) =
        split_command_line("\"C:\\Program Files\\Acme Studio\\unins000.exe\" /SILENT /NORESTART")
            .expect("split quoted command");
    assert_eq!(prog, "C:\\Program Files\\Acme Studio\\unins000.exe");
    assert_eq!(args, vec!["/SILENT", "/NORESTART"]);

    // 2. Unquoted executable path with spaces ending in .exe
    let (prog2, args2) = split_command_line("C:\\Program Files\\Vendor App\\Uninstall.exe /S")
        .expect("split unquoted .exe command");
    assert_eq!(prog2, "C:\\Program Files\\Vendor App\\Uninstall.exe");
    assert_eq!(args2, vec!["/S"]);

    // 3. MSI /I{GUID} normalization to /X{GUID} + silent flags
    let msi_app = InstalledApp {
        id: "msi_test".to_string(),
        registry_subkey: String::new(),
        display_name: "Enterprise Agent".to_string(),
        display_version: "4.0.0".to_string(),
        publisher: "Acme Corp".to_string(),
        install_date: "2026-01-10".to_string(),
        install_location: None,
        uninstall_string: "MsiExec.exe /I{11111111-2222-3333-4444-555555555555}".to_string(),
        quiet_uninstall_string: None,
        display_icon: None,
        estimated_size_bytes: 1024,
        source: AppSource::Win32Machine64,
        installer_kind: detect_installer_kind(
            "MsiExec.exe /I{11111111-2222-3333-4444-555555555555}",
            None,
            1,
            AppSource::Win32Machine64,
        ),
        icon_image: None,
    };
    assert_eq!(msi_app.installer_kind, InstallerKind::Msi);

    let interactive_cmd = normalize_uninstall_command(&msi_app, false);
    assert_eq!(
        interactive_cmd,
        "MsiExec.exe /X{11111111-2222-3333-4444-555555555555}"
    );

    let silent_cmd = normalize_uninstall_command(&msi_app, true);
    assert_eq!(
        silent_cmd,
        "MsiExec.exe /X{11111111-2222-3333-4444-555555555555} /qn /norestart"
    );
}

#[test]
fn sandboxed_registry_and_appx_discovery_filters_and_enriches_accurately() {
    let guard = RegPrefixGuard::new("discovery");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let uninstall_base = format!(
        "{}\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        guard.prefix
    );

    // 1. Valid Win32 application (Inno Setup)
    let (valid_key, _) = hkcu
        .create_subkey(format!("{uninstall_base}\\AcmeVideoStudio_is1"))
        .expect("create valid app key");
    valid_key
        .set_value("DisplayName", &"Acme Video Studio")
        .unwrap();
    valid_key.set_value("DisplayVersion", &"2.4.1").unwrap();
    valid_key.set_value("Publisher", &"Acme Media Inc").unwrap();
    valid_key.set_value("InstallDate", &"20260415").unwrap();
    valid_key.set_value("EstimatedSize", &51200u32).unwrap(); // 50 MB in KB
    valid_key
        .set_value(
            "UninstallString",
            &"\"C:\\Program Files\\Acme Video Studio\\unins000.exe\"",
        )
        .unwrap();

    // 2. SystemComponent == 1 (Must be filtered out)
    let (sys_key, _) = hkcu
        .create_subkey(format!("{uninstall_base}\\HiddenSystemRuntime"))
        .expect("create system component key");
    sys_key
        .set_value("DisplayName", &"Internal System Runtime")
        .unwrap();
    sys_key
        .set_value("UninstallString", &"msiexec.exe /X{0000}")
        .unwrap();
    sys_key.set_value("SystemComponent", &1u32).unwrap();

    // 3. Security Update (Must be filtered out)
    let (upd_key, _) = hkcu
        .create_subkey(format!("{uninstall_base}\\KB999999"))
        .expect("create update key");
    upd_key
        .set_value("DisplayName", &"Security Update KB999999")
        .unwrap();
    upd_key
        .set_value("UninstallString", &"wusa.exe /uninstall")
        .unwrap();
    upd_key
        .set_value("ReleaseType", &"Security Update")
        .unwrap();

    // 4. Valid UWP / AppX Store application + 1 Framework package (Must filter framework)
    let appx_base = format!(
        "{}\\Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages",
        guard.prefix
    );
    let (store_key, _) = hkcu
        .create_subkey(format!(
            "{appx_base}\\AcmeMedia.PhotoSuite_3.1.0.0_x64__zpdnekdrzrea0"
        ))
        .expect("create store app key");
    store_key
        .set_value("DisplayName", &"Acme Photo Suite")
        .unwrap();

    let (fw_key, _) = hkcu
        .create_subkey(format!(
            "{appx_base}\\Microsoft.VCLibs.140.00_14.0.30704.0_x64__8wekyb3d8bbwe"
        ))
        .expect("create framework appx key");
    fw_key
        .set_value("DisplayName", &"Microsoft Visual C++ 2015 UWP Runtime")
        .unwrap();

    let discovered = scan_installed_apps_with_prefix(&guard.prefix);
    assert_eq!(
        discovered.len(),
        2,
        "Must discover exactly the 1 valid Win32 app and 1 valid Store app while skipping SystemComponent, Security Update, and VCLibs framework"
    );

    let win32_app = discovered
        .iter()
        .find(|a| a.display_name == "Acme Video Studio")
        .expect("Acme Video Studio present");
    assert_eq!(win32_app.display_version, "2.4.1");
    assert_eq!(win32_app.publisher, "Acme Media Inc");
    assert_eq!(win32_app.install_date, "2026-04-15");
    assert_eq!(win32_app.estimated_size_bytes, 51200 * 1024);
    assert_eq!(win32_app.installer_kind, InstallerKind::InnoSetup);

    let store_app = discovered
        .iter()
        .find(|a| a.display_name == "Acme Photo Suite")
        .expect("Acme Photo Suite present");
    assert_eq!(store_app.source, AppSource::WindowsStoreAppX);
    assert_eq!(store_app.installer_kind, InstallerKind::AppX);
    assert_eq!(store_app.display_version, "3.1.0.0");
}

#[test]
fn complete_uninstall_executes_native_uninstaller_and_purges_all_remnants_safely() {
    let sb = EnvSandbox::new("complete_uninstall");
    let guard = RegPrefixGuard::new("complete_uninstall");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Create simulated installation directory with main binary + leftover plugin cache
    let install_dir = sb.root.join("Programs\\Nebula Code Editor");
    let main_exe = sb.write_mock("Programs\\Nebula Code Editor\\nebula.exe", 4096);
    let residual_plugin = sb.write_mock("Programs\\Nebula Code Editor\\plugins\\addon.cache", 2048);

    // 2. Create leftover folders across %LOCALAPPDATA% (AppData\Local), %APPDATA% (AppData\Roaming), %PROGRAMDATA%, Start Menu, and Desktop
    let local_remnant_1 =
        sb.write_mock("AppData\\Local\\Nebula Code Editor\\Cache\\index.dat", 8192);
    let local_remnant_2 = sb.write_mock(
        "AppData\\Local\\NebulaLabs\\Nebula Code Editor\\telemetry.log",
        1024,
    );
    let roaming_remnant = sb.write_mock("AppData\\Roaming\\Nebula Code Editor\\settings.json", 512);
    let progdata_remnant = sb.write_mock("ProgramData\\Nebula Code Editor\\license.cache", 512);
    let start_menu_lnk = sb.write_mock(
        "AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Nebula Code Editor.lnk",
        256,
    );
    let desktop_lnk = sb.write_mock("Desktop\\Nebula Code Editor.lnk", 256);

    // 3. Create protected sibling folders that MUST NOT be touched
    let protected_ms = sb.write_mock("AppData\\Local\\Microsoft\\Windows\\important.dat", 1024);
    let unrelated_app = sb.write_mock("AppData\\Local\\OtherStudioApp\\user.db", 1024);

    // 4. Create Registry Uninstall key and Software\<Publisher>\<AppName> remnant keys
    let uninstall_subkey = format!(
        "{}\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\NebulaEditor",
        guard.prefix
    );
    let (u_key, _) = hkcu.create_subkey(&uninstall_subkey).unwrap();
    u_key
        .set_value("DisplayName", &"Nebula Code Editor")
        .unwrap();
    u_key.set_value("DisplayVersion", &"1.9.0").unwrap();
    u_key.set_value("Publisher", &"NebulaLabs").unwrap();
    u_key
        .set_value("InstallLocation", &install_dir.to_string_lossy().as_ref())
        .unwrap();

    // Native uninstaller command: deletes only `nebula.exe`, leaving `plugins\addon.cache`, AppData, shortcuts, and registry keys behind
    let uninstaller_cmd = format!("cmd.exe /C del /F /Q \"{}\"", main_exe.display());
    u_key
        .set_value("UninstallString", &uninstaller_cmd)
        .unwrap();

    let reg_software_remnant =
        format!("{}\\Software\\NebulaLabs\\Nebula Code Editor", guard.prefix);
    let (sw_key, _) = hkcu.create_subkey(&reg_software_remnant).unwrap();
    sw_key.set_value("LastOpened", &"2026-10-18").unwrap();

    let unrelated_reg_key = format!("{}\\Software\\OtherVendor\\OtherApp", guard.prefix);
    let (other_key, _) = hkcu.create_subkey(&unrelated_reg_key).unwrap();
    other_key.set_value("Keep", &1u32).unwrap();

    // 5. Discover the app via our registry scanner
    let apps = scan_installed_apps_with_prefix(&guard.prefix);
    assert_eq!(apps.len(), 1);
    let app = &apps[0];
    assert_eq!(app.display_name, "Nebula Code Editor");

    // Verify pre-scan finds the remnants
    let pre_remnants = scan_app_remnants_with_prefix(app, &guard.prefix);
    assert!(
        pre_remnants.len() >= 7,
        "Expected at least 7 remnant targets (InstallDir, LocalAppData, Roaming, ProgramData, Shortcuts, Registry), found {}",
        pre_remnants.len()
    );

    // 6. Execute Complete Uninstall (Native Uninstaller + Deep Remnant Purge)
    let report = uninstall_application_with_prefix(
        app,
        UninstallOptions {
            run_native_uninstaller: true,
            prefer_silent: false,
            purge_remnants: true,
            clean_mode: CleanMode::Standard,
        },
        &guard.prefix,
    );

    assert!(report.uninstaller_ran, "Native uninstaller must run");
    assert!(
        report.uninstaller_success,
        "Native uninstaller must succeed: {:?}",
        report.error_message
    );
    assert_eq!(report.uninstaller_exit_code, Some(0));
    assert!(report.remnants_removed >= 7);
    assert_eq!(report.remnants_failed, 0);
    assert!(report.bytes_reclaimed > 0);

    // 7. Verify 100% of application files, folders, shortcuts, and registry remnants are gone
    assert!(!main_exe.exists(), "Main executable must be removed");
    assert!(
        !residual_plugin.exists() && !install_dir.exists(),
        "Residual InstallLocation must be completely purged"
    );
    assert!(
        !local_remnant_1.exists(),
        "%LOCALAPPDATA%\\<AppName> must be purged"
    );
    assert!(
        !local_remnant_2.exists(),
        "%LOCALAPPDATA%\\<Publisher>\\<AppName> must be purged"
    );
    assert!(
        !roaming_remnant.exists(),
        "%APPDATA%\\<AppName> must be purged"
    );
    assert!(
        !progdata_remnant.exists(),
        "%PROGRAMDATA%\\<AppName> must be purged"
    );
    assert!(
        !start_menu_lnk.exists(),
        "Start Menu shortcut must be purged"
    );
    assert!(!desktop_lnk.exists(), "Desktop shortcut must be purged");

    assert!(
        hkcu.open_subkey_with_flags(&uninstall_subkey, KEY_READ)
            .is_err(),
        "Orphaned Uninstall registry key must be purged"
    );
    assert!(
        hkcu.open_subkey_with_flags(&reg_software_remnant, KEY_READ)
            .is_err(),
        "Orphaned HKCU\\Software\\<Publisher>\\<AppName> registry key must be purged"
    );

    // 8. Verify protected shared vendor folders and unrelated apps were NOT touched
    assert!(
        protected_ms.exists(),
        "Protected Microsoft folder must never be deleted"
    );
    assert!(
        unrelated_app.exists(),
        "Unrelated application data must never be deleted"
    );
    assert!(
        hkcu.open_subkey_with_flags(&unrelated_reg_key, KEY_READ | KEY_WRITE)
            .is_ok(),
        "Unrelated registry key must never be deleted"
    );
    let _ = fs::metadata(&protected_ms);
}

#[test]
fn live_system_app_discovery_enumerates_host_applications() {
    let apps = scan_installed_apps();
    assert!(
        !apps.is_empty(),
        "Live system scan on Windows host must discover at least one installed application"
    );
    for app in &apps {
        assert!(
            !app.display_name.trim().is_empty(),
            "Every discovered application must have a non-empty DisplayName"
        );
        assert!(
            !app.uninstall_string.trim().is_empty(),
            "Every discovered application must have a valid UninstallString"
        );
    }
}

#[test]
fn app_monogram_generates_correct_two_letter_codes() {
    use crate::uninstaller::icons::app_monogram;

    assert_eq!(app_monogram("Visual Studio Code"), "VS");
    assert_eq!(app_monogram("Google Chrome"), "GC");
    assert_eq!(app_monogram("Obsidian"), "OB");
    assert_eq!(app_monogram("7-Zip"), "7Z");
    assert_eq!(app_monogram("   "), "AP");
    assert_eq!(app_monogram("X"), "XP");
}

#[test]
fn live_system_app_icon_extraction_or_fallback() {
    use crate::uninstaller::icons::{extract_app_icon, resolve_icon_path};

    let apps = scan_installed_apps();
    assert!(!apps.is_empty());

    // Verify resolve_icon_path and extract_app_icon for candidate host apps
    let mut extracted_count = 0;
    for app in apps.iter().take(20) {
        if let Some((path, idx)) = resolve_icon_path(app) {
            if let Some(img) = extract_app_icon(&path, idx) {
                assert!(img.width() > 0 && img.height() > 0);
                assert_eq!(img.as_raw().len(), img.width() * img.height() * 4);
                extracted_count += 1;
            }
        }
    }
    // We expect at least one host application among the first 20 to have an extractable icon
    assert!(
        extracted_count > 0,
        "Expected at least one Windows host app to provide an extractable icon"
    );
}
