use std::time::Duration;

use chrono::{NaiveDateTime, ParseResult};

/// Parse a timestamp in milliseconds to a timestamp string on the form `HH:MM:SS.zzz` where `zzz` is the millisecond fraction.
pub fn parse_timestamp(timestamp_ms: u32) -> String {
    let duration = Duration::from_millis(timestamp_ms as u64);
    let hours = (duration.as_secs() % 86400) / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;
    let seconds = duration.as_secs() % 60;
    let milliseconds = duration.subsec_millis();

    format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}")
}

/// Parse a timestamp from a slice of bytes and a timestamp format string e.g. `"%Y-%m-%dT%H:%M:%S"` would parse `"2024-09-26T10:27:39"`
pub fn timestamp_from_raw(timestamp_raw: &[u8], fmt: &str) -> ParseResult<NaiveDateTime> {
    let timestamp_as_utf8 = String::from_utf8_lossy(timestamp_raw);
    let trimmed_ts = timestamp_as_utf8.trim_end_matches(char::from(0));
    NaiveDateTime::parse_from_str(trimmed_ts, fmt)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_timestamp_from_raw() {
        let date_format = "%Y-%m-%dT%H:%M:%S";

        let timestamp_str = String::from("2024-09-26T10:27:39");
        assert!(NaiveDateTime::parse_from_str(&timestamp_str, date_format).is_ok());
        let timestamp_bytes = timestamp_str.as_bytes();
        let parsed_timestamp =
            timestamp_from_raw(timestamp_bytes, date_format).expect("Failed to parse timestamp");
        assert_eq!(
            parsed_timestamp,
            NaiveDate::from_ymd_opt(2024, 9, 26)
                .expect("Invalid input")
                .and_hms_opt(10, 27, 39)
                .expect("Invalid input")
        );
    }
}
