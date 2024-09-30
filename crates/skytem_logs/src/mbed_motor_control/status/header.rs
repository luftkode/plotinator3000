use crate::mbed_motor_control::StartupTimestamp;

use super::super::{
    GitBranchData, GitRepoStatusData, GitShortShaData, MbedMotorControlLogHeader,
    ProjectVersionData, UniqueDescriptionData,
};
use log_if::prelude::*;
use serde_big_array::BigArray;
use std::fmt;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Clone, Copy)]
pub struct StatusLogHeader {
    #[serde(with = "BigArray")]
    unique_description: UniqueDescriptionData,
    version: u16,
    project_version: ProjectVersionData,
    git_short_sha: GitShortShaData,
    #[serde(with = "BigArray")]
    git_branch: GitBranchData,
    git_repo_status: GitRepoStatusData,
    startup_timestamp: StartupTimestamp,
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
        startup_timestamp: StartupTimestamp,
    ) -> Self {
        Self {
            unique_description,
            version,
            project_version,
            git_short_sha,
            git_branch,
            git_repo_status,
            startup_timestamp,
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

    fn startup_timestamp_raw(&self) -> &StartupTimestamp {
        &self.startup_timestamp
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
            writeln!(f, "SHA: {git_short_sha}")?;
        }
        let is_dirty = self.git_repo_status();
        if !is_dirty.is_empty() {
            writeln!(f, "Repo status: dirty")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self};
    use testresult::TestResult;

    const TEST_DATA: &str =
        "../../test_data/mbed_motor_control/20240926_121708/status_20240926_121708_00.bin";

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let status_log_header = StatusLogHeader::from_reader(&mut data.as_slice())?;
        eprintln!("{status_log_header}");
        assert_eq!(
            status_log_header.unique_description(),
            StatusLogHeader::UNIQUE_DESCRIPTION
        );
        assert_eq!(status_log_header.version, 1);
        assert_eq!(status_log_header.project_version(), "1.3.0");
        assert_eq!(status_log_header.git_branch(), "fix-23-pid-loop-in-standby");
        assert_eq!(status_log_header.git_short_sha(), "fe6e412");
        Ok(())
    }
}
