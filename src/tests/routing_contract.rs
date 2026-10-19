use std::collections::HashSet;

use crate::modules::rules::catalog;
use crate::ui::model::default_targets;

/// Layer 1: Verifies all 63 built-in UI targets have unique IDs, sequential numbers,
/// valid category breakdown, zero "Pending scan" strings, and dedicated module prefixes.
#[test]
fn all_63_ui_targets_are_routed_and_have_no_placeholders() {
    let targets = default_targets();
    assert_eq!(targets.len(), 63, "Expected exactly 63 built-in UI targets");

    let count_cat = |cat: &str| targets.iter().filter(|t| t.category == cat).count();
    assert_eq!(count_cat("system"), 13, "SYSTEM should have 13 targets");
    assert_eq!(count_cat("browsers"), 18, "BROWSERS should have 18 targets");
    assert_eq!(count_cat("privacy"), 5, "PRIVACY should have 5 targets");
    assert_eq!(count_cat("graphics"), 5, "GRAPHICS should have 5 targets");
    assert_eq!(count_cat("apps"), 8, "APPS should have 8 targets");
    assert_eq!(count_cat("dev"), 8, "DEV & AI should have 8 targets");
    assert_eq!(count_cat("registry"), 6, "REGISTRY should have 6 targets");

    let mut seen_ids = HashSet::new();
    let mut seen_nums = HashSet::new();

    for (idx, item) in targets.iter().enumerate() {
        assert!(
            seen_ids.insert(item.id),
            "Duplicate target ID found: {}",
            item.id
        );
        let expected_num = format!("{:02}", idx + 1);
        assert_eq!(
            item.number, expected_num,
            "Target {} has out-of-sequence number {}",
            item.id, item.number
        );
        assert!(
            seen_nums.insert(item.number),
            "Duplicate target number found: {}",
            item.number
        );
        assert!(
            !item.name_prefix.is_empty(),
            "{} missing name_prefix",
            item.id
        );
        assert!(
            !item.name_suffix.is_empty(),
            "{} missing name_suffix",
            item.id
        );
        assert!(!item.tagline.is_empty(), "{} missing tagline", item.id);
        assert!(!item.scope.is_empty(), "{} missing scope", item.id);
        assert!(
            !item.size.to_lowercase().contains("pending"),
            "Target {} still has Pending size placeholder: {}",
            item.id,
            item.size
        );
        assert!(
            !item.count.to_lowercase().contains("pending"),
            "Target {} still has Pending count placeholder: {}",
            item.id,
            item.count
        );

        let expected_prefix = match item.category {
            "system" => "fs_",
            "browsers" => "br_",
            "privacy" => "priv_",
            "graphics" => "gfx_",
            "apps" => "app_",
            "dev" => "dev_",
            "registry" => "reg_",
            other => panic!("Unexpected category '{other}' on target {}", item.id),
        };
        assert!(
            item.id.starts_with(expected_prefix),
            "Target {} in category '{}' must start with '{}'",
            item.id,
            item.category,
            expected_prefix
        );
    }
}

#[test]
fn safe_rule_catalog_is_valid_and_has_unique_ids() {
    let cat = catalog().expect("embedded rule catalog must be valid JSON");
    assert_eq!(cat.version, 1);
    assert!(!cat.rules.is_empty());
    let ids: HashSet<_> = cat.rules.iter().map(|rule| &rule.id).collect();
    assert_eq!(ids.len(), cat.rules.len());
    assert!(cat.rules.iter().all(|rule| !rule.default_enabled));
}
