use std::{ops::RangeInclusive, time::Duration};

use chrono::{DateTime, Timelike};
use egui_plot::{GridMark, PlotPoint};

/// Format a timestamp in milliseconds into `HH:MM:SS.ms`
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
pub fn format_label_ns(plot_name: &str, val: &PlotPoint) -> String {
    let time_ns = val.x;
    let time_s = time_ns / NANOS_PER_SEC as f64;
    let remainder_ns = time_s.fract() * NANOS_PER_SEC as f64;
    let dt = DateTime::from_timestamp(time_s as i64, remainder_ns as u32)
        .unwrap_or_else(|| panic!("Timestamp value out of range: {time_ns}"));
    format!(
        "{plot_name}\ny: {y:.4}\n{h:02}:{m:02}:{s:02}.{subsec_ms:03}",
        y = val.y,
        h = dt.hour(),
        m = dt.minute(),
        s = dt.second(),
        subsec_ms = dt.timestamp_subsec_millis()
    )
}

pub fn format_delta_xy(delta_x_time_ns: f64, delta_y: f64) -> String {
    let delta_x = Duration::from_secs_f64(delta_x_time_ns);
    format!("Δt:{delta_x:.02?}\nΔy:{delta_y:.4}")
}

/// Format a value to a human readable byte magnitude description
#[must_use]
pub fn format_data_size(size_bytes: usize) -> String {
    const KI_B_VAL: usize = 1024;
    const KI_B_DIVIDER: f64 = 1024_f64;
    const MI_B_VAL: usize = 1024 * KI_B_VAL;
    const MI_B_DIVIDER: f64 = MI_B_VAL as f64;
    const GI_B_VAL: usize = 1024 * MI_B_VAL;
    const GI_B_DIVIDER: f64 = GI_B_VAL as f64;
    match size_bytes {
        0..=KI_B_VAL => {
            format!("{size_bytes:.2} B")
        }
        1025..=MI_B_VAL => {
            let kib_bytes = size_bytes as f64 / KI_B_DIVIDER;
            format!("{kib_bytes:.2} KiB")
        }
        1_048_577..=GI_B_VAL => {
            let mib_bytes = size_bytes as f64 / MI_B_DIVIDER;
            format!("{mib_bytes:.2} MiB")
        }
        _ => {
            let gib_bytes = size_bytes as f64 / GI_B_DIVIDER;
            format!("{gib_bytes:.2} GiB")
        }
    }
}
