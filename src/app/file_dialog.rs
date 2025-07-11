use plotinator_supported_formats::SupportedFormat;

use crate::plot::LogPlotUi;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub const MAGIC_HEADER_PLOT_DATA: &str = "DONT TOUCH: PLOTINATOR3000 PLOT DATA FILE";
pub const MAGIC_HEADER_PLOT_UI_STATE: &str = "DONT TOUCH: PLOTINATOR3000 PLOT UI STATE FILE";

pub fn max_magic_header_len() -> usize {
    MAGIC_HEADER_PLOT_DATA
        .len()
        .max(MAGIC_HEADER_PLOT_UI_STATE.len())
}

/// Represents the content parsed from a "magic" file.
pub(crate) enum MagicFileContent {
    PlotData(Vec<SupportedFormat>),
    PlotUi(Box<LogPlotUi>),
}

/// Determines if the given bytes start with a known magic header and returns its length and type.
/// Returns `None` if no magic header is found.
pub(crate) fn parse_magic_header_from_bytes(bytes: &[u8]) -> Option<(usize, bool)> {
    // Ensure we don't end up parsing a huge amount of data
    let maybe_header_bytes = if bytes.len() > max_magic_header_len() {
        &bytes[..max_magic_header_len()]
    } else {
        bytes
    };
    let header_str = String::from_utf8_lossy(maybe_header_bytes); // Use lossy for robustness

    if header_str.starts_with(MAGIC_HEADER_PLOT_DATA) {
        Some((MAGIC_HEADER_PLOT_DATA.len(), false))
    } else if header_str.starts_with(MAGIC_HEADER_PLOT_UI_STATE) {
        Some((MAGIC_HEADER_PLOT_UI_STATE.len(), true))
    } else {
        None
    }
}

/// Deserializes JSON bytes into either `PlotData` or `PlotUi` based on the `is_plot_ui` flag.
///
/// NOTE: The header needs to be removed from the bytes, otherwise the serialization will go wrong
pub(crate) fn deserialize_magic_content_from_bytes(
    json_bytes: &[u8],
    is_plot_ui: bool,
) -> anyhow::Result<MagicFileContent> {
    let json_str = String::from_utf8_lossy(json_bytes); // Ensure valid UTF-8 for JSON parsing

    let content = if is_plot_ui {
        let log_plot_ui = serde_json::from_str::<LogPlotUi>(&json_str)?;
        MagicFileContent::PlotUi(Box::new(log_plot_ui))
    } else {
        serde_json::from_str::<Vec<SupportedFormat>>(&json_str).map(MagicFileContent::PlotData)?
    };
    Ok(content)
}
