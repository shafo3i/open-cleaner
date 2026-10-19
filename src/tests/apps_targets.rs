use super::sandbox::{EnvSandbox, assert_target_purge_cycle};

#[test]
fn apps_category_targets_scan_and_purge_correctly() {
    let sb = EnvSandbox::new("apps");

    sb.write_mock("AppData/Local/Steam/htmlcache/Cache/Cache_Data/f_01", 512);
    sb.write_mock(
        "Program Files (x86)/Steam/appcache/httpcache/00/http.bin",
        512,
    );
    sb.write_mock("Program Files (x86)/Steam/logs/content_log.txt", 512);
    sb.write_mock("Program Files (x86)/Steam/dumps/crash.dmp", 512);
    sb.write_mock(
        "AppData/Local/EpicGamesLauncher/Saved/webcache_4430/Cache/Cache_Data/f_01",
        512,
    );
    sb.write_mock(
        "AppData/Local/EpicGamesLauncher/Saved/Logs/EpicGamesLauncher.log",
        512,
    );
    sb.write_mock("AppData/Roaming/Zoom/logs/zoom_crash.log", 512);
    sb.write_mock(
        "AppData/Roaming/Zoom/data/WebviewCacheX64/user1/EBWebView/Default/Cache/Cache_Data/f_01",
        512,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/OneDrive/logs/Common/DeviceHealthSummaryConfiguration.ini.log",
        512,
    );
    sb.write_mock("AppData/Local/Microsoft/Office/16.0/Wef/wef_cache.bin", 512);
    sb.write_mock("AppData/Roaming/discord/Cache/Cache_Data/f_001", 1024);
    sb.write_mock("AppData/Roaming/discord/GPUCache/data_0", 512);
    sb.write_mock(
        "AppData/Local/Packages/MSTeams_8wekyb3d8bbwe/LocalCache/Microsoft/MSTeams/EBWebView/WV2Profile_tfw/Cache/Cache_Data/f_01",
        1024,
    );
    sb.write_mock("AppData/Roaming/Slack/Cache/Cache_Data/f_01", 512);
    sb.write_mock("AppData/Roaming/Slack/logs/default.log", 512);
    sb.write_mock("AppData/Local/Spotify/Data/audio_chunk.bnk", 1024);
    sb.write_mock("AppData/Local/Spotify/Browser/Cache/Cache_Data/f_01", 512);
    sb.write_mock(
        "AppData/Local/Packages/MicrosoftWindows.Client.WebExperience_cw5n1h2txyewy/LocalState/EBWebView/Default/Cache/Cache_Data/f_01",
        512,
    );
    sb.write_mock(
        "AppData/Local/Packages/Microsoft.WindowsStore_8wekyb3d8bbwe/LocalCache/store_tile.bin",
        512,
    );

    for target_id in [
        "app_steam_cache",
        "app_epic_games_cache",
        "app_zoom_cache",
        "app_onedrive_office_logs",
        "app_discord_cache",
        "app_teams_slack_cache",
        "app_spotify_cache",
        "app_windows_widgets_store",
    ] {
        assert_target_purge_cycle(target_id);
    }
}
