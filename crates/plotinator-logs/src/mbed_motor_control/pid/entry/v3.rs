use std::{fmt, io};

use crate::util::parse_timestamp;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt as _;
use plotinator_log_if::log::LogEntry;

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogEntryV3 {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub rpm: f32,
    pub pid_output: f32,
    pub servo_duty_cycle: f32,
    pub rpm_error_count: u32,
    pub first_valid_rpm_count: u32,
    pub fan_on: bool,
    pub vbat: f32,
}

impl LogEntry for PidLogEntryV3 {
    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        // Start with 0 bytes read
        let mut total_bytes_read = 0;

        // Read each field and accumulate the number of bytes read immediately after
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&timestamp_ms);

        let timestamp_ms_str = parse_timestamp(timestamp_ms);

        let rpm = reader.read_f32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&rpm);

        let pid_output = reader.read_f32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&pid_output);

        let servo_duty_cycle = reader.read_f32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&servo_duty_cycle);

        let rpm_error_count = reader.read_u32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&rpm_error_count);

        let first_valid_rpm_count = reader.read_u32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&first_valid_rpm_count);

        let fan_on_byte = reader.read_u8()?;
        let fan_on = fan_on_byte == 1;
        total_bytes_read += size_of_val(&fan_on_byte);

        let vbat = reader.read_f32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&vbat);

        // Return the instance and the total bytes read
        Ok((
            Self {
                timestamp_ms_str,
                timestamp_ms,
                rpm,
                pid_output,
                servo_duty_cycle,
                rpm_error_count,
                first_valid_rpm_count,
                fan_on,
                vbat,
            },
            total_bytes_read,
        ))
    }

    fn timestamp_ns(&self) -> f64 {
        (self.timestamp_ms as u64 * 1_000_000) as f64
    }
}

impl fmt::Display for PidLogEntryV3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {} {} {} {} {} {}",
            self.timestamp_ms,
            self.rpm,
            self.pid_output,
            self.servo_duty_cycle,
            self.rpm_error_count,
            self.first_valid_rpm_count,
            self.fan_on,
            self.vbat
        )
    }
}

#[cfg(test)]
mod tests {
    use test_util::*;

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_from_reader_success() -> TestResult {
        // Sample binary data representing a PidLogEntry
        let data: Vec<u8> = vec![
            0x01, 0x00, 0x00, 0x00, // timestamp_ms = 1
            0x00, 0x00, 0x80, 0x3F, // rpm = 1.0
            0x00, 0x00, 0x80, 0x3F, // pid_output = 1.0
            0x00, 0x00, 0x80, 0x3F, // servo_duty_cycle = 1.0
            0x00, 0x00, 0x00, 0x00, // rpm_error_count = 0
            0x01, 0x00, 0x00, 0x00, // first_valid_rpm_count = 1
            0x01, // fan_on = true
            0x00, 0x00, 0x00, 0x00, // vbat = 0.0
        ];

        let mut cursor = Cursor::new(data);
        let (entry, bytes_read) = PidLogEntryV3::from_reader(&mut cursor)?;

        assert_eq!(bytes_read, 29); // Total bytes read (8 fields)
        assert_eq!(entry.timestamp_ms, 1);
        assert_eq!(entry.rpm, 1.0);
        assert_eq!(entry.pid_output, 1.0);
        assert_eq!(entry.servo_duty_cycle, 1.0);
        assert_eq!(entry.rpm_error_count, 0);
        assert_eq!(entry.first_valid_rpm_count, 1);
        assert_eq!(entry.timestamp_ms_str, "00:00:00.001"); // assuming the parse_timestamp converts 1 ms to this string
        assert!(entry.fan_on);
        assert_eq!(entry.vbat, 0.);
        Ok(())
    }

    #[test]
    fn test_from_reader_incomplete_data() {
        // Incomplete binary data for testing error handling
        let data: Vec<u8> = vec![0x01, 0x00, 0x00]; // Only part of the timestamp

        let mut cursor = Cursor::new(data);
        let result = PidLogEntryV3::from_reader(&mut cursor);

        assert!(result.is_err()); // Should return an error due to insufficient data
    }

    #[test]
    fn test_display() {
        // Create a sample PidLogEntry
        let entry = PidLogEntryV3 {
            timestamp_ms_str: String::from("00:00:00.001"),
            timestamp_ms: 1,
            rpm: 1.0,
            pid_output: 1.0,
            servo_duty_cycle: 1.0,
            rpm_error_count: 0,
            first_valid_rpm_count: 1,
            fan_on: false,
            vbat: 1.0,
        };

        let display_output = format!("{entry}");
        assert_eq!(display_output, "1: 1 1 1 0 1 false 1");

        assert_eq!(entry.timestamp_ns(), 1_000_000.0); // 1 ms in nanoseconds
    }
}
