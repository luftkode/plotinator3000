#![warn(clippy::all, rust_2018_idioms)]

use std::{path::PathBuf, sync::OnceLock};

pub use app::App;
use axoupdater::Version;
mod app;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_OWNER: &str = "luftkode";

pub const APP_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const APP_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const APP_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

pub static APP_VERSION: OnceLock<Version> = OnceLock::new();
pub fn get_app_version() -> &'static Version {
    APP_VERSION.get_or_init(|| Version::new(
        APP_VERSION_MAJOR.parse().expect("Invalid major version"),
        APP_VERSION_MINOR.parse().expect("Invalid minor version"),
        APP_VERSION_PATCH.parse().expect("Invalid patch version"),
    ))
}

pub static APP_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();
pub fn get_app_install_dir() -> &'static PathBuf {
    APP_INSTALL_DIR.get_or_init(|| {
        let exe_path = std::env::current_exe().expect("Could not find executable");
        exe_path.parent().expect("Could not find parent directory").to_path_buf()
    })
}

pub mod plot;
#[cfg(not(target_arch = "wasm32"))]
pub mod updater;
pub mod util;
