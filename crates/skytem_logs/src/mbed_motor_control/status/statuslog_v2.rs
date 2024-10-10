use std::{fmt, io};

use chrono::{DateTime, Utc};
use log_if::prelude::*;

use crate::mbed_motor_control::mbed_header::MbedMotorControlLogHeader;

use super::{entry::StatusLogEntry, header_v2::StatusLogHeaderV2};

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogV2 {
    header: StatusLogHeaderV2,
    entries: Vec<StatusLogEntry>,
    timestamp_ns: Vec<f64>,
    labels: Vec<PlotLabels>,
    all_plots_raw: Vec<RawPlot>,
    startup_timestamp: DateTime<Utc>,
}

impl SkytemLog for StatusLogV2 {
    type Entry = StatusLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = StatusLogHeaderV2::from_reader(reader)?;
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

        let timestamps_with_state_changes =
            parse_timestamps_with_state_changes(&vec_of_entries, startup_timestamp_ns);
        let labels = vec![PlotLabels::new(
            timestamps_with_state_changes,
            ExpectedPlotRange::OneToOneHundred,
        )];
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
            labels,
            timestamp_ns,
            all_plots_raw,
            startup_timestamp,
        })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl GitMetadata for StatusLogV2 {
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

impl Plotable for StatusLogV2 {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.all_plots_raw
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.startup_timestamp
    }

    fn descriptive_name(&self) -> &str {
        "Mbed Status v2"
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        Some(&self.labels)
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        let mut metadata = vec![
            (
                "Project Version".to_string(),
                self.project_version().unwrap_or_else(|| "N/A".to_string()),
            ),
            (
                "Git Branch".to_owned(),
                self.git_branch().unwrap_or_else(|| "N/A".to_string()),
            ),
            (
                "Git Repo Status".to_owned(),
                self.git_repo_status().unwrap_or_else(|| "Clean".to_owned()),
            ),
            (
                "Git Short SHA".to_owned(),
                self.git_short_sha().unwrap_or_else(|| "N/A".to_owned()),
            ),
            (
                "Startup Timestamp".to_owned(),
                self.startup_timestamp.naive_utc().to_string(),
            ),
            ("Config values".to_owned(), "".to_owned()),
        ];

        metadata.extend_from_slice(&self.header.mbed_config().field_value_pairs());

        Some(metadata)
    }
}

impl fmt::Display for StatusLogV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Header: {}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

fn parse_timestamps_with_state_changes(
    entries: &[StatusLogEntry],
    startup_timestamp_ns: f64,
) -> Vec<([f64; 2], String)> {
    let mut result = Vec::new();
    let mut last_state = None;

    for entry in entries {
        // Check if the current state is different from the last recorded state
        if last_state != Some(entry.motor_state) {
            // apply negative offset if we're going to a state with a lower value
            let offset = if last_state.is_some_and(|ls| ls as u8 > entry.motor_state as u8) {
                -0.5
            } else {
                0.5
            };
            result.push((
                [
                    entry.timestamp_ns() + startup_timestamp_ns,
                    (entry.motor_state as u8) as f64 + offset,
                ],
                entry.motor_state.to_string(),
            ));
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
        "../../test_data/mbed_motor_control/v2/20240822_085220/status_20240822_085220_00.bin";

    use crate::{mbed_motor_control::status::entry::MotorState, parse_and_display_log_entries};

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let status_log = StatusLogV2::from_reader(&mut data.as_slice())?;
        eprintln!("{}", status_log.header);

        let first_entry = status_log.entries().first().expect("Empty entries vec");
        assert_eq!(first_entry.engine_temp, 4.770642);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 4.730086);
        assert_eq!(first_entry.setpoint, 2500.0);
        assert_eq!(first_entry.motor_state, MotorState::POWER_HOLD);
        let second_entry = &status_log.entries[1];
        assert_eq!(second_entry.engine_temp, 4.770642);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 4.75265);
        assert_eq!(second_entry.setpoint, 2500.0);
        assert_eq!(second_entry.motor_state, MotorState::POWER_HOLD);

        let last_entry = status_log.entries().last().expect("Empty entries vec");
        assert_eq!(last_entry.timestamp_ns(), 14846000000.0);
        assert_eq!(last_entry.engine_temp, 4.770642);
        assert!(!last_entry.fan_on);
        assert_eq!(last_entry.vbat, 4.7075214);
        assert_eq!(last_entry.setpoint, 2500.0);
        assert_eq!(last_entry.motor_state, MotorState::POWER_HOLD);
        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display() -> TestResult {
        let file = File::open(TEST_DATA)?;
        let mut reader = io::BufReader::new(file);
        let header = StatusLogHeaderV2::from_reader(&mut reader)?;
        println!("{header}");
        parse_and_display_log_entries::<StatusLogEntry, _>(&mut reader, Some(10));
        Ok(())
    }
}
