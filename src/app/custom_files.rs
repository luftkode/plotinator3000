use std::{
    fs,
    io::{self, Read as _, Seek as _},
    path::Path,
};

use plotinator_supported_formats::SupportedFormat;

use crate::plot::LogPlotUi;

pub const CUSTOM_HEADER_PLOT_DATA: &str = "DONT TOUCH: PLOTINATOR3000 PLOT DATA FILE";
pub const CUSTOM_HEADER_PLOT_UI_STATE: &str = "DONT TOUCH: PLOTINATOR3000 PLOT UI STATE FILE";

pub fn max_custom_header_len() -> usize {
    CUSTOM_HEADER_PLOT_DATA
        .len()
        .max(CUSTOM_HEADER_PLOT_UI_STATE.len())
}

/// Represents the content parsed from a custom Plotinator3000 file.
pub(crate) enum CustomFileContent {
    PlotData(Vec<SupportedFormat>),
    PlotUi(Box<LogPlotUi>),
}

/// Attempts to parse a file that might contain a Plotinator3000 custom header.
pub(crate) fn try_parse_custom_file_from_buf(raw_contents: &[u8]) -> Option<CustomFileContent> {
    let (custom_header_len, is_plot_ui) = parse_custom_header_from_bytes(raw_contents)?;

    let raw_contents_without_header = &raw_contents[custom_header_len..];
    match deserialize_custom_content_from_bytes(raw_contents_without_header, is_plot_ui) {
        Ok(content) => Some(content),
        Err(e) => {
            log::error!("Failed to deserialize custom file content from buffer: {e}");
            None
        }
    }
}

/// Attempts to parse a file that might contain a Plotinator3000 custom header.
pub(crate) fn try_parse_custom_file(path: &Path) -> io::Result<Option<CustomFileContent>> {
    let mut file = fs::File::open(path)?;
    let max_header_len = CUSTOM_HEADER_PLOT_DATA
        .len()
        .max(CUSTOM_HEADER_PLOT_UI_STATE.len());
    let mut header_buf = vec![0u8; max_header_len];

    // Attempt to read the header. If it's too short, it's not a custom file.
    if file.read_exact(&mut header_buf).is_err() {
        return Ok(None);
    }

    let Some((custom_header_len, is_plot_ui)) = parse_custom_header_from_bytes(&header_buf) else {
        return Ok(None);
    };

    // Seek past the custom header to the actual data
    file.seek(io::SeekFrom::Start(custom_header_len as u64))?;

    let mut data_bytes = Vec::new();
    #[allow(clippy::verbose_file_reads, reason = "false positive?")]
    file.read_to_end(&mut data_bytes)?; // Read the rest of the file into bytes

    match deserialize_custom_content_from_bytes(&data_bytes, is_plot_ui) {
        Ok(content) => Ok(Some(content)),
        Err(e) => {
            log::error!("Failed to deserialize custom file content from {path:?}: {e}");
            Ok(None)
        }
    }
}

/// Determines if the given bytes start with a known custom header and returns its length and type.
/// Returns `None` if no custom header is found.
pub(crate) fn parse_custom_header_from_bytes(bytes: &[u8]) -> Option<(usize, bool)> {
    // Ensure we don't end up parsing a huge amount of data
    let maybe_header_bytes = if bytes.len() > max_custom_header_len() {
        &bytes[..max_custom_header_len()]
    } else {
        bytes
    };
    let header_str = String::from_utf8_lossy(maybe_header_bytes); // Use lossy for robustness

    if header_str.starts_with(CUSTOM_HEADER_PLOT_DATA) {
        Some((CUSTOM_HEADER_PLOT_DATA.len(), false))
    } else if header_str.starts_with(CUSTOM_HEADER_PLOT_UI_STATE) {
        Some((CUSTOM_HEADER_PLOT_UI_STATE.len(), true))
    } else {
        None
    }
}

/// Deserializes JSON bytes into either `PlotData` or `PlotUi` based on the `is_plot_ui` flag.
///
/// NOTE: The header needs to be removed from the bytes, otherwise the serialization will go wrong
pub(crate) fn deserialize_custom_content_from_bytes(
    json_bytes: &[u8],
    is_plot_ui: bool,
) -> anyhow::Result<CustomFileContent> {
    let json_str = String::from_utf8_lossy(json_bytes); // Ensure valid UTF-8 for JSON parsing

    let content = if is_plot_ui {
        let log_plot_ui = serde_json::from_str::<LogPlotUi>(&json_str)?;
        CustomFileContent::PlotUi(Box::new(log_plot_ui))
    } else {
        serde_json::from_str::<Vec<SupportedFormat>>(&json_str).map(CustomFileContent::PlotData)?
    };
    Ok(content)
}
