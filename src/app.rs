#[cfg(not(target_arch = "wasm32"))]
use crate::app::download::DownloadUi;
use crate::plot::LogPlotUi;
use dropped_files::handle_dropped_files;
use egui::{RichText, UiKind};
use egui_notify::Toasts;

use file_dialog as fd;
use loaded_files::LoadedFiles;

pub(crate) mod custom_files;
mod dropped_files;
mod file_dialog;
pub mod loaded_files;
mod misc;
mod util;

#[cfg(not(target_arch = "wasm32"))]
mod download;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    // Set on the very first frame of starting the app
    #[serde(skip)]
    first_frame: bool,
    #[serde(skip)]
    toasts: Toasts,

    loaded_files: LoadedFiles,
    plot: LogPlotUi,
    font_size: f32,
    font_size_init: bool,
    error_message: Option<String>,

    #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
    #[serde(skip)]
    pub(crate) mqtt: crate::mqtt::Mqtt,

    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    web_file_dialog: fd::web::WebFileDialog,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    native_file_dialog: fd::native::NativeFileDialog,

    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    keep_repainting: bool,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    download_ui: DownloadUi,
}

impl Default for App {
    fn default() -> Self {
        Self {
            first_frame: true,
            toasts: Toasts::default(),
            loaded_files: LoadedFiles::default(),
            plot: LogPlotUi::default(),
            font_size: Self::DEFAULT_FONT_SIZE,
            font_size_init: false,
            error_message: None,

            #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
            mqtt: crate::mqtt::Mqtt::default(),

            #[cfg(target_arch = "wasm32")]
            web_file_dialog: fd::web::WebFileDialog::default(),

            #[cfg(not(target_arch = "wasm32"))]
            native_file_dialog: fd::native::NativeFileDialog::default(),

            #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
            keep_repainting: true,

            #[cfg(not(target_arch = "wasm32"))]
            download_ui: DownloadUi::default(),
        }
    }
}

impl App {
    const DEFAULT_FONT_SIZE: f32 = 16.0;

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.font_size_init = false;
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_picked_files();

        #[cfg(not(target_arch = "wasm32"))]
        self.download_ui
            .poll_download_messages(ctx, &mut self.toasts);

        if !self.font_size_init {
            misc::configure_text_styles(ctx, self.font_size);
        }

        show_top_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            misc::notify_if_logs_added(&mut self.toasts, self.loaded_files.loaded());
            self.plot.ui(
                ui,
                &mut self.first_frame,
                &self.loaded_files.take_loaded_files(),
                &mut self.toasts,
                #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                &mut self.mqtt,
            );

            if self.plot.plot_count() == 0 {
                // Display the message when plots are shown
                util::draw_empty_state(ui);
            }

            match handle_dropped_files(ctx, &mut self.loaded_files) {
                Ok(Some(new_plot_ui_state)) => self.load_new_plot_ui_state(new_plot_ui_state),
                Err(e) => self.error_message = Some(e.to_string()),
                Ok(None) => (),
            }

            misc::show_error(ui, self);
            misc::show_warn_on_debug_build(ui);
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.download_ui.show_download_window(ctx);

        self.toasts.show(ctx);
    }
}

impl App {
    fn load_new_plot_ui_state(&mut self, new: Box<LogPlotUi>) {
        self.first_frame = true; // Necessary to reset some caching
        self.plot = *new;
    }

    fn poll_picked_files(&mut self) {
        #[cfg(target_arch = "wasm32")]
        match self
            .web_file_dialog
            .poll_received_files(&mut self.loaded_files)
        {
            Ok(Some(new_plot_ui_state)) => self.load_new_plot_ui_state(new_plot_ui_state),
            Err(e) => self.error_message = Some(e.to_string()),
            Ok(None) => (),
        }
        #[cfg(not(target_arch = "wasm32"))]
        match self
            .native_file_dialog
            .parse_picked_files(&mut self.loaded_files)
        {
            Ok(Some(new_plot_ui_state)) => self.load_new_plot_ui_state(new_plot_ui_state),
            Err(e) => self.error_message = Some(e.to_string()),
            Ok(None) => (),
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "There's a lot of buttons, just don't put other logic here"
)]
fn show_top_panel(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            misc::show_app_reset_button(ui, app);
            if ui
                .button(RichText::new(format!(
                    "{icon} Open File",
                    icon = egui_phosphor::regular::FOLDER_OPEN
                )))
                .clicked()
            {
                #[cfg(target_arch = "wasm32")]
                app.web_file_dialog.open(ctx.clone());
                #[cfg(not(target_arch = "wasm32"))]
                app.native_file_dialog.open();
            }

            ui.menu_button(
                RichText::new(format!(
                    "{icon} Save",
                    icon = egui_phosphor::regular::FLOPPY_DISK
                )),
                |ui| {
                    // Option to export the entire UI state for later restoration
                    if ui.button("Plot UI State").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        file_dialog::native::NativeFileDialog::save_plot_ui(&app.plot);
                        #[cfg(target_arch = "wasm32")]
                        file_dialog::web::WebFileDialog::save_plot_ui(&app.plot);

                        ui.close_kind(UiKind::Menu);
                    }

                    // Option to export just the raw plot data
                    if ui.button("Plot Data").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        file_dialog::native::NativeFileDialog::save_plot_data(
                            app.plot.stored_plot_files(),
                            #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                            app.mqtt.mqtt_plot_data.as_ref(),
                        );
                        #[cfg(target_arch = "wasm32")]
                        file_dialog::web::WebFileDialog::save_plot_data(
                            app.plot.stored_plot_files(),
                        );
                        ui.close_kind(UiKind::Menu);
                    }
                },
            );

            #[cfg(not(target_arch = "wasm32"))]
            misc::not_wasm_show_download_button(ui, app);

            misc::show_font_size_drag_value(ui, ctx, app);
            misc::show_theme_toggle_buttons(ui);
            misc::show_homepage_link(ui);

            #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
            crate::profiling::ui_add_keep_repainting_checkbox(ui, &mut app.keep_repainting);

            if cfg!(target_arch = "wasm32") {
                ui.label(format!("Plotinator3000 v{}", env!("CARGO_PKG_VERSION")));
            }

            #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
            crate::mqtt::show_mqtt_connect_button(app, ctx, ui);
            misc::collapsible_instructions(ui);
        });
    });
}
