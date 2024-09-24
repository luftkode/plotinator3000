use egui::{Color32, RichText};
use egui_phosphor::regular;
use egui_plot::PlotBounds;

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AxisConfig {
    link_x: bool,
    link_cursor_x: bool,
    show_axes: bool,
    y_axis_lock: YAxisLock,
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            link_x: true,
            link_cursor_x: true,
            show_axes: true,
            y_axis_lock: YAxisLock::default(),
        }
    }
}

impl AxisConfig {
    pub fn show_axes(&self) -> bool {
        self.show_axes
    }
    pub fn link_x(&self) -> bool {
        self.link_x
    }
    pub fn link_cursor_x(&self) -> bool {
        self.link_cursor_x
    }
    pub fn handle_y_axis_lock<F>(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi,
        plot_type: PlotType,
        between_bounds_update_fn: F,
    ) where
        F: FnOnce(&mut egui_plot::PlotUi),
    {
        self.y_axis_lock
            .handle(plot_ui, plot_type, between_bounds_update_fn);
    }

    pub fn toggle_axis_cfg_ui(&mut self, ui: &mut egui::Ui) {
        let linked_x_axis_text = format!(
            "{} Linked Axes",
            if self.link_x {
                regular::LINK_SIMPLE
            } else {
                regular::LINK_BREAK
            }
        );
        _ = ui.toggle_value(&mut self.link_x, linked_x_axis_text);
        let linked_x_cursor_text = format!(
            "{} Linked Cursors",
            if self.link_cursor_x {
                regular::LINK_SIMPLE
            } else {
                regular::LINK_BREAK
            }
        );
        _ = ui.toggle_value(&mut self.link_cursor_x, linked_x_cursor_text);
        let show_axes_text = format!(
            "{} Axes",
            if self.show_axes {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );
        _ = ui.toggle_value(&mut self.show_axes, show_axes_text);
        let is_y_axis_locked = self.y_axis_lock.lock_y_axis;
        let lock_y_axis_text = RichText::new(format!(
            "{} Lock Y-axis",
            if is_y_axis_locked {
                regular::LOCK_LAMINATED
            } else {
                regular::LOCK_SIMPLE_OPEN
            }
        ));
        let lock_y_axis_text = if is_y_axis_locked {
            lock_y_axis_text.color(Color32::RED)
        } else {
            lock_y_axis_text
        };

        _ = ui.toggle_value(&mut self.y_axis_lock.lock_y_axis, lock_y_axis_text);
    }
}

#[derive(Copy, Clone)]
pub enum PlotType {
    Percentage,
    Hundreds,
    Thousands,
}

#[derive(Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct YAxisLock {
    lock_y_axis: bool,
    y_bounds_percentage: Option<PlotBounds>,
    y_bounds_hundreds: Option<PlotBounds>,
    y_bounds_thousands: Option<PlotBounds>,
    y_bounds_generator: Option<PlotBounds>,
}

impl YAxisLock {
    pub fn handle<F>(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi,
        plot_type: PlotType,
        between_bounds_update_fn: F,
    ) where
        F: FnOnce(&mut egui_plot::PlotUi),
    {
        let bounds = self.get_bounds(plot_type);
        if self.lock_y_axis {
            if let Some(y_bounds) = &bounds {
                let mut plot_bounds = plot_ui.plot_bounds();
                plot_bounds.set_y(y_bounds);
                plot_ui.set_plot_bounds(plot_bounds);
            }
        }
        between_bounds_update_fn(plot_ui);

        if !self.lock_y_axis {
            self.set_bounds(plot_type, plot_ui.plot_bounds());
        }
    }

    fn get_bounds(&self, plot_type: PlotType) -> Option<PlotBounds> {
        match plot_type {
            PlotType::Percentage => self.y_bounds_percentage,
            PlotType::Hundreds => self.y_bounds_hundreds,
            PlotType::Thousands => self.y_bounds_thousands,
        }
    }

    fn set_bounds(&mut self, plot_type: PlotType, bounds: PlotBounds) {
        match plot_type {
            PlotType::Percentage => self.y_bounds_percentage = Some(bounds),
            PlotType::Hundreds => self.y_bounds_hundreds = Some(bounds),
            PlotType::Thousands => self.y_bounds_thousands = Some(bounds),
        }
    }
}
