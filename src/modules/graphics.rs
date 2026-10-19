use std::env;
use std::path::PathBuf;
use std::time::Instant;

use super::CleanerModule;
use crate::core::cleaner::clean_directory_contents;
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct GraphicsCleaner;

impl GraphicsCleaner {
    pub fn new() -> Self {
        Self
    }

    fn get_env_path(var_name: &str) -> Option<PathBuf> {
        env::var_os(var_name).map(PathBuf::from)
    }

    /// 1. Explorer Thumbnail & Icon Databases
    pub fn clean_thumbnails(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let explorer_dir = local_appdata
                .join("Microsoft")
                .join("Windows")
                .join("Explorer");

            if explorer_dir.exists() {
                clean_directory_contents(
                    &explorer_dir,
                    false,
                    true,
                    &|p| {
                        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                            let lower = name.to_ascii_lowercase();
                            (lower.starts_with("thumbcache_") || lower.starts_with("iconcache_"))
                                && lower.ends_with(".db")
                        } else {
                            false
                        }
                    },
                    mode,
                    report,
                    "Explorer Thumbnail & Icon Cache",
                );

                let thumb_to_delete = explorer_dir.join("ThumbCacheToDelete");
                if thumb_to_delete.exists() {
                    clean_directory_contents(
                        &thumb_to_delete,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Explorer ThumbCacheToDelete",
                    );
                }
            }
        }
    }

    /// 2. DirectX 11/12 Compiled Shader Cache
    pub fn clean_directx_shaders(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let d3d_cache = local_appdata.join("D3DSCache");
            if d3d_cache.exists() {
                clean_directory_contents(
                    &d3d_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "DirectX Shader Cache",
                );
            }
        }
    }

    /// 3. NVIDIA & AMD GPU Driver Shader Caches
    pub fn clean_gpu_shaders(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let shader_dirs = [
                (
                    local_appdata.join("NVIDIA").join("DXCache"),
                    "NVIDIA DXCache",
                ),
                (
                    local_appdata.join("NVIDIA").join("GLCache"),
                    "NVIDIA GLCache",
                ),
                (
                    local_appdata.join("NVIDIA Corporation").join("NV_Cache"),
                    "NVIDIA NV_Cache",
                ),
                (local_appdata.join("AMD").join("DxCache"), "AMD DxCache"),
                (local_appdata.join("AMD").join("GLCache"), "AMD GLCache"),
                (local_appdata.join("AMD").join("VkCache"), "AMD VkCache"),
            ];
            for (dir, label) in shader_dirs {
                if dir.exists() {
                    clean_directory_contents(&dir, true, true, &|_| true, mode, report, label);
                }
            }
        }

        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            let nv_low = userprofile
                .join("AppData")
                .join("LocalLow")
                .join("NVIDIA")
                .join("PerDriverVersion");
            for sub in ["DXCache", "GLCache"] {
                let dir = nv_low.join(sub);
                if dir.exists() {
                    clean_directory_contents(
                        &dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "NVIDIA PerDriver Shader Cache",
                    );
                }
            }
        }
    }

    /// 4. Intel GPU Shader Cache & Vulkan Pipeline Cache
    pub fn clean_intel_vulkan_shaders(&self, mode: CleanMode, report: &mut ModuleReport) {
        let mut dirs = Vec::new();
        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            dirs.push((
                userprofile
                    .join("AppData")
                    .join("LocalLow")
                    .join("Intel")
                    .join("ShaderCache"),
                "Intel LocalLow ShaderCache",
            ));
        }
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            dirs.push((
                local_appdata.join("Intel").join("ShaderCache"),
                "Intel Local ShaderCache",
            ));
            dirs.push((local_appdata.join("VulkanCache"), "Vulkan Pipeline Cache"));
        }
        if let Some(appdata) = Self::get_env_path("APPDATA") {
            dirs.push((
                appdata.join("Intel").join("ShaderCache"),
                "Intel Roaming ShaderCache",
            ));
        }
        if let Some(progdata) = Self::get_env_path("PROGRAMDATA") {
            dirs.push((
                progdata.join("Intel").join("ShaderCache"),
                "Intel ProgramData ShaderCache",
            ));
        }

        for (dir, label) in dirs {
            if dir.exists() {
                clean_directory_contents(&dir, true, true, &|_| true, mode, report, label);
            }
        }
    }

    /// 5. Windows Photos & Media Engine Cache
    pub fn clean_windows_photos_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let photos_pkg = local_appdata
                .join("Packages")
                .join("Microsoft.Windows.Photos_8wekyb3d8bbwe");
            if !photos_pkg.exists() {
                return;
            }

            let temp_state = photos_pkg.join("TempState");
            if temp_state.exists() {
                clean_directory_contents(
                    &temp_state,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Windows Photos TempState",
                );
            }

            let local_state = photos_pkg.join("LocalState");
            if local_state.exists() {
                clean_directory_contents(
                    &local_state,
                    false,
                    true,
                    &|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                let lower = name.to_ascii_lowercase();
                                lower.ends_with(".db")
                                    || lower.ends_with(".db-wal")
                                    || lower.ends_with(".db-shm")
                                    || lower.ends_with(".sqlite-wal")
                                    || lower.ends_with(".sqlite-shm")
                                    || lower.ends_with(".etl")
                                    || lower.ends_with(".bak")
                                    || lower.ends_with(".log")
                                    || lower.ends_with(".old")
                            })
                            .unwrap_or(false)
                    },
                    mode,
                    report,
                    "Windows Photos LocalState DB & Logs",
                );

                for tile_sub in [
                    "PhotosAppTile",
                    "PhotosAppBackground",
                    "PhotosAppLockscreen",
                ] {
                    let tile_dir = local_state.join(tile_sub);
                    if tile_dir.exists() {
                        clean_directory_contents(
                            &tile_dir,
                            true,
                            true,
                            &|p| {
                                p.extension()
                                    .and_then(|e| e.to_str())
                                    .map(|ext| {
                                        ext.eq_ignore_ascii_case("jpg")
                                            || ext.eq_ignore_ascii_case("png")
                                    })
                                    .unwrap_or(false)
                            },
                            mode,
                            report,
                            "Windows Photos Tile Cache",
                        );
                    }
                }

                let ebwebview = local_state.join("EBWebView");
                if ebwebview.exists() {
                    for root_shader in [
                        "GrShaderCache",
                        "ShaderCache",
                        "GraphiteDawnCache",
                        "DawnGraphiteCache",
                        "DawnWebGPUCache",
                    ] {
                        let dir = ebwebview.join(root_shader);
                        if dir.exists() {
                            clean_directory_contents(
                                &dir,
                                true,
                                true,
                                &|_| true,
                                mode,
                                report,
                                "Windows Photos WebView Shader Cache",
                            );
                        }
                    }

                    let default_prof = ebwebview.join("Default");
                    if default_prof.exists() {
                        for c_dir in [
                            "Cache",
                            "Code Cache",
                            "GPUCache",
                            "DawnGraphiteCache",
                            "DawnWebGPUCache",
                            "Service Worker\\CacheStorage",
                            "Service Worker\\ScriptCache",
                        ] {
                            let target = default_prof.join(c_dir);
                            if target.exists() {
                                clean_directory_contents(
                                    &target,
                                    true,
                                    true,
                                    &|_| true,
                                    mode,
                                    report,
                                    "Windows Photos Media & WebView Cache",
                                );
                            }
                        }
                    }
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
            "gfx_thumbnails" | "fs_thumbnails" => {
                self.clean_thumbnails(mode, report);
                true
            }
            "gfx_directx_shaders" | "fs_directx_shaders" => {
                self.clean_directx_shaders(mode, report);
                true
            }
            "gfx_gpu_shaders" | "fs_gpu_shaders" => {
                self.clean_gpu_shaders(mode, report);
                true
            }
            "gfx_intel_vulkan_shaders" => {
                self.clean_intel_vulkan_shaders(mode, report);
                true
            }
            "gfx_windows_photos_cache" => {
                self.clean_windows_photos_cache(mode, report);
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for GraphicsCleaner {
    fn id(&self) -> u32 {
        4
    }

    fn name(&self) -> &'static str {
        "Graphics, Shader & Media Purifier"
    }

    fn description(&self) -> &'static str {
        "Purges DirectX, NVIDIA, AMD, Intel & Vulkan shader caches, Explorer thumbnails, and Windows Photos media caches"
    }

    fn targets_summary(&self) -> &'static str {
        "Explorer Thumbnails, D3DSCache, NVIDIA/AMD/Intel/Vulkan Shaders, Windows Photos Media Cache"
    }

    fn safety_level(&self) -> &'static str {
        "SAFE // REBUILDABLE GPU SHADER & THUMBNAIL CACHES"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let tasks = [
            "Explorer Thumbnail & Icon Caches",
            "DirectX Compiled Shader Cache",
            "NVIDIA & AMD GPU Driver Shaders",
            "Intel ShaderCache & Vulkan Pipeline Cache",
            "Windows Photos & Media Engine Cache",
        ];
        let total = tasks.len();

        render_progress(tasks[0], 1, total);
        self.clean_thumbnails(mode, &mut report);

        render_progress(tasks[1], 2, total);
        self.clean_directx_shaders(mode, &mut report);

        render_progress(tasks[2], 3, total);
        self.clean_gpu_shaders(mode, &mut report);

        render_progress(tasks[3], 4, total);
        self.clean_intel_vulkan_shaders(mode, &mut report);

        render_progress(tasks[4], 5, total);
        self.clean_windows_photos_cache(mode, &mut report);

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
