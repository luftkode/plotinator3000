use std::{ops::RangeInclusive, time::Duration};

use chrono::{DateTime, Timelike};
use egui_plot::{GridMark, PlotPoint};

/// Function to format milliseconds into HH:MM:SS.ms
pub fn format_ms_timestamp(timestamp_ms: f64) -> String {
    let duration = Duration::from_millis(timestamp_ms as u64);
    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;
    let seconds = duration.as_secs() % 60;

    format!(
        "{:1}:{:02}:{:02}.{:03}",
        hours,
        minutes,
        seconds,
        duration.subsec_millis()
    )
}

/// The first parameter of formatter is the raw tick value as f64. The second parameter of formatter is the currently shown range on this axis.
///
/// Assumes x is time in nanoseconds
pub fn format_time(mark: GridMark, _range: &RangeInclusive<f64>) -> String {
    let ns = mark.value;
    let sec = ns / NANOS_PER_SEC as f64;
    let ns_remainder = sec.fract() * NANOS_PER_SEC as f64;
    let dt = DateTime::from_timestamp(sec as i64, ns_remainder as u32)
        .unwrap_or_else(|| panic!("Timestamp value out of range: {sec}"));
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// Assumes x is time in nanoseconds
pub fn format_label_ns(_s: &str, val: &PlotPoint) -> String {
    let time_ns = val.x;
    let time_s = time_ns / NANOS_PER_SEC as f64;
    let remainder_ns = time_s.fract() * NANOS_PER_SEC as f64;
    let dt = DateTime::from_timestamp(time_s as i64, remainder_ns as u32)
        .unwrap_or_else(|| panic!("Timestamp value out of range: {}", val.x));
    format!(
        "y: {y:.4}\n{h:02}:{m:02}:{s:02}.{subsec_ms:03}",
        y = val.y,
        h = dt.hour(),
        m = dt.minute(),
        s = dt.second(),
        subsec_ms = dt.timestamp_subsec_millis()
    )
}
