pub(crate) mod plot_app;

/// Orchestrates Plotinator3000 GUI, both the primary plotting viewport and the map viewport
pub struct GlobalApp {
    // The first time geo spatial data is loaded, we pop up the map window, but not on subsequent loads
    #[cfg(feature = "map")]
    has_map_opened: bool,
    #[cfg(feature = "map")]
    map_view: plotinator_map_ui::MapViewPort,
    app: crate::PlotApp,
}

impl GlobalApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            #[cfg(feature = "map")]
            has_map_opened: false,
            #[cfg(feature = "map")]
            map_view: plotinator_map_ui::MapViewPort::default(),
            app: crate::PlotApp::new(cc),
        }
    }

    #[cfg(feature = "map")]
    fn open_map_viewport(&mut self) {
        if self.map_view.open {
            return;
        }

        if let (Some(cmd_send), Some(plot_msg_recv)) = self.map_view.open() {
            self.app.map_commander.init(cmd_send, plot_msg_recv);
        }
    }

    #[cfg(feature = "map")]
    fn toggle_open_map_viewport(&mut self) {
        if self.map_view.open {
            self.map_view.close();
        } else {
            self.open_map_viewport();
        }
    }
}

impl eframe::App for GlobalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(feature = "map")]
        self.map_view.update(ctx);
        self.app.update(ctx, frame);

        #[cfg(feature = "map")]
        {
            if !self.has_map_opened && self.app.map_commander.any_primary_data_received {
                self.has_map_opened = true;
                self.open_map_viewport();
            }
            if self.app.map_commander.map_button_clicked {
                self.app.map_commander.map_button_clicked = false;
                self.toggle_open_map_viewport();
            }
            self.app.map_commander.sync_open(self.map_view.open);
        }
    }
}
