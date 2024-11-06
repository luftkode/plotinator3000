#![allow(
    clippy::result_large_err,
    reason = "This lint is triggered by the axoupdater library, due to their AxoupdateResult enum being very large. The updater is only run once, so performance doesn't really suffer."
)]
use crate::{APP_NAME, APP_OWNER};
use axoupdater::AxoupdateResult;
use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub static APP_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();
/// Returns the parent of the parent of the executable directory due to the installation being done at <target_dir>/bin/<new_plotinator_binary>
/// so if we point at /bin/<current_exe> the axoupdater would install the update at /bin/bin/<new_exe>, therefor we go one level higher
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

/// A wrapper around the [`axoupdater::AxoUpdater`] struct that sets up the updater with the correct parameters
/// for plotinator3000. It doesn't use an install receipt but sets the necessary parameters manually.
pub(crate) struct PlotinatorUpdater {
    updater: axoupdater::AxoUpdater,
}

impl PlotinatorUpdater {
    pub fn new() -> axoupdater::AxoupdateResult<Self> {
        let mut updater = axoupdater::AxoUpdater::new_for(APP_NAME);
        updater.set_install_dir(get_app_install_dir().to_string_lossy().into_owned());
        updater.set_current_version(crate::get_app_version().clone())?;
        updater.set_release_source(axoupdater::ReleaseSource {
            release_type: axoupdater::ReleaseSourceType::GitHub,
            owner: APP_OWNER.to_owned(),
            name: APP_NAME.to_owned(),
            app_name: APP_NAME.to_owned(),
        });
        updater.disable_installer_output();

        Ok(Self { updater })
    }

    #[allow(
        dead_code,
        reason = "We need to override this to test installation path behaviour"
    )]
    pub fn set_install_dir(&mut self, path: &Path) {
        self.updater
            .set_install_dir(path.to_string_lossy().into_owned());
    }

    pub fn is_update_needed(&mut self) -> axoupdater::AxoupdateResult<bool> {
        self.updater.is_update_needed_sync()
    }

    pub fn always_update(&mut self, setting: bool) {
        self.updater.always_update(setting);
    }

    pub fn run(&mut self) -> AxoupdateResult<Option<axoupdater::UpdateResult>> {
        self.updater.run_sync()
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
pub fn update_if_applicable() -> axoupdater::AxoupdateResult<bool> {
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
            match is_update_available() {
                Ok(is_update_available) => {
                    if is_update_available {
                        // show update window and perform upgrade or cancel it
                        if let Ok(did_update) = ui::show_simple_update_window() {
                            if did_update {
                                log::info!("Update performed... Closing");
                                return Ok(true);
                            }
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
    if let Ok(value) = env::var(BYPASS_UPDATES_ENV_VAR) {
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
fn is_update_available() -> axoupdater::AxoupdateResult<bool> {
    let mut updater = PlotinatorUpdater::new()?;

    if cfg!(debug_assertions) {
        if FORCE_UPGRADE {
            log::warn!("Forcing upgrade");
        }
        updater.always_update(FORCE_UPGRADE); // Set to test it
    }
    if updater.is_update_needed()? {
        log::warn!("{APP_NAME} is outdated and should be upgraded");
        Ok(true)
    } else {
        log::info!("{APP_NAME} is up to date");
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use testresult::TestResult;

    #[test]
    fn test_is_update_available() {
        let check_update = is_update_available();
        assert!(check_update.is_ok());
    }

    #[test]
    fn test_plotinator_updater() -> TestResult {
        let tmp_dir = tempdir()?;
        let updater = PlotinatorUpdater::new();
        assert!(updater.is_ok());
        let mut updater = updater.unwrap();
        updater.set_install_dir(tmp_dir.path());

        // Test update check functionality
        let update_needed = updater.is_update_needed();
        assert!(update_needed.is_ok());

        // Test Updating
        updater.always_update(true);
        let update_result = updater.run();
        assert!(update_result.is_ok());

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
