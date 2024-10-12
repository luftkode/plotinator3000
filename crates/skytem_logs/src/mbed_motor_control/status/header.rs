use std::{fmt, io};

use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use v1::StatusLogHeaderV1;
use v2::StatusLogHeaderV2;

use crate::{
    mbed_motor_control::mbed_header::{
        MbedMotorControlLogHeader, UniqueDescriptionData, SIZEOF_UNIQ_DESC,
    },
    parse_unique_description,
};

mod v1;
mod v2;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum StatusLogHeader {
    V1(StatusLogHeaderV1),
    V2(StatusLogHeaderV2),
}

impl fmt::Display for StatusLogHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1(h) => write!(f, "{h}"),
            Self::V2(h) => write!(f, "{h}"),
        }
    }
}

impl StatusLogHeader {
    const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-STATUS-LOG-2024";

    pub(super) fn from_reader(reader: &mut impl io::Read) -> io::Result<Self> {
        let mut unique_description: UniqueDescriptionData = [0u8; SIZEOF_UNIQ_DESC];
        reader.read_exact(&mut unique_description)?;
        if parse_unique_description(&unique_description) != Self::UNIQUE_DESCRIPTION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not an Mbed StatusLog",
            ));
        }
        let version = reader.read_u16::<LittleEndian>()?;
        let header = match version {
            1 => Self::V1(StatusLogHeaderV1::from_reader_with_uniq_descr_version(
                reader,
                unique_description,
                version,
            )?),
            2 => Self::V2(StatusLogHeaderV2::from_reader_with_uniq_descr_version(
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
