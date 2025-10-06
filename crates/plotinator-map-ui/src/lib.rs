use egui::{
    Button, CentralPanel, Color32, Frame, MenuBar, RichText, TopBottomPanel, Ui, ViewportBuilder,
    ViewportId,
};
use egui_phosphor::regular::{
    CHECK_CIRCLE, CHECK_SQUARE, CIRCLE, GLOBE, GLOBE_HEMISPHERE_WEST, SQUARE,
};
use plotinator_log_if::prelude::GeoSpatialData;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use walkers::Map;

use crate::{
    commander::MapUiCommander,
    draw::{DrawSettings, TelemetryLabelSettings},
    map_state::MapState,
};

/// Messages sent from main app to map viewport
#[derive(strum_macros::Display)]
pub enum MapCommand {
    AddGeoData(GeoSpatialData),
    /// Cursor position on the time axis
    CursorPos(f64),
    FitToAllPaths,
    /// Remove all [`GeoSpatialData`]
    Reset,
}

pub mod commander;
mod draw;
mod map_state;

#[derive(Default, Deserialize, Serialize)]
pub struct MapViewPort {
    pub open: bool,
    pub geo_data: Vec<PathEntry>,
    map_state: MapState,
    #[serde(skip)]
    cmd_recv: Option<Receiver<MapCommand>>,
    /// The time corresponding to the cursor position in the plot area
    #[serde(skip)]
    plot_time_cursor_pos: Option<f64>,
    #[serde(skip)]
    hovered_path: Option<usize>, // index of hovered path
}

impl MapViewPort {
    /// Open the [`MapViewPort`]
    ///
    /// if it's the first time it's opened, it will start loading map tiles and
    /// return a [Sender<MapCommand>] for interacting with the Map from other contexts
    pub fn open(&mut self, ctx: &egui::Context) -> Option<Sender<MapCommand>> {
        if self.map_state.tile_state.is_none() {
            egui_extras::install_image_loaders(ctx);
            self.map_state.init(ctx.clone());
        }
        let mut maybe_map_send = None;
        if self.cmd_recv.is_none() {
            let (cmd_send, cmd_recv) = MapUiCommander::channels();
            maybe_map_send = Some(cmd_send);
            self.cmd_recv = Some(cmd_recv);
        }
        self.open = true;

        maybe_map_send
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn poll_commands(&mut self) {
        let mut cursor_pos: Option<f64> = None;
        while let Ok(cmd) = self
            .cmd_recv
            .as_ref()
            .expect("unsound condition")
            .try_recv()
        {
            match cmd {
                MapCommand::AddGeoData(geo_data) => {
                    if let Some(first_point) = geo_data.points.first() {
                        let has_speed = first_point.speed.is_some();
                        let has_altitude = first_point.altitude.is_some();
                        let has_heading = first_point.heading.is_some();
                        log::info!(
                            "Received geo data {}, points include speed={has_speed}, altitude={has_altitude}, heading={has_heading}",
                            geo_data.name
                        );
                    } else {
                        log::info!("Received basic geo data {}", geo_data.name);
                    }

                    self.add_geo_data(geo_data);
                    self.map_state.zoom_to_fit(&self.geo_data);
                }
                MapCommand::CursorPos(time_pos) => {
                    log::trace!("Got cursor time: {time_pos:.}");
                    cursor_pos = Some(time_pos);
                }
                MapCommand::FitToAllPaths => {
                    self.map_state.zoom_to_fit(&self.geo_data);
                }
                MapCommand::Reset => self.geo_data.clear(),
            }
        }
        if let Some(pos) = cursor_pos {
            self.plot_time_cursor_pos = Some(pos);
        }
    }

    pub fn add_geo_data(&mut self, geo_data: GeoSpatialData) {
        debug_assert!(
            geo_data.points.iter().all(|p| !p.timestamp.is_nan()
                && !p.position.x().is_nan()
                && !p.position.y().is_nan()
                && !p
                    .altitude
                    .is_some_and(|a| a.is_nan() && p.speed.is_some_and(|s| s.is_nan()))
                && !p.heading.is_some_and(|h| h.is_nan())),
            "GeoSpatialData with NaN values: {}",
            geo_data.name
        );
        self.geo_data.push(PathEntry {
            data: geo_data,
            settings: Default::default(),
        });
    }

    /// Shows the map viewport and handles its UI.
    /// This is the primary drawing method to be called from your main app's update loop.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let mut is_still_open = true;

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("map_viewport"),
            ViewportBuilder::default()
                .with_title("Map")
                .with_inner_size([800.0, 600.0])
                .with_drag_and_drop(false)
                .with_always_on_top(),
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    is_still_open = false;
                }

                self.poll_commands();

                TopBottomPanel::top("map_top_panel").show(ctx, |ui| {
                    self.show_menu_bar(ui);
                });

                CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                    self.show_map_panel(ui);
                });

                self.show_legend_window(ctx);
            },
        );

        // If the user requested to close the window, update the state.
        if !is_still_open {
            self.close();
        }
    }

    /// Renders the menu bar at the top of the viewport.
    fn show_menu_bar(&mut self, ui: &mut Ui) {
        MenuBar::new().ui(ui, |ui| {
            let map_state = self
                .map_state
                .tile_state_as_mut()
                .expect("map_tile_state is required but not initialized");

            let is_satellite = map_state.is_satellite;
            let (icon, text) = if is_satellite {
                (GLOBE_HEMISPHERE_WEST, "Satellite")
            } else {
                (GLOBE, "Open Street Map")
            };
            if ui
                .add_sized(
                    [150.0, 0.0],
                    Button::new(RichText::new(format!("{icon} {text}")).strong()),
                )
                .clicked()
            {
                self.map_state.toggle_map_style(ui.ctx().clone());
            }
        });
    }

    /// Renders the main map panel and all geographical data on it.
    fn show_map_panel(&mut self, ui: &mut Ui) {
        let map_center_position = self.map_state.data().center_position;
        let tile_state = self
            .map_state
            .tile_state_as_mut()
            .expect("map_tile_state is required but not initialized");

        let zoom_level = tile_state.zoom_level();

        let map = Map::new(
            Some(tile_state.tiles.as_mut()),
            &mut tile_state.map_memory,
            map_center_position,
        )
        .double_click_to_zoom(true);

        map.show(ui, |ui, projector, _map_rect| {
            for (i, path) in self.geo_data.iter().enumerate() {
                if !path.settings.visible {
                    continue;
                }

                let is_hovered = self.hovered_path == Some(i);

                let draw_settings = DrawSettings {
                    draw_heading_arrows: zoom_level > 18.0 && path.settings.show_heading,
                    telemetry_label: TelemetryLabelSettings {
                        draw: zoom_level > 19.4,
                        with_speed: path.settings.show_speed,
                        with_altitude: path.settings.show_altitude,
                    },
                };

                draw::draw_path(ui, projector, &path.data, &draw_settings);

                if is_hovered {
                    draw::highlight_whole_path(ui.painter(), projector, &path.data);
                }
            }

            if let Some(cursor_time) = self.plot_time_cursor_pos {
                draw::draw_cursor_highlights(ui.painter(), projector, &self.geo_data, cursor_time);
            }
        });
    }

    /// Renders the legend window with path information.
    fn show_legend_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Legend")
            .title_bar(true)
            .resizable(true)
            .default_pos(egui::pos2(0.0, 32.0))
            .default_size([200.0, 150.0])
            .show(ctx, |ui| {
                if self.geo_data.is_empty() {
                    ui.label("No paths loaded");
                } else {
                    self.show_legend_grid(ui);
                }

                // Reset hovered state if the mouse leaves the legend window
                if !ui.ui_contains_pointer() {
                    self.hovered_path = None;
                }
            });
    }

    fn show_legend_grid(&mut self, ui: &mut Ui) {
        egui::Grid::new("legend_grid").striped(true).show(ui, |ui| {
            // Column headers
            ui.label(""); // empty cell for toggle + name
            ui.label("vel");
            ui.label("alt");
            ui.label("hdg");
            ui.end_row();

            for (i, path_entry) in self.geo_data.iter_mut().enumerate() {
                let path = &path_entry.data;

                let mut path_ui_hovered = false;
                ui.horizontal(|ui| {
                    // Visibility toggle
                    let indicator = if path_entry.settings.visible {
                        RichText::new(CHECK_SQUARE).color(path.color).weak()
                    } else {
                        RichText::new(SQUARE).color(path.color).strong()
                    };
                    if ui.button(indicator).clicked() {
                        path_entry.settings.visible = !path_entry.settings.visible;
                    }
                    ui.label(RichText::new(&path.name).strong());

                    if ui.ui_contains_pointer() {
                        path_ui_hovered = true;
                    }
                });

                let first_point = path.points.first();
                let mut attr_indicator_label = |has_attr: bool, show_attr: &mut bool| {
                    let has_attr_text = if has_attr {
                        if *show_attr {
                            RichText::new(CHECK_CIRCLE).color(Color32::GREEN)
                        } else {
                            RichText::new(CIRCLE).color(Color32::GREEN).weak()
                        }
                    } else {
                        RichText::new(CIRCLE).weak()
                    };
                    let resp = ui.button(has_attr_text);
                    if resp.clicked() {
                        *show_attr = !*show_attr;
                    }
                    if resp.hovered() {
                        path_ui_hovered = true;
                    }
                };

                // Velocity column
                let has_speed = first_point.and_then(|p| p.speed).is_some();
                attr_indicator_label(has_speed, &mut path_entry.settings.show_speed);

                // Altitude column
                let has_alt = first_point.and_then(|p| p.altitude).is_some();
                attr_indicator_label(has_alt, &mut path_entry.settings.show_altitude);

                // Heading column
                let has_heading = first_point.and_then(|p| p.heading).is_some();
                attr_indicator_label(has_heading, &mut path_entry.settings.show_heading);
                ui.end_row();

                // Hover highlighting
                if path_ui_hovered {
                    self.hovered_path = Some(i);
                }
            }
        });
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct PathEntrySettings {
    pub visible: bool,
    pub show_heading: bool,  // if applicable
    pub show_altitude: bool, // if applicable
    pub show_speed: bool,    // if applicable
}

impl Default for PathEntrySettings {
    fn default() -> Self {
        Self {
            visible: true,
            show_heading: true,
            show_altitude: true,
            show_speed: true,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PathEntry {
    pub data: GeoSpatialData,
    pub settings: PathEntrySettings,
}
