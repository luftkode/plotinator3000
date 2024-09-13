use crate::logs::{parse_to_vec, GitMetadata, Log, LogEntry};
use crate::util::parse_timestamp;
use byteorder::{LittleEndian, ReadBytesExt};
use serde_big_array::BigArray;
use std::{fmt, io};
use strum_macros::{Display, FromRepr};

use super::{
    GitBranchData, GitRepoStatusData, GitShortShaData, MbedMotorControlLogHeader,
    ProjectVersionData, UniqueDescriptionData,
};

#[allow(non_camel_case_types)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, FromRepr, Display,
)]
pub enum MotorState {
    POWER_HOLD = 0,
    ECU_ON_WAIT_PUMP,
    ECU_ON_WAIT_PRESS_START,
    DO_IGNITION,
    IGNITION_END,
    WAIT_FOR_T_STANDBY,
    STANDBY_WAIT_FOR_CAP,
    STANDBY_WAIT_FOR_T_RUN,
    STANDBY_READY,
    RUNNING,
    WAIT_TIME_SHUTDOWN,
    INVALID_STATE,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLog {
    header: StatusLogHeader,
    entries: Vec<StatusLogEntry>,
    timestamps_with_state_changes: Vec<(u32, MotorState)>, // for memoization
}

impl Log for StatusLog {
    type Entry = StatusLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = StatusLogHeader::from_reader(reader)?;
        let vec_of_entries: Vec<StatusLogEntry> = parse_to_vec(reader);
        let timestamps_with_state_changes = parse_timestamps_with_state_changes(&vec_of_entries);
        Ok(Self {
            header,
            entries: vec_of_entries,
            timestamps_with_state_changes,
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

    for entry in entries.iter() {
        // Check if the current state is different from the last recorded state
        if last_state != Some(entry.motor_state) {
            result.push((entry.timestamp_ms, entry.motor_state));
            last_state = Some(entry.motor_state);
        }
    }
    result
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogHeader {
    #[serde(with = "BigArray")]
    unique_description: UniqueDescriptionData,
    version: u16,
    project_version: ProjectVersionData,
    git_short_sha: GitShortShaData,
    #[serde(with = "BigArray")]
    git_branch: GitBranchData,
    git_repo_status: GitRepoStatusData,
}

impl GitMetadata for StatusLogHeader {
    fn project_version(&self) -> String {
        String::from_utf8_lossy(self.project_version_raw())
            .trim_end_matches(char::from(0))
            .to_owned()
    }

    fn git_short_sha(&self) -> String {
        String::from_utf8_lossy(self.git_short_sha_raw())
            .trim_end_matches(char::from(0))
            .to_owned()
    }

    fn git_branch(&self) -> String {
        String::from_utf8_lossy(self.git_branch_raw())
            .trim_end_matches(char::from(0))
            .to_owned()
    }

    fn git_repo_status(&self) -> String {
        String::from_utf8_lossy(self.git_repo_status_raw())
            .trim_end_matches(char::from(0))
            .to_owned()
    }
}

impl MbedMotorControlLogHeader for StatusLogHeader {
    const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-STATUS-LOG-2024";

    fn unique_description_bytes(&self) -> &[u8; 128] {
        &self.unique_description
    }

    fn version(&self) -> u16 {
        self.version
    }

    fn new(
        unique_description: UniqueDescriptionData,
        version: u16,
        project_version: ProjectVersionData,
        git_short_sha: GitShortShaData,
        git_branch: GitBranchData,
        git_repo_status: GitRepoStatusData,
    ) -> Self {
        Self {
            unique_description,
            version,
            project_version,
            git_short_sha,
            git_branch,
            git_repo_status,
        }
    }

    fn project_version_raw(&self) -> &ProjectVersionData {
        &self.project_version
    }

    fn git_short_sha_raw(&self) -> &GitShortShaData {
        &self.git_short_sha
    }

    fn git_branch_raw(&self) -> &GitBranchData {
        &self.git_branch
    }

    fn git_repo_status_raw(&self) -> &GitRepoStatusData {
        &self.git_repo_status
    }
}

impl fmt::Display for StatusLogHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}-v{}", self.unique_description(), self.version)?;
        writeln!(f, "Project Version: {}", self.project_version())?;
        let git_branch = self.git_branch();
        if !git_branch.is_empty() {
            writeln!(f, "Branch: {}", self.git_branch())?;
        }
        let git_short_sha = self.git_short_sha();
        if !git_short_sha.is_empty() {
            writeln!(f, "SHA: {}", git_short_sha)?;
        }
        let is_dirty = self.git_repo_status();
        if !is_dirty.is_empty() {
            writeln!(f, "Repo status: dirty")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub engine_temp: f32,
    pub fan_on: bool,
    pub vbat: f32,
    pub setpoint: f32,
    pub motor_state: MotorState,
}

impl StatusLogEntry {
    pub fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
}

impl std::fmt::Display for StatusLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {} {} {}",
            self.timestamp_ms,
            self.engine_temp,
            self.fan_on,
            self.vbat,
            self.setpoint,
            self.motor_state
        )
    }
}

impl LogEntry for StatusLogEntry {
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let engine_temp = reader.read_f32::<LittleEndian>()?;
        let fan_on = reader.read_u8()? == 1;
        let vbat = reader.read_f32::<LittleEndian>()?;
        let setpoint = reader.read_f32::<LittleEndian>()?;
        let motor_state = match MotorState::from_repr(reader.read_u8()?.into()) {
            Some(st) => st,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid motor state",
                ))
            }
        };
        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            engine_temp,
            fan_on,
            vbat,
            setpoint,
            motor_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use testresult::TestResult;

    const TEST_DATA: &str =
        "test_data/mbed_motor_control/old_rpm_algo/status_20240912_122203_00.bin";

    use crate::logs::parse_and_display_log_entries;

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let status_log = StatusLog::from_reader(&mut data.as_slice())?;
        eprintln!("{}", status_log.header);
        assert_eq!(
            status_log.header.unique_description(),
            StatusLogHeader::UNIQUE_DESCRIPTION
        );
        assert_eq!(status_log.header.version, 0);
        assert_eq!(status_log.header.project_version(), "1.0.0");
        assert_eq!(status_log.header.git_branch(), "fix-release-workflow");
        assert_eq!(status_log.header.git_short_sha(), "56fc61b");

        let first_entry = status_log.entries.first().unwrap();
        assert_eq!(first_entry.engine_temp, 4.770642);
        assert!(!first_entry.fan_on);
        assert_eq!(first_entry.vbat, 4.211966);
        assert_eq!(first_entry.setpoint, 2500.0);
        assert_eq!(first_entry.motor_state, MotorState::POWER_HOLD);
        let second_entry = status_log.entries.get(1).unwrap();
        assert_eq!(second_entry.engine_temp, 4.770642);
        assert!(!second_entry.fan_on);
        assert_eq!(second_entry.vbat, 4.219487);
        assert_eq!(second_entry.setpoint, 2500.0);
        assert_eq!(second_entry.motor_state, MotorState::POWER_HOLD);

        let last_entry = status_log.entries().last().unwrap();
        assert_eq!(last_entry.timestamp_ms(), 17492);
        assert_eq!(last_entry.engine_temp, 4.770642);
        assert!(!last_entry.fan_on);
        assert_eq!(last_entry.vbat, 4.219487);
        assert_eq!(last_entry.setpoint, 0.0);
        assert_eq!(last_entry.motor_state, MotorState::WAIT_TIME_SHUTDOWN);
        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_motor_state_deserialize() -> TestResult {
        assert_eq!(MotorState::DO_IGNITION, MotorState::from_repr(3).unwrap());
        assert_eq!(
            MotorState::WAIT_TIME_SHUTDOWN,
            MotorState::from_repr(10).unwrap()
        );
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
