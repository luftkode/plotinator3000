use egui::{Color32, Pos2, Stroke, Vec2};
use plotinator_log_if::prelude::{GeoPoint, GeoSpatialData};
use walkers::Projector;

use crate::PathEntry;

pub(crate) fn draw_path(
    ui: &mut egui::Ui,
    projector: &Projector,
    path: &GeoSpatialData,
    should_draw_heading_arrows: bool,
    should_draw_height_labels: bool,
) {
    if path.points.len() < 2 {
        return;
    }

    let painter = ui.painter();
    let path_color = path.color;

    // We need the full GeoPoint data at each screen position to access speed and heading.
    let screen_points: Vec<(Pos2, &GeoPoint)> = path
        .points
        .iter()
        .map(|p| (projector.project(p.position).to_pos2(), p))
        .collect();

    // Draw the path as a colored line with altitude-based opacity
    const MAX_ALTITUDE: f64 = 1000.0;

    for window in screen_points.windows(2) {
        // Use the altitude from the first point of the segment
        let altitude = window[0].1.altitude.unwrap_or(0.0);
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

    // Get the speed range for the entire path to normalize arrow lengths.
    let speed_range = path.speed_bounds();

    // Draw circles at each point
    for (point_pos, _geo_point) in &screen_points {
        painter.circle_stroke(*point_pos, 2.0, Stroke::new(1.0, path_color));
    }

    // Draw heading arrows with distance-based filtering
    if should_draw_heading_arrows {
        draw_heading_arrows(painter, &screen_points, path_color, speed_range);
    }

    if should_draw_height_labels {
        draw_altitude_labels(painter, &screen_points);
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
    path_color: Color32,
    speed_range: (f64, f64),
) {
    const MIN_ARROW_DISTANCE: f32 = 20.0; // Minimum pixels between arrows

    let mut last_arrow_pos: Option<Pos2> = None;

    for (point_pos, geo_point) in screen_points.iter() {
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
            draw_heading_arrow(painter, *point_pos, geo_point, path_color, speed_range);
            last_arrow_pos = Some(*point_pos);
        }
    }
}

pub(crate) fn draw_heading_arrow(
    painter: &egui::Painter,
    center: Pos2,
    geo_point: &plotinator_log_if::prelude::GeoPoint,
    _path_color: Color32, // No longer needed, but kept for signature consistency
    speed_range: (f64, f64),
) {
    let Some(heading_deg) = geo_point.heading else {
        return;
    };

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

    // Calculate the two barbs for the arrowhead by rotating the backward vector
    let barb_length = arrow_length * 0.4;
    let barb_angle = 25.0_f32.to_radians();
    let back_dir = -dir;

    let rot = egui::emath::Rot2::from_angle(barb_angle);
    let barb1 = tip + (rot * back_dir) * barb_length;
    let barb2 = tip + (rot.inverse() * back_dir) * barb_length;

    let outline_color = Color32::BLACK;
    let outline_stroke = Stroke::new(1.5, outline_color);

    painter.line_segment([center, tip], outline_stroke);
    painter.line_segment([tip, barb1], outline_stroke);
    painter.line_segment([tip, barb2], outline_stroke);
}

fn draw_start_marker(painter: &egui::Painter, center: Pos2) {
    const MARKER_RADIUS: f32 = 6.0;

    // Draw white outline for better visibility
    painter.circle_filled(center, MARKER_RADIUS + 1.0, Color32::WHITE);
    // Draw black filled circle
    painter.circle_filled(center, MARKER_RADIUS, Color32::BLACK);
}

fn draw_end_marker(painter: &egui::Painter, center: Pos2) {
    const CROSS_SIZE: f32 = 8.0;
    const CROSS_THICKNESS: f32 = 2.5;

    let stroke = Stroke::new(CROSS_THICKNESS, Color32::BLACK);
    let outline_stroke = Stroke::new(CROSS_THICKNESS + 1.0, Color32::WHITE);

    // Draw white outline for better visibility
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, -CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, CROSS_SIZE),
        ],
        outline_stroke,
    );
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, -CROSS_SIZE),
        ],
        outline_stroke,
    );

    // Draw black cross
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, -CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, CROSS_SIZE),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, -CROSS_SIZE),
        ],
        stroke,
    );
}

pub(crate) fn draw_altitude_labels(painter: &egui::Painter, screen_points: &[(Pos2, &GeoPoint)]) {
    const MIN_LABEL_DISTANCE: f32 = 60.0; // Minimum pixels between labels

    let mut last_label_pos: Option<Pos2> = None;

    for (i, (point_pos, geo_point)) in screen_points.iter().enumerate() {
        // Skip if no altitude data
        let Some(altitude) = geo_point.altitude else {
            continue;
        };

        // Check every 10th point as a candidate
        if i % 10 != 0 {
            continue;
        }

        // Check distance from last drawn label
        let should_draw = if let Some(last_pos) = last_label_pos {
            let distance = point_pos.distance(last_pos);
            distance >= MIN_LABEL_DISTANCE
        } else {
            true // Always draw the first label
        };

        if should_draw {
            draw_altitude_label(painter, *point_pos, altitude);
            last_label_pos = Some(*point_pos);
        }
    }
}

fn draw_altitude_label(painter: &egui::Painter, point: Pos2, altitude: f64) {
    let text = format!("{:.0}m", altitude);
    let font_id = egui::FontId::proportional(11.0);
    let text_color = Color32::BLACK;
    let bg_color = Color32::from_rgba_unmultiplied(255, 255, 255, 200);

    // Offset the text slightly above and to the right of the point
    let text_pos = point + Vec2::new(5.0, -8.0);

    // Get text dimensions for background
    let galley = painter.layout_no_wrap(text.clone(), font_id.clone(), text_color);
    let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
    let padded_rect = text_rect.expand(2.0);

    // Draw background
    painter.rect_filled(padded_rect, 2.0, bg_color);

    // Draw text
    painter.text(text_pos, egui::Align2::LEFT_TOP, text, font_id, text_color);
}

pub(crate) fn draw_cursor_highlights(
    ui: &mut egui::Ui,
    projector: &Projector,
    geo_data_series: &[PathEntry],
    cursor_time: f64,
) {
    const MAX_TIME_DELTA: f64 = 2_000_000_000.0; // Maximum 2 seconds in nanoseconds
    const HIGHLIGHT_RADIUS: f32 = 8.0;

    let painter = ui.painter();

    for path in geo_data_series {
        if !path.visible {
            continue;
        }
        let geo_data = &path.data;
        // Find the closest point within the time threshold
        let mut closest_point: Option<(&GeoPoint, f64)> = None;

        for point in &geo_data.points {
            let time_delta = (point.timestamp - cursor_time).abs();

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
            painter.circle_stroke(
                screen_pos,
                HIGHLIGHT_RADIUS,
                Stroke::new(3.0, geo_data.color),
            );

            // Inner filled circle for visibility
            painter.circle_filled(screen_pos, 4.0, geo_data.color);
        }
    }
}

pub(crate) fn highlight_whole_path(
    ui: &mut egui::Ui,
    projector: &Projector,
    path: &GeoSpatialData,
) {
    let painter = ui.painter();

    let screen_points: Vec<Pos2> = path
        .points
        .iter()
        .map(|p| projector.project(p.position).to_pos2())
        .collect();

    if screen_points.len() < 2 {
        return;
    }

    // Draw a thicker halo around the path
    let highlight_color =
        Color32::from_rgba_unmultiplied(path.color.r(), path.color.g(), path.color.b(), 120);

    for window in screen_points.windows(2) {
        painter.line_segment([window[0], window[1]], Stroke::new(6.0, highlight_color));
    }

    // highlight each point
    for point in &screen_points {
        painter.circle_stroke(*point, 6.0, Stroke::new(2.0, highlight_color));
    }
}
