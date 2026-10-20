use std::thread;
use std::time::Duration;

use crate::ui::app::{SystemCleanerApp, TargetReport, WorkerResult};
use crate::ui::model::TargetStatus;

#[test]
fn purge_preempts_inflight_scan_without_blocking() {
    let mut app = SystemCleanerApp::default();
    // Simulate an active background startup scan
    app.worker_gen = 1;
    app.metrics.is_busy = true;
    app.is_purging = false;

    // Triggering a purge while scan is busy MUST preempt the scan, not return early
    app.run_purge(vec!["__test_noop_target__".to_string()]);
    assert_eq!(
        app.worker_gen, 2,
        "run_purge must increment worker_gen to invalidate in-flight scan"
    );
    assert!(
        app.is_purging,
        "run_purge must set is_purging = true even when metrics.is_busy was true"
    );
    assert!(app.metrics.is_busy);

    // Wait for the lightweight noop worker thread to finish and process its message
    for _ in 0..50 {
        app.check_worker_messages();
        if !app.is_purging {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!app.is_purging, "CleanFinished must clear is_purging flag");
    assert!(
        !app.metrics.is_busy,
        "CleanFinished must clear metrics.is_busy flag"
    );
}

#[test]
fn stale_scan_generation_is_ignored_after_preemption() {
    let mut app = SystemCleanerApp::default();
    app.worker_gen = 5;
    app.is_purging = true;
    app.metrics.is_busy = true;
    app.metrics.status_text = "PURGING 8 SELECTED TARGETS...".to_string();

    // Send a stale ScanFinished from generation 4
    app.tx
        .send(WorkerResult::ScanFinished {
            scan_gen: 4,
            total_reclaimable: 999_999,
            total_scanned: 500,
            total_locked: 0,
            duration: 1.2,
            target_reports: vec![TargetReport {
                id: "dev_node_cache".to_string(),
                bytes: 999_999,
                items: 500,
                remaining_bytes: 999_999,
                remaining_items: 500,
                locked: 0,
                permission_denied: 0,
                failed: 0,
            }],
        })
        .expect("send stale ScanFinished");

    app.check_worker_messages();

    // Stale scan must be ignored
    assert!(app.is_purging);
    assert!(app.metrics.is_busy);
    assert_eq!(app.metrics.status_text, "PURGING 8 SELECTED TARGETS...");
}

#[test]
fn registry_and_file_targets_transition_to_cleaned_status() {
    let mut app = SystemCleanerApp::default();
    app.is_purging = true;
    app.metrics.is_busy = true;

    app.tx
        .send(WorkerResult::CleanFinished {
            total_reclaimed: 1024 * 1024,
            total_cleaned: 16,
            total_locked: 0,
            duration: 0.42,
            target_reports: vec![
                TargetReport {
                    id: "reg_app_paths".to_string(),
                    bytes: 0,
                    items: 6,
                    remaining_bytes: 0,
                    remaining_items: 0,
                    locked: 0,
                    permission_denied: 0,
                    failed: 0,
                },
                TargetReport {
                    id: "dev_node_cache".to_string(),
                    bytes: 1024 * 1024,
                    items: 10,
                    remaining_bytes: 0,
                    remaining_items: 0,
                    locked: 0,
                    permission_denied: 0,
                    failed: 0,
                },
            ],
        })
        .expect("send CleanFinished");

    app.check_worker_messages();

    assert!(!app.is_purging);
    assert!(!app.metrics.is_busy);

    let reg_item = app
        .targets
        .iter()
        .find(|t| t.id == "reg_app_paths")
        .expect("reg_app_paths target");
    assert_eq!(
        reg_item.status,
        TargetStatus::Cleaned,
        "Purged registry target with 0 remaining keys must transition to TargetStatus::Cleaned"
    );
    assert_eq!(reg_item.size, "0 keys");
    assert_eq!(reg_item.count, "0 keys");

    let dev_item = app
        .targets
        .iter()
        .find(|t| t.id == "dev_node_cache")
        .expect("dev_node_cache target");
    assert_eq!(
        dev_item.status,
        TargetStatus::Cleaned,
        "Purged dev target with 0 remaining items must transition to TargetStatus::Cleaned"
    );
    assert_eq!(dev_item.size, "0 B");
    assert_eq!(dev_item.count, "0 files");
}

#[test]
fn uninstaller_progress_messages_update_bar_and_completion_modal() {
    use crate::uninstaller::{UninstallPhase, UninstallProgress, UninstallReport};

    let mut app = SystemCleanerApp::default();
    app.uninstaller_state.is_uninstalling = true;

    app.tx
        .send(WorkerResult::AppUninstallProgress {
            progress: UninstallProgress {
                app_id: "Win32User::TestApp".to_string(),
                app_name: "Nebula Studio".to_string(),
                phase: UninstallPhase::RunningNativeUninstaller,
                fraction: 0.28,
                detail: "Running native uninstaller...".to_string(),
            },
        })
        .expect("send AppUninstallProgress");

    app.check_worker_messages();

    let prog = app
        .uninstaller_state
        .active_progress
        .as_ref()
        .expect("active_progress populated");
    assert!((prog.fraction - 0.28).abs() < f32::EPSILON);
    assert_eq!(prog.phase, UninstallPhase::RunningNativeUninstaller);
    assert!(!app.uninstaller_state.progress_log.is_empty());

    app.tx
        .send(WorkerResult::AppUninstallFinished {
            report: UninstallReport {
                app_name: "Nebula Studio".to_string(),
                uninstaller_ran: true,
                uninstaller_success: true,
                uninstaller_exit_code: Some(0),
                uninstaller_command: "unins000.exe".to_string(),
                remnants_found: 4,
                remnants_removed: 4,
                remnants_failed: 0,
                bytes_reclaimed: 2048,
                removed_locations: vec!["C:\\Temp\\Nebula".to_string()],
                error_message: None,
            },
            updated_apps: Vec::new(),
        })
        .expect("send AppUninstallFinished");

    app.check_worker_messages();

    assert!(!app.uninstaller_state.is_uninstalling);
    assert!(
        app.uninstaller_state.show_completion_modal,
        "Modal must stay open in completion state until user clicks DONE / CLOSE"
    );
    assert!(app.uninstaller_state.last_report.is_some());
}

