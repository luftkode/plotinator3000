use crate::logs::{parse_to_vec, parse_unique_description, Log, LogEntry};
use crate::util::parse_timestamp;
use byteorder::{LittleEndian, ReadBytesExt};
use serde_big_array::BigArray;
use std::io;
use std::io::Read;

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLog {
    header: PidLogHeader,
    entries: Vec<PidLogEntry>,
}

impl Log for PidLog {
    type Entry = PidLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = PidLogHeader::from_reader(reader)?;
        let vec_of_entries: Vec<PidLogEntry> = parse_to_vec(reader);

        Ok(Self {
            header,
            entries: vec_of_entries,
        })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl std::fmt::Display for PidLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Header: {}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogHeader {
    #[serde(with = "BigArray")]
    unique_description: [u8; 128],
    version: u16,
}

impl PidLogHeader {
    pub const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-PID-LOG";

    fn unique_description(&self) -> String {
        parse_unique_description(self.unique_description)
    }

    pub fn is_valid_header(&self) -> bool {
        self.unique_description() == Self::UNIQUE_DESCRIPTION
    }

    pub fn is_buf_header(bytes: &mut &[u8]) -> io::Result<bool> {
        let deserialized = Self::from_reader(bytes)?;
        Ok(deserialized.is_valid_header())
    }
}

impl PidLogHeader {
    pub fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut unique_description = [0u8; 128];
        reader.read_exact(&mut unique_description)?;
        let version = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            unique_description,
            version,
        })
    }
}

impl std::fmt::Display for PidLogHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-v{}", self.unique_description(), self.version)
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub rpm: f32,
    pub pid_err: f32,
    pub servo_duty_cycle: f32,
}

impl LogEntry for PidLogEntry {
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let rpm = reader.read_f32::<LittleEndian>()?;
        let pid_err = reader.read_f32::<LittleEndian>()?;
        let servo_duty_cycle = reader.read_f32::<LittleEndian>()?;

        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            rpm,
            pid_err,
            servo_duty_cycle,
        })
    }
    fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
}

impl std::fmt::Display for PidLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {}",
            self.timestamp_ms, self.rpm, self.pid_err, self.servo_duty_cycle
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    const TEST_DATA: &str = "test_data/pid_20240906_081235_00.bin";

    use testresult::TestResult;

    use crate::logs::parse_and_display_log_entries;

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let pidlog = PidLog::from_reader(&mut data.as_slice())?;
        eprintln!("{}", pidlog.header);
        assert_eq!(
            pidlog.header.unique_description(),
            PidLogHeader::UNIQUE_DESCRIPTION
        );
        assert_eq!(pidlog.header.version, 0);
        let first_entry = pidlog.entries.first().unwrap();
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_err, 1.0);
        assert_eq!(first_entry.servo_duty_cycle, 2.0);
        let second_entry = pidlog.entries.get(1).unwrap();
        assert_eq!(second_entry.rpm, 123.0);
        assert_eq!(second_entry.pid_err, 456.0);
        assert_eq!(second_entry.servo_duty_cycle, 789.0);
        //eprintln!("{pidlog}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display() -> TestResult {
        let file = File::open(TEST_DATA)?;
        let mut reader = io::BufReader::new(file);
        let _ = PidLogHeader::from_reader(&mut reader)?;
        parse_and_display_log_entries::<PidLogEntry, _>(&mut reader, Some(10));
        Ok(())
    }
}
