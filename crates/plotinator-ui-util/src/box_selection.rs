use egui_plot::{PlotBounds, PlotPoint, PlotUi};

use crate::PlotType;

/// Holds the state for a drag-to-select box operation on a plot.
#[derive(Clone, Copy, Debug, Default)]
pub struct BoxSelection {
    // Which plot type the box selection belongs to
    plot_type: Option<PlotType>,
    selected: Option<PlotBounds>,
    start: Option<PlotPoint>,
    is_selecting: bool,
    x_key_was_down: bool,
}

impl BoxSelection {
    pub fn record_key_and_pointer_events(&mut self, plot_ui: &mut PlotUi, plot_type: PlotType) {
        let x_key_down = plot_ui.ctx().input(|i| i.key_down(egui::Key::X));
        let pointer_coord = plot_ui.pointer_coordinate();

        // Detect actual key press (transition from up to down)
        let x_key_pressed_now = x_key_down && !self.x_key_was_down;
        // Detect actual key release (transition from down to up)
        let x_key_released_now = !x_key_down && self.x_key_was_down;

        self.x_key_was_down = x_key_down;

        if x_key_pressed_now {
            self.start_selection(pointer_coord, plot_type);
        }

        // Complete selection when X key is actually released
        if x_key_released_now && self.is_selecting {
            if let (Some(start), Some(end)) = (self.start, pointer_coord) {
                log::debug!("Completing selection from {start:?} to {end:?}");
                self.complete_selection(start, end);
            } else {
                log::debug!("Canceling selection - missing coordinates");
            }
            self.stop_selection();
        }

        if self.plot_type.is_some_and(|pt| pt == plot_type) && self.is_selecting && x_key_down {
            self.draw_selection_box(plot_ui, pointer_coord);
        }
    }

    fn draw_selection_box(&self, plot_ui: &mut PlotUi, pointer_coord: Option<PlotPoint>) {
        if let (Some(start_plot), Some(current_plot)) = (self.start, pointer_coord) {
            let start_screen = plot_ui.screen_from_plot(start_plot);
            let current_screen = plot_ui.screen_from_plot(current_plot);
            let selection_rect = egui::Rect::from_two_pos(start_screen, current_screen);

            let painter = plot_ui.ctx().debug_painter();

            painter.rect_filled(
                selection_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(100, 100, 200, 50),
            );
            painter.rect_stroke(
                selection_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::WHITE),
                egui::StrokeKind::Middle,
            );
        }
    }

    fn stop_selection(&mut self) {
        self.is_selecting = false;
        self.start = None;
    }

    fn start_selection(&mut self, pointer_coord: Option<PlotPoint>, plot_type: PlotType) {
        if let Some(coord) = pointer_coord {
            log::debug!("Starting selection plot type: {plot_type} at {coord:?}");
            self.plot_type = Some(plot_type);
            self.start = Some(coord);
            self.is_selecting = true;
        }
    }

    fn complete_selection(&mut self, start: PlotPoint, end: PlotPoint) {
        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        let min_y = start.y.min(end.y);
        let max_y = start.y.max(end.y);
        let bounds = PlotBounds::from_min_max([min_x, min_y], [max_x, max_y]);
        log::info!("Box selection completed. Bounds: {bounds:?}");
        self.selected = Some(bounds);
    }

    /// Return the last selected `PlotBounds`
    pub fn selected(&self) -> Option<PlotBounds> {
        self.selected
    }
}
