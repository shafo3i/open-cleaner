use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use super::CleanerModule;
use crate::core::cleaner::clean_directory_contents;
use crate::core::ui::render_progress;
use crate::core::{CleanMode, ModuleReport};

pub struct DevToolsCleaner;

impl DevToolsCleaner {
    pub fn new() -> Self {
        Self
    }

    fn get_env_path(var_name: &str) -> Option<PathBuf> {
        env::var_os(var_name).map(PathBuf::from)
    }

    /// 1. Node.js Package Caches (npm, pnpm, Yarn, Bun)
    pub fn clean_node_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let npm_cache = local_appdata.join("npm-cache");
            if npm_cache.exists() {
                for sub in ["_cacache", "_logs", "_npx"] {
                    let dir = npm_cache.join(sub);
                    if dir.exists() {
                        clean_directory_contents(
                            &dir,
                            true,
                            true,
                            &|_| true,
                            mode,
                            report,
                            "npm Package & Log Cache",
                        );
                    }
                }
            }

            let pnpm_cache = local_appdata.join("pnpm-cache");
            if pnpm_cache.exists() {
                clean_directory_contents(
                    &pnpm_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "pnpm Metadata & Tarball Cache",
                );
            }

            let yarn_cache = local_appdata.join("Yarn").join("Cache");
            if yarn_cache.exists() {
                clean_directory_contents(
                    &yarn_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Yarn Package Cache",
                );
            }
        }

        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            let bun_cache = userprofile.join(".bun").join("install").join("cache");
            if bun_cache.exists() {
                clean_directory_contents(
                    &bun_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Bun Package Install Cache",
                );
            }
        }
    }

    /// 2. Python Package Caches (pip, uv, Poetry)
    pub fn clean_python_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let python_dirs = [
                (
                    local_appdata.join("pip").join("Cache"),
                    "Python pip HTTP & Wheel Cache",
                ),
                (
                    local_appdata.join("uv").join("cache"),
                    "Astral uv Package Cache",
                ),
                (
                    local_appdata.join("pypoetry").join("Cache"),
                    "Poetry Package Cache",
                ),
            ];
            for (dir, label) in python_dirs {
                if dir.exists() {
                    clean_directory_contents(&dir, true, true, &|_| true, mode, report, label);
                }
            }
        }
    }

    /// 3. Java & Android Build Caches (Gradle Caches & Daemon Logs)
    pub fn clean_gradle_maven_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            let gradle_root = userprofile.join(".gradle");
            if gradle_root.exists() {
                let gradle_caches = gradle_root.join("caches");
                if gradle_caches.exists() {
                    clean_directory_contents(
                        &gradle_caches,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "Gradle Build & Artifact Cache",
                    );
                }

                let gradle_daemon = gradle_root.join("daemon");
                if gradle_daemon.exists() {
                    clean_directory_contents(
                        &gradle_daemon,
                        true,
                        true,
                        &|p| {
                            p.extension()
                                .and_then(|e| e.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("log"))
                                .unwrap_or(false)
                        },
                        mode,
                        report,
                        "Gradle Daemon Out Logs",
                    );
                }
            }
        }
    }

    /// 4. JetBrains IDE System Caches, JCEF WebView & Logs (preserves LocalHistory & plugins)
    pub fn clean_jetbrains_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let jb_root = local_appdata.join("JetBrains");
            if !jb_root.exists() {
                return;
            }

            let Ok(entries) = fs::read_dir(&jb_root) else {
                return;
            };
            for entry in entries.flatten() {
                let ide_dir = entry.path();
                if !ide_dir.is_dir() {
                    continue;
                }
                for sub in [
                    "caches",
                    "jcef_cache",
                    "log",
                    "tmp",
                    "_temp_",
                    "splash",
                    "event-log-data",
                    "junie-jcp-outbox",
                    "llm-jcp-outbox",
                ] {
                    let target = ide_dir.join(sub);
                    if target.exists() {
                        clean_directory_contents(
                            &target,
                            true,
                            true,
                            &|_| true,
                            mode,
                            report,
                            "JetBrains IDE System Cache & Logs",
                        );
                    }
                }
            }
        }
    }

    /// 5. .NET & NuGet HTTP v3 / Plugin Caches
    pub fn clean_nuget_dotnet_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let nuget_root = local_appdata.join("NuGet");
            for sub in ["v3-cache", "plugins-cache", "Cache"] {
                let dir = nuget_root.join(sub);
                if dir.exists() {
                    clean_directory_contents(
                        &dir,
                        true,
                        true,
                        &|_| true,
                        mode,
                        report,
                        "NuGet HTTP & Plugin Cache",
                    );
                }
            }

            let nuget_scratch = local_appdata.join("Temp").join("NuGetScratch");
            if nuget_scratch.exists() {
                clean_directory_contents(
                    &nuget_scratch,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "NuGet Scratch Cache",
                );
            }
        }
    }

    /// 6. VS Code & Cursor IDE Caches, CachedData & Logs
    pub fn clean_vscode_cursor_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(appdata) = Self::get_env_path("APPDATA") {
            for (editor_folder, label) in [
                ("Code", "VS Code Cache & Logs"),
                ("Code - Insiders", "VS Code Insiders Cache & Logs"),
                ("Cursor", "Cursor AI IDE Cache & Logs"),
            ] {
                let editor_root = appdata.join(editor_folder);
                if !editor_root.exists() {
                    continue;
                }
                for sub in [
                    "Cache",
                    "CachedData",
                    "Code Cache",
                    "GPUCache",
                    "DawnGraphiteCache",
                    "DawnWebGPUCache",
                    "logs",
                    "Crashpad\\reports",
                ] {
                    let dir = editor_root.join(sub);
                    if dir.exists() {
                        clean_directory_contents(&dir, true, true, &|_| true, mode, report, label);
                    }
                }
            }
        }
    }

    /// 7. Rust Cargo Registry Archive Cache (.crate tarballs)
    pub fn clean_cargo_rust_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            let cargo_cache = userprofile.join(".cargo").join("registry").join("cache");
            if cargo_cache.exists() {
                clean_directory_contents(
                    &cargo_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Rust Cargo Registry Crate Cache",
                );
            }
        }
    }

    /// 8. AI & Headless Browser Caches (HuggingFace Hub, Playwright, Puppeteer)
    pub fn clean_ai_models_cache(&self, mode: CleanMode, report: &mut ModuleReport) {
        if let Some(userprofile) = Self::get_env_path("USERPROFILE") {
            let hf_cache = userprofile.join(".cache").join("huggingface");
            if hf_cache.exists() {
                clean_directory_contents(
                    &hf_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "HuggingFace Hub Cache",
                );
            }

            let puppeteer_cache = userprofile.join(".cache").join("puppeteer");
            if puppeteer_cache.exists() {
                clean_directory_contents(
                    &puppeteer_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Puppeteer Browser Cache",
                );
            }
        }

        if let Some(local_appdata) = Self::get_env_path("LOCALAPPDATA") {
            let playwright_cache = local_appdata.join("ms-playwright");
            if playwright_cache.exists() {
                clean_directory_contents(
                    &playwright_cache,
                    true,
                    true,
                    &|_| true,
                    mode,
                    report,
                    "Playwright Headless Browser Cache",
                );
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
            "dev_node_cache" => {
                self.clean_node_cache(mode, report);
                true
            }
            "dev_python_cache" => {
                self.clean_python_cache(mode, report);
                true
            }
            "dev_gradle_maven_cache" => {
                self.clean_gradle_maven_cache(mode, report);
                true
            }
            "dev_jetbrains_cache" => {
                self.clean_jetbrains_cache(mode, report);
                true
            }
            "dev_nuget_dotnet_cache" => {
                self.clean_nuget_dotnet_cache(mode, report);
                true
            }
            "dev_vscode_cursor_cache" => {
                self.clean_vscode_cursor_cache(mode, report);
                true
            }
            "dev_cargo_rust_cache" => {
                self.clean_cargo_rust_cache(mode, report);
                true
            }
            "dev_ai_models_cache" => {
                self.clean_ai_models_cache(mode, report);
                true
            }
            _ => false,
        }
    }
}

impl CleanerModule for DevToolsCleaner {
    fn id(&self) -> u32 {
        6
    }

    fn name(&self) -> &'static str {
        "Developer & AI Toolchain Purifier"
    }

    fn description(&self) -> &'static str {
        "Purges package manager caches (npm/pnpm/pip/uv/Gradle/NuGet/Cargo), IDE caches (JetBrains/VS Code/Cursor), and AI/browser caches"
    }

    fn targets_summary(&self) -> &'static str {
        "npm/pnpm/Yarn, pip/uv, Gradle, JetBrains IDEs, NuGet, VS Code & Cursor, Cargo Registry, HuggingFace & Playwright"
    }

    fn safety_level(&self) -> &'static str {
        "SAFE // REBUILDABLE PACKAGE MANAGER & IDE CACHES"
    }

    fn execute(&self, mode: CleanMode) -> ModuleReport {
        let mut report = ModuleReport::new(self.name());
        let start_time = Instant::now();

        let tasks = [
            "Node.js Package Caches (npm, pnpm, Yarn, Bun)",
            "Python Package Caches (pip, uv, Poetry)",
            "Java & Android Build Caches (Gradle)",
            "JetBrains IDE System Caches & Logs",
            ".NET & NuGet HTTP / Plugin Caches",
            "VS Code & Cursor AI IDE Caches & Logs",
            "Rust Cargo Registry Crate Cache",
            "AI & Headless Browser Caches (HuggingFace, Playwright)",
        ];
        let total = tasks.len();

        render_progress(tasks[0], 1, total);
        self.clean_node_cache(mode, &mut report);

        render_progress(tasks[1], 2, total);
        self.clean_python_cache(mode, &mut report);

        render_progress(tasks[2], 3, total);
        self.clean_gradle_maven_cache(mode, &mut report);

        render_progress(tasks[3], 4, total);
        self.clean_jetbrains_cache(mode, &mut report);

        render_progress(tasks[4], 5, total);
        self.clean_nuget_dotnet_cache(mode, &mut report);

        render_progress(tasks[5], 6, total);
        self.clean_vscode_cursor_cache(mode, &mut report);

        render_progress(tasks[6], 7, total);
        self.clean_cargo_rust_cache(mode, &mut report);

        render_progress(tasks[7], 8, total);
        self.clean_ai_models_cache(mode, &mut report);

        report.duration_ms = start_time.elapsed().as_millis();
        report
    }
}
