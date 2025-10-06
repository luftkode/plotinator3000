#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::result_large_err,
    reason = "This lint is triggered by the axoupdater library, due to their AxoupdateResult enum being very large. The updater is only run once, so performance doesn't really suffer."
)]
use axoupdater::AxoupdateResult;
use semver::Version;
use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const APP_ICON: &[u8] = include_bytes!("../../../assets/skytem-icon-256.png");
pub const APP_NAME: &str = "plotinator3000";
pub const APP_OWNER: &str = "luftkode";

pub static APP_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();
/// Returns the parent of the parent of the executable directory.
///
/// This is due to the installation being done at <`target_dir`>/bin/<`new_plotinator_binary`>
/// so if we point at /bin/<`current_exe`> the axoupdater would install the update at /bin/bin/<`new_exe`>, therefor we go one level higher
pub fn get_app_install_dir() -> &'static PathBuf {
    APP_INSTALL_DIR.get_or_init(|| {
        let exe_path = std::env::current_exe().expect("Could not find executable");
        log::info!("Executable path: {}", exe_path.display());
        exe_path
            .parent()
            .expect("Could not find parent directory")
            .parent()
            .expect("Could not find parent's parent directory")
            .to_path_buf()
    })
}

const DISABLE_UPDATES_FILE: &str = "plotinator_disable_updates";
const BYPASS_UPDATES_ENV_VAR: &str = "PLOTINATOR_BYPASS_UPDATES";
// Use this to debug the update workflow (or use the environment variable)
const FORCE_UPGRADE: bool = false;

mod ui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckUpdateResult {
    NoConnection,
    UpdateAvailable,
    NoUpdate,
}

/// A wrapper around the [`axoupdater::AxoUpdater`] struct that sets up the updater with the correct parameters
/// for plotinator3000. It doesn't use an install receipt but sets the necessary parameters manually.
pub(crate) struct PlotinatorUpdater {
    updater: axoupdater::AxoUpdater,
    version: Version,
}

impl PlotinatorUpdater {
    pub fn new(version: semver::Version) -> axoupdater::AxoupdateResult<Self> {
        let mut updater = axoupdater::AxoUpdater::new_for(APP_NAME);
        updater.set_install_dir(get_app_install_dir().to_string_lossy().into_owned());
        updater.set_current_version(version.clone())?;
        updater.set_release_source(axoupdater::ReleaseSource {
            release_type: axoupdater::ReleaseSourceType::GitHub,
            owner: APP_OWNER.to_owned(),
            name: APP_NAME.to_owned(),
            app_name: APP_NAME.to_owned(),
        });
        updater.disable_installer_output();
        if let Ok(t) = env::var("GITHUB_TOKEN") {
            updater.set_github_token(&t);
        }

        Ok(Self { updater, version })
    }

    #[allow(
        dead_code,
        reason = "We need to override this to test installation path behaviour"
    )]
    pub fn set_install_dir(&mut self, path: &Path) {
        self.updater
            .set_install_dir(path.to_string_lossy().into_owned());
    }

    pub fn is_update_needed(&mut self) -> axoupdater::AxoupdateResult<CheckUpdateResult> {
        let remote_version = match tokio::runtime::Builder::new_current_thread()
            .worker_threads(1)
            .max_blocking_threads(128)
            .enable_all()
            .build()
            .expect("Initializing tokio runtime failed")
            .block_on(self.updater.query_new_version())
        {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                match e {
                    axoupdater::AxoupdateError::Reqwest(e) if e.is_connect() => {
                        log::warn!("No internet - can't check for updates");
                        return Ok(CheckUpdateResult::NoConnection);
                    }
                    _ => return Err(e),
                }
            }
        };

        match remote_version {
            Some(v) => {
                if v > &self.version {
                    log::info!("New version available");
                    Ok(CheckUpdateResult::UpdateAvailable)
                } else {
                    Ok(CheckUpdateResult::NoUpdate)
                }
            }
            None => Ok(CheckUpdateResult::NoUpdate),
        }
    }

    pub fn always_update(&mut self, setting: bool) {
        self.updater.always_update(setting);
    }

    pub fn run(&mut self) -> AxoupdateResult<Option<axoupdater::UpdateResult>> {
        self.updater.run_sync()
    }
}

/// Returns `Ok(true)` if process is running as admin
/// else it launches it as admin and returns `Ok(false)` if the admin process ran with success otherwise returns an error.
#[cfg(target_os = "windows")]
fn is_admin_run_elevated() -> io::Result<bool> {
    use elevated_command::Command as AdminCommand;
    use std::process::Command as StdCommand;
    use ui::pre_admin_window::pre_admin_window_user_clicked_update;

    if AdminCommand::is_elevated() {
        Ok(true)
    } else {
        if !pre_admin_window_user_clicked_update().unwrap_or(false) {
            return Ok(false);
        }
        let exe_abs_path = std::env::args()
            .next()
            .expect("Failed retrieving program path");
        let cmd = StdCommand::new(exe_abs_path);
        let elevated_cmd = AdminCommand::new(cmd);
        let out = elevated_cmd
            .output()
            .expect("Failed executing elevated command");
        // It succeeded if the return code is greater than 32
        // https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew#return-value
        let exit_code = out.status.code().unwrap_or(100);
        if exit_code > 32 {
            log::info!("Update succeeded");
            Ok(false)
        } else {
            log::error!("Update failed!");
            Err(io::Error::other(format!(
                "Elevating permission failed with: {err_msg}",
                err_msg = if exit_code == 5 {
                    "Permission not allowed".to_owned()
                } else {
                    exit_code.to_string()
                }
            )))
        }
    }
}

/// Handles showning update related UI and all the logic involved in performing an upgrade.
///
/// # Returns
/// - `Ok(true)` if the app should be restarted, e.g. because an update was performed or update settings were changed
/// - `Ok(false)` if it shouldn't, e.g. because updates were disabled or bypassed
#[allow(
    clippy::result_large_err,
    reason = "This function is only called once, so performance doesn't really suffer, Besides this lint is due to the axoupdater library, not really our fault"
)]
pub fn update_if_applicable(version: Version) -> axoupdater::AxoupdateResult<bool> {
    if !bypass_updates() {
        if is_updates_disabled() {
            if ui::updates_disabled::show_simple_updates_are_disabled_window()
                .is_ok_and(|updates_re_enabled| updates_re_enabled)
            {
                log::info!("Updates are re-enabled");
                return Ok(true);
            } else {
                log::debug!("Continuing with updates disabled");
                return Ok(false);
            }
        } else {
            match is_update_available(version.clone()) {
                Ok(is_update_available) => {
                    if is_update_available {
                        #[cfg(target_os = "windows")]
                        match is_admin_run_elevated() {
                            Ok(is_admin) => {
                                if !is_admin {
                                    return Ok(true);
                                }
                            }
                            Err(e) => {
                                ui::error_window::show_error_occurred(&e.to_string());
                                return Ok(false);
                            }
                        }

                        // show update window and perform upgrade or cancel it
                        if let Ok(did_update) = ui::show_simple_update_window(version)
                            && did_update
                        {
                            log::info!("Update performed... Closing");
                            return Ok(true);
                        }
                    } else {
                        log::info!("Already running newest version");
                    }
                    return Ok(false);
                }
                Err(e) => {
                    log::error!("Error checking for update: {e}");
                    return Err(e);
                }
            }
        }
    }
    Ok(false)
}

/// Check for the environment variable to bypass updates
fn bypass_updates() -> bool {
    // This is generally set in the build environment (see config.toml)
    if let Ok(value) = env::var(BYPASS_UPDATES_ENV_VAR) {
        // If we're in the build environment and we detect CI, we don't allow bypassing updates
        if let Ok(value) = env::var("GITHUB_ACTIONS")
            && value == "true"
        {
            log::info!("GitHub actions detected, disabling bypass updates");
            return false;
        }
        if value == "1" || value.eq_ignore_ascii_case("true") {
            log::info!("Update bypassed due to environment variable.");
            return true;
        }
    }
    false
}

/// Creates a file in the same directory as the executable which is used to indicate that
/// updates are disabled (not the best solution...)
fn create_disable_update_file() -> io::Result<()> {
    let exe_dir = get_app_install_dir();
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    File::create(disable_updates_file)?;
    log::info!("Updates disabled");
    Ok(())
}

/// Removes the file that indicates that updates are disabled
fn remove_disable_update_file() -> io::Result<()> {
    let exe_dir = get_app_install_dir();
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    fs::remove_file(disable_updates_file)?;
    log::info!("Updates re-enabled");
    Ok(())
}

/// Checks for the file that indicates that updates are disabled
fn is_updates_disabled() -> bool {
    // Get the path of the executable
    let exe_dir = get_app_install_dir();

    // Check if the plotinator_disable_updates file exists
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    if disable_updates_file.exists() {
        log::warn!("Update bypassed due to presence of '{DISABLE_UPDATES_FILE}' file.");
        return true;
    }
    false
}

/// Uses the [`axoupdater::AxoUpdater`] to query for a newer version than what is currently installed
#[allow(
    clippy::result_large_err,
    reason = "This function is only called once, so performance doesn't really suffer, Besides this lint is due to the axoupdater library, not really our fault"
)]
fn is_update_available(version: Version) -> axoupdater::AxoupdateResult<bool> {
    let mut updater = PlotinatorUpdater::new(version)?;

    if cfg!(debug_assertions) {
        if FORCE_UPGRADE {
            log::warn!("Forcing upgrade");
        }
        updater.always_update(FORCE_UPGRADE); // Set to test it
    }
    log::debug!("Checking for update");
    match updater.is_update_needed()? {
        CheckUpdateResult::NoConnection => {
            log::info!("Cannot check for update");
            Ok(false)
        }
        CheckUpdateResult::NoUpdate => {
            log::info!("{APP_NAME} is up to date");
            Ok(false)
        }
        CheckUpdateResult::UpdateAvailable => {
            log::warn!("{APP_NAME} is outdated and should be upgraded");
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use testresult::TestResult;

    /// This is added because the updater tests kept failing in CI on macos-latest, so this serves to be a sanity check if we can access the github api.
    /// EDIT: It seemed to be due to rate limiting and should be fixed by setting the github token on axoupdater
    #[tokio::test]
    async fn test_github_api_auth_ok() -> TestResult {
        use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
        let mut headers = HeaderMap::new();

        // Try to get token from environment
        if let Ok(token) = env::var("GITHUB_TOKEN") {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
            );
        }

        // Always set a user agent
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("rust-github-api-client"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        let response = client
            .get("https://api.github.com/repos/luftkode/plotinator3000/releases")
            .send()
            .await?;

        eprintln!("{}", response.status());
        assert!(response.status().is_success());

        Ok(())
    }

    #[test]
    fn test_is_update_available() {
        let update_available = is_update_available(semver::Version::new(1, 0, 0)).unwrap();
        assert!(update_available);
    }

    #[test]
    #[ignore = r#"Ruins your ~/.profile and ~/.bashrc or ~/.zshrc (or whatever you use)
    during the installation process by placing e.g. '. "/tmp/.tmp5gCyCy/env"' in it
    every time you run the test. So it is only run in CI by default (in the nextest 'ci' profile."#]
    fn test_plotinator_updater() -> TestResult {
        let tmp_dir = tempdir()?;
        let updater = PlotinatorUpdater::new(semver::Version::new(1, 0, 0));
        assert!(updater.is_ok());
        let mut updater = updater.unwrap();
        updater.set_install_dir(tmp_dir.path());

        // Test update check functionality
        let update_needed = updater.is_update_needed().unwrap();
        assert_eq!(
            update_needed,
            CheckUpdateResult::UpdateAvailable,
            "Expected update available when version is 1.0.0"
        );

        // Test Updating
        updater.always_update(true);
        let update_result = updater.run()?.unwrap();
        assert_eq!(
            update_result.install_prefix.as_str(),
            tmp_dir.path().to_string_lossy()
        );

        //  The current behaviour is to install at <install_path>/bin/<new_binary>
        //  these assertions serve to verify that this behaviour does not suddenly
        //  change and break updates without notice.
        let bin_path = tmp_dir.path().join("bin");
        assert!(bin_path.exists());
        let updated_bin = if cfg!(target_os = "windows") {
            bin_path.join(format!("{APP_NAME}.exe"))
        } else {
            bin_path.join(APP_NAME)
        };
        assert!(updated_bin.exists());

        Ok(())
    }
}
