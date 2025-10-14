use egui::{Color32, Painter, Pos2, Stroke, Vec2, vec2};
use plotinator_log_if::prelude::GeoPoint;
use plotinator_proc_macros::log_time;
use walkers::Projector;

use crate::{
    draw::labels::{LabelPlacer, TelemetryLabelSettings},
    geo_path::GeoPath,
};

pub(crate) mod labels;

pub struct DrawSettings {
    pub(crate) draw_heading_arrows: bool,
    pub(crate) telemetry_label: TelemetryLabelSettings,
}

#[log_time]
pub(crate) fn draw_path(
    ui: &egui::Ui,
    projector: &Projector,
    path: &impl GeoPath,
    settings: &DrawSettings,
    label_placer: &mut LabelPlacer,
) {
    if path.points().len() < 2 {
        return;
    }

    let path_color = path.color();

    // We need the full GeoPoint data at each screen position to access speed and heading.
    let screen_points: Vec<(Pos2, &GeoPoint)> = path
        .points()
        .iter()
        .map(|p| (projector.project(p.position).to_pos2(), p))
        .collect();

    if settings.telemetry_label.draw {
        label_placer.collect_label_candidates(
            &screen_points,
            40.0,
            path_color,
            &settings.telemetry_label,
        );
    }

    draw_path_inner(
        ui,
        &screen_points,
        path_color,
        path.speed_bounds(),
        settings,
    );
}

#[log_time]
pub(crate) fn draw_path_inner(
    ui: &egui::Ui,
    screen_points: &[(Pos2, &GeoPoint)],
    path_color: Color32,
    speed_range: (f64, f64),
    settings: &DrawSettings,
) {
    let painter = ui.painter();

    // Draw the path as a colored line with altitude-based opacity
    const MAX_ALTITUDE: f64 = 400.0;

    for window in screen_points.windows(2) {
        // Use the altitude from the first point of the segment
        let altitude = window[0].1.altitude.map_or(0.0, |a| a.val());
        let opacity = (altitude / MAX_ALTITUDE).clamp(0.0, 1.0);

        // Scale the alpha channel of the path color based on altitude
        let alpha = (255.0 * opacity) as u8;
        let segment_color = Color32::from_rgba_unmultiplied(
            path_color.r(),
            path_color.g(),
            path_color.b(),
            alpha.max(20), // Minimum alpha of 20 to ensure visibility
        );

        let stroke = Stroke::new(3.0, segment_color);
        painter.line_segment([window[0].0, window[1].0], stroke);
    }

    // Draw circles at each point
    for (point_pos, _geo_point) in screen_points {
        painter.circle_stroke(*point_pos, 2.0, Stroke::new(1.0, path_color));
    }

    // Draw heading arrows with distance-based filtering
    if settings.draw_heading_arrows {
        draw_heading_arrows(painter, screen_points, speed_range);
    }

    // Draw start marker (filled black circle)
    if let Some((start_pos, _)) = screen_points.first() {
        draw_start_marker(painter, *start_pos);
    }

    // Draw end marker (black cross)
    if let Some((end_pos, _)) = screen_points.last() {
        draw_end_marker(painter, *end_pos);
    }
}

pub(crate) fn draw_heading_arrows(
    painter: &egui::Painter,
    screen_points: &[(Pos2, &GeoPoint)],
    speed_range: (f64, f64),
) {
    const MIN_ARROW_DISTANCE: f32 = 20.0; // Minimum pixels between arrows

    let mut last_arrow_pos: Option<Pos2> = None;

    for (point_pos, geo_point) in screen_points {
        // Skip if no heading data
        if geo_point.heading.is_none() {
            continue;
        }

        // Check distance from last drawn arrow
        let should_draw = if let Some(last_pos) = last_arrow_pos {
            let distance = point_pos.distance(last_pos);
            distance >= MIN_ARROW_DISTANCE
        } else {
            true // Always draw the first arrow
        };

        if should_draw {
            draw_heading_arrow(painter, *point_pos, geo_point, speed_range);
            last_arrow_pos = Some(*point_pos);
        }
    }
}

#[inline]
pub(crate) fn draw_heading_arrow(
    painter: &egui::Painter,
    center: Pos2,
    geo_point: &plotinator_log_if::prelude::GeoPoint,
    speed_range: (f64, f64),
) {
    let Some((main_line, barb1, barb2)) = calculate_arrow_geometry(center, geo_point, speed_range)
    else {
        return;
    };
    let outline_color = Color32::BLACK;
    let outline_stroke = Stroke::new(1.5, outline_color);
    painter.line_segment(main_line, outline_stroke);
    painter.line_segment(barb1, outline_stroke);
    painter.line_segment(barb2, outline_stroke);
}

#[inline]
pub(crate) fn calculate_arrow_geometry(
    center: Pos2,
    geo_point: &GeoPoint,
    speed_range: (f64, f64),
) -> Option<([Pos2; 2], [Pos2; 2], [Pos2; 2])> {
    let heading_deg = geo_point.heading?;

    const MIN_ARROW_LENGTH: f32 = 4.0;
    const MAX_ARROW_LENGTH: f32 = 30.0;
    const DEFAULT_ARROW_LENGTH: f32 = 12.0;

    let arrow_length = if let Some(speed) = geo_point.speed {
        let (min_speed, max_speed) = speed_range;
        if max_speed > min_speed {
            let speed_ratio =
                ((speed - min_speed) / (max_speed - min_speed)).clamp(0.0, 1.0) as f32;
            MIN_ARROW_LENGTH + speed_ratio * (MAX_ARROW_LENGTH - MIN_ARROW_LENGTH)
        } else {
            DEFAULT_ARROW_LENGTH
        }
    } else {
        DEFAULT_ARROW_LENGTH
    };

    // 0° North -> Up, 90° East -> Right
    let angle_rad = (90.0 - heading_deg).to_radians() as f32;
    let dir = Vec2::new(angle_rad.cos(), -angle_rad.sin());
    let tip = center + dir * arrow_length;

    // Calculate barbs
    let barb_length = arrow_length * 0.4;
    let barb_angle = 25.0_f32.to_radians();
    let back_dir = -dir;
    let rot = egui::emath::Rot2::from_angle(barb_angle);
    let barb1 = tip + (rot * back_dir) * barb_length;
    let barb2 = tip + (rot.inverse() * back_dir) * barb_length;

    Some(([center, tip], [tip, barb1], [tip, barb2]))
}

fn draw_start_marker(painter: &egui::Painter, center: Pos2) {
    const MARKER_RADIUS: f32 = 6.0;
    painter.circle_filled(center, MARKER_RADIUS + 1.0, Color32::WHITE);
    painter.circle_filled(center, MARKER_RADIUS, Color32::BLACK);
}

fn draw_end_marker(painter: &Painter, center: Pos2) {
    const CROSS_SIZE: f32 = 8.0;
    const CROSS_THICKNESS: f32 = 2.5;

    let stroke = Stroke::new(CROSS_THICKNESS, Color32::BLACK);
    let outline_stroke = Stroke::new(CROSS_THICKNESS + 1.0, Color32::WHITE);

    fn line1(center: Pos2) -> [Pos2; 2] {
        [
            center + vec2(-CROSS_SIZE, -CROSS_SIZE),
            center + vec2(CROSS_SIZE, CROSS_SIZE),
        ]
    }
    fn line2(center: Pos2) -> [Pos2; 2] {
        [
            center + Vec2::new(-CROSS_SIZE, CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, -CROSS_SIZE),
        ]
    }
    // Draw white outline for better visibility
    painter.line_segment(line1(center), outline_stroke);
    painter.line_segment(line2(center), outline_stroke);
    // Draw black cross
    painter.line_segment(line1(center), stroke);
    painter.line_segment(line2(center), stroke);
}

/// Find the closest point to the cursor in the geo spatial data and highlight it if it is close enough
pub(crate) fn draw_pointer_highlights(
    painter: &Painter,
    projector: &Projector,
    paths: &[impl GeoPath],
    cursor_time: f64,
) {
    const MAX_TIME_DELTA: f64 = 2_000_000_000.0; // Maximum 2 seconds in nanoseconds
    const HIGHLIGHT_RADIUS: f32 = 8.0;

    for path in paths {
        if !path.is_visible() {
            continue;
        }
        let geo_data = &path.points();
        // Find the closest point within the time threshold

        // Binary search to find the insertion point for cursor_time
        let candidate_idx =
            match geo_data.binary_search_by(|point| point.timestamp.total_cmp(&cursor_time)) {
                Ok(exact_idx) => exact_idx,
                Err(insert_idx) => insert_idx,
            };

        let mut closest_point: Option<(&GeoPoint, f64)> = None;

        // Check the point at candidate_idx and the one before it (if exists)
        if candidate_idx < geo_data.len() {
            let point = &geo_data[candidate_idx];
            let time_delta = point.timestamp - cursor_time;
            if time_delta <= MAX_TIME_DELTA {
                closest_point = Some((point, time_delta));
            }
        }

        // Check the point before cursor_time
        if candidate_idx > 0 {
            let point = &geo_data[candidate_idx - 1];
            let time_delta = cursor_time - point.timestamp;
            if time_delta <= MAX_TIME_DELTA {
                match closest_point {
                    None => closest_point = Some((point, time_delta)),
                    Some((_, current_delta)) if time_delta < current_delta => {
                        closest_point = Some((point, time_delta));
                    }
                    _ => {}
                }
            }
        }

        // Draw highlight for the closest point if found
        if let Some((point, _)) = closest_point {
            let screen_pos = projector.project(point.position).to_pos2();
            // First, draw a thicker black ring as a backdrop
            painter.circle_stroke(
                screen_pos,
                HIGHLIGHT_RADIUS,
                Stroke::new(5.0, Color32::BLACK),
            );

            // Then, draw the colored ring on top
            painter.circle_stroke(screen_pos, HIGHLIGHT_RADIUS, Stroke::new(3.0, path.color()));

            // Inner filled circle for visibility
            painter.circle_filled(screen_pos, 4.0, path.color());
        }
    }
}

pub(crate) fn highlight_whole_path(painter: &Painter, projector: &Projector, path: &impl GeoPath) {
    let screen_points: Vec<Pos2> = path
        .points()
        .iter()
        .map(|p| projector.project(p.position).to_pos2())
        .collect();

    highlight_all_points(painter, &screen_points, path.color());
}

pub(crate) fn highlight_all_points(painter: &Painter, screen_points: &[Pos2], color: Color32) {
    if screen_points.len() < 2 {
        return;
    }

    // Draw a thicker halo around the path
    let highlight_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 120);

    for window in screen_points.windows(2) {
        painter.line_segment([window[0], window[1]], Stroke::new(6.0, highlight_color));
    }

    // highlight each point
    for point in screen_points {
        painter.circle_stroke(*point, 6.0, Stroke::new(2.0, highlight_color));
    }
}

/// Highlights a single point, to show that the pointer hovering at the point is recognized
pub(crate) fn draw_hover_point_highlight(painter: &Painter, p: Pos2, color: Color32) {
    painter.circle_filled(p, 7.0, Color32::WHITE);
    painter.circle_filled(p, 5.0, color);
}
