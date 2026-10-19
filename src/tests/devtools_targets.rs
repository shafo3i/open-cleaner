use super::sandbox::{EnvSandbox, assert_target_purge_cycle};

#[test]
fn devtools_category_targets_scan_and_purge_correctly() {
    let sb = EnvSandbox::new("devtools");

    sb.write_mock(
        "AppData/Local/npm-cache/_cacache/content-v2/sha512/pkg.tgz",
        1024,
    );
    sb.write_mock("AppData/Local/pnpm-cache/v3/files/00/pkg.tar", 1024);
    sb.write_mock(".bun/install/cache/pkg.bun", 512);
    sb.write_mock("AppData/Local/pip/Cache/http/wheel.whl", 1024);
    sb.write_mock("AppData/Local/uv/cache/archive-v0/pkg.whl", 1024);
    sb.write_mock(".gradle/caches/modules-2/files-2.1/jar.bin", 1024);
    sb.write_mock(".gradle/daemon/8.5/daemon-1234.out.log", 512);
    let protected_gradle_props = sb.write_mock(".gradle/gradle.properties", 256);
    sb.write_mock(
        "AppData/Local/JetBrains/RustRover2024.3/caches/attrib.dat",
        1024,
    );
    sb.write_mock("AppData/Local/JetBrains/RustRover2024.3/log/idea.log", 512);
    let protected_jetbrains_history = sb.write_mock(
        "AppData/Local/JetBrains/RustRover2024.3/LocalHistory/changes.storageData",
        1024,
    );
    sb.write_mock("AppData/Local/NuGet/v3-cache/index.json", 512);
    sb.write_mock("AppData/Roaming/Code/Cache/Cache_Data/f_01", 1024);
    sb.write_mock("AppData/Roaming/Code/CachedData/v8_code.bin", 512);
    sb.write_mock("AppData/Roaming/Cursor/Cache/Cache_Data/f_01", 1024);
    let protected_vscode_settings = sb.write_mock("AppData/Roaming/Code/User/settings.json", 256);
    sb.write_mock(
        ".cargo/registry/cache/index.crates.io-6f17d22bba15001f/serde-1.0.210.crate",
        1024,
    );
    sb.write_mock(
        ".cache/huggingface/hub/models--bert/blobs/weights.bin",
        2048,
    );
    sb.write_mock("AppData/Local/ms-playwright/chromium-1140/chrome.dll", 1024);

    for target_id in [
        "dev_node_cache",
        "dev_python_cache",
        "dev_gradle_maven_cache",
        "dev_jetbrains_cache",
        "dev_nuget_dotnet_cache",
        "dev_vscode_cursor_cache",
        "dev_cargo_rust_cache",
        "dev_ai_models_cache",
    ] {
        assert_target_purge_cycle(target_id);
    }

    assert!(
        protected_gradle_props.exists(),
        "Gradle gradle.properties must be preserved"
    );
    assert!(
        protected_jetbrains_history.exists(),
        "JetBrains LocalHistory must be preserved"
    );
    assert!(
        protected_vscode_settings.exists(),
        "VS Code User/settings.json must be preserved"
    );
}
