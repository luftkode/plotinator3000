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

/// Parse output of `date +%s.%N`
pub(crate) fn parse_timestamp_to_nanos_f64(
    timestamp_str: &str,
) -> Result<f64, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = timestamp_str.trim().split('.').collect();

    if parts.len() != 2 {
        return Err("Invalid timestamp format, expected seconds.nanoseconds".into());
    }

    let seconds = parts[0].parse::<f64>()?;

    let nanos_str = parts[1];
    // Convert to nanoseconds - we need to account for precision
    let fraction = format!("0.{nanos_str}").parse::<f64>()?;
    let nanos = fraction * 1_000_000_000.0;

    // Total nanoseconds since epoch
    Ok((seconds * 1_000_000_000.0) + nanos)
}

pub(crate) fn timestamped_client_id(name: impl Into<String>) -> String {
    let mut client_id = name.into();
    client_id.push('-');
    client_id.push_str(&now_timestamp().to_string());
    client_id
}