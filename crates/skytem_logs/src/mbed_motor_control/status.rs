use std::{fmt, io};

use super::MbedMotorControlLogHeader;
use entry::{MotorState, StatusLogEntry};
use header::StatusLogHeader;
use log_if::{util::parse_to_vec, LogEntry};
use plot_util::{raw_plot_from_log_entry, ExpectedPlotRange, RawPlot};

pub mod entry;
pub mod header;

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLog {
    header: StatusLogHeader,
    entries: Vec<StatusLogEntry>,
    timestamps_ms: Vec<f64>,
    timestamps_with_state_changes: Vec<(u32, MotorState)>, // for memoization
    all_plots_raw: Vec<RawPlot>,
}

impl log_if::Log for StatusLog {
    type Entry = StatusLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = StatusLogHeader::from_reader(reader)?;
        let vec_of_entries: Vec<StatusLogEntry> = parse_to_vec(reader);
        let timestamps_with_state_changes = parse_timestamps_with_state_changes(&vec_of_entries);
        let timestamps_ms: Vec<f64> = vec_of_entries.iter().map(|e| e.timestamp_ms()).collect();
        let engine_temp_plot_raw = raw_plot_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ms(),
            |e| e.engine_temp as f64,
        );
        let fan_on_plot_raw = raw_plot_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ms(),
            |e| (e.fan_on as u8) as f64,
        );
        let vbat_plot_raw =
            raw_plot_from_log_entry(&vec_of_entries, |e| e.timestamp_ms(), |e| e.vbat as f64);
        let setpoint_plot_raw =
            raw_plot_from_log_entry(&vec_of_entries, |e| e.timestamp_ms(), |e| e.setpoint as f64);
        let motor_state_plot_raw = raw_plot_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ms(),
            |e| (e.motor_state as u8) as f64,
        );
        let all_plots_raw = vec![
            (
                engine_temp_plot_raw,
                String::from("Engine Temp °C"),
                ExpectedPlotRange::OneToOneHundred,
            ),
            (
                fan_on_plot_raw,
                String::from("Fan On"),
                ExpectedPlotRange::Percentage,
            ),
            (
                vbat_plot_raw,
                String::from("Vbat [V]"),
                ExpectedPlotRange::OneToOneHundred,
            ),
            (
                setpoint_plot_raw,
                String::from("Setpoint"),
                ExpectedPlotRange::Thousands,
            ),
            (
                motor_state_plot_raw,
                String::from("Motor State"),
                ExpectedPlotRange::OneToOneHundred,
            ),
        ];

        Ok(Self {
            header,
            entries: vec_of_entries,
            timestamps_with_state_changes,
            timestamps_ms,
            all_plots_raw,
        })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl StatusLog {
    pub fn timestamps_with_state_changes(&self) -> &[(u32, MotorState)] {
        &self.timestamps_with_state_changes
    }
    pub fn all_plots_raw(&self) -> &[RawPlot] {
        &self.all_plots_raw
    }
}

impl fmt::Display for StatusLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Header: {}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

fn parse_timestamps_with_state_changes(entries: &[StatusLogEntry]) -> Vec<(u32, MotorState)> {
    let mut result = Vec::new();
    let mut last_state = None;

    for entry in entries {
        // Check if the current state is different from the last recorded state
        if last_state != Some(entry.motor_state) {
            result.push((entry.timestamp_ms, entry.motor_state));
            last_state = Some(entry.motor_state);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use log_if::Log;
    use std::fs::{self, File};
    use testresult::TestResult;

    const TEST_DATA: &str =
        "../../test_data/mbed_motor_control/new_rpm_algo/status_20240923_120015_00.bin";

    use crate::{mbed_motor_control::MbedMotorControlLogHeader, parse_and_display_log_entries};

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let status_log = StatusLog::from_reader(&mut data.as_slice())?;
        eprintln!("{}", status_log.header);

        let first_entry = status_log.entries().first().expect("Empty entries vec");
        assert_eq!(first_entry.engine_temp, 66.63043);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 12.222223);
        assert_eq!(first_entry.setpoint, 2500.0);
        assert_eq!(first_entry.motor_state, MotorState::ECU_ON_WAIT_PUMP);
        let second_entry = &status_log.entries[1];
        assert_eq!(second_entry.engine_temp, 65.76087);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 12.229744);
        assert_eq!(second_entry.setpoint, 2500.0);
        assert_eq!(second_entry.motor_state, MotorState::ECU_ON_WAIT_PUMP);

        let last_entry = status_log.entries().last().expect("Empty entries vec");
        assert_eq!(last_entry.timestamp_ms(), 736113.0);
        assert_eq!(last_entry.engine_temp, 81.32979);
        assert!(last_entry.fan_on);
        assert_eq!(last_entry.vbat, 11.665642);
        assert_eq!(last_entry.setpoint, 0.0);
        assert_eq!(last_entry.motor_state, MotorState::WAIT_TIME_SHUTDOWN);
        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display() -> TestResult {
        let file = File::open(TEST_DATA)?;
        let mut reader = io::BufReader::new(file);
        let header = StatusLogHeader::from_reader(&mut reader)?;
        println!("{header}");
        parse_and_display_log_entries::<StatusLogEntry, _>(&mut reader, Some(10));
        Ok(())
    }
}
