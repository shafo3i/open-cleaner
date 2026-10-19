//! Registry-based discovery of removable Windows bloatware.
//!
//! Discovery is read-only and does not require elevation. It enumerates:
//! 1. Current-user Appx packages (`HKCU\...\AppModel\Repository\Packages`).
//! 2. Provisioned (all-user) Appx packages (`HKLM\...\Appx\AppxAllUserStore\Applications`).
//! 3. Telemetry / advertising privacy policies that are not yet in their privacy-preserving state.
//!
//! Only entries present in the curated catalog are surfaced, so the bloat tab never lists
//! the entire Appx repository (that is the general uninstaller's job).

use std::collections::HashMap;
use std::process::Command;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

use super::executor::is_elevated;
use super::model::{BloatCategory, BloatItem, BloatRisk, BloatTarget, PolicyScope};

const APPX_USER_PACKAGES: &str = "Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages";
const APPX_PROVISIONED_PACKAGES: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\Applications";

struct StoreAppSpec {
    /// Appx package identity name (lowercase), matched against the first `_`-delimited
    /// segment of the package full name.
    identity: &'static str,
    name: &'static str,
    description: &'static str,
    risk: BloatRisk,
}

struct PolicySpec {
    scope: PolicyScope,
    key_path: &'static str,
    value_name: &'static str,
    disable_value: u32,
    name: &'static str,
    description: &'static str,
    risk: BloatRisk,
}

const STORE_APP_SPECS: &[StoreAppSpec] = &[
    // ── SAFE: promotional / third-party preinstalls ────────────────────────────────
    StoreAppSpec {
        identity: "king.com.candycrushsaga",
        name: "Candy Crush Saga",
        description: "Preinstalled promotional game.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "king.com.bubblewitch3saga",
        name: "Bubble Witch 3 Saga",
        description: "Preinstalled promotional game.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "king.com.farmheroessaga",
        name: "Farm Heroes Saga",
        description: "Preinstalled promotional game.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "king.com.candycrushfriends",
        name: "Candy Crush Friends",
        description: "Preinstalled promotional game.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "spotifyab.spotifymusic",
        name: "Spotify Music",
        description: "Preinstalled music streaming promotion.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "facebook.facebook",
        name: "Facebook",
        description: "Preinstalled social media promotion.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "flipboard.flipboard",
        name: "Flipboard",
        description: "Preinstalled news aggregation promotion.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "microsoft.skypeapp",
        name: "Skype",
        description: "Consumer communication app.",
        risk: BloatRisk::Safe,
    },
    StoreAppSpec {
        identity: "microsoftcorporationii.microsoftfamily",
        name: "Microsoft Family",
        description: "Family safety companion app.",
        risk: BloatRisk::Safe,
    },
    // ── CAREFUL: first-party consumer apps (re-installable from the Store) ────────
    StoreAppSpec {
        identity: "microsoft.bingweather",
        name: "Weather",
        description: "Built-in weather app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.bingnews",
        name: "News",
        description: "Built-in news reader.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.gethelp",
        name: "Get Help",
        description: "Windows support assistant.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.getstarted",
        name: "Tips",
        description: "Onboarding tips app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.microsoftofficehub",
        name: "Office",
        description: "Microsoft 365 entry point.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.microsoftsolitairecollection",
        name: "Solitaire Collection",
        description: "Preinstalled game collection.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.microsoftstickynotes",
        name: "Sticky Notes",
        description: "Desktop note-taking app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.mixedreality.portal",
        name: "Mixed Reality Portal",
        description: "Windows Mixed Reality companion.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.office.onenote",
        name: "OneNote",
        description: "Note-taking app for Windows 10/11.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.people",
        name: "People",
        description: "Contacts app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.screensketch",
        name: "Snipping Tool",
        description: "Screenshot capture utility.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.todos",
        name: "Microsoft To Do",
        description: "Task management app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windowsalarms",
        name: "Alarms & Clock",
        description: "Alarm, timer and clock app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windowscamera",
        name: "Camera",
        description: "Webcam capture app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windowscommunicationsapps",
        name: "Mail & Calendar",
        description: "Built-in email and calendar client.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windowsfeedbackhub",
        name: "Feedback Hub",
        description: "Windows feedback submission app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windowsmaps",
        name: "Maps",
        description: "Built-in mapping app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windowssoundrecorder",
        name: "Voice Recorder",
        description: "Audio recording app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.windows.photos",
        name: "Photos",
        description: "Photo gallery and editor.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.zunemusic",
        name: "Music (Groove)",
        description: "Built-in music player.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.zunevideo",
        name: "Movies & TV",
        description: "Built-in video player.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.mspaint",
        name: "Paint",
        description: "Classic paint application.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.clipchamp.clipchamp",
        name: "Clipchamp",
        description: "Preinstalled video editor.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.powerautomatedesktop",
        name: "Power Automate",
        description: "Desktop automation runtime.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.whiteboard",
        name: "Whiteboard",
        description: "Collaborative whiteboard app.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.xbox.tcui",
        name: "Xbox TCUI",
        description: "Xbox system integration component.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.xboxapp",
        name: "Xbox",
        description: "Xbox console companion.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.xboxgameoverlay",
        name: "Xbox Game Bar",
        description: "In-game overlay and recording.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.xboxgamingoverlay",
        name: "Xbox Gaming Overlay",
        description: "Gaming overlay runtime.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.xboxidentityprovider",
        name: "Xbox Identity Provider",
        description: "Xbox sign-in component.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.xboxspeechtotextoverlay",
        name: "Xbox Speech-to-Text Overlay",
        description: "Xbox voice transcription overlay.",
        risk: BloatRisk::Careful,
    },
    StoreAppSpec {
        identity: "microsoft.yourphone",
        name: "Phone Link",
        description: "Phone-to-PC integration app.",
        risk: BloatRisk::Careful,
    },
    // ── EXPERT: system-integrated components ───────────────────────────────────────
    StoreAppSpec {
        identity: "microsoft.549981c3f5f10",
        name: "Cortana",
        description: "Digital assistant (system-integrated).",
        risk: BloatRisk::Expert,
    },
];

const POLICY_SPECS: &[PolicySpec] = &[
    PolicySpec {
        scope: PolicyScope::MachineOnly,
        key_path: "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
        value_name: "AllowTelemetry",
        disable_value: 0,
        name: "Windows Diagnostic Telemetry",
        description: "Reduces diagnostic data collection to the minimum 'Security' level.",
        risk: BloatRisk::Expert,
    },
    PolicySpec {
        scope: PolicyScope::Both,
        key_path: "SOFTWARE\\Policies\\Microsoft\\Windows\\CloudContent",
        value_name: "DisableWindowsConsumerFeatures",
        disable_value: 1,
        name: "Suggested Apps (Consumer Features)",
        description: "Stops Windows from automatically installing suggested promotional apps.",
        risk: BloatRisk::Careful,
    },
    PolicySpec {
        scope: PolicyScope::Both,
        key_path: "SOFTWARE\\Policies\\Microsoft\\Windows\\CloudContent",
        value_name: "DisableWindowsSpotlightFeatures",
        disable_value: 1,
        name: "Windows Spotlight",
        description: "Disables lock-screen promotions and Spotlight content.",
        risk: BloatRisk::Careful,
    },
    PolicySpec {
        scope: PolicyScope::Both,
        key_path: "SOFTWARE\\Policies\\Microsoft\\Windows\\CloudContent",
        value_name: "DisableSoftLanding",
        disable_value: 1,
        name: "Soft Landing Promos",
        description: "Disables post-update onboarding and promotional screens.",
        risk: BloatRisk::Careful,
    },
    PolicySpec {
        scope: PolicyScope::UserOnly,
        key_path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AdvertisingInfo",
        value_name: "Enabled",
        disable_value: 0,
        name: "Advertising ID",
        description: "Disables the per-user advertising ID used for targeted ads.",
        risk: BloatRisk::Safe,
    },
    PolicySpec {
        scope: PolicyScope::Both,
        key_path: "SOFTWARE\\Policies\\Microsoft\\Windows\\Explorer",
        value_name: "DisableSearchBoxSuggestions",
        disable_value: 1,
        name: "Search Box Web Suggestions",
        description: "Disables web suggestions in Windows Search.",
        risk: BloatRisk::Safe,
    },
];

struct CapabilitySpec {
    prefix: &'static str,
    name: &'static str,
    description: &'static str,
    risk: BloatRisk,
}

const CAPABILITY_SPECS: &[CapabilitySpec] = &[
    CapabilitySpec {
        prefix: "media.windowsmediaplayer",
        name: "Windows Media Player",
        description: "Legacy media player optional feature.",
        risk: BloatRisk::Expert,
    },
    CapabilitySpec {
        prefix: "browser.internetexplorer",
        name: "Internet Explorer Mode",
        description: "Legacy rendering engine for IE mode in Edge.",
        risk: BloatRisk::Expert,
    },
    CapabilitySpec {
        prefix: "mathrecognizer",
        name: "Math Recognizer",
        description: "Legacy math input recognition feature.",
        risk: BloatRisk::Careful,
    },
    CapabilitySpec {
        prefix: "xps.viewer",
        name: "XPS Viewer",
        description: "Legacy XPS document viewer.",
        risk: BloatRisk::Careful,
    },
];

/// Scans the system for removable bloatware and unapplied privacy policies.
/// Sorted least-risky first so the UI presents Safe items before Careful/Expert.
pub fn scan_bloatware() -> Vec<BloatItem> {
    let mut items = Vec::new();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let user_map = scan_appx_map(&hkcu, APPX_USER_PACKAGES);
    let provisioned_map = scan_appx_map(&hklm, APPX_PROVISIONED_PACKAGES);

    for spec in STORE_APP_SPECS {
        let installed = user_map.get(spec.identity).cloned();
        let provisioned = provisioned_map.get(spec.identity).cloned();
        if installed.is_none() && provisioned.is_none() {
            continue;
        }

        items.push(BloatItem {
            id: format!("bloat_app_{}", spec.identity),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            risk: spec.risk,
            category: BloatCategory::StoreApp,
            target: BloatTarget::StoreApp {
                installed_full_name: installed,
                provisioned_full_name: provisioned,
            },
            selected: false,
        });
    }

    for spec in POLICY_SPECS {
        if policy_already_applied(spec) {
            continue;
        }

        items.push(BloatItem {
            id: format!("bloat_policy_{}", spec.value_name),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            risk: spec.risk,
            category: BloatCategory::Policy,
            target: BloatTarget::Policy {
                scope: spec.scope,
                key_path: spec.key_path,
                value_name: spec.value_name,
                disable_value: spec.disable_value,
            },
            selected: false,
        });
    }

    // Optional-feature capabilities require elevation to enumerate (DISM).
    if is_elevated() {
        let installed = list_installed_capabilities();
        for spec in CAPABILITY_SPECS {
            let Some(capability_id) = installed
                .iter()
                .find(|name| name.to_ascii_lowercase().starts_with(spec.prefix))
                .cloned()
            else {
                continue;
            };

            items.push(BloatItem {
                id: format!("bloat_cap_{}", spec.prefix),
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                risk: spec.risk,
                category: BloatCategory::Capability,
                target: BloatTarget::Capability { capability_id },
                selected: false,
            });
        }
    }

    items.sort_by(|a, b| {
        a.risk
            .cmp(&b.risk)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    items
}

/// Maps package identity name (lowercase) -> package full name for a given repository key.
fn scan_appx_map(root: &RegKey, path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(key) = root.open_subkey_with_flags(path, KEY_READ) else {
        return map;
    };

    for full_name in key.enum_keys().flatten() {
        let identity = full_name
            .split('_')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if identity.is_empty() {
            continue;
        }
        map.entry(identity).or_insert(full_name);
    }

    map
}

fn policy_already_applied(spec: &PolicySpec) -> bool {
    match spec.scope {
        PolicyScope::UserOnly => policy_value_matches(&RegKey::predef(HKEY_CURRENT_USER), spec),
        PolicyScope::MachineOnly => policy_value_matches(&RegKey::predef(HKEY_LOCAL_MACHINE), spec),
        PolicyScope::Both => {
            policy_value_matches(&RegKey::predef(HKEY_CURRENT_USER), spec)
                || policy_value_matches(&RegKey::predef(HKEY_LOCAL_MACHINE), spec)
        }
    }
}

fn policy_value_matches(root: &RegKey, spec: &PolicySpec) -> bool {
    let Ok(key) = root.open_subkey_with_flags(spec.key_path, KEY_READ) else {
        return false;
    };

    matches!(key.get_value::<u32, _>(spec.value_name), Ok(value) if value == spec.disable_value)
}

/// Enumerates installed Windows capabilities via DISM. Requires elevation.
fn list_installed_capabilities() -> Vec<String> {
    let script = "Get-WindowsCapability -Online | Where-Object { $_.State -eq 'Installed' } | ForEach-Object { $_.Name }";
    let Ok(out) = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
    else {
        return Vec::new();
    };

    if !out.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}
