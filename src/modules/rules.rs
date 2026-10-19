use std::env;
use std::path::PathBuf;

use serde::Deserialize;

use crate::core::cleaner::clean_directory_contents;
use crate::core::{CleanMode, ModuleReport};

const SAFE_RULES_JSON: &str = include_str!("../../assets/rules/safe-rules.json");

#[derive(Debug, Deserialize)]
pub struct RuleCatalog {
    pub version: u32,
    pub rules: Vec<SafeRule>,
}

#[derive(Debug, Deserialize)]
pub struct SafeRule {
    pub id: String,
    pub category: String,
    pub name: String,
    pub root_env: String,
    pub relative_path: String,
    pub recursive: bool,
    pub default_enabled: bool,
}

pub fn catalog() -> Result<RuleCatalog, serde_json::Error> {
    serde_json::from_str(SAFE_RULES_JSON)
}

pub fn execute_target(target_id: &str, mode: CleanMode, report: &mut ModuleReport) -> bool {
    let catalog = match catalog() {
        Ok(catalog) => catalog,
        Err(error) => {
            report.record_failed(
                PathBuf::new(),
                0,
                error.to_string(),
                "Embedded safe-rule catalog",
            );
            return false;
        }
    };
    if catalog.version != 1 {
        report.record_failed(
            PathBuf::new(),
            0,
            format!("unsupported rule catalog version {}", catalog.version),
            "Embedded safe-rule catalog",
        );
        return false;
    }
    let Some(rule) = catalog.rules.iter().find(|rule| rule.id == target_id) else {
        return false;
    };
    let Some(root) = env::var_os(&rule.root_env).map(PathBuf::from) else {
        return true;
    };
    let path = root.join(&rule.relative_path);
    let policy = if rule.default_enabled {
        "default-enabled"
    } else {
        "opt-in"
    };
    clean_directory_contents(
        &path,
        rule.recursive,
        true,
        &|_| true,
        mode,
        report,
        &format!("Safe rule: {} ({}, {policy})", rule.name, rule.category),
    );
    true
}
