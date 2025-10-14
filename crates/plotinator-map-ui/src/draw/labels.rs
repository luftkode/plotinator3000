use egui::{Color32, FontId, Painter, Pos2, Rect, epaint::Galley, vec2};
use plotinator_log_if::prelude::GeoPoint;
use plotinator_proc_macros::log_time;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) struct TelemetryLabelSettings {
    pub(crate) draw: bool,
    pub(crate) with_speed: bool,
    pub(crate) with_altitude: bool,
}

impl TelemetryLabelSettings {
    pub(crate) fn draw_speed(&self) -> bool {
        self.with_speed
    }
    pub(crate) fn draw_altitude(&self) -> bool {
        self.with_altitude
    }
}

pub struct CandidatePoint {
    pos: Pos2,
    altitude: Option<f64>,
    speed: Option<f64>,
    color: Color32,
}

/// Represents a fully laid-out label ready to be drawn.
pub struct PlacedLabel {
    pub rect: Rect,
    pub galleys: Option<Vec<Arc<Galley>>>,
    pub path_color: Color32,
}

/// Manages the placement of non-overlapping labels on a 2D plane.
#[derive(Deserialize, Serialize)]
pub struct LabelPlacer {
    placed_rects: Vec<Rect>,
    grid: Vec<SmallVec<[usize; 4]>>,
    grid_dims: (usize, usize),
    cell_size: f32,
    padded_screen_rect: Rect,
    // buffers for caching allocations
    #[serde(skip)]
    candidate_buffer: Vec<CandidatePoint>,
    #[serde(skip)]
    label_buffer: Vec<PlacedLabel>,
}

impl Default for LabelPlacer {
    fn default() -> Self {
        Self::new(64.0)
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
            padded_screen_rect: Rect::ZERO,
            candidate_buffer: Vec::new(),
            label_buffer: Vec::new(),
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

        // Store screen rect for bounds checking during label collection
        self.padded_screen_rect = screen_rect.expand(20.);

        let new_grid_dims = (
            (screen_rect.width() / self.cell_size).ceil() as usize + 1,
            (screen_rect.height() / self.cell_size).ceil() as usize + 1,
        );

        if self.grid_dims != new_grid_dims {
            self.grid_dims = new_grid_dims;
            self.grid
                .resize_with(self.grid_dims.0 * self.grid_dims.1, SmallVec::new);
        }
    }

    /// Collects candidate labels from screen points, filtering by screen distance.
    ///
    /// This should be called for each path to gather all candidate labels.
    /// The `min_screen_distance` (in pixels) ensures labels are spaced out regardless of zoom.
    /// Results are accumulated in the internal buffer.
    #[log_time]
    pub fn collect_label_candidates(
        &mut self,
        screen_points: &[(Pos2, &GeoPoint)],
        min_screen_distance: f32,
        path_color: Color32,
        settings: &TelemetryLabelSettings,
    ) {
        if !(settings.draw_altitude() || settings.draw_speed()) {
            return;
        }

        let mut last_label_pos: Option<Pos2> = None;

        for (screen_pos, geo_point) in screen_points {
            if !self.padded_screen_rect.contains(*screen_pos) {
                continue;
            }
            // Check if this point is far enough from the last labeled point
            let should_place = if let Some(last_pos) = last_label_pos {
                let distance = (*screen_pos - last_pos).length();
                distance >= min_screen_distance
            } else {
                true
            };

            if should_place {
                self.candidate_buffer.push(CandidatePoint {
                    pos: *screen_pos,
                    altitude: if settings.draw_altitude() {
                        geo_point.altitude.map(|a| a.val())
                    } else {
                        None
                    },
                    speed: if settings.draw_speed() {
                        geo_point.speed
                    } else {
                        None
                    },
                    color: path_color,
                });
                last_label_pos = Some(*screen_pos);
            }
        }
    }

    /// Calculates and places all labels, handling collisions.
    ///
    /// Call this after collecting all candidates for all paths.
    #[log_time]
    pub fn place_all_labels(&mut self, painter: &Painter) {
        self.label_buffer.clear();

        for candidate in &self.candidate_buffer {
            let (rect, galleys) = calculate_label_layout(painter, candidate);
            self.label_buffer.push(PlacedLabel {
                rect,
                galleys: Some(galleys),
                path_color: candidate.color,
            });
        }

        let mut placed_labels = std::mem::take(&mut self.label_buffer);
        for label in &mut placed_labels {
            let bounds = self.get_grid_bounds(&label.rect);
            if !self.collides_with_bounds(&label.rect, bounds) {
                draw_label(painter, label);
                self.register_label_with_bounds(label.rect, bounds);
            }
        }

        self.label_buffer = placed_labels;
        self.label_buffer.clear();
        self.candidate_buffer.clear();
    }

    #[inline]
    fn collides_with_bounds(&self, rect: &Rect, bounds: (usize, usize, usize, usize)) -> bool {
        let (min_x, min_y, max_x, max_y) = bounds;

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

    #[inline]
    fn register_label_with_bounds(&mut self, rect: Rect, bounds: (usize, usize, usize, usize)) {
        let new_index = self.placed_rects.len();
        self.placed_rects.push(rect);

        let (min_x, min_y, max_x, max_y) = bounds;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_index = y * self.grid_dims.0 + x;
                if let Some(cell) = self.grid.get_mut(cell_index) {
                    cell.push(new_index);
                }
            }
        }
    }

    #[inline]
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
#[inline]
fn calculate_label_layout(painter: &Painter, point: &CandidatePoint) -> (Rect, Vec<Arc<Galley>>) {
    let mut lines = Vec::with_capacity(2);
    if let Some(altitude) = point.altitude {
        lines.push(format!("{altitude:.0} m"));
    }
    if let Some(speed) = point.speed {
        lines.push(format!("{speed:.1} km/h"));
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

    let text_pos = point.pos + vec2(5.0, -8.0);
    let text_rect = Rect::from_min_size(text_pos, vec2(max_width, total_height));
    let padded_rect = text_rect.expand(2.0);

    (padded_rect, galleys)
}

/// Blends a path color with white to create a subtle tinted background.
///
/// This ensures the background is light enough for black text while maintaining
/// visual connection to the path color.
#[inline]
fn get_label_background_color(path_color: Color32) -> Color32 {
    let [r, g, b, _] = path_color.to_srgba_unmultiplied();

    // Blend the path color with white (factor of 0.3 means 70% white, 30% path color)
    // This creates a subtle tint that doesn't overpower the text
    let blend_factor = 0.3;
    let r = ((r as f32) * blend_factor + 255.0 * (1.0 - blend_factor)) as u8;
    let g = ((g as f32) * blend_factor + 255.0 * (1.0 - blend_factor)) as u8;
    let b = ((b as f32) * blend_factor + 255.0 * (1.0 - blend_factor)) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, 220)
}

/// Draws a label to the screen with path-colored background.
#[inline]
fn draw_label(painter: &Painter, label: &mut PlacedLabel) {
    let bg_color = get_label_background_color(label.path_color);
    const TEXT_COLOR: Color32 = Color32::BLACK;
    const LINE_SPACING: f32 = 2.0;

    painter.rect_filled(label.rect, 2.0, bg_color);

    let mut current_pos = label.rect.min + vec2(2.0, 2.0);
    if let Some(galleys) = label.galleys.take() {
        for galley in galleys {
            let curr_galley_size_y = galley.size().y;
            painter.galley(current_pos, galley, TEXT_COLOR);
            current_pos.y += curr_galley_size_y + LINE_SPACING;
        }
    }
}
