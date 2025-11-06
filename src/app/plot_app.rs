use plotinator_supported_formats::SupportedFormat;
use smallvec::SmallVec;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::app::plot_app::download::DownloadUi;
use egui::{RichText, UiKind};
use egui_notify::Toasts;
use egui_phosphor::regular::{FLOPPY_DISK, FOLDER_OPEN};
use plotinator_background_parser::{ParserThreads, loaded_format::LoadedSupportedFormat};
use plotinator_plot_ui::LogPlotUi;
use plotinator_ui_file_io::{ParseStatusWindow, ParseUpdate};

use plotinator_file_io::file_dialog::{self as fd, native::NativeFileDialog};
mod handle_input;
mod misc;
mod supported_formats_table;

#[cfg(not(target_arch = "wasm32"))]
mod download;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PlotApp {
    // Set on the very first frame of starting the app
    #[serde(skip)]
    first_frame: bool,
    #[serde(skip)]
    pub(crate) toasts: Toasts,
    #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
    pub(crate) map_commander: plotinator_map_ui::commander::MapUiCommander,

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

    #[serde(skip)]
    background_parser: ParserThreads,
    #[serde(skip)]
    parse_update_rx: Receiver<ParseUpdate>,
    #[serde(skip)]
    status_window: ParseStatusWindow,
    #[serde(skip)]
    loaded_custom_files: Vec<SupportedFormat>,
}

impl Default for PlotApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            first_frame: true,
            toasts: Toasts::default(),
            plot: LogPlotUi::default(),
            font_size: Self::DEFAULT_FONT_SIZE,
            font_size_init: false,
            error_message: None,

            #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
            map_commander: plotinator_map_ui::commander::MapUiCommander::default(),

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

            parse_update_rx: rx,
            background_parser: ParserThreads::new(tx),
            status_window: ParseStatusWindow::new(),
            loaded_custom_files: vec![],
        }
    }
}

impl PlotApp {
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

impl eframe::App for PlotApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_picked_files();
        while let Ok(update) = self.parse_update_rx.try_recv() {
            self.status_window.handle_update(update);
        }
        self.status_window.draw(ctx);

        #[cfg(not(target_arch = "wasm32"))]
        self.download_ui
            .poll_download_messages(ctx, &mut self.toasts);

        if !self.font_size_init {
            misc::configure_text_styles(ctx, self.font_size);
        }

        show_top_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut new_loaded_files = self.background_parser.poll();
            for custom_loaded_file in self.loaded_custom_files.drain(..) {
                new_loaded_files.push(LoadedSupportedFormat::new(custom_loaded_file));
            }

            misc::notify_if_logs_added(&mut self.toasts, &new_loaded_files);

            self.plot.ui(
                ui,
                &mut self.first_frame,
                &mut new_loaded_files,
                &mut self.toasts,
                #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
                &mut self.map_commander,
                #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                &mut self.mqtt,
            );

            #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
            {
                for mut file in new_loaded_files {
                    for geo_data in file.take_geo_spatial_data() {
                        self.map_commander.add_geo_data(geo_data);
                    }
                }
                if let Some(mqtt_geo_points) = self
                    .mqtt
                    .mqtt_plot_data
                    .as_mut()
                    .and_then(|d| d.take_geo_points())
                {
                    self.map_commander.add_mqtt_geo_points(mqtt_geo_points);
                }
                if let Some(mqtt_geo_altitudes) = self
                    .mqtt
                    .mqtt_plot_data
                    .as_mut()
                    .and_then(|d| d.take_geo_altitudes())
                {
                    self.map_commander
                        .add_mqtt_geo_altitudes(mqtt_geo_altitudes);
                }
            }

            if self.plot.plot_count() == 0 {
                supported_formats_table::draw_empty_state(ui); // Display the message when no plots are shown
            }

            let dropped_files = plotinator_file_io::dropped_files::take_dropped_files(ctx);
            self.handle_input_files(dropped_files);
            misc::show_error(ui, self);
            misc::show_warn_on_debug_build(ui);
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.download_ui.show_download_window(ctx);

        self.toasts.show(ctx);

        if self.background_parser.active_threads() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

impl PlotApp {
    fn load_new_plot_ui_state(&mut self, new: Box<LogPlotUi>) {
        self.first_frame = true; // Necessary to reset some caching
        self.plot = *new;
    }

    fn poll_picked_files(&mut self) {
        let picked_files = self.native_file_dialog.take_picked_files();
        if !picked_files.is_empty() {
            log::debug!("Got picked file: {picked_files:?}");
        }
        self.handle_input_files(picked_files.into());
    }

    fn handle_input_files(&mut self, mut input_paths: SmallVec<[PathBuf; 1]>) {
        use plotinator_file_io::custom_files::CustomFileContent;
        match NativeFileDialog::try_parse_custom_files(&mut input_paths) {
            Ok(custom_files) => {
                for cf in custom_files {
                    match cf {
                        CustomFileContent::PlotData(mut pd) => {
                            log::info!("Loading {} plot data files from", pd.len());
                            self.loaded_custom_files.append(&mut pd);
                        }
                        CustomFileContent::PlotUi(new_plot_ui_state) => {
                            self.load_new_plot_ui_state(new_plot_ui_state);
                        }
                    }
                }
            }
            Err(e) => self.error_message = Some(e.to_string()),
        }
        for p in input_paths {
            self.background_parser.parse_path(p);
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "There's a lot of buttons, just don't put other logic here"
)]
fn show_top_panel(app: &mut PlotApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            misc::show_app_reset_button(ui, app);
            if ui
                .button(RichText::new(format!("{FOLDER_OPEN} Open File")))
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

            #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
            {
                use egui_phosphor::regular::GLOBE_HEMISPHERE_WEST;
                let mut txt = RichText::new(format!("{GLOBE_HEMISPHERE_WEST} Map"));
                if app.map_commander.is_open() {
                    txt = txt.color(egui::Color32::GREEN).strong();
                }
                if ui.button(txt).clicked() {
                    app.map_commander.map_button_clicked = true;
                }
            }

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
