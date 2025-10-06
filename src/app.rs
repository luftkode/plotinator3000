pub(crate) mod plot_app;

use plotinator_map_ui::MapViewPort;

/// Orchestrates Plotinator3000 GUI, both the primary plotting viewport and the map viewport
pub struct GlobalApp {
    // The first time geo spatial data is loaded, we pop up the map window, but not on subsequent loads
    has_map_opened: bool,
    app: crate::PlotApp,
    map_view: MapViewPort,
}

impl GlobalApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            has_map_opened: false,
            app: crate::PlotApp::new(cc),
            map_view: MapViewPort::default(),
        }
    }

    fn open_map_viewport(&mut self, ctx: &egui::Context) {
        if self.map_view.open {
            return;
        }

        if let Some(cmd_send) = self.map_view.open(ctx) {
            self.app.map_commander.init(cmd_send);
        }
    }
}

impl eframe::App for GlobalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.map_view.show(ctx);
        self.app.update(ctx, frame);

        if !self.has_map_opened && self.app.map_commander.any_data_received {
            self.has_map_opened = true;
            self.open_map_viewport(ctx);
        }
        if self.app.map_commander.map_button_clicked {
            self.app.map_commander.map_button_clicked = false;
            if self.map_view.open {
                self.map_view.close();
            } else {
                self.open_map_viewport(ctx);
            }
        }
        self.app.map_commander.sync_open(self.map_view.open);
    }
}
