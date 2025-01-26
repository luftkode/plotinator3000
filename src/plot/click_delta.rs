use std::ops::RangeInclusive;

use egui::Color32;
use egui_plot::{Line, MarkerShape, PlotBounds, PlotPoint, PlotPoints, PlotUi, Points};
use serde::{Deserialize, Serialize};

use super::PlotType;

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct ClickDelta {
    // Which plot type the click belongs to
    plot_type: Option<PlotType>,
    first_click: Option<[f64; 2]>,
    second_click: Option<[f64; 2]>,
    pixels_per_point: f64,
}

impl ClickDelta {
    fn delta_x(x1: f64, x2: f64) -> f64 {
        (x2 - x1).abs() / 1_000_000_000.
    }

    // Calculate the x offset for the text that describes the point delta
    fn calc_text_x_offset(x2: f64, range_x: RangeInclusive<f64>) -> f64 {
        let plot_bounds_delta_x = (range_x.start() - range_x.end()).abs();
        let x2_closest_right = (x2 - range_x.end()).abs() < (x2 - range_x.start()).abs();
        if x2_closest_right {
            -plot_bounds_delta_x / 50.
        } else {
            plot_bounds_delta_x / 50.
        }
    }

    // Calculate the y offset for the text that describes the point delta
    fn calc_text_y_offset(y2: f64, range_y: RangeInclusive<f64>) -> f64 {
        let plot_bounds_delta_y = (range_y.start() - range_y.end()).abs();

        let y2_closest_top = (y2 - range_y.end()).abs() < (y2 - range_y.start()).abs();
        if y2_closest_top {
            -plot_bounds_delta_y / 20.
        } else {
            plot_bounds_delta_y / 20.
        }
    }

    pub fn get_delta_text(a: [f64; 2], b: [f64; 2], plot_bounds: PlotBounds) -> egui_plot::Text {
        let [x1, y1] = a;
        let [x2, y2] = b;
        let delta_x = Self::delta_x(x1, x2);
        let delta_y = y2 - y1;
        let delta_formatted = crate::util::format_delta_xy(delta_x, delta_y);
        let x_offset = Self::calc_text_x_offset(x2, plot_bounds.range_x());
        let y_offset = Self::calc_text_y_offset(y2, plot_bounds.range_y());
        let dist_x = x2 - x1;
        let label_x = x1 + dist_x / 2. + x_offset;
        let label_y = y1 + delta_y / 2. + y_offset;
        let label_point = PlotPoint::new(label_x, label_y);
        egui_plot::Text::new(label_point, delta_formatted).highlight(true)
    }

    pub fn set_next_click(&mut self, click: PlotPoint, plot_type: PlotType) {
        if self.plot_type.is_some_and(|pt| pt == plot_type) {
            if self.second_click.is_some() {
                self.replace_first_click([click.x, click.y]);
                self.first_click = None;
                self.second_click = None;
            } else if self.first_click.is_some() {
                self.replace_second_click([click.x, click.y]);
            } else {
                self.replace_first_click([click.x, click.y]);
            }
        } else {
            self.plot_type = Some(plot_type);
            self.second_click = None;
            self.replace_first_click([click.x, click.y]);
        }
    }

    fn replace_first_click(&mut self, click: [f64; 2]) {
        self.first_click = Some(click);
    }

    fn replace_second_click(&mut self, click: [f64; 2]) {
        self.second_click = Some(click);
    }

    pub fn plot_type(&self) -> Option<PlotType> {
        self.plot_type
    }

    pub fn get_click_points(&self) -> Option<Points> {
        match (self.first_click, self.second_click) {
            (None, None) => None,
            (None, Some(_)) => unreachable!("Second click populated when first is empty"),
            (Some(fp), None) => Some(Points::new(fp)),
            (Some(fp), Some(sp)) => Some(Points::new(vec![fp, sp])),
        }
    }

    pub fn get_click_coords(&self) -> (Option<[f64; 2]>, Option<[f64; 2]>) {
        (self.first_click, self.second_click)
    }

    fn delta_color(plot_ui_ctx: &egui::Context) -> Color32 {
        match plot_ui_ctx.theme() {
            egui::Theme::Dark => Color32::WHITE,
            egui::Theme::Light => Color32::BLACK,
        }
    }

    fn ui_delta_line(plot_ui: &mut PlotUi, delta_line: Line, a: [f64; 2], b: [f64; 2]) {
        let delta_text = Self::get_delta_text(a, b, plot_ui.plot_bounds());
        plot_ui.text(delta_text);
        plot_ui.add(delta_line);
    }

    fn ui_click_markers(plot_ui: &mut PlotUi, points: Points, color: Color32) {
        plot_ui.add(
            points
                .shape(MarkerShape::Cross)
                .highlight(true)
                .radius(4.0)
                .color(color),
        );
    }

    pub fn ui(&self, plot_ui: &mut PlotUi, plot_type: PlotType) {
        // We only paint the click delta graphics on the plot type that matches the one that was clicked
        if self.plot_type().is_some_and(|pt| pt == plot_type) {
            if let Some(p) = self.get_click_points() {
                let delta_color = Self::delta_color(plot_ui.ctx());
                Self::ui_click_markers(plot_ui, p, delta_color);

                match self.get_click_coords() {
                    (None, None) => (),
                    (None, Some(_)) => unreachable!("Second click populated when first is empty"),
                    (Some(fpoint), None) => {
                        if let Some(pointer_coord) = plot_ui.pointer_coordinate() {
                            let pcoord = [pointer_coord.x, pointer_coord.y];
                            let delta_line = Line::new(PlotPoints::new([fpoint, pcoord].into()))
                                .color(delta_color);

                            Self::ui_delta_line(plot_ui, delta_line, fpoint, pcoord);
                        }
                    }
                    (Some(fpoint), Some(spoint)) => {
                        let delta_line = Line::new(PlotPoints::new([fpoint, spoint].into()))
                            .color(delta_color)
                            .name("Δ Click")
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 });

                        Self::ui_delta_line(plot_ui, delta_line, fpoint, spoint);
                    }
                }
            }
        }
    }
}
