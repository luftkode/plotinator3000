#![warn(clippy::all, rust_2018_idioms)]

use std::sync::OnceLock;

pub use app::App;
use semver::Version;
mod app;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_OWNER: &str = "luftkode";

pub const APP_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const APP_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const APP_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

pub static APP_VERSION: OnceLock<Version> = OnceLock::new();
pub fn get_app_version() -> &'static Version {
    APP_VERSION.get_or_init(|| {
        Version::new(
            APP_VERSION_MAJOR.parse().expect("Invalid major version"),
            APP_VERSION_MINOR.parse().expect("Invalid minor version"),
            APP_VERSION_PATCH.parse().expect("Invalid patch version"),
        )
    })
}

pub mod plot;
#[cfg(not(target_arch = "wasm32"))]
pub mod updater;
pub mod util;
