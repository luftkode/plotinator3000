use std::{fmt, io};

use super::MbedMotorControlLogHeader;
use chrono::{DateTime, Utc};
use entry::{MotorState, StatusLogEntry};
use header::StatusLogHeader;
use log_if::prelude::*;

pub mod entry;
pub mod header;

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLog {
    header: StatusLogHeader,
    entries: Vec<StatusLogEntry>,
    timestamp_ns: Vec<f64>,
    timestamps_with_state_changes: Vec<(f64, MotorState)>, // for memoization
    all_plots_raw: Vec<RawPlot>,
    startup_timestamp: DateTime<Utc>,
}

impl SkytemLog for StatusLog {
    type Entry = StatusLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = StatusLogHeader::from_reader(reader)?;
        let vec_of_entries: Vec<StatusLogEntry> = parse_to_vec(reader);
        let startup_timestamp = header
            .startup_timestamp()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .and_utc();
        let startup_timestamp_ns = startup_timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range")
            as f64;
        let timestamp_ns: Vec<f64> = vec_of_entries
            .iter()
            .map(|e| startup_timestamp_ns + e.timestamp_ns())
            .collect();

        let timestamps_with_state_changes = parse_timestamps_with_state_changes(&vec_of_entries);
        let engine_temp_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.engine_temp as f64,
        );
        let fan_on_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| (e.fan_on as u8) as f64,
        );
        let vbat_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.vbat as f64,
        );
        let setpoint_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.setpoint as f64,
        );
        let motor_state_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| (e.motor_state as u8) as f64,
        );
        let all_plots_raw = vec![
            RawPlot::new(
                "Engine Temp °C".into(),
                engine_temp_plot_raw,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Fan On".into(),
                fan_on_plot_raw,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Vbat [V]".into(),
                vbat_plot_raw,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Setpoint".into(),
                setpoint_plot_raw,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "Motor State".into(),
                motor_state_plot_raw,
                ExpectedPlotRange::OneToOneHundred,
            ),
        ];
        // Iterate through the plots and make sure all the first timestamps match
        if let Some(first_plot) = all_plots_raw.first() {
            if let Some([first_timestamp, ..]) = first_plot.points().first() {
                for p in &all_plots_raw {
                    if let Some([current_first_timestamp, ..]) = p.points().first() {
                        debug_assert_eq!(current_first_timestamp, first_timestamp, "First timestamp of plots are not equal, was an offset applied to some plots but not all?");
                    }
                }
            }
        }

        Ok(Self {
            header,
            entries: vec_of_entries,
            timestamps_with_state_changes,
            timestamp_ns,
            all_plots_raw,
            startup_timestamp,
        })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl GitMetadata for StatusLog {
    fn project_version(&self) -> Option<String> {
        self.header.project_version()
    }

    fn git_short_sha(&self) -> Option<String> {
        self.header.git_short_sha()
    }

    fn git_branch(&self) -> Option<String> {
        self.header.git_branch()
    }

    fn git_repo_status(&self) -> Option<String> {
        self.header.git_repo_status()
    }
}

impl Plotable for StatusLog {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.all_plots_raw
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.startup_timestamp
    }

    fn unique_name(&self) -> &str {
        "Mbed Status log 2024"
    }
}

impl StatusLog {
    /// If we don't match up with other plot points it is because the date was offset so we need to apply the offset here as well
    pub fn update_timestamp_with_state_changes(&mut self) {
        if let Some((first_st_change_timestamp, _)) = self.timestamps_with_state_changes.first_mut()
        {
            if let Some(first_entry) = self.entries.first() {
                let first_entry_timestamp = first_entry.timestamp_ns();
                if first_entry_timestamp != *first_st_change_timestamp {
                    let offset = first_entry_timestamp - *first_st_change_timestamp;
                    for (timestamp, _) in &mut self.timestamps_with_state_changes {
                        *timestamp += offset;
                    }
                }
            }
        }
    }

    pub fn timestamps_with_state_changes(&self) -> &[(f64, MotorState)] {
        &self.timestamps_with_state_changes
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

fn parse_timestamps_with_state_changes(entries: &[StatusLogEntry]) -> Vec<(f64, MotorState)> {
    let mut result = Vec::new();
    let mut last_state = None;

    for entry in entries {
        // Check if the current state is different from the last recorded state
        if last_state != Some(entry.motor_state) {
            result.push((entry.timestamp_ns(), entry.motor_state));
            last_state = Some(entry.motor_state);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use testresult::TestResult;

    const TEST_DATA: &str =
        "../../test_data/mbed_motor_control/20240926_121708/status_20240926_121708_00.bin";

    use crate::{mbed_motor_control::MbedMotorControlLogHeader, parse_and_display_log_entries};

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let status_log = StatusLog::from_reader(&mut data.as_slice())?;
        eprintln!("{}", status_log.header);

        let first_entry = status_log.entries().first().expect("Empty entries vec");
        assert_eq!(first_entry.engine_temp, 64.394905);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 11.76342);
        assert_eq!(first_entry.setpoint, 1800.0);
        assert_eq!(first_entry.motor_state, MotorState::ECU_ON_WAIT_PUMP);
        let second_entry = &status_log.entries[1];
        assert_eq!(second_entry.engine_temp, 64.394905);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 11.718291);
        assert_eq!(second_entry.setpoint, 1800.0);
        assert_eq!(second_entry.motor_state, MotorState::ECU_ON_WAIT_PUMP);

        let last_entry = status_log.entries().last().expect("Empty entries vec");
        assert_eq!(last_entry.timestamp_ns(), 930624.0 * 1_000_000.0);
        assert_eq!(last_entry.engine_temp, 77.31132);
        assert!(!last_entry.fan_on);
        assert_eq!(last_entry.vbat, 11.996582);
        assert_eq!(last_entry.setpoint, 3600.0);
        assert_eq!(last_entry.motor_state, MotorState::STANDBY_WAIT_FOR_CAP);
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
