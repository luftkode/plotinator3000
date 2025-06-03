use std::{fmt, io};

use byteorder::LittleEndian;
use byteorder::ReadBytesExt as _;
use serde::{Deserialize, Serialize};
use v1::StatusLogHeaderV1;
use v2::StatusLogHeaderV2;
use v3::StatusLogHeaderV3;
use v4::StatusLogHeaderV4;
use v6::StatusLogHeaderV6;

use crate::mbed_motor_control::mbed_header::MbedMotorControlLogHeader as _;
use crate::{
    mbed_motor_control::mbed_header::{SIZEOF_UNIQ_DESC, UniqueDescriptionData},
    parse_unique_description,
};

mod v1;
mod v2;
mod v3;
mod v4;
mod v6;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum StatusLogHeader {
    V1(StatusLogHeaderV1),
    V2(StatusLogHeaderV2),
    V3(StatusLogHeaderV3),
    V5(StatusLogHeaderV4),
    V6(StatusLogHeaderV6),
}

impl fmt::Display for StatusLogHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1(h) => write!(f, "{h}"),
            Self::V2(h) => write!(f, "{h}"),
            Self::V3(h) => write!(f, "{h}"),
            Self::V5(h) => write!(f, "{h}"),
            Self::V6(h) => write!(f, "{h}"),
        }
    }
}

impl StatusLogHeader {
    pub(super) fn version(&self) -> usize {
        match self {
            Self::V1(_) => 1,
            Self::V2(_) => 2,
            Self::V3(_) => 3,
            Self::V5(_) => 5,
            Self::V6(_) => 6,
        }
    }

    pub(super) fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut total_bytes_read = 0;

        // Read unique description
        let mut unique_description: UniqueDescriptionData = [0u8; SIZEOF_UNIQ_DESC];
        reader.read_exact(&mut unique_description)?;
        total_bytes_read += SIZEOF_UNIQ_DESC;

        // Validate unique description
        if parse_unique_description(&unique_description) != super::UNIQUE_DESCRIPTION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not an Mbed PidLog",
            ));
        }

        // Read version
        let version = reader.read_u16::<LittleEndian>()?;
        total_bytes_read += std::mem::size_of::<u16>();

        // Match the version and read the appropriate header
        let header = match version {
            1 => {
                let (header, bytes_read) = StatusLogHeaderV1::from_reader_with_uniq_descr_version(
                    reader,
                    unique_description,
                    version,
                )?;
                total_bytes_read += bytes_read;
                Self::V1(header)
            }
            2 => {
                let (header, bytes_read) = StatusLogHeaderV2::from_reader_with_uniq_descr_version(
                    reader,
                    unique_description,
                    version,
                )?;
                total_bytes_read += bytes_read;
                Self::V2(header)
            }
            3 => {
                let (header, bytes_read) = StatusLogHeaderV3::from_reader_with_uniq_descr_version(
                    reader,
                    unique_description,
                    version,
                )?;
                total_bytes_read += bytes_read;
                Self::V3(header)
            }

            4 => panic!(
                "Unsupported version. This log version is a pre-release version for MBED v4. The log version for the MBED v4 is 5."
            ),
            5 => {
                let (header, bytes_read) = StatusLogHeaderV4::from_reader_with_uniq_descr_version(
                    reader,
                    unique_description,
                    version,
                )?;
                total_bytes_read += bytes_read;
                Self::V5(header)
            }
            6 => {
                let (header, bytes_read) = StatusLogHeaderV6::from_reader_with_uniq_descr_version(
                    reader,
                    unique_description,
                    version,
                )?;
                total_bytes_read += bytes_read;
                Self::V6(header)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unsupported version: {version}"),
                ));
            }
        };

        // Return the header and the total number of bytes read
        Ok((header, total_bytes_read))
    }
}
