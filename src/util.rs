use std::time::Duration;

use chrono::{DateTime, Timelike as _};
use egui_plot::PlotPoint;

pub const NANOS_PER_SEC: u32 = 1_000_000_000;
pub const SECS_PER_DAY: u32 = 24 * 60 * 60;
pub const SECS_PER_H: u16 = 60 * 60;

pub const MINS_PER_DAY: u16 = 24 * 60;
pub const MINS_PER_H: u8 = 60;

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

/// Assumes x is time in nanoseconds
pub fn format_label_ns(plot_name: &str, val: &PlotPoint) -> String {
    let time_ns = val.x;
    let time_s = time_ns / NANOS_PER_SEC as f64;
    let ns_remainder = time_s.fract() * NANOS_PER_SEC as f64;
    let Some(dt) = DateTime::from_timestamp(time_s as i64, ns_remainder as u32) else {
        // Will happen if the user zooms out where the X-axis is extended >100 years
        log::warn!("Timestamp value out of range: {time_s}");
        return "out of range".to_owned();
    };
    format!(
        "{plot_name}\ny: {y:.4}\n{h:02}:{m:02}:{s:02}.{subsec_ms:03}",
        y = val.y,
        h = dt.hour(),
        m = dt.minute(),
        s = dt.second(),
        subsec_ms = dt.timestamp_subsec_millis()
    )
}

pub fn format_delta_xy(delta_x_time_s: f64, delta_y: f64) -> String {
    format!(
        "Δt:{delta_x}\nΔy:{delta_y:.4}",
        delta_x = format_time_s(delta_x_time_s)
    )
}

/// Formats seconds to a human readable strings from milliseconds up to days.
fn format_time_s(time_s: f64) -> String {
    const SECOND: f64 = 1.0;
    const MINUTE: f64 = 60.0;
    const HOUR: f64 = 3600.0;
    const DAY: f64 = 86_400.0;
    const WEEK: f64 = 604_800.0;

    match time_s {
        t if t < SECOND => format!("{:.4}ms", t * 1000.0),
        t if t < MINUTE => format!("{t:.3}s"),
        t if t < HOUR => {
            let (m, s) = div_rem(t, MINUTE);
            format!("{m}m{s:.2}s")
        }
        t if t < DAY => {
            let (h, rem) = div_rem(t, HOUR);
            let (m, s) = div_rem(rem, MINUTE);
            format!("{h}h{m}m{s:.2}s")
        }
        t if t < WEEK => {
            let (d, rem) = div_rem(t, DAY);
            let (h, rem) = div_rem(rem, HOUR);
            let (m, s) = div_rem(rem, MINUTE);
            format!("{d}d{h}h{m}m{s:.1}s")
        }
        t => {
            let (d, rem) = div_rem(t, DAY);
            let (h, rem) = div_rem(rem, HOUR);
            let (m, s) = div_rem(rem, MINUTE);
            format!("{d}d{h}h{m}m{s:.1}s")
        }
    }
}

/// Helper function to perform division and get remainder in one step
#[inline]
fn div_rem(dividend: f64, divisor: f64) -> (u32, f64) {
    let quotient = (dividend / divisor) as u32;
    let remainder = dividend - (quotient as f64 * divisor);
    (quotient, remainder)
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
