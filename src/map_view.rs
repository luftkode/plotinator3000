use plotinator_map_ui::MapViewPort;

pub struct AppWithMap {
    // The first time geo spatial data is loaded, we pop up the map window, but not on subsequent loads
    has_map_opened: bool,
    app: crate::App,
    map_view: MapViewPort,
}

impl AppWithMap {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            has_map_opened: false,
            app: crate::App::new(cc),
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

impl eframe::App for AppWithMap {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Windows", |ui| {
                    if ui.button("Show Map").clicked() {
                        self.open_map_viewport(ctx);
                        ui.close();
                    }
                });
            });
        });

        self.map_view.show(ctx);
        self.app.update(ctx, frame);

        if !self.has_map_opened && self.app.map_commander.any_data_received {
            self.has_map_opened = true;
            self.open_map_viewport(ctx);
        }
        self.app.map_commander.sync_open(self.map_view.open);
    }
}
