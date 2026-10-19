use super::sandbox::{EnvSandbox, assert_target_purge_cycle};

#[test]
fn graphics_category_targets_scan_and_purge_correctly() {
    let sb = EnvSandbox::new("graphics");

    sb.write_mock(
        "AppData/Local/Microsoft/Windows/Explorer/thumbcache_256.db",
        1024,
    );
    sb.write_mock(
        "AppData/Local/Microsoft/Windows/Explorer/iconcache_32.db",
        512,
    );
    sb.write_mock("AppData/Local/D3DSCache/bb84c2/shader.val", 1024);
    sb.write_mock("AppData/Local/NVIDIA/DXCache/nv_shader.bin", 1024);
    sb.write_mock("AppData/Local/AMD/DxCache/amd_shader.bin", 1024);
    sb.write_mock("AppData/LocalLow/Intel/ShaderCache/intel_shader.bin", 1024);
    sb.write_mock("AppData/Local/VulkanCache/vk_pipeline.bin", 512);
    sb.write_mock(
        "AppData/Local/Packages/Microsoft.Windows.Photos_8wekyb3d8bbwe/TempState/preview.tmp",
        512,
    );
    sb.write_mock(
        "AppData/Local/Packages/Microsoft.Windows.Photos_8wekyb3d8bbwe/LocalState/MediaDb.v1.db",
        1024,
    );
    sb.write_mock(
        "AppData/Local/Packages/Microsoft.Windows.Photos_8wekyb3d8bbwe/LocalState/EBWebView/Default/Cache/Cache_Data/f_01",
        512,
    );

    for target_id in [
        "gfx_thumbnails",
        "gfx_directx_shaders",
        "gfx_gpu_shaders",
        "gfx_intel_vulkan_shaders",
        "gfx_windows_photos_cache",
    ] {
        assert_target_purge_cycle(target_id);
    }
}
