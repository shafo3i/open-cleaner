#![cfg_attr(not(feature = "cli"), windows_subsystem = "windows")]

pub mod bloat;
mod core;
mod modules;
#[cfg(test)]
mod tests;
mod ui;
pub mod uninstaller;
mod updater;

use core::ui::{
    TerminalTheme, init_terminal, render_launcher_header, render_module_card,
    render_module_summary, render_suite_summary,
};
use core::{CleanMode, SuiteReport};
use modules::CleanerModule;
use modules::browser::BrowserPrivacyEraser;
use modules::filesystem::FileSystemCleaner;
use modules::registry::RegistryAnalyzer;
use std::env;
use std::io::{self, BufRead, Write};

fn print_help() {
    println!(
        r#"
SYSTEM CLEANER // Open-Cleaner Modular System Suite
Usage: open-cleaner [OPTIONS]

Options:
  --gui                     Launch System Cleaner Native GUI [Default]
  --cli, -i                 Launch interactive terminal menu
  -s, --scan, --dry-run     Run in preview mode (scan only, no deletion)
  -c, --clean               Execute standard file deletion
  -z, --secure              Execute secure data sanitization (zero-fill wipe)
  -m, --module <NAME|ID>    Target specific module:
                              1, fs, filesystem    File System Purifier
                              2, browser           Browser Privacy Eraser
                              3, registry          Registry Analyzer & Verifier
                              all                  Run all modules sequentially [Default]
  -v, --verbose             Print individual item paths
  -h, --help                Show this help screen
"#
    );
}

fn run_pipeline(modules: &[Box<dyn CleanerModule>], mode: CleanMode, verbose: bool) -> SuiteReport {
    render_launcher_header(mode);

    // Render overview cards for all queued modules
    for m in modules {
        render_module_card(
            m.id(),
            m.name(),
            "QUEUED",
            m.targets_summary(),
            m.safety_level(),
        );
    }
    println!();

    let mut suite_report = SuiteReport::default();

    for m in modules {
        println!(
            "  {}▶ Executing [{}]: {}{}...",
            TerminalTheme::ACCENT_CYAN,
            m.id(),
            m.name(),
            TerminalTheme::RESET
        );

        let report = m.execute(mode);
        render_module_summary(&report);

        if verbose && !report.details.is_empty() {
            println!(
                "    {}Detailed Entries (sample):{}",
                TerminalTheme::DIM,
                TerminalTheme::RESET
            );
            for (idx, item) in report.details.iter().take(15).enumerate() {
                println!(
                    "      [{:>2}] {:?} | {:<12} | {} [{}]",
                    idx + 1,
                    item.status,
                    core::ui::format_bytes(item.size_bytes),
                    item.path.display(),
                    item.description
                );
            }
            if report.details.len() > 15 {
                println!("      ... and {} more items", report.details.len() - 15);
            }
        }

        println!();
        suite_report.add_report(report);
    }

    render_suite_summary(&suite_report, mode);
    suite_report
}

fn interactive_menu() {
    render_launcher_header(CleanMode::DryRun);

    let fs_module = FileSystemCleaner::new();
    let browser_module = BrowserPrivacyEraser::new();
    let reg_module = RegistryAnalyzer::new();

    render_module_card(
        1,
        fs_module.name(),
        "ACTIVE",
        fs_module.targets_summary(),
        fs_module.safety_level(),
    );
    render_module_card(
        2,
        browser_module.name(),
        "ACTIVE",
        browser_module.targets_summary(),
        browser_module.safety_level(),
    );
    render_module_card(
        3,
        reg_module.name(),
        "GUARDED",
        reg_module.targets_summary(),
        reg_module.safety_level(),
    );

    println!();
    println!(
        "{}  Available Operations:{}",
        TerminalTheme::BOLD,
        TerminalTheme::RESET
    );
    println!(
        "    {} [S]{} Scan & Preview All Modules (Safe Audit)",
        TerminalTheme::ACCENT_CYAN,
        TerminalTheme::RESET
    );
    println!(
        "    {} [C]{} Standard Purge All Modules",
        TerminalTheme::ACCENT_EMERALD,
        TerminalTheme::RESET
    );
    println!(
        "    {} [Z]{} Secure Purge (Zero-Fill Sanitization)",
        TerminalTheme::ACCENT_AMBER,
        TerminalTheme::RESET
    );
    println!(
        "    {} [1]{} Purge File System Only",
        TerminalTheme::FG_WHITE,
        TerminalTheme::RESET
    );
    println!(
        "    {} [2]{} Purge Browser Privacy Only",
        TerminalTheme::FG_WHITE,
        TerminalTheme::RESET
    );
    println!(
        "    {} [3]{} Audit Registry Integrity Only",
        TerminalTheme::FG_WHITE,
        TerminalTheme::RESET
    );
    println!(
        "    {} [Q]{} Quit",
        TerminalTheme::FG_MUTED,
        TerminalTheme::RESET
    );
    println!();
    print!(
        "  {}Select Action [S/C/Z/1/2/3/Q]:{} ",
        TerminalTheme::BOLD,
        TerminalTheme::RESET
    );
    let _ = io::stdout().flush();

    let mut input = String::new();
    let stdin = io::stdin();
    if stdin.lock().read_line(&mut input).is_err() {
        return;
    }

    let choice = input.trim().to_uppercase();
    match choice.as_str() {
        "S" => {
            let modules: Vec<Box<dyn CleanerModule>> = vec![
                Box::new(FileSystemCleaner::new()),
                Box::new(BrowserPrivacyEraser::new()),
                Box::new(RegistryAnalyzer::new()),
            ];
            run_pipeline(&modules, CleanMode::DryRun, true);
        }
        "C" => {
            let modules: Vec<Box<dyn CleanerModule>> = vec![
                Box::new(FileSystemCleaner::new()),
                Box::new(BrowserPrivacyEraser::new()),
                Box::new(RegistryAnalyzer::new()),
            ];
            run_pipeline(&modules, CleanMode::Standard, false);
        }
        "Z" => {
            let modules: Vec<Box<dyn CleanerModule>> = vec![
                Box::new(FileSystemCleaner::new()),
                Box::new(BrowserPrivacyEraser::new()),
                Box::new(RegistryAnalyzer::new()),
            ];
            run_pipeline(&modules, CleanMode::SecureZero, false);
        }
        "1" => {
            let modules: Vec<Box<dyn CleanerModule>> = vec![Box::new(FileSystemCleaner::new())];
            run_pipeline(&modules, CleanMode::Standard, true);
        }
        "2" => {
            let modules: Vec<Box<dyn CleanerModule>> = vec![Box::new(BrowserPrivacyEraser::new())];
            run_pipeline(&modules, CleanMode::Standard, true);
        }
        "3" => {
            let modules: Vec<Box<dyn CleanerModule>> = vec![Box::new(RegistryAnalyzer::new())];
            run_pipeline(&modules, CleanMode::DryRun, true);
        }
        "Q" => {
            println!("Exiting System Cleaner.");
        }
        _ => {
            println!("Defaulting to Preview Scan...");
            let modules: Vec<Box<dyn CleanerModule>> = vec![
                Box::new(FileSystemCleaner::new()),
                Box::new(BrowserPrivacyEraser::new()),
                Box::new(RegistryAnalyzer::new()),
            ];
            run_pipeline(&modules, CleanMode::DryRun, false);
        }
    }
}

fn main() {
    init_terminal();

    let args: Vec<String> = env::args().collect();

    // Default to launching native GUI if no arguments or explicit --gui
    if args.len() <= 1 || args.iter().any(|a| a == "--gui") {
        ui::launch_gui();
        return;
    }

    if args.iter().any(|a| a == "--cli" || a == "-i") {
        interactive_menu();
        return;
    }

    let mut mode = CleanMode::DryRun;
    let mut selected_module = "all".to_string();
    let mut verbose = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-s" | "--scan" | "--dry-run" => {
                mode = CleanMode::DryRun;
            }
            "-c" | "--clean" => {
                mode = CleanMode::Standard;
            }
            "-z" | "--secure" => {
                mode = CleanMode::SecureZero;
            }
            "-v" | "--verbose" => {
                verbose = true;
            }
            "-m" | "--module" => {
                if i + 1 < args.len() {
                    selected_module = args[i + 1].to_lowercase();
                    i += 1;
                }
            }
            other => {
                eprintln!("Unknown argument: {other}. Use --help for usage instructions.");
                return;
            }
        }
        i += 1;
    }

    let mut modules: Vec<Box<dyn CleanerModule>> = Vec::new();

    match selected_module.as_str() {
        "1" | "fs" | "filesystem" => {
            modules.push(Box::new(FileSystemCleaner::new()));
        }
        "2" | "browser" => {
            modules.push(Box::new(BrowserPrivacyEraser::new()));
        }
        "privacy" => {
            modules.push(Box::new(modules::privacy::PrivacyCleaner::new()));
        }
        "graphics" => {
            modules.push(Box::new(modules::graphics::GraphicsCleaner::new()));
        }
        "apps" => {
            modules.push(Box::new(modules::apps::AppCacheCleaner::new()));
        }
        "dev" | "devtools" => {
            modules.push(Box::new(modules::devtools::DevToolsCleaner::new()));
        }
        "3" | "registry" => {
            modules.push(Box::new(RegistryAnalyzer::new()));
        }
        "all" => {
            modules.push(Box::new(FileSystemCleaner::new()));
            modules.push(Box::new(BrowserPrivacyEraser::new()));
            modules.push(Box::new(modules::privacy::PrivacyCleaner::new()));
            modules.push(Box::new(modules::graphics::GraphicsCleaner::new()));
            modules.push(Box::new(modules::apps::AppCacheCleaner::new()));
            modules.push(Box::new(modules::devtools::DevToolsCleaner::new()));
            modules.push(Box::new(RegistryAnalyzer::new()));
        }
        other => {
            let is_safe_rule = modules::rules::catalog()
                .map(|catalog| catalog.rules.iter().any(|rule| rule.id == other))
                .unwrap_or(false);
            if is_safe_rule {
                render_launcher_header(mode);
                let report = modules::execute_target(other, mode);
                render_module_summary(&report);
                let mut suite = SuiteReport::default();
                suite.add_report(report);
                render_suite_summary(&suite, mode);
                return;
            }
            eprintln!(
                "Unknown module or rule: '{other}'. Choose from: 1, 2, 3, fs, browser, privacy, graphics, apps, devtools, registry, all, or a safe-rule ID"
            );
            return;
        }
    }

    run_pipeline(&modules, mode, verbose);
}
