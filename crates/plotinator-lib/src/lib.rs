#![warn(clippy::all, rust_2018_idioms)]

use semver::Version;
use std::sync::OnceLock;
mod app;
use crate::app::App;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_OWNER: &str = "luftkode";

#[cfg(not(target_arch = "wasm32"))]
pub const APP_ICON: &[u8] = include_bytes!("../../../assets/skytem-icon-256.png");

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
#[cfg(feature = "selfupdater")]
#[cfg(not(target_arch = "wasm32"))]
pub mod updater;
pub mod util;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
pub fn run_app() -> eframe::Result {
    // Log to stderr (if run with `RUST_LOG=debug`).
    env_logger::init();

    #[cfg(feature = "selfupdater")]
    match crate::updater::update_if_applicable() {
        Ok(needs_restart) => {
            if needs_restart {
                return Ok(());
            }
        }
        Err(e) => {
            return Err(eframe::Error::AppCreation(
                format!("Error in updater: {e}").into(),
            ))
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 800.0])
            .with_min_inner_size([100.0, 80.0])
            .with_drag_and_drop(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(crate::APP_ICON).expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        &format!("Plotinator3000 v{}", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
pub fn run_app() -> eframe::Result {
    use eframe::wasm_bindgen::JsCast as _;
    // Redirect `log` message to `console.log` and friends:
    _ = eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
    Ok(())
}
