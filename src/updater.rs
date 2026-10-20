//! Background update checking and self-update against GitHub Releases.

use serde::Deserialize;

pub const REPO_OWNER: &str = "shafo3i";
pub const REPO_NAME: &str = "open-cleaner";
pub const BIN_NAME: &str = "open-cleaner";

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInfo {
    pub tag_name: String,
}

impl UpdateInfo {
    pub fn version(&self) -> String {
        self.tag_name.trim_start_matches(['v', 'V']).to_string()
    }
}

/// Returns `Some(UpdateInfo)` when a newer release exists than the running build.
pub fn check_for_update() -> Option<UpdateInfo> {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;

    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    let response = ureq::get(&url)
        .set("User-Agent", "open-cleaner-updater")
        .call()
        .ok()?;

    let body = response.into_string().ok()?;
    let info: UpdateInfo = serde_json::from_str(&body).ok()?;

    let latest = semver::Version::parse(&info.version()).ok()?;
    if latest > current { Some(info) } else { None }
}

fn try_update_with_target(target: &str) -> Result<bool, String> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(env!("CARGO_PKG_VERSION"))
        .target(target)
        .show_download_progress(false)
        .no_confirm(true)
        .build()
        .map_err(|e| format!("update configuration failed: {e}"))?
        .update()
        .map_err(|e| format!("update failed: {e}"))?;

    match status {
        self_update::Status::Updated(_) => Ok(true),
        self_update::Status::UpToDate(_) => Ok(false),
    }
}

/// Downloads the latest release, replaces the running binary, and returns `true`
/// when a new version was installed (the caller should then restart). Returns
/// `false` if the running build is already current.
pub fn perform_update() -> Result<bool, String> {
    // 1. Try standard target triple (x86_64-pc-windows-msvc for CI matrix archives)
    match try_update_with_target("x86_64-pc-windows-msvc") {
        Ok(res) => Ok(res),
        Err(orig_err) => {
            // 2. Fallback to direct raw binary asset names (e.g. open-cleaner.exe)
            let direct_target = if cfg!(windows) {
                "open-cleaner.exe"
            } else {
                "open-cleaner"
            };
            try_update_with_target(direct_target).map_err(|_| orig_err)
        }
    }
}
