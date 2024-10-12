use crate::{
    mbed_motor_control::mbed_header::{
        MbedMotorControlLogHeader, UniqueDescriptionData, SIZEOF_UNIQ_DESC,
    },
    parse_unique_description,
};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::{fmt, io};
use v1::PidLogHeaderV1;
use v2::PidLogHeaderV2;

mod v1;
mod v2;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PidLogHeader {
    V1(PidLogHeaderV1),
    V2(PidLogHeaderV2),
}

impl fmt::Display for PidLogHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1(h) => write!(f, "{h}"),
            Self::V2(h) => write!(f, "{h}"),
        }
    }
}

impl PidLogHeader {
    pub(super) fn from_reader(reader: &mut impl io::Read) -> io::Result<Self> {
        let mut unique_description: UniqueDescriptionData = [0u8; SIZEOF_UNIQ_DESC];
        reader.read_exact(&mut unique_description)?;
        if parse_unique_description(&unique_description) != super::UNIQUE_DESCRIPTION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not an Mbed PidLog",
            ));
        }
        let version = reader.read_u16::<LittleEndian>()?;
        let header = match version {
            1 => Self::V1(PidLogHeaderV1::from_reader_with_uniq_descr_version(
                reader,
                unique_description,
                version,
            )?),
            2 => Self::V2(PidLogHeaderV2::from_reader_with_uniq_descr_version(
                reader,
                unique_description,
                version,
            )?),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unsupported version",
                ))
            }
        };
        Ok(header)
    }
}
