use chrono::{DateTime, Utc};
use log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    io::{self, Read},
    path::Path,
};

use crate::{
    mbed_motor_control::{
        mbed_config::MbedConfig,
        mbed_header::{MbedMotorControlLogHeader, SIZEOF_UNIQ_DESC},
    },
    parse_unique_description,
};

use super::{
    entry::{
        convert_v1_to_status_log_entry, convert_v2_to_status_log_entry,
        convert_v3_to_status_log_entry, v1::StatusLogEntryV1, v2::StatusLogEntryV2,
        v3::StatusLogEntryV3, StatusLogEntry,
    },
    header::StatusLogHeader,
};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StatusLog {
    header: StatusLogHeader,
    entries: Vec<StatusLogEntry>,
    timestamp_ns: Vec<f64>,
    labels: Vec<PlotLabels>,
    all_plots_raw: Vec<RawPlot>,
    startup_timestamp: DateTime<Utc>,
}

impl StatusLog {
    /// Checks if the file at the given path is a valid [`PidLog`] file
    pub fn file_is_valid(path: &Path) -> bool {
        let Ok(mut file) = fs::File::open(path) else {
            return false;
        };

        let mut buffer = vec![0u8; SIZEOF_UNIQ_DESC + 2];
        match file.read_exact(&mut buffer) {
            Ok(_) => Self::is_buf_valid(&buffer),
            Err(_) => false, // Return false if we can't read enough bytes
        }
    }

    // helper function build all the plots that can be made from a statuslog
    fn build_raw_plots(startup_timestamp_ns: f64, entries: &[StatusLogEntry]) -> Vec<RawPlot> {
        let entry_count = entries.len();
        let mut engine_temp_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut fan_on_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut vbat_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut setpoint_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut motor_state_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut runtime_h_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);

        for e in entries {
            match e {
                StatusLogEntry::V1(e) => {
                    engine_temp_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        e.engine_temp as f64,
                    ]);
                    fan_on_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.fan_on as u8) as f64,
                    ]);
                    vbat_plot_raw.push([e.timestamp_ns() + startup_timestamp_ns, e.vbat as f64]);
                    setpoint_plot_raw
                        .push([e.timestamp_ns() + startup_timestamp_ns, e.setpoint as f64]);

                    motor_state_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.motor_state as u8) as f64,
                    ]);
                }
                StatusLogEntry::V2(e) => {
                    engine_temp_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        e.engine_temp as f64,
                    ]);
                    fan_on_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.fan_on as u8) as f64,
                    ]);
                    vbat_plot_raw.push([e.timestamp_ns() + startup_timestamp_ns, e.vbat as f64]);
                    setpoint_plot_raw
                        .push([e.timestamp_ns() + startup_timestamp_ns, e.setpoint as f64]);

                    motor_state_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.motor_state as u8) as f64,
                    ]);
                }
                StatusLogEntry::V3(e) => {
                    engine_temp_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        e.engine_temp as f64,
                    ]);
                    fan_on_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.fan_on as u8) as f64,
                    ]);
                    vbat_plot_raw.push([e.timestamp_ns() + startup_timestamp_ns, e.vbat as f64]);
                    setpoint_plot_raw
                        .push([e.timestamp_ns() + startup_timestamp_ns, e.setpoint as f64]);

                    motor_state_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.motor_state as u8) as f64,
                    ]);
                    runtime_h_plot_raw.push([
                        e.timestamp_ns() + startup_timestamp_ns,
                        (e.runtime_s as f64) / 3600., // Convert from seconds to hours
                    ]);
                }
            }
        }

        Self::collect_raw_plots(
            engine_temp_plot_raw,
            fan_on_plot_raw,
            vbat_plot_raw,
            setpoint_plot_raw,
            motor_state_plot_raw,
            runtime_h_plot_raw,
        )
    }

    // Simply takes all vectors with raw plot points and collects them into a vector of `RawPlot`
    fn collect_raw_plots(
        engine_temp: Vec<[f64; 2]>,
        fan_on: Vec<[f64; 2]>,
        vbat: Vec<[f64; 2]>,
        setpoint: Vec<[f64; 2]>,
        motor_state: Vec<[f64; 2]>,
        runtime_h: Vec<[f64; 2]>,
    ) -> Vec<RawPlot> {
        let mut raw_plots = vec![];
        if !engine_temp.is_empty() {
            raw_plots.push(RawPlot::new(
                "Engine Temp °C".into(),
                engine_temp,
                ExpectedPlotRange::OneToOneHundred,
            ));
        }
        if !fan_on.is_empty() {
            raw_plots.push(RawPlot::new(
                "Fan On".into(),
                fan_on,
                ExpectedPlotRange::Percentage,
            ));
        }
        if !vbat.is_empty() {
            raw_plots.push(RawPlot::new(
                "Vbat [V]".into(),
                vbat,
                ExpectedPlotRange::OneToOneHundred,
            ));
        }
        if !setpoint.is_empty() {
            raw_plots.push(RawPlot::new(
                "Setpoint".into(),
                setpoint,
                ExpectedPlotRange::Thousands,
            ));
        }

        if !motor_state.is_empty() {
            raw_plots.push(RawPlot::new(
                "Motor State".into(),
                motor_state,
                ExpectedPlotRange::OneToOneHundred,
            ));
        }
        if !runtime_h.is_empty() {
            raw_plots.push(RawPlot::new(
                "Runtime [h]".into(),
                runtime_h,
                ExpectedPlotRange::OneToOneHundred,
            ));
        }
        raw_plots
    }
}

impl Parseable for StatusLog {
    const DESCRIPTIVE_NAME: &str = "Mbed Status Log";

    /// Probes the buffer and check if it starts with [`Self::UNIQUE_DESCRIPTION`] and therefor contains a valid [`PidLog`]
    fn is_buf_valid(content: &[u8]) -> bool {
        if content.len() < SIZEOF_UNIQ_DESC + 2 {
            return false;
        }

        let unique_description = &content[..SIZEOF_UNIQ_DESC];
        parse_unique_description(unique_description) == super::UNIQUE_DESCRIPTION
    }

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut total_bytes_read: usize = 0;
        let (header, bytes_read) = StatusLogHeader::from_reader(reader)?;
        total_bytes_read += bytes_read;

        let (vec_of_entries, entry_bytes_read): (Vec<StatusLogEntry>, usize) =
            if header.version() < 3 {
                let (v1_vec, entry_bytes_read) = parse_to_vec::<StatusLogEntryV1>(reader);
                (convert_v1_to_status_log_entry(v1_vec), entry_bytes_read)
            } else if header.version() < 5 {
                let (v2_vec, entry_bytes_read) = parse_to_vec::<StatusLogEntryV2>(reader);
                (convert_v2_to_status_log_entry(v2_vec), entry_bytes_read)
            } else if header.version() == 5 {
                let (v3_vec, entry_bytes_read) = parse_to_vec::<StatusLogEntryV3>(reader);
                (convert_v3_to_status_log_entry(v3_vec), entry_bytes_read)
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported header version: {}", header.version()),
                ));
            };
        total_bytes_read += entry_bytes_read;
        let startup_timestamp = match header {
            StatusLogHeader::V1(h) => h.startup_timestamp(),
            StatusLogHeader::V2(h) => h.startup_timestamp(),
            StatusLogHeader::V3(h) => h.startup_timestamp(),
            StatusLogHeader::V5(h) => h.startup_timestamp(),
        }
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

        let all_plots_raw = Self::build_raw_plots(startup_timestamp_ns, &vec_of_entries);
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

        Ok((
            Self {
                header,
                entries: vec_of_entries,
                labels,
                timestamp_ns,
                all_plots_raw,
                startup_timestamp,
            },
            total_bytes_read,
        ))
    }
}

impl GitMetadata for StatusLog {
    fn project_version(&self) -> Option<String> {
        match &self.header {
            StatusLogHeader::V1(h) => h.project_version(),
            StatusLogHeader::V2(h) => h.project_version(),
            StatusLogHeader::V3(h) => h.project_version(),
            StatusLogHeader::V5(h) => h.project_version(),
        }
    }
    fn git_short_sha(&self) -> Option<String> {
        match &self.header {
            StatusLogHeader::V1(h) => h.git_short_sha(),
            StatusLogHeader::V2(h) => h.git_short_sha(),
            StatusLogHeader::V3(h) => h.git_short_sha(),
            StatusLogHeader::V5(h) => h.git_short_sha(),
        }
    }

    fn git_branch(&self) -> Option<String> {
        match &self.header {
            StatusLogHeader::V1(h) => h.git_branch(),
            StatusLogHeader::V2(h) => h.git_branch(),
            StatusLogHeader::V3(h) => h.git_branch(),
            StatusLogHeader::V5(h) => h.git_branch(),
        }
    }

    fn git_repo_status(&self) -> Option<String> {
        match &self.header {
            StatusLogHeader::V1(h) => h.git_repo_status(),
            StatusLogHeader::V2(h) => h.git_repo_status(),
            StatusLogHeader::V3(h) => h.git_repo_status(),
            StatusLogHeader::V5(h) => h.git_repo_status(),
        }
    }
}

impl Plotable for StatusLog {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.all_plots_raw
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.startup_timestamp
    }

    fn descriptive_name(&self) -> &str {
        match self.header {
            StatusLogHeader::V1(_) => "Mbed Status v1",
            StatusLogHeader::V2(_) => "Mbed Status v2",
            StatusLogHeader::V3(_) => "Mbed Status v3",
            StatusLogHeader::V5(_) => "Mbed Status v5",
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        Some(&self.labels)
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        let mut metadata = vec![
            (
                "Project Version".to_owned(),
                self.project_version().unwrap_or_else(|| "N/A".to_owned()),
            ),
            (
                "Git Branch".to_owned(),
                self.git_branch().unwrap_or_else(|| "N/A".to_owned()),
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
        ];

        match self.header {
            // V1 has no more than that
            StatusLogHeader::V1(_) => (),
            // V2 also has config values
            StatusLogHeader::V2(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
            StatusLogHeader::V3(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
            StatusLogHeader::V5(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
        }

        Some(metadata)
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

fn parse_timestamps_with_state_changes(
    entries: &[StatusLogEntry],
    startup_timestamp_ns: f64,
) -> Vec<([f64; 2], String)> {
    let mut result = Vec::new();
    let mut last_state = None;

    for entry in entries {
        // Check if the current state is different from the last recorded state
        if last_state != Some(entry.motor_state()) {
            // apply negative offset if we're going to a state with a lower value
            let offset = if last_state.is_some_and(|ls| ls > entry.motor_state()) {
                -0.5
            } else {
                0.5
            };
            result.push((
                [
                    entry.timestamp_ns() + startup_timestamp_ns,
                    (entry.motor_state()) as f64 + offset,
                ],
                entry.motor_state_string(),
            ));
            last_state = Some(entry.motor_state());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_util::*;

    use crate::{
        mbed_motor_control::status::entry::{v1::MotorState, v2},
        parse_and_display_log_entries,
    };

    #[test]
    fn test_deserialize_v1() -> TestResult {
        let mut data = MBED_STATUS_V1_BYTES;
        let full_data_len = data.len();
        let (status_log, bytes_read) = StatusLog::from_reader(&mut data)?;

        eprintln!("{}", status_log.header);
        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 16371);
        let first_entry = status_log.entries.first().expect("Empty entries vec");
        match first_entry {
            StatusLogEntry::V1(first_entry) => {
                assert_eq!(first_entry.engine_temp, 64.394905);
                assert!(!first_entry.fan_on);
                assert_eq!(first_entry.vbat, 11.76342);
                assert_eq!(first_entry.setpoint, 1800.0);
                assert_eq!(first_entry.motor_state, MotorState::ECU_ON_WAIT_PUMP);
            }
            StatusLogEntry::V2(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v1"),
        }

        let second_entry = &status_log.entries[1];
        match second_entry {
            StatusLogEntry::V1(second_entry) => {
                assert_eq!(second_entry.engine_temp, 64.394905);
                assert!(!second_entry.fan_on);
                assert_eq!(second_entry.vbat, 11.718291);
                assert_eq!(second_entry.setpoint, 1800.0);
                assert_eq!(second_entry.motor_state, MotorState::ECU_ON_WAIT_PUMP);
            }
            StatusLogEntry::V2(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v1"),
        }

        let last_entry = status_log.entries.last().expect("Empty entries vec");
        match last_entry {
            StatusLogEntry::V1(last_entry) => {
                assert_eq!(last_entry.timestamp_ns(), 930624.0 * 1_000_000.0);
                assert_eq!(last_entry.engine_temp, 77.31132);
                assert!(!last_entry.fan_on);
                assert_eq!(last_entry.vbat, 11.996582);
                assert_eq!(last_entry.setpoint, 3600.0);
                assert_eq!(last_entry.motor_state, MotorState::STANDBY_WAIT_FOR_CAP);
            }
            StatusLogEntry::V2(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v1"),
        }

        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display_v1() -> TestResult {
        let file = fs::File::open(mbed_status_v1())?;
        let mut reader = io::BufReader::new(file);
        let (header, bytes_read) = StatusLogHeader::from_reader(&mut reader)?;
        assert_eq!(bytes_read, 261);
        println!("{header}");
        parse_and_display_log_entries::<StatusLogEntryV1>(&mut reader, Some(10));
        Ok(())
    }

    #[test]
    fn test_deserialize_v2() -> TestResult {
        let mut data = MBED_STATUS_V2_BYTES;
        let full_data_len = data.len();
        let (status_log, bytes_read) = StatusLog::from_reader(&mut data)?;

        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 12281);
        eprintln!("{}", status_log.header);

        let first_entry = status_log.entries.first().expect("Empty entries vec");
        let first_entry: &StatusLogEntryV1 = match first_entry {
            StatusLogEntry::V1(status_log_entry_v1) => status_log_entry_v1,
            StatusLogEntry::V2(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v1"),
        };
        assert_eq!(first_entry.engine_temp, 4.8440366);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 11.928035);
        assert_eq!(first_entry.setpoint, 2500.0);
        assert_eq!(first_entry.motor_state, MotorState::POWER_HOLD);
        let second_entry = &status_log.entries[1];
        let second_entry = match second_entry {
            StatusLogEntry::V1(status_log_entry_v1) => status_log_entry_v1,
            StatusLogEntry::V2(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v1"),
        };
        assert_eq!(second_entry.engine_temp, 4.8623853);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 11.943078);
        assert_eq!(second_entry.setpoint, 2500.0);
        assert_eq!(second_entry.motor_state, MotorState::POWER_HOLD);

        let last_entry = status_log.entries.last().expect("Empty entries vec");
        let last_entry = match last_entry {
            StatusLogEntry::V1(status_log_entry_v1) => status_log_entry_v1,
            StatusLogEntry::V2(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v1"),
        };
        assert_eq!(last_entry.timestamp_ns(), 687042000000.0);
        assert_eq!(last_entry.engine_temp, 76.5566);
        assert!(!last_entry.fan_on);
        assert_eq!(last_entry.vbat, 13.868547);
        assert_eq!(last_entry.setpoint, 3600.0);
        assert_eq!(last_entry.motor_state, MotorState::STANDBY_READY);
        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display_v2() -> TestResult {
        let file = fs::File::open(mbed_status_v2())?;
        let mut reader = io::BufReader::new(file);
        let (header, bytes_read) = StatusLogHeader::from_reader(&mut reader)?;

        println!("{header}");
        assert_eq!(bytes_read, 293);
        parse_and_display_log_entries::<StatusLogEntryV1>(&mut reader, Some(10));
        Ok(())
    }

    #[test]
    fn test_deserialize_v3() -> TestResult {
        let mut data = MBED_STATUS_V3_BYTES;
        let full_data_len = data.len();
        let (status_log, bytes_read) = StatusLog::from_reader(&mut data)?;

        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 509);
        eprintln!("{}", status_log.header);

        let first_entry = status_log.entries.first().expect("Empty entries vec");
        let first_entry = match first_entry {
            StatusLogEntry::V1(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v2"),
            StatusLogEntry::V2(status_log_entry_v2) => status_log_entry_v2,
        };
        assert_eq!(first_entry.engine_temp, 40.482456);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 12.958463);
        assert_eq!(first_entry.setpoint, 2500.0);
        assert_eq!(first_entry.motor_state, v2::MotorState::POWER_HOLD);
        let second_entry = &status_log.entries[1];
        let second_entry = match second_entry {
            StatusLogEntry::V1(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v2"),
            StatusLogEntry::V2(status_log_entry_v2) => status_log_entry_v2,
        };
        assert_eq!(second_entry.engine_temp, 40.482456);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 12.928377);
        assert_eq!(second_entry.setpoint, 2500.0);
        assert_eq!(second_entry.motor_state, v2::MotorState::ECU_ON_WAIT_PUMP);

        let last_entry = status_log.entries.last().expect("Empty entries vec");
        let last_entry = match last_entry {
            StatusLogEntry::V1(_) | StatusLogEntry::V3(_) => panic!("Expected status log entry v2"),
            StatusLogEntry::V2(status_log_entry_v2) => status_log_entry_v2,
        };
        assert_eq!(last_entry.timestamp_ns(), 14300000000.0);
        assert_eq!(last_entry.engine_temp, 40.964912);
        assert!(!last_entry.fan_on);
        assert_eq!(last_entry.vbat, 12.552309);
        assert_eq!(last_entry.setpoint, 2500.0);
        assert_eq!(last_entry.motor_state, v2::MotorState::IDLE);
        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_deserialize_v5() -> TestResult {
        let mut data = MBED_STATUS_V5_BYTES;
        let full_data_len = data.len();
        let (status_log, bytes_read) = StatusLog::from_reader(&mut data)?;

        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 157165);
        eprintln!("{}", status_log.header);

        let first_entry = status_log.entries.first().expect("Empty entries vec");
        let first_entry = match first_entry {
            StatusLogEntry::V1(_) | StatusLogEntry::V2(_) => panic!("Expected status log entry v3"),
            StatusLogEntry::V3(status_log_entry_v3) => status_log_entry_v3,
        };
        assert_eq!(first_entry.engine_temp, 74.63115);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 12.477095);
        assert_eq!(first_entry.setpoint, 1234.0);
        assert_eq!(first_entry.motor_state, v2::MotorState::ECU_ON_WAIT_PUMP);
        let second_entry = &status_log.entries[1];
        let second_entry = match second_entry {
            StatusLogEntry::V1(_) | StatusLogEntry::V2(_) => panic!("Expected status log entry v3"),
            StatusLogEntry::V3(status_log_entry_v2) => status_log_entry_v2,
        };
        assert_eq!(second_entry.engine_temp, 74.63115);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 12.477095);
        assert_eq!(second_entry.setpoint, 1234.0);
        assert_eq!(second_entry.motor_state, v2::MotorState::ECU_ON_WAIT_PUMP);

        let last_entry = status_log.entries.last().expect("Empty entries vec");
        let last_entry = match last_entry {
            StatusLogEntry::V1(_) | StatusLogEntry::V2(_) => panic!("Expected status log entry v3"),
            StatusLogEntry::V3(status_log_entry_v2) => status_log_entry_v2,
        };
        assert_eq!(last_entry.timestamp_ns(), 906388000000.0);
        assert_eq!(last_entry.engine_temp, 78.820755);
        assert!(last_entry.fan_on);
        assert_eq!(last_entry.vbat, 12.236411);
        assert_eq!(last_entry.setpoint, 2500.0);
        assert_eq!(last_entry.motor_state, v2::MotorState::WAIT_TIME_SHUTDOWN);
        //eprintln!("{status_log}");
        Ok(())
    }
}
