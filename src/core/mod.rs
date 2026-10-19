pub mod cleaner;
pub mod ui;

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanMode {
    DryRun,
    Standard,
    SecureZero,
}

impl CleanMode {
    pub fn is_dry_run(&self) -> bool {
        matches!(self, CleanMode::DryRun)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CleanMode::DryRun => "PREVIEW (DRY-RUN)",
            CleanMode::Standard => "STANDARD DELETE",
            CleanMode::SecureZero => "SECURE ZERO-OVERWRITE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStatus {
    Scanned,
    Cleaned,
    SkippedLocked,
    SkippedPermission,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct CleanItem {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub status: ItemStatus,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleReport {
    pub module_name: String,
    pub items_scanned: usize,
    pub items_cleaned: usize,
    pub items_skipped_locked: usize,
    pub items_skipped_permission: usize,
    pub items_failed: usize,
    pub bytes_reclaimed: u64,
    pub duration_ms: u128,
    pub details: Vec<CleanItem>,
}

impl ModuleReport {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            module_name: name.into(),
            ..Default::default()
        }
    }

    pub fn record_scanned(&mut self, path: PathBuf, size: u64, desc: impl Into<String>) {
        self.items_scanned += 1;
        self.bytes_reclaimed += size;
        self.details.push(CleanItem {
            path,
            size_bytes: size,
            status: ItemStatus::Scanned,
            description: desc.into(),
        });
    }

    pub fn record_cleaned(&mut self, path: PathBuf, size: u64, desc: impl Into<String>) {
        self.items_scanned += 1;
        self.items_cleaned += 1;
        self.bytes_reclaimed += size;
        self.details.push(CleanItem {
            path,
            size_bytes: size,
            status: ItemStatus::Cleaned,
            description: desc.into(),
        });
    }

    pub fn record_skipped_locked(&mut self, path: PathBuf, size: u64, desc: impl Into<String>) {
        self.items_skipped_locked += 1;
        self.details.push(CleanItem {
            path,
            size_bytes: size,
            status: ItemStatus::SkippedLocked,
            description: desc.into(),
        });
    }

    pub fn record_skipped_permission(&mut self, path: PathBuf, size: u64, desc: impl Into<String>) {
        self.items_skipped_permission += 1;
        self.details.push(CleanItem {
            path,
            size_bytes: size,
            status: ItemStatus::SkippedPermission,
            description: desc.into(),
        });
    }

    pub fn record_failed(
        &mut self,
        path: PathBuf,
        size: u64,
        err: String,
        desc: impl Into<String>,
    ) {
        self.items_scanned += 1;
        self.items_failed += 1;
        self.details.push(CleanItem {
            path,
            size_bytes: size,
            status: ItemStatus::Failed(err),
            description: desc.into(),
        });
    }
}

#[derive(Debug, Clone, Default)]
pub struct SuiteReport {
    pub module_reports: Vec<ModuleReport>,
    pub total_scanned: usize,
    pub total_cleaned: usize,
    pub total_skipped: usize,
    pub total_bytes_reclaimed: u64,
    pub total_duration_ms: u128,
}

impl SuiteReport {
    pub fn add_report(&mut self, report: ModuleReport) {
        self.total_scanned += report.items_scanned;
        self.total_cleaned += report.items_cleaned;
        self.total_skipped += report.items_skipped_locked + report.items_skipped_permission;
        self.total_bytes_reclaimed += report.bytes_reclaimed;
        self.total_duration_ms += report.duration_ms;
        self.module_reports.push(report);
    }
}
