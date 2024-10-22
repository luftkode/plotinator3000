#![warn(clippy::all, rust_2018_idioms)]

pub use app::App;
mod app;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

pub mod plot;
#[cfg(not(target_arch = "wasm32"))]
pub mod updater;
pub mod util;
