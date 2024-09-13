use crate::logs::{parse_to_vec, Log};
use entry::PidLogEntry;
use header::PidLogHeader;

use std::{fmt, io};

use super::MbedMotorControlLogHeader;

pub mod entry;
pub mod header;

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Header: {}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    const TEST_DATA: &str = "test_data/mbed_motor_control/old_rpm_algo/pid_20240912_122203_00.bin";

    use header::PidLogHeader;
    use testresult::TestResult;

    use crate::logs::{
        mbed_motor_control::MbedMotorControlLogHeader, parse_and_display_log_entries,
    };

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let pidlog = PidLog::from_reader(&mut data.as_slice())?;

        let first_entry = pidlog.entries.first().unwrap();
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_err, 0.0);
        assert_eq!(first_entry.servo_duty_cycle, 0.03075);
        let second_entry = pidlog.entries.get(1).unwrap();
        assert_eq!(second_entry.rpm, 0.0);
        assert_eq!(second_entry.pid_err, 0.0);
        assert_eq!(second_entry.servo_duty_cycle, 0.03075);
        //eprintln!("{pidlog}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display() -> TestResult {
        let file = File::open(TEST_DATA)?;
        let mut reader = io::BufReader::new(file);
        let header = PidLogHeader::from_reader(&mut reader)?;
        println!("{header}");
        parse_and_display_log_entries::<PidLogEntry, _>(&mut reader, Some(10));
        Ok(())
    }
}
