use crate::{parse_unique_description, util::timestamp_from_raw};

use byteorder::{LittleEndian, ReadBytesExt};
use chrono::{NaiveDateTime, ParseResult};
use log_if::prelude::*;

use std::{
    fmt::Display,
    io::{self, Read},
    mem::size_of,
};

use super::mbed_config::MbedConfig;

pub type UniqueDescriptionData = [u8; 128];
pub const SIZEOF_UNIQ_DESC: usize = size_of::<UniqueDescriptionData>();
pub type ProjectVersionData = [u8; 32];
pub const SIZEOF_PROJECT_VERSION: usize = size_of::<ProjectVersionData>();
pub type GitShortShaData = [u8; 8];
pub const SIZEOF_GIT_SHORT_SHA: usize = size_of::<GitShortShaData>();
pub type GitBranchData = [u8; 64];
pub const SIZEOF_GIT_BRANCH: usize = size_of::<GitBranchData>();
pub type GitRepoStatusData = [u8; 7];
pub const SIZEOF_GIT_REPO_STATUS: usize = size_of::<GitRepoStatusData>();
pub type StartupTimestamp = [u8; 20];
pub const SIZEOF_STARTUP_TIMESTAMP: usize = size_of::<StartupTimestamp>();

pub trait MbedMotorControlLogHeader: GitMetadata + Sized + Display + Send + Sync + Clone {
    /// Size of the header type in bytes if represented in raw binary
    const RAW_SIZE: usize;
    const VERSION: u16;

    /// Unique description is a field in the header that identifies the kind of log
    const UNIQUE_DESCRIPTION: &'static str;

    fn unique_description_bytes(&self) -> &UniqueDescriptionData;
    fn version(&self) -> u16;
    fn project_version_raw(&self) -> &ProjectVersionData;
    fn git_short_sha_raw(&self) -> &GitShortShaData;
    fn git_branch_raw(&self) -> &GitBranchData;
    fn git_repo_status_raw(&self) -> &GitRepoStatusData;
    fn startup_timestamp_raw(&self) -> &StartupTimestamp;
    fn startup_timestamp(&self) -> ParseResult<NaiveDateTime> {
        timestamp_from_raw(self.startup_timestamp_raw(), "%Y-%m-%dT%H:%M:%S")
    }

    fn unique_description(&self) -> String {
        parse_unique_description(self.unique_description_bytes())
    }

    /// Deserialize a header for a `reader` that implements [Read]
    fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self>;

    /// Deserialize a header with a reader starting just after the version field.
    fn from_reader_with_uniq_descr_version(
        reader: &mut impl io::Read,
        unique_description: UniqueDescriptionData,
        version: u16,
    ) -> io::Result<Self>;
}

/// Helper trait such that Pid and Status log v1 can reuse all this code
pub trait BuildMbedLogHeaderV1: Sized + MbedMotorControlLogHeader {
    /// Deserialize a header for a `reader` that implements [Read]
    fn build_from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut unique_description: UniqueDescriptionData = [0u8; SIZEOF_UNIQ_DESC];
        reader.read_exact(&mut unique_description)?;
        let version = reader.read_u16::<LittleEndian>()?;
        Self::build_from_reader_with_uniq_descr_version(reader, unique_description, version)
    }

    fn build_from_reader_with_uniq_descr_version(
        reader: &mut impl Read,
        unique_description: UniqueDescriptionData,
        version: u16,
    ) -> io::Result<Self> {
        let mut project_version: ProjectVersionData = ProjectVersionData::default();
        reader.read_exact(&mut project_version)?;
        let mut git_short_sha: GitShortShaData = GitShortShaData::default();
        reader.read_exact(&mut git_short_sha)?;
        let mut git_branch: GitBranchData = [0u8; SIZEOF_GIT_BRANCH];
        reader.read_exact(&mut git_branch)?;
        let mut git_repo_status: GitRepoStatusData = GitRepoStatusData::default();
        reader.read_exact(&mut git_repo_status)?;
        let mut startup_timestamp: StartupTimestamp = StartupTimestamp::default();
        reader.read_exact(&mut startup_timestamp)?;
        Ok(Self::new(
            unique_description,
            version,
            project_version,
            git_short_sha,
            git_branch,
            git_repo_status,
            startup_timestamp,
        ))
    }

    fn new(
        unique_description: UniqueDescriptionData,
        version: u16,
        project_version: ProjectVersionData,
        git_short_sha: GitShortShaData,
        git_branch: GitBranchData,
        git_repo_status: GitRepoStatusData,
        startup_timestamp: StartupTimestamp,
    ) -> Self;
}

/// Helper trait such that Pid and Status log v2 can reuse all this code
pub trait BuildMbedLogHeaderV2: Sized + MbedMotorControlLogHeader {
    /// Deserialize a header for a `reader` that implements [Read]
    fn build_from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut unique_description: UniqueDescriptionData = [0u8; SIZEOF_UNIQ_DESC];
        reader.read_exact(&mut unique_description)?;
        let version = reader.read_u16::<LittleEndian>()?;
        Self::build_from_reader_with_uniq_descr_version(reader, unique_description, version)
    }

    fn build_from_reader_with_uniq_descr_version(
        reader: &mut impl Read,
        unique_description: UniqueDescriptionData,
        version: u16,
    ) -> io::Result<Self> {
        let mut project_version: ProjectVersionData = ProjectVersionData::default();
        reader.read_exact(&mut project_version)?;
        let mut git_short_sha: GitShortShaData = GitShortShaData::default();
        reader.read_exact(&mut git_short_sha)?;
        let mut git_branch: GitBranchData = [0u8; SIZEOF_GIT_BRANCH];
        reader.read_exact(&mut git_branch)?;
        let mut git_repo_status: GitRepoStatusData = GitRepoStatusData::default();
        reader.read_exact(&mut git_repo_status)?;
        let mut startup_timestamp: StartupTimestamp = StartupTimestamp::default();
        reader.read_exact(&mut startup_timestamp)?;
        let mbed_config = MbedConfig::from_reader(reader)?;
        Ok(Self::new(
            unique_description,
            version,
            project_version,
            git_short_sha,
            git_branch,
            git_repo_status,
            startup_timestamp,
            mbed_config,
        ))
    }

    // Not much to do about this lint other than wrap some arguments in another struct but it is not worth the effort, this is a simple constructor
    #[allow(clippy::too_many_arguments)]
    fn new(
        unique_description: UniqueDescriptionData,
        version: u16,
        project_version: ProjectVersionData,
        git_short_sha: GitShortShaData,
        git_branch: GitBranchData,
        git_repo_status: GitRepoStatusData,
        startup_timestamp: StartupTimestamp,
        mbed_config: MbedConfig,
    ) -> Self;
}
