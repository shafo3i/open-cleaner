use std::fs;
use std::path::PathBuf;

use crate::core::CleanMode;
use crate::core::cleaner::{CleanResult, secure_delete_file};

fn temp_test_file(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("open-cleaner-{name}-{}", std::process::id()));
    let _ = fs::remove_file(&path);
    fs::write(&path, b"test content").expect("create test file");
    path
}

#[test]
fn dry_run_does_not_delete_file() {
    let path = temp_test_file("dry-run");
    assert!(matches!(
        secure_delete_file(&path, CleanMode::DryRun),
        CleanResult::DryRunFound(12)
    ));
    assert!(path.exists());
    fs::remove_file(path).expect("cleanup test file");
}

#[test]
fn standard_delete_removes_file() {
    let path = temp_test_file("standard-delete");
    assert!(matches!(
        secure_delete_file(&path, CleanMode::Standard),
        CleanResult::Cleaned(12)
    ));
    assert!(!path.exists());
}

#[test]
fn secure_zero_wipes_and_removes_file() {
    let path = temp_test_file("secure-zero");
    assert!(matches!(
        secure_delete_file(&path, CleanMode::SecureZero),
        CleanResult::Cleaned(12)
    ));
    assert!(!path.exists());
}
