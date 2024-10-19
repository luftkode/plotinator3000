#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use logviewer_rs::updater;

    // Log to stderr (if run with `RUST_LOG=debug`).
    env_logger::init();

    if !updater::bypass_updates().is_ok_and(|r| r) {
        if updater::is_updates_disabled().is_ok_and(|r| r) {
            if updater::show_simple_updates_are_disabled_window()
                .is_ok_and(|updates_re_enabled| updates_re_enabled)
            {
                log::info!("Updates are re-enabled");
                return Ok(());
            } else {
                log::debug!("Continuing with updates disabled");
            }
        } else {
            match updater::is_update_available() {
                Ok(is_update_available) => {
                    if is_update_available {
                        // show update window and perform upgrade or cancel it
                        if let Ok(did_update) = updater::show_simple_update_window() {
                            if did_update {
                                log::info!("Update performed... Closing");
                                return Ok(());
                            }
                        }
                    } else {
                        log::info!("Already running newest version");
                    }
                }
                Err(e) => log::error!("Error checking for update: {e}"),
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 800.0])
            .with_min_inner_size([100.0, 80.0])
            .with_drag_and_drop(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(
                    &include_bytes!("../assets/skytem-icon-256.png")[..],
                )
                .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        &format!("Logviewer v{}", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Ok(Box::new(logviewer_rs::App::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
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
                Box::new(|cc| Ok(Box::new(logviewer_rs::App::new(cc)))),
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
}
