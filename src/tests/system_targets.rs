use super::sandbox::{EnvSandbox, assert_target_purge_cycle};

#[test]
fn system_category_targets_scan_and_purge_correctly() {
    let sb = EnvSandbox::new("system");

    sb.write_mock("Temp/temp_artifact.tmp", 512);
    sb.write_mock("Windows/Temp/sys_temp.tmp", 512);
    sb.write_mock("AppData/Local/CrashDumps/app.exe.1234.dmp", 1024);
    sb.write_mock("Windows/MEMORY.DMP", 2048);
    sb.write_mock("Windows/Minidump/010126-01.dmp", 1024);
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/WER/ReportArchive/Report1/Report.wer",
        512,
    );
    sb.write_mock(
        "ProgramData/Microsoft/Windows/WER/ReportQueue/Report2/Report.wer",
        512,
    );
    sb.write_mock("Windows/Logs/CBS/CBS.log", 1024);
    sb.write_mock("AppData/Local/Microsoft/Windows/WindowsUpdate.log", 512);
    sb.write_mock(
        "Windows/SoftwareDistribution/DeliveryOptimization/Cache/do_chunk.bin",
        1024,
    );
    sb.write_mock("Windows/Logs/MoSetup/BlueBox.etl", 1024);
    sb.write_mock("ProgramData/USOShared/Logs/System/UsoCore.etl", 512);
    sb.write_mock(
        "AppData/Local/Packages/MicrosoftWindows.Client.CBS_cw5n1h2txyewy/TempState/ScreenClip.png",
        512,
    );
    sb.write_mock(
        "Windows/SoftwareDistribution/Download/update_payload.cab",
        2048,
    );
    sb.write_mock(
        "Windows/SoftwareDistribution/DataStore/Logs/edb00001.log",
        512,
    );
    sb.write_mock("Windows/inf/setupapi.dev.log", 512);
    sb.write_mock("Windows/Panther/setupact.log", 512);
    sb.write_mock(
        "AppData/Local/Microsoft/CLR_v4.0/UsageLogs/app.exe.log",
        512,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/INetCache/IE/cached_page.htm",
        512,
    );
    let protected_inet = sb.write_mock(
        "AppData/Local/Microsoft/Windows/INetCache/IE/container.dat",
        256,
    );
    sb.write_mock(
        "AppData/LocalLow/Microsoft/CryptnetUrlCache/Content/cert_blob.bin",
        512,
    );
    sb.write_mock(
        "AppData/LocalLow/Microsoft/CryptnetUrlCache/MetaData/cert_meta.bin",
        512,
    );

    for target_id in [
        "fs_user_temp",
        "fs_win_temp",
        "fs_crash_dumps",
        "fs_wer",
        "fs_logs",
        "fs_delivery_opt",
        "fs_event_traces",
        "fs_client_cbs_temp",
        "fs_wu_download",
        "fs_setup_clr_logs",
        "fs_inet_cache",
        "fs_cryptnet_cache",
    ] {
        assert_target_purge_cycle(target_id);
    }

    assert!(
        protected_inet.exists(),
        "WinINet index database container.dat must be preserved"
    );
}
