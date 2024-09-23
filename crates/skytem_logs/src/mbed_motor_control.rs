use super::parse_unique_description;
use byteorder::{LittleEndian, ReadBytesExt};
use log_if::GitMetadata;
use std::{
    fs,
    io::{self, Read},
    mem::size_of,
    path::Path,
};

pub mod pid;
pub mod status;

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

pub trait MbedMotorControlLogHeader: GitMetadata + Sized {
    /// Size of the header type in bytes if represented in raw binary
    const RAW_SIZE: usize = 130;
    /// Unique description is a field in the header that identifies the kind of log
    const UNIQUE_DESCRIPTION: &'static str;

    fn unique_description_bytes(&self) -> &UniqueDescriptionData;
    fn version(&self) -> u16;
    fn project_version_raw(&self) -> &ProjectVersionData;
    fn git_short_sha_raw(&self) -> &GitShortShaData;
    fn git_branch_raw(&self) -> &GitBranchData;
    fn git_repo_status_raw(&self) -> &GitRepoStatusData;

    fn unique_description(&self) -> String {
        parse_unique_description(*self.unique_description_bytes())
    }

    /// Returns whether or not a header is valid, meaning its unique description field matches the type
    ///
    /// After deserializing arbitrary bytes this method can be used to check
    /// whether or not the result matches a header of the deserialized type
    fn is_valid_header(&self) -> bool {
        self.unique_description() == Self::UNIQUE_DESCRIPTION
    }

    /// Deserialize a header for a `reader` that implements [Read]
    fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut unique_description: UniqueDescriptionData = [0u8; 128];
        reader.read_exact(&mut unique_description)?;
        let version = reader.read_u16::<LittleEndian>()?;
        let mut project_version: ProjectVersionData = [0u8; 32];
        reader.read_exact(&mut project_version)?;
        let mut git_short_sha: GitShortShaData = [0u8; 8];
        reader.read_exact(&mut git_short_sha)?;
        let mut git_branch: GitBranchData = [0u8; 64];
        reader.read_exact(&mut git_branch)?;
        let mut git_repo_status: GitRepoStatusData = [0u8; 7];
        reader.read_exact(&mut git_repo_status)?;
        Ok(Self::new(
            unique_description,
            version,
            project_version,
            git_short_sha,
            git_branch,
            git_repo_status,
        ))
    }

    /// Deserialize a header from a byte slice
    fn from_slice(slice: &[u8]) -> io::Result<Self> {
        if slice.len() < Self::RAW_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Slice too short",
            ));
        }
        let mut pos = 0;
        let unique_description: UniqueDescriptionData =
            slice[..SIZEOF_UNIQ_DESC].try_into().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to read unique description: {e}"),
                )
            })?;
        pos += SIZEOF_UNIQ_DESC;
        let size_of_version = size_of::<u16>();
        let version =
            u16::from_le_bytes(slice[pos..pos + size_of_version].try_into().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to read version: {e}"),
                )
            })?);
        pos += size_of_version;
        let project_version: ProjectVersionData = slice[pos..pos + SIZEOF_PROJECT_VERSION]
            .try_into()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to read project version: {e}"),
                )
            })?;
        pos += SIZEOF_PROJECT_VERSION;
        let git_short_sha: GitShortShaData = slice[pos..pos + SIZEOF_GIT_SHORT_SHA]
            .try_into()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to read git short SHA: {e}"),
                )
            })?;
        pos += SIZEOF_GIT_SHORT_SHA;
        let git_branch: GitBranchData =
            slice[pos..pos + SIZEOF_GIT_BRANCH]
                .try_into()
                .map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Failed to read Git Branch: {e}"),
                    )
                })?;
        pos += SIZEOF_GIT_BRANCH;
        let git_repo_status = slice[pos..pos + SIZEOF_GIT_REPO_STATUS]
            .try_into()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to read Git Repo Status: {e}"),
                )
            })?;

        Ok(Self::new(
            unique_description,
            version,
            project_version,
            git_short_sha,
            git_branch,
            git_repo_status,
        ))
    }

    /// Attempts to deserialize a header from `bytes` and returns whether or not a valid header was deserialized
    ///
    /// Useful for probing bytes for whether they match a given log type
    fn is_buf_header(bytes: &[u8]) -> io::Result<bool> {
        let deserialized = Self::from_slice(bytes)?;
        Ok(deserialized.is_valid_header())
    }

    /// Attempts to deserialize a header from `reader` and returns whether or not a valid header was deserialized
    ///
    /// Useful for probing bytes for whether they match a given log type
    fn reader_starts_with_header<R: Read>(reader: &mut R) -> io::Result<bool> {
        let deserialized = Self::from_reader(reader)?;
        Ok(deserialized.is_valid_header())
    }

    /// Attempts to deserialize a header from the file at `fpath`
    /// and returns whether or not a valid header was deserialized
    ///
    /// Useful for probing a file for whether it matches a given log type
    fn file_starts_with_header(fpath: &Path) -> io::Result<bool> {
        let mut file = fs::File::open(fpath)?;
        Self::reader_starts_with_header(&mut file)
    }

    fn new(
        unique_description: UniqueDescriptionData,
        version: u16,
        project_version: ProjectVersionData,
        git_short_sha: GitShortShaData,
        git_branch: GitBranchData,
        git_repo_status: GitRepoStatusData,
    ) -> Self;
}
