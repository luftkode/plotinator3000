use crate::logs::{parse_to_vec, Log, LogEntry};
use crate::util::parse_timestamp;
use byteorder::{LittleEndian, ReadBytesExt};
use serde_big_array::BigArray;
use std::{fmt, io};

use super::{
    GitBranchData, GitMetadata, GitRepoStatusData, GitShortShaData, MbedMotorControlLogHeader,
    ProjectVersionData, UniqueDescriptionData,
};

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

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogHeader {
    #[serde(with = "BigArray")]
    unique_description: UniqueDescriptionData,
    version: u16,
    project_version: ProjectVersionData,
    git_short_sha: GitShortShaData,
    #[serde(with = "BigArray")]
    git_branch: GitBranchData,
    git_repo_status: GitRepoStatusData,
}

impl GitMetadata for PidLogHeader {
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

    fn git_short_sha(&self) -> String {
        String::from_utf8_lossy(self.git_short_sha_raw())
            .trim_end_matches(char::from(0))
            .to_owned()
    }
}

impl MbedMotorControlLogHeader for PidLogHeader {
    const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-PID-LOG-2024";

    fn unique_description_bytes(&self) -> &UniqueDescriptionData {
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

impl PidLogEntry {
    pub fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
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

    const TEST_DATA: &str = "test_data/mbed_motor_control/old_rpm_algo/pid_20240912_122203_00.bin";

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
        let header = PidLogHeader::from_reader(&mut reader)?;
        println!("{header}");
        parse_and_display_log_entries::<PidLogEntry, _>(&mut reader, Some(10));
        Ok(())
    }
}
