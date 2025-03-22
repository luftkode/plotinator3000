use egui_plot::PlotPoint;

#[must_use]
pub(crate) fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as f64
}

/// Timestamps a value with the current system time and returns it as a [`PlotPoint`]
#[must_use]
pub(crate) fn point_now(value: f64) -> PlotPoint {
    PlotPoint {
        x: now_timestamp(),
        y: value,
    }
}
