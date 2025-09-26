#[cfg(not(target_arch = "wasm32"))]
use crate::app::download::DownloadUi;
use egui::{RichText, UiKind};
use egui_notify::Toasts;
use egui_phosphor::regular::FLOPPY_DISK;
use plotinator_plot_ui::LogPlotUi;

use plotinator_file_io::{file_dialog as fd, loaded_files::LoadedFiles};

mod misc;
mod supported_formats_table;

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
    pub(crate) mqtt: plotinator_mqtt_ui::connection::MqttConnection,

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

    // Disable saving app state if we have over 100k loaded data points
    // the lag gets noticeable depending on the performance of the device, but if we disable it already at 100k we
    // will definitely not notice it, even on low performing devices.
    #[serde(skip)]
    disable_app_state_storage: bool,
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
            mqtt: plotinator_mqtt_ui::connection::MqttConnection::default(),

            #[cfg(target_arch = "wasm32")]
            web_file_dialog: fd::web::WebFileDialog::default(),

            #[cfg(not(target_arch = "wasm32"))]
            native_file_dialog: fd::native::NativeFileDialog::default(),

            #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
            keep_repainting: true,

            #[cfg(not(target_arch = "wasm32"))]
            download_ui: DownloadUi::default(),

            disable_app_state_storage: false,
        }
    }
}

impl App {
    const DEFAULT_FONT_SIZE: f32 = 16.0;
    const DISABLE_STORAGE_THRESHOLD: u32 = 100_000; // Disable saving app state if we have over 100k loaded data points

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
        plotinator_macros::profile_function!();
        self.disable_app_state_storage =
            self.plot.total_data_points() > Self::DISABLE_STORAGE_THRESHOLD;

        if self.disable_app_state_storage {
            log::debug!("Saving app state is disabled - skipping");
        } else {
            eframe::set_value(storage, eframe::APP_KEY, self);
        }
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
                supported_formats_table::draw_empty_state(ui); // Display the message when no plots are shown
            }

            match plotinator_file_io::dropped_files::handle_dropped_files(
                ctx,
                &mut self.loaded_files,
            ) {
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

            ui.menu_button(RichText::new(format!("{FLOPPY_DISK} Save")), |ui| {
                // Option to export the entire UI state for later restoration
                if ui.button("Plot UI State").clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    fd::native::NativeFileDialog::save_plot_ui(&app.plot);
                    #[cfg(target_arch = "wasm32")]
                    fd::web::WebFileDialog::save_plot_ui(&app.plot);

                    ui.close_kind(UiKind::Menu);
                }

                // Option to export just the raw plot data
                if ui.button("Plot Data").clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    fd::native::NativeFileDialog::save_plot_data(
                        app.plot.stored_plot_files(),
                        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                        app.mqtt.mqtt_plot_data.as_ref(),
                    );
                    #[cfg(target_arch = "wasm32")]
                    fd::web::WebFileDialog::save_plot_data(app.plot.stored_plot_files());
                    ui.close_kind(UiKind::Menu);
                }

                // Option to export individual plot data
                if ui.button("Individual Plot data").clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    fd::native::NativeFileDialog::save_individual_plots(
                        app.plot.individual_plots(),
                    );
                    #[cfg(target_arch = "wasm32")]
                    fd::web::WebFileDialog::save_individual_plots(app.plot.individual_plots());
                    ui.close_kind(UiKind::Menu);
                }
            });

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
