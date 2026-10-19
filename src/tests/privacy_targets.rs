use super::sandbox::{EnvSandbox, assert_target_purge_cycle};

#[test]
fn privacy_category_targets_scan_and_purge_correctly() {
    let sb = EnvSandbox::new("privacy");

    sb.write_mock("AppData/Roaming/Microsoft/Windows/Recent/secret.lnk", 256);
    sb.write_mock(
        "AppData/Roaming/Microsoft/Windows/Recent/AutomaticDestinations/f01b4d95cf55d32a.automaticDestinations-ms",
        512,
    );
    sb.write_mock(
        "AppData/Roaming/Microsoft/Windows/Recent/CustomDestinations/app.customDestinations-ms",
        512,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/Notifications/wpnidm/toast_img.png",
        512,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/ActionCenterCache/banner.jpg",
        512,
    );
    sb.write_mock(
        "AppData/Local/ConnectedDevicesPlatform/L.user/ActivitiesCache.db",
        1024,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Terminal Server Client/Cache/bcache24.bmc",
        1024,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/History/History.IE5/index.dat",
        512,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/ConnectedSearch/History/query.lnk",
        256,
    );
    sb.write_mock("AppData/Local/Microsoft/Windows/WebCache/V0100042.log", 512);
    let protected_webcache_db = sb.write_mock(
        "AppData/Local/Microsoft/Windows/WebCache/WebCacheV01.dat",
        1024,
    );

    for target_id in [
        "priv_recent_docs",
        "priv_notifications",
        "priv_activity_history",
        "priv_rdp_cache",
        "priv_explorer_history",
    ] {
        assert_target_purge_cycle(target_id);
    }

    assert!(
        protected_webcache_db.exists(),
        "Active WebCacheV01.dat database must be preserved"
    );
}
