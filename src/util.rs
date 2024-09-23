use std::time::Duration;

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
