use chrono::{DateTime, Utc};
use log_if::{log::LogEntry, parseable::Parseable, prelude::*};
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
        convert_v1_to_pid_log_entry, convert_v2_to_pid_log_entry, v1::PidLogEntryV1,
        v2::PidLogEntryV2, PidLogEntry,
    },
    header::PidLogHeader,
};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PidLog {
    header: PidLogHeader,
    entries: Vec<PidLogEntry>,
    timestamps_ns: Vec<f64>,
    all_plots_raw: Vec<RawPlot>,
    startup_timestamp: DateTime<Utc>,
}

impl PidLog {
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

    // helper function build all the plots that can be made from a pidlog
    fn build_raw_plots(startup_timestamp_ns: f64, entries: &[PidLogEntry]) -> Vec<RawPlot> {
        let entry_count = entries.len();
        let mut rpm_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut pid_output_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut servo_duty_cycle_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut rpm_error_count_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut first_valid_rpm_count_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut fan_on_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);
        let mut vbat_plot_raw: Vec<[f64; 2]> = Vec::with_capacity(entry_count);

        for e in entries {
            match e {
                PidLogEntry::V1(e) => {
                    let ts: f64 = e.timestamp_ns() + startup_timestamp_ns;
                    rpm_plot_raw.push([ts, e.rpm.into()]);
                    pid_output_plot_raw.push([ts, e.pid_output.into()]);
                    servo_duty_cycle_plot_raw.push([ts, e.servo_duty_cycle.into()]);
                    rpm_error_count_plot_raw.push([ts, e.rpm_error_count as f64]);
                    first_valid_rpm_count_plot_raw.push([ts, e.first_valid_rpm_count as f64]);
                }
                PidLogEntry::V2(e) => {
                    let ts: f64 = e.timestamp_ns() + startup_timestamp_ns;
                    rpm_plot_raw.push([ts, e.rpm.into()]);
                    pid_output_plot_raw.push([ts, e.pid_output.into()]);
                    servo_duty_cycle_plot_raw.push([ts, e.servo_duty_cycle.into()]);
                    rpm_error_count_plot_raw.push([ts, e.rpm_error_count as f64]);
                    first_valid_rpm_count_plot_raw.push([ts, e.first_valid_rpm_count as f64]);
                    fan_on_plot_raw.push([ts, (e.fan_on as u8) as f64]);
                    vbat_plot_raw.push([ts, e.vbat as f64]);
                }
            }
        }

        Self::collect_raw_plots(
            rpm_plot_raw,
            pid_output_plot_raw,
            servo_duty_cycle_plot_raw,
            rpm_error_count_plot_raw,
            first_valid_rpm_count_plot_raw,
            fan_on_plot_raw,
            vbat_plot_raw,
        )
    }

    // Simply takes all vectors with raw plot points and collects them into a vector of `RawPlot`
    fn collect_raw_plots(
        rpm: Vec<[f64; 2]>,
        pid_output: Vec<[f64; 2]>,
        servo_duty_cycle: Vec<[f64; 2]>,
        rpm_error_count: Vec<[f64; 2]>,
        first_valid_rpm_count: Vec<[f64; 2]>,
        fan_on: Vec<[f64; 2]>,
        vbat: Vec<[f64; 2]>,
    ) -> Vec<RawPlot> {
        let mut raw_plots = vec![];
        if !rpm.is_empty() {
            raw_plots.push(RawPlot::new(
                "RPM".into(),
                rpm,
                ExpectedPlotRange::Thousands,
            ));
        }
        if !pid_output.is_empty() {
            raw_plots.push(RawPlot::new(
                "PID Output".into(),
                pid_output,
                ExpectedPlotRange::Percentage,
            ));
        }
        if !servo_duty_cycle.is_empty() {
            raw_plots.push(RawPlot::new(
                "Servo Duty Cycle".into(),
                servo_duty_cycle,
                ExpectedPlotRange::Percentage,
            ));
        }
        if !rpm_error_count.is_empty() {
            raw_plots.push(RawPlot::new(
                "RPM Error Count".into(),
                rpm_error_count,
                ExpectedPlotRange::OneToOneHundred,
            ));
        }
        if !first_valid_rpm_count.is_empty() {
            raw_plots.push(RawPlot::new(
                "First Valid RPM Count".into(),
                first_valid_rpm_count,
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
        raw_plots
    }
}

impl Parseable for PidLog {
    const DESCRIPTIVE_NAME: &str = "Mbed PID log";

    /// Probes the buffer and check if it starts with [`super::UNIQUE_DESCRIPTION`] and therefor contains a valid [`PidLog`]
    fn is_buf_valid(content: &[u8]) -> bool {
        if content.len() < SIZEOF_UNIQ_DESC + 2 {
            return false;
        }

        let unique_description = &content[..SIZEOF_UNIQ_DESC];
        parse_unique_description(unique_description) == super::UNIQUE_DESCRIPTION
    }

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut total_bytes_read: usize = 0;
        let (header, bytes_read) = PidLogHeader::from_reader(reader)?;
        total_bytes_read += bytes_read;

        let (vec_of_entries, entry_bytes_read): (Vec<PidLogEntry>, usize) = match header.version() {
            ..=4 => {
                let (v1_vec, entry_bytes_read) = parse_to_vec::<PidLogEntryV1>(reader);
                (convert_v1_to_pid_log_entry(v1_vec), entry_bytes_read)
            }
            5 => {
                let (v2_vec, entry_bytes_read) = parse_to_vec::<PidLogEntryV2>(reader);
                (convert_v2_to_pid_log_entry(v2_vec), entry_bytes_read)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported header version: {}", header.version()),
                ))
            }
        };

        total_bytes_read += entry_bytes_read;

        let startup_timestamp = match &header {
            PidLogHeader::V1(h) => h.startup_timestamp(),
            PidLogHeader::V2(h) => h.startup_timestamp(),
            PidLogHeader::V3(h) => h.startup_timestamp(),
            PidLogHeader::V4(h) => h.startup_timestamp(),
            PidLogHeader::V5(h) => h.startup_timestamp(),
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .and_utc();
        let startup_timestamp_ns = startup_timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range")
            as f64;
        let timestamps_ns: Vec<f64> = vec_of_entries
            .iter()
            .map(|e| startup_timestamp_ns + e.timestamp_ns())
            .collect();

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
                timestamps_ns,
                all_plots_raw,
                startup_timestamp,
            },
            total_bytes_read,
        ))
    }
}

impl GitMetadata for PidLog {
    fn project_version(&self) -> Option<String> {
        match &self.header {
            PidLogHeader::V1(h) => h.project_version(),
            PidLogHeader::V2(h) => h.project_version(),
            PidLogHeader::V3(h) => h.project_version(),
            PidLogHeader::V4(h) => h.project_version(),
            PidLogHeader::V5(h) => h.project_version(),
        }
    }

    fn git_short_sha(&self) -> Option<String> {
        match &self.header {
            PidLogHeader::V1(h) => h.git_short_sha(),
            PidLogHeader::V2(h) => h.git_short_sha(),
            PidLogHeader::V3(h) => h.git_short_sha(),
            PidLogHeader::V4(h) => h.git_short_sha(),
            PidLogHeader::V5(h) => h.git_short_sha(),
        }
    }

    fn git_branch(&self) -> Option<String> {
        match &self.header {
            PidLogHeader::V1(h) => h.git_branch(),
            PidLogHeader::V2(h) => h.git_branch(),
            PidLogHeader::V3(h) => h.git_branch(),
            PidLogHeader::V4(h) => h.git_branch(),
            PidLogHeader::V5(h) => h.git_branch(),
        }
    }

    fn git_repo_status(&self) -> Option<String> {
        match &self.header {
            PidLogHeader::V1(h) => h.git_repo_status(),
            PidLogHeader::V2(h) => h.git_repo_status(),
            PidLogHeader::V3(h) => h.git_repo_status(),
            PidLogHeader::V4(h) => h.git_repo_status(),
            PidLogHeader::V5(h) => h.git_repo_status(),
        }
    }
}

impl Plotable for PidLog {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.all_plots_raw
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.startup_timestamp
    }

    fn descriptive_name(&self) -> &str {
        match self.header {
            PidLogHeader::V1(_) => "Mbed PID v1",
            PidLogHeader::V2(_) => "Mbed PID v2",
            PidLogHeader::V3(_) => "Mbed PID v3",
            PidLogHeader::V4(_) => "Mbed PID v4",
            PidLogHeader::V5(_) => "Mbed PID v5",
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
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
            PidLogHeader::V1(_) => (),
            // V2 also has config values
            PidLogHeader::V2(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
            PidLogHeader::V3(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
            PidLogHeader::V4(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
            PidLogHeader::V5(h) => {
                metadata.push(("Config values".to_owned(), String::new()));
                metadata.extend_from_slice(&h.mbed_config().field_value_pairs());
            }
        }

        Some(metadata)
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
    use super::*;
    use test_util::*;

    use crate::parse_and_display_log_entries;

    #[test]
    fn test_deserialize_v1() -> TestResult {
        let mut data = MBED_PID_V1_BYTES;
        let full_data_len = data.len();
        let (pidlog, bytes_read) = PidLog::from_reader(&mut data)?;

        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 873981);
        let first_entry = match pidlog.entries.first().expect("Empty entries") {
            PidLogEntry::V1(pid_log_entry_v1) => pid_log_entry_v1,
            PidLogEntry::V2(_) => panic!("Expected pid log entry v1"),
        };
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_output, 0.17777778);
        assert_eq!(first_entry.servo_duty_cycle, 0.04185);
        assert_eq!(first_entry.rpm_error_count, 0);
        assert_eq!(first_entry.first_valid_rpm_count, 0);

        let second_entry = match &pidlog.entries[1] {
            PidLogEntry::V1(pid_log_entry_v1) => pid_log_entry_v1,
            PidLogEntry::V2(_) => panic!("Expected pid log entry v1"),
        };
        assert_eq!(second_entry.rpm, 0.0);
        assert_eq!(second_entry.pid_output, 0.17777778);
        assert_eq!(second_entry.servo_duty_cycle, 0.04185);
        assert_eq!(second_entry.rpm_error_count, 0);
        assert_eq!(second_entry.first_valid_rpm_count, 0);
        //eprintln!("{pidlog}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display_v1() -> TestResult {
        let file = fs::File::open(mbed_pid_v1())?;
        let mut reader = io::BufReader::new(file);
        let (header, bytes_read) = PidLogHeader::from_reader(&mut reader)?;
        assert_eq!(bytes_read, 261);
        println!("{header}");
        parse_and_display_log_entries::<PidLogEntryV1>(&mut reader, Some(10));
        Ok(())
    }

    #[test]
    fn test_deserialize_v2() -> TestResult {
        let mut data = MBED_PID_V2_BYTES;
        let full_data_len = data.len();
        let (pidlog, bytes_read) = PidLog::from_reader(&mut data)?;
        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 722429);
        let first_entry = match pidlog.entries.first().expect("Empty entries") {
            PidLogEntry::V1(pid_log_entry_v1) => pid_log_entry_v1,
            PidLogEntry::V2(_) => panic!("Expected pid log entry v1"),
        };
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_output, 0.0);
        assert_eq!(first_entry.servo_duty_cycle, 0.03825);
        assert_eq!(first_entry.rpm_error_count, 0);
        assert_eq!(first_entry.first_valid_rpm_count, 0);

        let second_entry = match &pidlog.entries[1] {
            PidLogEntry::V1(pid_log_entry_v1) => pid_log_entry_v1,
            PidLogEntry::V2(_) => panic!("Expected pid log entry v1"),
        };
        assert_eq!(second_entry.rpm, 0.0);
        assert_eq!(second_entry.pid_output, 0.0);
        assert_eq!(second_entry.servo_duty_cycle, 0.03825);
        assert_eq!(second_entry.rpm_error_count, 0);
        assert_eq!(second_entry.first_valid_rpm_count, 0);
        //eprintln!("{pidlog}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display_v2() -> TestResult {
        let file = fs::File::open(mbed_pid_v2())?;
        let mut reader = io::BufReader::new(file);
        let (header, bytes_read) = PidLogHeader::from_reader(&mut reader)?;
        assert_eq!(bytes_read, 293);
        println!("{header}");
        parse_and_display_log_entries::<PidLogEntryV1>(&mut reader, Some(10));
        Ok(())
    }

    #[test]
    fn test_deserialize_v4() -> TestResult {
        let mut data = MBED_PID_V4_BYTES;
        let full_data_len = data.len();
        let (pidlog, bytes_read) = PidLog::from_reader(&mut data)?;
        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 834543);
        let first_entry = match pidlog.entries.first().expect("Empty entries") {
            PidLogEntry::V1(pid_log_entry_v1) => pid_log_entry_v1,
            PidLogEntry::V2(_) => panic!("Expected pid log entry v1"),
        };
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_output, 0.0);
        assert_eq!(first_entry.servo_duty_cycle, 0.0);
        assert_eq!(first_entry.rpm_error_count, 0);
        assert_eq!(first_entry.first_valid_rpm_count, 0);

        let second_entry = match &pidlog.entries[1] {
            PidLogEntry::V1(pid_log_entry_v1) => pid_log_entry_v1,
            PidLogEntry::V2(_) => panic!("Expected pid log entry v1"),
        };
        assert_eq!(second_entry.rpm, 0.0);
        assert_eq!(second_entry.pid_output, 0.0);
        assert_eq!(second_entry.servo_duty_cycle, 0.0);
        assert_eq!(second_entry.rpm_error_count, 0);
        assert_eq!(second_entry.first_valid_rpm_count, 0);
        //eprintln!("{pidlog}");
        Ok(())
    }

    #[test]
    fn test_parse_and_display_v4() -> TestResult {
        let file = fs::File::open(mbed_pid_v4())?;
        let mut reader = io::BufReader::new(file);
        let (header, bytes_read) = PidLogHeader::from_reader(&mut reader)?;
        assert_eq!(bytes_read, 327);
        println!("{header}");
        parse_and_display_log_entries::<PidLogEntryV1>(&mut reader, Some(10));
        Ok(())
    }

    #[test]
    fn test_deserialize_v5_regular() -> TestResult {
        let mut data = MBED_PID_V5_REGULAR_BYTES;
        let full_data_len = data.len();
        let (pidlog, bytes_read) = PidLog::from_reader(&mut data)?;
        assert!(bytes_read <= full_data_len);
        assert_eq!(bytes_read, 170996);
        let first_entry = match pidlog.entries.first().expect("Empty entries") {
            PidLogEntry::V1(_) => panic!("Expected pid log entry v2"),
            PidLogEntry::V2(pid_log_entry_v2) => pid_log_entry_v2,
        };
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_output, 0.0);
        assert_eq!(first_entry.servo_duty_cycle, 0.0);
        assert_eq!(first_entry.rpm_error_count, 0);
        assert_eq!(first_entry.first_valid_rpm_count, 0);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 0.);

        let last_entry = match &pidlog.entries.last().unwrap() {
            PidLogEntry::V1(_) => panic!("Expected pid log entry v2"),
            PidLogEntry::V2(e) => e,
        };
        assert_eq!(last_entry.rpm, 2499.4272);
        assert_eq!(last_entry.pid_output, 0.034495242);
        assert_eq!(last_entry.servo_duty_cycle, 0.04055);
        assert_eq!(last_entry.rpm_error_count, 7);
        assert_eq!(last_entry.first_valid_rpm_count, 2);
        assert!(!last_entry.fan_on);
        assert_eq!(last_entry.vbat, 0.);
        //eprintln!("{pidlog}");
        Ok(())
    }
}
