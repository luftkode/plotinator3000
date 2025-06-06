#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// Don't enable on ARM64 Linux due to:
// 'c_src/mimalloc/src/options.c:215:19: error: expansion of date or time macro is not reproducible [-Werror,-Wdate-time]'
#[cfg(not(any(
    target_arch = "wasm32",
    all(target_arch = "aarch64", target_os = "linux")
)))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc; // Much faster allocator, frames rendered ~25% faster on windows 11

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    // Log to stderr (if run with `RUST_LOG=debug`).

    #[cfg(feature = "selfupdater")]
    use plotinator3000::get_app_version;
    env_logger::init();

    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    plotinator3000::profiling::start_puffin_server();

    #[cfg(feature = "selfupdater")]
    match plotinator_updater::update_if_applicable(get_app_version().clone()) {
        Ok(needs_restart) => {
            if needs_restart {
                return Ok(());
            }
        }
        Err(e) => {
            return Err(eframe::Error::AppCreation(
                format!("Error in updater: {e}").into(),
            ));
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 800.0])
            .with_min_inner_size([100.0, 80.0])
            .with_drag_and_drop(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(plotinator3000::APP_ICON)
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        &format!("Plotinator3000 v{}", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Ok(Box::new(plotinator3000::App::new(cc)))),
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
                Box::new(|cc| Ok(Box::new(plotinator3000::App::new(cc)))),
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
