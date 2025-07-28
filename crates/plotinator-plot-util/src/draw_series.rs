use egui::{Color32, Stroke};
use egui_plot::{Line, PlotPoint, PlotPoints, Polygon};
use serde::{Deserialize, Serialize};

/// Defines how a plot series should be rendered.
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
pub enum SeriesDrawMode {
    /// Draw a line connecting points, with rhombuses on each point if they are not too dense.
    LineWithEmphasis { width: f32 },
    /// Draw only the connecting line.
    Line { width: f32 },
    /// Draw only rhombuses on each point.
    Scatter { width: f32 },
}

impl Default for SeriesDrawMode {
    fn default() -> Self {
        Self::LineWithEmphasis { width: 1.5 }
    }
}

impl SeriesDrawMode {
    // Final stop for constructing a line series and handing it over to `egui_plot`
    pub(crate) fn draw_series<'p>(
        &self,
        plot_ui: &mut egui_plot::PlotUi<'p>,
        points: PlotPoints<'p>,
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

                let rhombus = Polygon::new(label, rhombus_vertices)
                    .allow_hover(false) // Make it non-interactive (otherwise cursor might snap to a polygon point, which misleads them to believe it's an actual data point)
                    .stroke(egui::Stroke::new(self.polygon_stroke_width(), color));

                plot_ui.polygon(rhombus);
            }
        }

        let line = Line::new(label, points)
            .width(self.line_width())
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
            Self::LineWithEmphasis { .. } | Self::Line { .. } => true,
            Self::Scatter { .. } => false,
        }
    }

    fn draw_rhombus(&self) -> bool {
        matches!(self, Self::Scatter { .. })
    }

    fn polygon_stroke_width(&self) -> f32 {
        match self {
            // If the line gets quite thick, we want the polygons to scale as well so we can still see point emphasis
            Self::LineWithEmphasis { width } => (width - 0.5).max(1.0),
            Self::Line { .. } => 0.0,
            Self::Scatter { .. } => 1.5, // We scale the rhombus radius instead of the thickness
        }
    }

    fn line_width(&self) -> f32 {
        match self {
            Self::LineWithEmphasis { width } | Self::Line { width } => *width,
            // A line width of zero hides the line plot, but allows the scatter plot to work in the same way as the line plot in terms of
            // auto bounds, and lets users see the values of individual data points by hovering near the polygons
            Self::Scatter { .. } => 0.0,
        }
    }

    fn rhombus_radius(&self) -> f64 {
        match self {
            Self::LineWithEmphasis { .. } | Self::Line { .. } => 1.5,
            Self::Scatter { width } => (*width).into(),
        }
    }

    /// Check if points are spaced widely enough to draw markers
    ///
    /// Only applies in emphasis mode
    fn emphasize_points(&self, points: &[PlotPoint], dvalue_dpos: [f64; 2]) -> bool {
        if !matches!(self, Self::LineWithEmphasis { .. }) {
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
