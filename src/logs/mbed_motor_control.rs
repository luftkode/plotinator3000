use super::parse_unique_description;
use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

pub mod pid;
pub mod status;

pub trait MbedMotorControlLogHeader: Sized {
    /// Size of the header type in bytes if represented in raw binary
    const RAW_SIZE: usize = 130;
    /// Unique description is a field in the header that identifies the kind of log
    const UNIQUE_DESCRIPTION: &'static str;

    fn unique_description_bytes(&self) -> &[u8; 128];
    fn version(&self) -> u16;

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
        let mut unique_description = [0u8; 128];
        reader.read_exact(&mut unique_description)?;
        let version = reader.read_u16::<LittleEndian>()?;
        Ok(Self::new(unique_description, version))
    }

    /// Deserialize a header from a byte slice
    fn from_slice(slice: &[u8]) -> io::Result<Self> {
        if slice.len() < Self::RAW_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Slice too short",
            ));
        }

        let unique_description: [u8; 128] = slice[..128].try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to read unique description",
            )
        })?;

        let version = u16::from_le_bytes(slice[128..130].try_into().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read version: {e}"),
            )
        })?);

        Ok(Self::new(unique_description, version))
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

    fn new(unique_description: [u8; 128], version: u16) -> Self;
}
