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
    const RAW_SIZE: usize = 130;
    const UNIQUE_DESCRIPTION: &'static str;

    fn unique_description_bytes(&self) -> &[u8; 128];
    fn version(&self) -> u16;

    fn unique_description(&self) -> String {
        parse_unique_description(*self.unique_description_bytes())
    }

    fn is_valid_header(&self) -> bool {
        self.unique_description() == Self::UNIQUE_DESCRIPTION
    }

    fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut unique_description = [0u8; 128];
        reader.read_exact(&mut unique_description)?;
        let version = reader.read_u16::<LittleEndian>()?;
        Ok(Self::new(unique_description, version))
    }

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

        let version =
            u16::from_le_bytes(slice[128..130].try_into().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Failed to read version")
            })?);

        Ok(Self::new(unique_description, version))
    }

    fn is_buf_header(bytes: &[u8]) -> io::Result<bool> {
        let deserialized = Self::from_slice(bytes)?;
        Ok(deserialized.is_valid_header())
    }

    fn reader_starts_with_header<R: Read>(reader: &mut R) -> io::Result<bool> {
        let deserialized = Self::from_reader(reader)?;
        Ok(deserialized.is_valid_header())
    }

    fn file_starts_with_header(fpath: &Path) -> io::Result<bool> {
        let mut file = fs::File::open(fpath).unwrap();
        Self::reader_starts_with_header(&mut file)
    }

    fn new(unique_description: [u8; 128], version: u16) -> Self;
}
