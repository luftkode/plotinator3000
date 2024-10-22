use std::{
    env,
    fs::{self, File},
    io,
};

use crate::APP_NAME;

const DISABLE_UPDATES_FILE: &str = "logviewer_disable_updates";
const BYPASS_UPDATES_ENV_VAR: &str = "LOGVIEWER_BYPASS_UPDATES";
// Use this to debug the update workflow
const FORCE_UPGRADE: bool = true;

mod ui;

/// Handles showning update related UI and all the logic involved in performing an upgrade.
///
/// # Returns
/// - `Ok(true)` if the app should be restarted, e.g. because an update was performed or update settings were changed
/// - `Ok(false)` if it shouldn't, e.g. because updates were disabled or bypassed
#[allow(
    clippy::result_large_err,
    reason = "This function is only called once, so performance doesn't really suffer, Besides this lint is due to the axoupdater library, not really out fault"
)]
pub fn update_if_applicable() -> axoupdater::AxoupdateResult<bool> {
    if !bypass_updates() {
        if is_updates_disabled().unwrap_or(false) {
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
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .expect("Could not find executable directory");
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    File::create(disable_updates_file)?;
    log::info!("Updates disabled");
    Ok(())
}

/// Removes the file that indicates that updates are disabled
fn remove_disable_update_file() -> io::Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .expect("Could not find executable directory");
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    fs::remove_file(disable_updates_file)?;
    log::info!("Updates re-enabled");
    Ok(())
}

/// Checks for the file that indicates that updates are disabled
fn is_updates_disabled() -> io::Result<bool> {
    // Get the path of the executable
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find executable directory",
        )
    })?;

    // Check if the logviewer_disable_updates file exists
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    if disable_updates_file.exists() {
        log::warn!("Update bypassed due to presence of '{DISABLE_UPDATES_FILE}' file.");
        return Ok(true);
    }
    Ok(false)
}

/// Uses the [`axoupdater::AxoUpdater`] to query for a newer version than what is currently installed
#[allow(
    clippy::result_large_err,
    reason = "This function is only called once, so performance doesn't really suffer, Besides this lint is due to the axoupdater library, not really out fault"
)]
fn is_update_available() -> axoupdater::AxoupdateResult<bool> {
    let mut updater = axoupdater::AxoUpdater::new_for(APP_NAME);
    updater.load_receipt()?;
    if cfg!(debug_assertions) {
        log::warn!("Forcing upgrade");
        updater.always_update(FORCE_UPGRADE); // Set to test it
    }
    if updater.is_update_needed_sync()? {
        log::warn!("{APP_NAME} is outdated and should be upgraded");
        Ok(true)
    } else {
        log::info!("{APP_NAME} is up to date");
        Ok(false)
    }
}
