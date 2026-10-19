use super::sandbox::{EnvSandbox, assert_target_purge_cycle};

#[test]
fn browser_category_targets_scan_and_purge_correctly() {
    let sb = EnvSandbox::new("browsers");

    for chromium_rel in [
        "AppData/Local/Google/Chrome/User Data",
        "AppData/Local/Microsoft/Edge/User Data",
        "AppData/Local/BraveSoftware/Brave-Browser/User Data",
        "AppData/Local/Vivaldi/User Data",
        "AppData/Roaming/Opera Software/Opera Stable",
    ] {
        sb.write_mock(format!("{chromium_rel}/ShaderCache/data_0"), 512);
        sb.write_mock(
            format!("{chromium_rel}/Default/Cache/Cache_Data/f_0001"),
            1024,
        );
        sb.write_mock(format!("{chromium_rel}/Default/Code Cache/js/index"), 512);
        sb.write_mock(format!("{chromium_rel}/Default/GPUCache/data_1"), 512);
        sb.write_mock(format!("{chromium_rel}/Default/Network/Cookies"), 512);
        sb.write_mock(
            format!("{chromium_rel}/Default/Network/Cookies-journal"),
            256,
        );
        sb.write_mock(format!("{chromium_rel}/Default/History"), 1024);
        sb.write_mock(format!("{chromium_rel}/Default/Visited Links"), 512);
    }
    let protected_chrome_bookmarks = sb.write_mock(
        "AppData/Local/Google/Chrome/User Data/Default/Bookmarks",
        512,
    );

    sb.write_mock(
        "AppData/Local/Mozilla/Firefox/Profiles/abcd.default-release/cache2/entries/entry1",
        1024,
    );
    sb.write_mock(
        "AppData/Local/Mozilla/Firefox/Profiles/abcd.default-release/startupCache/startup.bin",
        512,
    );
    sb.write_mock(
        "AppData/Roaming/Mozilla/Firefox/Profiles/abcd.default-release/cookies.sqlite",
        512,
    );
    sb.write_mock(
        "AppData/Roaming/Mozilla/Firefox/Profiles/abcd.default-release/places.sqlite",
        1024,
    );
    sb.write_mock(
        "AppData/Roaming/Mozilla/Firefox/Profiles/abcd.default-release/sessionstore-backups/recovery.jsonlz4",
        512,
    );
    let protected_firefox_passwords = sb.write_mock(
        "AppData/Roaming/Mozilla/Firefox/Profiles/abcd.default-release/key4.db",
        512,
    );

    for target_id in [
        "br_chrome_cache",
        "br_chrome_cookies",
        "br_chrome_history",
        "br_edge_cache",
        "br_edge_cookies",
        "br_edge_history",
        "br_firefox_cache",
        "br_firefox_cookies",
        "br_firefox_history",
        "br_opera_cache",
        "br_opera_cookies",
        "br_opera_history",
        "br_brave_cache",
        "br_brave_cookies",
        "br_brave_history",
        "br_vivaldi_cache",
        "br_vivaldi_cookies",
        "br_vivaldi_history",
    ] {
        assert_target_purge_cycle(target_id);
    }

    assert!(
        protected_chrome_bookmarks.exists(),
        "Chrome Bookmarks must be preserved"
    );
    assert!(
        protected_firefox_passwords.exists(),
        "Firefox key4.db password database must be preserved"
    );
}
