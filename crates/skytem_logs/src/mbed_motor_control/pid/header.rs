use std::fmt;

use crate::mbed_motor_control::StartupTimestamp;

use super::super::{
    GitBranchData, GitMetadata, GitRepoStatusData, GitShortShaData, MbedMotorControlLogHeader,
    ProjectVersionData, UniqueDescriptionData,
};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
pub struct PidLogHeader {
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

impl GitMetadata for PidLogHeader {
    fn project_version(&self) -> Option<String> {
        Some(
            String::from_utf8_lossy(self.project_version_raw())
                .trim_end_matches(char::from(0))
                .to_owned(),
        )
    }
    fn git_branch(&self) -> Option<String> {
        let git_branch_info = String::from_utf8_lossy(self.git_branch_raw())
            .trim_end_matches(char::from(0))
            .to_owned();
        if git_branch_info.is_empty() {
            None
        } else {
            Some(git_branch_info)
        }
    }

    fn git_repo_status(&self) -> Option<String> {
        let repo_status = String::from_utf8_lossy(self.git_repo_status_raw())
            .trim_end_matches(char::from(0))
            .to_owned();
        if repo_status.is_empty() {
            None
        } else {
            Some(repo_status)
        }
    }

    fn git_short_sha(&self) -> Option<String> {
        let short_sha = String::from_utf8_lossy(self.git_short_sha_raw())
            .trim_end_matches(char::from(0))
            .to_owned();
        if short_sha.is_empty() {
            None
        } else {
            Some(short_sha)
        }
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

impl fmt::Display for PidLogHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}-v{}", self.unique_description(), self.version)?;
        writeln!(
            f,
            "Project Version: {}",
            self.project_version()
                .unwrap_or_else(|| "<Missing>".to_owned())
        )?;
        if let Some(git_branch) = self.git_branch() {
            writeln!(f, "Branch: {git_branch}")?;
        }
        if let Some(git_short_sha) = self.git_short_sha() {
            writeln!(f, "SHA: {git_short_sha}")?;
        }
        if self.git_repo_status().is_some() {
            writeln!(f, "Repo status: dirty")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    const TEST_DATA: &str =
        "../../test_data/mbed_motor_control/20240926_121708/pid_20240926_121708_00.bin";

    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let pid_log_header = PidLogHeader::from_reader(&mut data.as_slice())?;
        eprintln!("{pid_log_header}");
        assert_eq!(
            pid_log_header.unique_description(),
            PidLogHeader::UNIQUE_DESCRIPTION
        );
        assert_eq!(pid_log_header.version, 1);
        assert_eq!(pid_log_header.project_version().unwrap(), "1.3.0");
        assert_eq!(
            pid_log_header.git_branch().unwrap(),
            "fix-23-pid-loop-in-standby"
        );
        assert_eq!(pid_log_header.git_short_sha().unwrap(), "fe6e412");
        Ok(())
    }
}
