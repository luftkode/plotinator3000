use chrono::{DateTime, Utc};
use entry::PidLogEntry;
use header::PidLogHeader;
use log_if::util::{plot_points_from_log_entry, ExpectedPlotRange};
use log_if::{util::parse_to_vec, LogEntry};
use log_if::{Log, Plotable, RawPlot};
use serde::{Deserialize, Serialize};
use std::{fmt, io};

use super::MbedMotorControlLogHeader;

pub mod entry;
pub mod header;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PidLog {
    header: PidLogHeader,
    entries: Vec<PidLogEntry>,
    timestamps_ns: Vec<f64>,
    all_plots_raw: Vec<RawPlot>,
    startup_timestamp: DateTime<Utc>,
}

impl Log for PidLog {
    type Entry = PidLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = PidLogHeader::from_reader(reader)?;
        let startup_timestamp = header
            .startup_timestamp()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .and_utc();
        let startup_timestamp_ns = startup_timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range")
            as f64;
        let vec_of_entries: Vec<PidLogEntry> = parse_to_vec(reader);
        let timestamps_ns: Vec<f64> = vec_of_entries
            .iter()
            .map(|e| startup_timestamp_ns + e.timestamp_ns())
            .collect();

        let rpm_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.rpm as f64,
        );
        let pid_err_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.pid_err as f64,
        );
        let servo_duty_cycle_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.servo_duty_cycle as f64,
        );
        let rpm_error_count_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.rpm_error_count as f64,
        );
        let first_valid_rpm_count_plot_raw = plot_points_from_log_entry(
            &vec_of_entries,
            |e| e.timestamp_ns() + startup_timestamp_ns,
            |e| e.first_valid_rpm_count as f64,
        );

        let all_plots_raw = vec![
            RawPlot::new("RPM".into(), rpm_plot_raw, ExpectedPlotRange::Thousands),
            RawPlot::new(
                "Pid Error".into(),
                pid_err_plot_raw,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Servo Duty Cycle".into(),
                servo_duty_cycle_plot_raw,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "RPM Error Count".into(),
                rpm_error_count_plot_raw,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "First Valid RPM Count".into(),
                first_valid_rpm_count_plot_raw,
                ExpectedPlotRange::Thousands,
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
            timestamps_ns,
            all_plots_raw,
            startup_timestamp,
        })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl Plotable for PidLog {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.all_plots_raw
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.startup_timestamp
    }

    fn unique_name(&self) -> &str {
        "Mbed PID log 2024"
    }
}

impl fmt::Display for PidLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    const TEST_DATA: &str =
        "../../test_data/mbed_motor_control/new_rpm_algo/pid_20240923_120015_00.bin";

    use header::PidLogHeader;
    use log_if::Log;
    use testresult::TestResult;

    use crate::{mbed_motor_control::MbedMotorControlLogHeader, parse_and_display_log_entries};

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let pidlog = PidLog::from_reader(&mut data.as_slice())?;

        let first_entry = pidlog.entries.first().expect("Empty entries");
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_err, 0.0);
        assert_eq!(first_entry.servo_duty_cycle, 0.0365);
        assert_eq!(first_entry.rpm_error_count, 0);
        assert_eq!(first_entry.first_valid_rpm_count, 0);

        let second_entry = &pidlog.entries[1];
        assert_eq!(second_entry.rpm, 0.0);
        assert_eq!(second_entry.pid_err, 0.0);
        assert_eq!(second_entry.servo_duty_cycle, 0.0365);
        assert_eq!(second_entry.rpm_error_count, 0);
        assert_eq!(second_entry.first_valid_rpm_count, 0);
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
