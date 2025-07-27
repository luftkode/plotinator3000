use egui::{Color32, Stroke};
use egui_plot::{Line, PlotPoint, PlotPoints, Polygon};
use serde::{Deserialize, Serialize};

/// Defines how a plot series should be rendered.
#[derive(Deserialize, Serialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeriesDrawMode {
    /// Draw a line connecting points, with rhombuses on each point if they are not too dense.
    #[default]
    LineWithEmphasis,
    /// Draw only the connecting line.
    Line,
    /// Draw only rhombuses on each point.
    Scatter,
}

impl SeriesDrawMode {
    // Final stop for constructing a line series and handing it over to `egui_plot`
    pub(crate) fn draw_series<'p>(
        &self,
        plot_ui: &mut egui_plot::PlotUi<'p>,
        points: PlotPoints<'p>,
        width: f32,
        label: &str,
        color: Color32,
        highlight: bool,
    ) {
        let dvalue_dpos = plot_ui.transform().dvalue_dpos();
        let draw_rhombus = self.draw_rhombus();

        let points_slice = points.points();
        let emphasize_points = self.emphasize_points(points_slice, dvalue_dpos);

        if draw_rhombus || emphasize_points {
            let [x_dvalue_dpos, y_dvalue_dpos] = dvalue_dpos;
            let dx = self.rhombus_radius() * x_dvalue_dpos;
            let dy = self.rhombus_radius() * y_dvalue_dpos;

            for point in points_slice {
                let top = [point.x, point.y + dy];
                let right = [point.x + dx, point.y];
                let bottom = [point.x, point.y - dy];
                let left = [point.x - dx, point.y];
                let rhombus_vertices = vec![top, right, bottom, left];

                // Use the adjustable width from the UI if we're displaying a scatter plot
                let stroke_width = if draw_rhombus {
                    width
                } else {
                    (width - 0.5).max(1.0)
                };

                let rhombus = Polygon::new(label, rhombus_vertices)
                    .allow_hover(false) // Make it non-interactive (otherwise cursor might snap to a polygon point, which misleads them to believe it's an actual data point)
                    .stroke(egui::Stroke::new(stroke_width, color));

                plot_ui.polygon(rhombus);
            }
        }

        let line = Line::new(label, points)
            .width(width)
            .color(color)
            .highlight(highlight);

        let line = if self.draw_line() {
            line
        } else {
            line.stroke(Stroke::new(0., color))
        };

        plot_ui.line(line);
    }

    const fn draw_line(&self) -> bool {
        match self {
            Self::LineWithEmphasis | Self::Line => true,
            Self::Scatter => false,
        }
    }

    fn draw_rhombus(&self) -> bool {
        *self == Self::Scatter
    }

    const fn rhombus_radius(&self) -> f64 {
        match self {
            Self::LineWithEmphasis | Self::Line => 2.0,
            Self::Scatter => 4.0,
        }
    }

    /// Check if points are spaced widely enough to draw markers
    ///
    /// Only applies in emphasis mode
    fn emphasize_points(&self, points: &[PlotPoint], dvalue_dpos: [f64; 2]) -> bool {
        if !(*self == Self::LineWithEmphasis) {
            return false;
        }

        let dx = self.rhombus_radius() * dvalue_dpos[0];

        // A single point can always be emphasized.
        if points.len() > 1 {
            // We check against 3x the rhombus width.
            let min_dist_x = 3. * dx;
            for p_pair in points.windows(2) {
                let [x1, x2] = [p_pair[0].x, p_pair[1].x];
                let delta_abs = (x2 - x1).abs();
                // protects against machine epsilon (floating point precision issue)
                if delta_abs == 0. {
                    continue;
                }
                if delta_abs < min_dist_x {
                    return false;
                }
            }
        }

        true
    }
}
