pub mod apps;
pub mod browser;
pub mod devtools;
pub mod filesystem;
pub mod graphics;
pub mod privacy;
pub mod registry;
pub mod rules;

use crate::core::{CleanMode, ModuleReport};

#[allow(dead_code)]
pub trait CleanerModule {
    fn id(&self) -> u32;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn targets_summary(&self) -> &'static str;
    fn safety_level(&self) -> &'static str;
    fn execute(&self, mode: CleanMode) -> ModuleReport;
}

pub fn try_execute_target(target_id: &str, mode: CleanMode) -> Option<ModuleReport> {
    let mut report = ModuleReport::new(target_id);
    let fs = filesystem::FileSystemCleaner::new();
    if fs.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    let br = browser::BrowserPrivacyEraser::new();
    if br.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    let priv_cleaner = privacy::PrivacyCleaner::new();
    if priv_cleaner.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    let gfx = graphics::GraphicsCleaner::new();
    if gfx.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    let app_cleaner = apps::AppCacheCleaner::new();
    if app_cleaner.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    let dev_cleaner = devtools::DevToolsCleaner::new();
    if dev_cleaner.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    let reg = registry::RegistryAnalyzer::new();
    if reg.clean_target(target_id, mode, &mut report) {
        return Some(report);
    }
    if rules::execute_target(target_id, mode, &mut report) {
        return Some(report);
    }
    None
}

pub fn execute_target(target_id: &str, mode: CleanMode) -> ModuleReport {
    try_execute_target(target_id, mode).unwrap_or_else(|| ModuleReport::new(target_id))
}
