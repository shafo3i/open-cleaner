use super::{CleanMode, ModuleReport, SuiteReport};
use std::io::{self, Write};

#[allow(dead_code)]
pub struct TerminalTheme;

#[allow(dead_code)]
impl TerminalTheme {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const DIM: &'static str = "\x1b[2m";

    // Palette
    pub const BG_CHARCOAL: &'static str = "\x1b[48;2;18;18;20m";
    pub const FG_WHITE: &'static str = "\x1b[38;2;245;245;250m";
    pub const FG_MUTED: &'static str = "\x1b[38;2;130;135;150m";
    pub const FG_BORDER: &'static str = "\x1b[38;2;60;65;80m";

    // Accents
    pub const ACCENT_CYAN: &'static str = "\x1b[38;2;0;229;255m";
    pub const ACCENT_EMERALD: &'static str = "\x1b[38;2;0;230;118m";
    pub const ACCENT_AMBER: &'static str = "\x1b[38;2;255;171;0m";
    pub const ACCENT_PURPLE: &'static str = "\x1b[38;2;187;134;252m";
    pub const ACCENT_RED: &'static str = "\x1b[38;2;255;82;82m";
}

/// Formats raw byte counts into human-readable representation
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Initializes Windows Virtual Terminal Processing for ANSI colors if available
pub fn init_terminal() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetStdHandle(nStdHandle: u32) -> isize;
            fn GetConsoleMode(hConsoleHandle: isize, lpMode: *mut u32) -> i32;
            fn SetConsoleMode(hConsoleHandle: isize, dwMode: u32) -> i32;
        }
        const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0u32;
            if GetConsoleMode(handle, &mut mode) != 0 {
                SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }
}

/// Renders the CLI header banner
#[allow(clippy::print_literal)]
pub fn render_launcher_header(mode: CleanMode) {
    let mode_str = mode.display_name();
    let mode_color = match mode {
        CleanMode::DryRun => TerminalTheme::ACCENT_CYAN,
        CleanMode::Standard => TerminalTheme::ACCENT_EMERALD,
        CleanMode::SecureZero => TerminalTheme::ACCENT_AMBER,
    };

    println!();
    println!(
        "{}{}{}",
        TerminalTheme::FG_BORDER,
        "╭─────────────────────────────────────────────────────────────────────────────╮",
        TerminalTheme::RESET
    );
    println!(
        "{}│  {}{}◈ SYSTEM CLEANER ◈{}                                                  {}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::BOLD,
        TerminalTheme::ACCENT_CYAN,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {}High-Performance Native Rust Purifier & Privacy Engine{}                 {}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::FG_MUTED,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}├─────────────────────────────────────────────────────────────────────────────┤{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  Operating Mode: {}{}{:<32}{} Architecture: {}x86_64 / WinNT{} {}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::BOLD,
        mode_color,
        mode_str,
        TerminalTheme::RESET,
        TerminalTheme::FG_WHITE,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}{}{}",
        TerminalTheme::FG_BORDER,
        "╰─────────────────────────────────────────────────────────────────────────────╯",
        TerminalTheme::RESET
    );
    println!();
}

/// Renders a module status card
pub fn render_module_card(id: u32, title: &str, status: &str, targets: &str, safety_level: &str) {
    let status_color = if status.contains("ACTIVE") || status.contains("RUNNING") {
        TerminalTheme::ACCENT_AMBER
    } else if status.contains("READY") || status.contains("COMPLETE") {
        TerminalTheme::ACCENT_EMERALD
    } else {
        TerminalTheme::ACCENT_PURPLE
    };

    println!(
        "{}┌─ [{}]{} {}{}{:<38}{} {}[ {} ]{} ─┐{}",
        TerminalTheme::FG_BORDER,
        id,
        TerminalTheme::RESET,
        TerminalTheme::BOLD,
        TerminalTheme::FG_WHITE,
        title,
        TerminalTheme::RESET,
        status_color,
        status,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {}Targets:  {}{:<61}{}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::FG_MUTED,
        TerminalTheme::FG_WHITE,
        targets,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {}Security: {}{:<61}{}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::FG_MUTED,
        TerminalTheme::ACCENT_CYAN,
        safety_level,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}└─────────────────────────────────────────────────────────────────────────────┘{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
}

/// Renders a dynamic progress bar
pub fn render_progress(action: &str, current: usize, total: usize) {
    let width: usize = 30;
    let ratio = if total > 0 {
        current as f64 / total as f64
    } else {
        1.0
    };
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    let pct = (ratio * 100.0) as u32;

    print!(
        "\r  {}[{}{}]{} {:>3}%  {}{:<38}{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::ACCENT_CYAN,
        bar,
        TerminalTheme::FG_BORDER,
        pct,
        TerminalTheme::FG_WHITE,
        action,
        TerminalTheme::RESET
    );
    let _ = io::stdout().flush();
}

/// Renders a single module execution summary card
pub fn render_module_summary(report: &ModuleReport) {
    println!();
    println!(
        "  {}◈ {} Execution Completed:{}",
        TerminalTheme::BOLD,
        report.module_name,
        TerminalTheme::RESET
    );
    println!(
        "    • Scanned Items:     {}{}{}",
        TerminalTheme::BOLD,
        report.items_scanned,
        TerminalTheme::RESET
    );
    println!(
        "    • Cleaned Items:     {}{}{}{}",
        TerminalTheme::BOLD,
        TerminalTheme::ACCENT_EMERALD,
        report.items_cleaned,
        TerminalTheme::RESET
    );
    println!(
        "    • Space Reclaimed:   {}{}{}{}",
        TerminalTheme::BOLD,
        TerminalTheme::ACCENT_CYAN,
        format_bytes(report.bytes_reclaimed),
        TerminalTheme::RESET
    );
    if report.items_skipped_locked > 0 {
        println!(
            "    • Skipped (In-Use):  {}{}{}{}",
            TerminalTheme::BOLD,
            TerminalTheme::ACCENT_AMBER,
            report.items_skipped_locked,
            TerminalTheme::RESET
        );
    }
    if report.items_skipped_permission > 0 {
        println!(
            "    • Skipped (Perm):    {}{}{}{}",
            TerminalTheme::BOLD,
            TerminalTheme::ACCENT_RED,
            report.items_skipped_permission,
            TerminalTheme::RESET
        );
    }
    println!(
        "    • Execution Time:    {}{:.2} ms{}",
        TerminalTheme::DIM,
        report.duration_ms,
        TerminalTheme::RESET
    );
}

/// Renders the overall system cleaner suite report
#[allow(clippy::print_literal)]
pub fn render_suite_summary(suite: &SuiteReport, mode: CleanMode) {
    println!();
    println!(
        "{}{}{}",
        TerminalTheme::FG_BORDER,
        "╭─────────────────────────────────────────────────────────────────────────────╮",
        TerminalTheme::RESET
    );
    println!(
        "{}│  {}{}◈ SYSTEM CLEANER // PURGE & AUDIT REPORT ◈{}                       {}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::BOLD,
        TerminalTheme::ACCENT_EMERALD,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}├─────────────────────────────────────────────────────────────────────────────┤{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );

    let action_label = if mode.is_dry_run() {
        "Estimated Space To Free:"
    } else {
        "Total Storage Reclaimed:"
    };
    println!(
        "{}│  {:<26} {}{:>14}{}                            {}│{}",
        TerminalTheme::FG_BORDER,
        action_label,
        TerminalTheme::BOLD,
        format_bytes(suite.total_bytes_reclaimed),
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {:<26} {}{:>14}{}                            {}│{}",
        TerminalTheme::FG_BORDER,
        "Total Items Processed:",
        TerminalTheme::BOLD,
        suite.total_scanned,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {:<26} {}{:>14}{}                            {}│{}",
        TerminalTheme::FG_BORDER,
        "Total Items Unlinked:",
        TerminalTheme::ACCENT_EMERALD,
        suite.total_cleaned,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {:<26} {}{:>14}{}                            {}│{}",
        TerminalTheme::FG_BORDER,
        "Active Locks / Skipped:",
        TerminalTheme::ACCENT_AMBER,
        suite.total_skipped,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  {:<26} {}{:>11.2} ms{}                            {}│{}",
        TerminalTheme::FG_BORDER,
        "Total Processing Time:",
        TerminalTheme::DIM,
        suite.total_duration_ms,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}├─────────────────────────────────────────────────────────────────────────────┤{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}│  Status: {}{}CLEAN OPERATION COMPLETE (0 Fatal Errors){}                        {}│{}",
        TerminalTheme::FG_BORDER,
        TerminalTheme::BOLD,
        TerminalTheme::ACCENT_CYAN,
        TerminalTheme::RESET,
        TerminalTheme::FG_BORDER,
        TerminalTheme::RESET
    );
    println!(
        "{}{}{}",
        TerminalTheme::FG_BORDER,
        "╰─────────────────────────────────────────────────────────────────────────────╯",
        TerminalTheme::RESET
    );
    println!();
}
