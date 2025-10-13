use egui::{Color32, FontId, Painter, Pos2, Rect, epaint::Galley, vec2};
use plotinator_log_if::prelude::GeoPoint;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::draw::TelemetryLabelSettings;

/// Pre-calculated label data ready for placement
pub struct PreCalculatedLabel {
    pub rect: Rect,
    pub galleys: Vec<Arc<Galley>>,
}

/// Manages the placement of non-overlapping labels on a 2D plane.
#[derive(Deserialize, Serialize)]
pub struct LabelPlacer {
    placed_rects: Vec<Rect>,
    grid: Vec<Vec<usize>>,
    grid_dims: (usize, usize),
    cell_size: f32,
    // buffers for caching
    #[serde(skip)]
    candidate_buffer: Vec<(Pos2, GeoPoint)>,
    #[serde(skip)]
    precalc_buffer: Vec<PreCalculatedLabel>,
}

impl Default for LabelPlacer {
    fn default() -> Self {
        Self::new(32.0)
    }
}

impl LabelPlacer {
    /// Creates a new `LabelPlacer` with a given cell size.
    pub fn new(cell_size: f32) -> Self {
        Self {
            placed_rects: Vec::with_capacity(128),
            grid: Vec::new(),
            grid_dims: (0, 0),
            cell_size,
            candidate_buffer: Vec::new(),
            precalc_buffer: Vec::new(),
        }
    }

    /// Prepares the placer for a new frame.
    ///
    /// This must be called once at the beginning of each frame, before drawing any paths.
    /// Pass the `Rect` from the map closure.
    pub fn begin_frame(&mut self, screen_rect: Rect) {
        self.placed_rects.clear();
        for cell in &mut self.grid {
            cell.clear();
        }

        let new_grid_dims = (
            (screen_rect.width() / self.cell_size).ceil() as usize + 1,
            (screen_rect.height() / self.cell_size).ceil() as usize + 1,
        );

        if self.grid_dims != new_grid_dims {
            self.grid_dims = new_grid_dims;
            self.grid
                .resize_with(self.grid_dims.0 * self.grid_dims.1, Vec::new);
        }
    }

    /// Collects candidate labels from screen points, filtering by screen distance.
    ///
    /// This should be called for each path to gather all candidate labels.
    /// The `min_screen_distance` (in pixels) ensures labels are spaced out regardless of zoom.
    /// Results are accumulated in the internal buffer.
    pub fn collect_label_candidates(
        &mut self,
        screen_points: &[(Pos2, &GeoPoint)],
        min_screen_distance: f32,
    ) {
        let mut last_label_pos: Option<Pos2> = None;

        for (screen_pos, geo_point) in screen_points {
            // Check if this point is far enough from the last labeled point
            let should_place = if let Some(last_pos) = last_label_pos {
                let distance = (*screen_pos - last_pos).length();
                distance >= min_screen_distance
            } else {
                true
            };

            if should_place {
                self.candidate_buffer
                    .push((*screen_pos, (*geo_point).clone()));
                last_label_pos = Some(*screen_pos);
            }
        }
    }

    /// Pre-calculates all label layouts for candidates.
    ///
    /// Call this after collecting all candidates and before placing labels.
    pub fn precalculate_labels(&mut self, painter: &Painter, settings: &TelemetryLabelSettings) {
        self.precalc_buffer.clear();

        for (screen_pos, geo_point) in &self.candidate_buffer {
            if let Some((rect, galleys)) =
                calculate_label_layout(painter, settings, *screen_pos, geo_point)
            {
                self.precalc_buffer
                    .push(PreCalculatedLabel { rect, galleys });
            }
        }
    }

    /// Places all pre-calculated labels, handling collisions.
    ///
    /// Call this after precalculate_labels.
    pub fn place_labels(&mut self, painter: &Painter) {
        for i in 0..self.precalc_buffer.len() {
            let label = &self.precalc_buffer[i];
            if !self.collides(&label.rect) {
                draw_precalculated_label(painter, label.rect, &label.galleys);
                self.register_label(label.rect);
            }
        }
    }

    /// Clears the candidate and precalc buffers for the next frame.
    ///
    /// Call this after placing labels.
    pub fn clear_frame_data(&mut self) {
        self.candidate_buffer.clear();
        self.precalc_buffer.clear();
    }

    fn collides(&self, rect: &Rect) -> bool {
        let (min_x, min_y, max_x, max_y) = self.get_grid_bounds(rect);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_index = y * self.grid_dims.0 + x;
                if let Some(cell) = self.grid.get(cell_index) {
                    for &placed_index in cell {
                        if rect.intersects(self.placed_rects[placed_index]) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn register_label(&mut self, rect: Rect) {
        let new_index = self.placed_rects.len();
        self.placed_rects.push(rect);

        let (min_x, min_y, max_x, max_y) = self.get_grid_bounds(&rect);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_index = y * self.grid_dims.0 + x;
                if let Some(cell) = self.grid.get_mut(cell_index) {
                    cell.push(new_index);
                }
            }
        }
    }

    fn get_grid_bounds(&self, rect: &Rect) -> (usize, usize, usize, usize) {
        let grid_w = self.grid_dims.0.saturating_sub(1);
        let grid_h = self.grid_dims.1.saturating_sub(1);

        let min_x = ((rect.min.x / self.cell_size).floor() as usize).min(grid_w);
        let min_y = ((rect.min.y / self.cell_size).floor() as usize).min(grid_h);
        let max_x = ((rect.max.x / self.cell_size).floor() as usize).min(grid_w);
        let max_y = ((rect.max.y / self.cell_size).floor() as usize).min(grid_h);
        (min_x, min_y, max_x, max_y)
    }
}

/// Calculates the layout and bounding box for a telemetry label.
fn calculate_label_layout(
    painter: &Painter,
    settings: &TelemetryLabelSettings,
    point: Pos2,
    geo_point: &GeoPoint,
) -> Option<(Rect, Vec<Arc<Galley>>)> {
    let mut lines = Vec::with_capacity(2);
    if settings.with_altitude {
        if let Some(altitude) = geo_point.altitude {
            lines.push(altitude.to_string());
        }
    }
    if settings.with_speed {
        if let Some(speed) = geo_point.speed {
            lines.push(format!("{speed:.1} km/h"));
        }
    }

    if lines.is_empty() {
        return None;
    }

    const FONT_ID: FontId = FontId::proportional(11.0);
    const TEXT_COLOR: Color32 = Color32::BLACK;
    const LINE_SPACING: f32 = 2.0;

    let galleys: Vec<Arc<Galley>> = lines
        .into_iter()
        .map(|line| painter.layout_no_wrap(line, FONT_ID.clone(), TEXT_COLOR))
        .collect();

    let mut max_width: f32 = 0.0;
    let mut total_height: f32 = 0.0;
    for galley in &galleys {
        max_width = max_width.max(galley.size().x);
        total_height += galley.size().y;
    }
    total_height += LINE_SPACING * (galleys.len().saturating_sub(1) as f32);

    let text_pos = point + vec2(5.0, -8.0);
    let text_rect = Rect::from_min_size(text_pos, vec2(max_width, total_height));
    let padded_rect = text_rect.expand(2.0);

    Some((padded_rect, galleys))
}

/// Draws a pre-calculated label to the screen.
fn draw_precalculated_label(painter: &Painter, rect: Rect, galleys: &[Arc<Galley>]) {
    let bg_color = Color32::from_rgba_unmultiplied(255, 255, 255, 200);
    const TEXT_COLOR: Color32 = Color32::BLACK;
    const LINE_SPACING: f32 = 2.0;

    painter.rect_filled(rect, 2.0, bg_color);

    let mut current_pos = rect.min + vec2(2.0, 2.0);
    for galley in galleys {
        painter.galley(current_pos, galley.clone(), TEXT_COLOR);
        current_pos.y += galley.size().y + LINE_SPACING;
    }
}
