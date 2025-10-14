#![cfg(not(target_arch = "wasm32"))]
use egui::{
    Align2, CentralPanel, Color32, Frame, Grid, MenuBar, Pos2, RichText, TopBottomPanel, Ui,
    ViewportBuilder, ViewportId, Window,
};
use egui_phosphor::regular::{
    AIRPLANE, CHECK_CIRCLE, CHECK_SQUARE, CIRCLE, GEAR, GLOBE, GLOBE_HEMISPHERE_WEST,
    SELECTION_ALL, SQUARE,
};
use plotinator_log_if::{
    prelude::PrimaryGeoSpatialData,
    rawplot::path_data::{AuxiliaryGeoSpatialData, GeoSpatialDataset},
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::mpsc::{Receiver, Sender};
use walkers::{Map, Position};

use crate::{
    commander::{MapCommand, MapUiChannels, MapUiCommander, PlotMessage},
    draw::{
        DrawSettings,
        labels::{LabelPlacer, TelemetryLabelSettings},
    },
    geo_path::{ClosestPoint, GeoPath, MqttGeoPath, PathEntry, find_closest_point_to_cursor},
    map_state::MapState,
};

pub mod commander;
mod draw;
pub(crate) mod geo_path;
mod map_state;

#[derive(Default, Deserialize, Serialize)]
pub struct MapViewPort {
    pub open: bool,
    pub geo_data: Vec<PathEntry>,
    mqtt_geo_data: SmallVec<[MqttGeoPath; 3]>,
    pub unmerged_aux_data: Vec<AuxiliaryGeoSpatialData>,
    map_state: MapState,

    label_placer: LabelPlacer,

    pub heading_arrow_threshold: f64,
    pub telemetry_label_threshold: f64,

    #[serde(skip)]
    cmd_rx: Option<Receiver<MapCommand>>,
    /// The time corresponding to the cursor position in the plot area
    #[serde(skip)]
    plot_time_pointer_pos: Option<f64>,
    /// Point on the map that is currently hovered
    #[serde(skip)]
    map_hovered_point: Option<ClosestPoint>,
    #[serde(skip)]
    hovered_path: Option<usize>, // index of hovered path
    #[serde(skip)]
    hovered_mqtt_path: Option<usize>,
    #[serde(skip)]
    mqtt_latest_position: Option<Position>,

    #[serde(skip)]
    plot_msg_tx: Option<Sender<PlotMessage>>,
    // What is the last position of the pointer that hovered on the map
    #[serde(skip)]
    pointer_hovered_pos: Option<Pos2>,
    // Is the map currently hovered on?
    #[serde(skip)]
    map_hovered: bool,
}

impl MapViewPort {
    /// Open the [`MapViewPort`]
    ///
    /// if it's the first time it's opened, it will start loading map tiles and
    /// return a [Sender<MapCommand>] for interacting with the Map from other contexts
    pub fn open(
        &mut self,
        ctx: &egui::Context,
    ) -> (Option<Sender<MapCommand>>, Option<Receiver<PlotMessage>>) {
        if self.map_state.tile_state.is_none() {
            egui_extras::install_image_loaders(ctx);
            self.map_state.init(ctx.clone());
        }
        let mut maybe_map_tx = None;
        let mut maybe_plot_rx = None;
        if self.cmd_rx.is_none() {
            let MapUiChannels {
                map_cmd_tx,
                map_cmd_rx,
                plot_msg_tx,
                plot_msg_rx,
            } = MapUiCommander::channels();
            maybe_map_tx = Some(map_cmd_tx);
            maybe_plot_rx = Some(plot_msg_rx);
            self.cmd_rx = Some(map_cmd_rx);
            self.plot_msg_tx = Some(plot_msg_tx);
        }
        self.open = true;

        (maybe_map_tx, maybe_plot_rx)
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn poll_commands(&mut self) {
        let mut pointer_pos: Option<f64> = None;
        while let Ok(cmd) = self.cmd_rx.as_ref().expect("unsound condition").try_recv() {
            match cmd {
                MapCommand::AddGeoData(geo_data) => {
                    match geo_data {
                        GeoSpatialDataset::PrimaryGeoSpatialData(mut primary_data) => {
                            if let Some(first_point) = primary_data.points.first() {
                                let has_speed = first_point.speed.is_some();
                                let has_altitude = first_point.altitude.is_some();
                                let has_heading = first_point.heading.is_some();
                                log::info!(
                                    "Received geo data {}, points include speed={has_speed}, altitude={has_altitude}, heading={has_heading}",
                                    primary_data.name
                                );
                            } else {
                                log::info!(
                                    "Received basic geo data {} with only coordinates",
                                    primary_data.name
                                );
                            }

                            for unmerged_aux in &self.unmerged_aux_data {
                                let _ = primary_data.merge_auxiliary(unmerged_aux, 5e9);
                            }

                            self.add_geo_data(primary_data);
                        }
                        GeoSpatialDataset::AuxGeoSpatialData(aux_data) => {
                            for path in &mut self.geo_data {
                                let _ = path.data.merge_auxiliary(&aux_data, 5e9);
                            }
                        }
                    }

                    self.zoom_to_fit();
                }
                MapCommand::MQTTGeoData(geo_points) => {
                    let maybe_first_data = self.mqtt_geo_data.is_empty();

                    // Iterate through incoming MQTT points (typically 1-2, max ~10)
                    for mqtt_point in geo_points.into_iter() {
                        let mut match_found = false;
                        for mqtt_geo_path in &mut self.mqtt_geo_data {
                            if mqtt_point.topic == mqtt_geo_path.topic {
                                match_found = true;
                                // Check if last point has matching timestamp (within microsecond precision)
                                if let Some(last_point) = mqtt_geo_path.points.last_mut()
                                    && (last_point.timestamp - mqtt_point.point.timestamp).abs()
                                        < 1e6
                                {
                                    *last_point = mqtt_point.point;
                                } else {
                                    mqtt_geo_path.push(mqtt_point.point);
                                }
                                break;
                            }
                        }
                        if !match_found {
                            self.mqtt_geo_data.push(mqtt_point.into());
                        }
                    }

                    if maybe_first_data && !self.mqtt_geo_data.is_empty() {
                        self.zoom_to_fit();
                        // After the initial zoom, start following the position automatically.
                        if let Some(tile_state) = self.map_state.tile_state_as_mut() {
                            tile_state.map_memory.follow_my_position();
                        }
                    }

                    // Update position for the map widget to follow.
                    if let Some(latest_point) = self
                        .mqtt_geo_data
                        .first()
                        .and_then(|path| path.points.last())
                    {
                        self.mqtt_latest_position = Some(latest_point.position);
                    }
                }
                MapCommand::PointerPos(time_pos) => {
                    log::trace!("Got pointer time: {time_pos:.}");
                    pointer_pos = Some(time_pos);
                }
                MapCommand::FitToAllPaths => {
                    self.zoom_to_fit();
                }
                MapCommand::Reset => {
                    self.geo_data.clear();
                    self.mqtt_geo_data.clear();
                }
            }
        }
        if let Some(pos) = pointer_pos {
            self.plot_time_pointer_pos = Some(pos);
        }
    }

    fn zoom_to_fit(&mut self) {
        self.map_state
            .zoom_to_fit(&self.geo_data, &self.mqtt_geo_data);
    }

    pub fn add_geo_data(&mut self, data: PrimaryGeoSpatialData) {
        self.geo_data.push(PathEntry::new(data));
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
                    self.pointer_hovered_pos = if ui.ui_contains_pointer() {
                        ctx.pointer_hover_pos()
                    } else {
                        None
                    };
                    let new_is_map_hovered = self.pointer_hovered_pos.is_some();
                    // Was hovered -> Not hovered any longer
                    if self.map_hovered && !new_is_map_hovered {
                        self.map_hovered_point = None;
                        self.send_map_pointer_pos(None);
                        self.map_hovered = false;
                    }
                    // Not hovered -> Now hovered
                    else if !self.map_hovered && new_is_map_hovered {
                        self.plot_time_pointer_pos = None;
                        self.map_hovered = true;
                    }
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

    // Sending `None` clear the cached value of the receiver
    fn send_map_pointer_pos(&mut self, pos: Option<(f64, Color32)>) {
        if let Some(tx) = &mut self.plot_msg_tx {
            tx.send(PlotMessage::PointerTimestamp(pos)).ok();
        }
    }

    /// Renders the menu bar at the top of the viewport.
    fn show_menu_bar(&mut self, ui: &mut Ui) {
        MenuBar::new().ui(ui, |ui| {


            ui.menu_button(format!("{GEAR} Display Settings"), |ui| {
            ui.label("Zoom thresholds");
            ui.add(
                egui::Slider::new(&mut self.heading_arrow_threshold, 10.0..=20.0)
                    .text("Heading arrows")
                    .suffix("x zoom"),
            );
            ui.add(
                egui::Slider::new(&mut self.telemetry_label_threshold, 10.0..=20.0)
                    .text("Telemetry labels")
                    .suffix("x zoom"),
                );
            });

            let (is_satellite, is_detached) = {
                let map_state = self
                    .map_state
                    .tile_state_as_mut()
                    .expect("map_tile_state is required but not initialized");
                (
                    map_state.is_satellite,
                    map_state.map_memory.detached().is_some(),
                )
            };

            let icon = if is_satellite {
                GLOBE_HEMISPHERE_WEST
            } else {
                GLOBE
            };
            if ui
                .button(RichText::new(format!("{icon} Toggle Map")))
                .clicked()
            {
                self.map_state.toggle_map_style(ui.ctx().clone());
            }

            if ui
                .button(RichText::new(format!("{SELECTION_ALL} Zoom to fit")))
                .clicked()
            {
                self.zoom_to_fit();
            }

            // The button is only enabled when the map is in a "detached" state, i.e., the user
            // has dragged it away from the followed position.
            let follow_button = egui::Button::new(RichText::new(format!("{AIRPLANE} Follow Position")));

            if ui.add_enabled(is_detached, follow_button)
                .on_hover_text("Follow the live position, locking the map's center to the latest coordinate received on MQTT. Only applies to the latest received MQTT point")
                .clicked()
            {
                self.map_state
                    .tile_state_as_mut()
                    .expect("map_tile_state is required but not initialized")
                    .map_memory
                    .follow_my_position();
            }

        });
    }

    /// Renders the main map panel and all geographical data on it.
    fn show_map_panel(&mut self, ui: &mut Ui) {
        let pointer_pos = self.pointer_hovered_pos;
        let fallback_position = self.map_state.data().center_position;
        let my_position = self.mqtt_latest_position.unwrap_or(fallback_position);

        let tile_state = self
            .map_state
            .tile_state_as_mut()
            .expect("map_tile_state is required but not initialized");

        let zoom_level = tile_state.zoom_level();
        let map = Map::new(
            Some(tile_state.tiles.as_mut()),
            &mut tile_state.map_memory,
            my_position,
        )
        .double_click_to_zoom(true);

        map.show(ui, |ui, projector, _map_rect| {
            let draw_heading = zoom_level > self.heading_arrow_threshold;
            let draw_telemetry_label = zoom_level > self.telemetry_label_threshold;

            let draw_settings_fn = |path: &dyn GeoPath| DrawSettings {
                draw_heading_arrows: draw_heading && path.path_settings().show_heading,
                telemetry_label: TelemetryLabelSettings {
                    draw: draw_telemetry_label,
                    with_speed: path.path_settings().show_speed,
                    with_altitude: path.path_settings().show_altitude,
                },
            };

            self.label_placer
                .begin_frame(ui.available_rect_before_wrap());

            // Draw regular paths
            for (i, path) in self.geo_data.iter().enumerate() {
                if !path.is_visible() {
                    continue;
                }
                let is_hovered = self.hovered_path == Some(i);

                draw::draw_path(
                    ui,
                    projector,
                    path,
                    &draw_settings_fn(path),
                    &mut self.label_placer,
                );
                if is_hovered {
                    draw::highlight_whole_path(ui.painter(), projector, path);
                }
            }

            // Draw MQTT paths
            for (i, path) in self.mqtt_geo_data.iter().enumerate() {
                if !path.is_visible() {
                    continue;
                }
                let is_hovered = self.hovered_mqtt_path == Some(i);

                draw::draw_path(
                    ui,
                    projector,
                    path,
                    &draw_settings_fn(path),
                    &mut self.label_placer,
                );
                if is_hovered {
                    draw::highlight_whole_path(ui.painter(), projector, path);
                }
            }

            if draw_telemetry_label {
                self.label_placer.place_all_labels(ui.painter());
            }

            if let Some(pointer_time) = self.plot_time_pointer_pos {
                draw::draw_pointer_highlights(
                    ui.painter(),
                    projector,
                    &self.geo_data,
                    pointer_time,
                );
                draw::draw_pointer_highlights(
                    ui.painter(),
                    projector,
                    &self.mqtt_geo_data,
                    pointer_time,
                );
            }

            if let Some(hovered_point) = &self.map_hovered_point {
                draw::draw_hover_point_highlight(
                    ui.painter(),
                    hovered_point.screen_pos,
                    hovered_point.path_color,
                );
            }

            if let Some(pointer_pos) = pointer_pos {
                let hovered_point = find_closest_point_to_cursor(
                    &self.geo_data,
                    &self.mqtt_geo_data,
                    pointer_pos,
                    projector,
                );

                if let Some(plot_tx) = &mut self.plot_msg_tx {
                    if let Some(point) = &hovered_point {
                        plot_tx
                            .send(PlotMessage::PointerTimestamp(Some((
                                point.timestamp,
                                point.path_color,
                            ))))
                            .ok();
                    } else if self.map_hovered_point.is_some() {
                        self.map_hovered_point = None;
                        plot_tx.send(PlotMessage::PointerTimestamp(None)).ok();
                    }
                }
                self.map_hovered_point = hovered_point;
            }
        });

        Self::zoom_controls(ui, &mut tile_state.map_memory);
    }

    /// Renders the legend window with path information.
    fn show_legend_window(&mut self, ctx: &egui::Context) {
        Window::new("Legend")
            .title_bar(true)
            .resizable(true)
            .default_pos(egui::pos2(0.0, 32.0))
            .default_size([200.0, 150.0])
            .show(ctx, |ui| {
                if self.geo_data.is_empty() && self.mqtt_geo_data.is_empty() {
                    ui.label("No paths loaded");
                } else {
                    self.show_legend_grid(ui);
                }

                // Reset hovered state if the mouse leaves the legend window
                if !ui.ui_contains_pointer() {
                    self.hovered_path = None;
                    self.hovered_mqtt_path = None;
                }
            });
    }

    #[allow(
        clippy::too_many_lines,
        reason = "it's only the legend grid, it's fine"
    )]
    fn show_legend_grid(&mut self, ui: &mut Ui) {
        Grid::new("legend_grid").striped(true).show(ui, |ui| {
            // Column headers
            ui.label(""); // empty cell for toggle + name
            ui.label("vel");
            ui.label("alt");
            ui.label("hdg");
            ui.end_row();

            // Helper for attribute indicator buttons
            let attr_button = |ui: &mut Ui, has_attr: bool, show_attr: &mut bool| -> bool {
                let text = if has_attr {
                    if *show_attr {
                        RichText::new(CHECK_CIRCLE).color(Color32::GREEN)
                    } else {
                        RichText::new(CIRCLE).color(Color32::GREEN).weak()
                    }
                } else {
                    RichText::new(CIRCLE).weak()
                };
                let resp = ui.button(text);
                if resp.clicked() {
                    *show_attr = !*show_attr;
                }
                resp.hovered()
            };

            // Regular paths
            for (i, path_entry) in self.geo_data.iter_mut().enumerate() {
                let path = &path_entry.data;
                let first_point = path.points.first();
                let mut hovered = false;

                ui.horizontal(|ui| {
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
                        hovered = true;
                    }
                });

                hovered |= attr_button(
                    ui,
                    first_point.and_then(|p| p.speed).is_some(),
                    &mut path_entry.settings.show_speed,
                );
                hovered |= attr_button(
                    ui,
                    first_point.and_then(|p| p.altitude).is_some(),
                    &mut path_entry.settings.show_altitude,
                );
                hovered |= attr_button(
                    ui,
                    first_point.and_then(|p| p.heading).is_some(),
                    &mut path_entry.settings.show_heading,
                );

                ui.end_row();
                if hovered {
                    self.hovered_path = Some(i);
                }
            }

            // MQTT paths
            for (i, mqtt_path) in self.mqtt_geo_data.iter_mut().enumerate() {
                let first_point = mqtt_path.points.first();
                let mut hovered = false;

                ui.horizontal(|ui| {
                    let indicator = if mqtt_path.settings.visible {
                        RichText::new(CHECK_SQUARE).color(mqtt_path.color).weak()
                    } else {
                        RichText::new(SQUARE).color(mqtt_path.color).strong()
                    };
                    if ui.button(indicator).clicked() {
                        mqtt_path.settings.visible = !mqtt_path.settings.visible;
                    }
                    ui.label(RichText::new(&mqtt_path.topic).strong());
                    if ui.ui_contains_pointer() {
                        hovered = true;
                    }
                });

                hovered |= attr_button(
                    ui,
                    first_point.and_then(|p| p.speed).is_some(),
                    &mut mqtt_path.settings.show_speed,
                );
                hovered |= attr_button(
                    ui,
                    first_point.and_then(|p| p.altitude).is_some(),
                    &mut mqtt_path.settings.show_altitude,
                );
                hovered |= attr_button(
                    ui,
                    first_point.and_then(|p| p.heading).is_some(),
                    &mut mqtt_path.settings.show_heading,
                );

                ui.end_row();
                if hovered {
                    self.hovered_mqtt_path = Some(i);
                }
            }
        });
    }

    /// Simple GUI to zoom in and out.
    pub fn zoom_controls(ui: &Ui, map_memory: &mut walkers::MapMemory) {
        Window::new("map_zoom_ctrls")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, [10., -10.])
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("➕").heading()).clicked() {
                        let _ = map_memory.zoom_in();
                    }

                    if ui.button(RichText::new("➖").heading()).clicked() {
                        let _ = map_memory.zoom_out();
                    }
                });
            });
    }
}
