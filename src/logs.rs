use std::{fmt::Display, io, mem};

pub mod pid;
pub mod status;

pub fn parse_to_vec<T: LogEntry>(bytes: &mut &[u8]) -> Vec<T> {
    let mut pos = 0;
    let mut v = Vec::new();
    while pos < bytes.len() {
        match T::from_buf(&mut &bytes[pos..]) {
            Ok(e) => v.push(e),
            Err(_) => {
                eprintln!("End of buffer at {pos}");
                break;
            }
        }
        pos += T::packed_footprint();
    }
    v
}

pub fn parse_and_display<T: LogEntry>(bytes: &mut &[u8]) {
    let mut pos = 0;
    while pos < bytes.len() {
        match T::from_buf(&mut &bytes[pos..]) {
            Ok(e) => println!("{e}"),
            Err(_) => {
                eprintln!("End of buffer at {pos}");
                break;
            }
        }
        pos += T::packed_footprint();
    }
}

pub trait LogEntry: Sized + Display {
    fn from_buf(bytes: &mut &[u8]) -> io::Result<Self>;
    fn packed_footprint() -> usize;
}

#[derive(Debug)]
pub struct LogHeader {
    version: u16,
}

impl LogEntry for LogHeader {
    fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let version = u16::from_le_bytes([bytes[0], bytes[1]]);
        Ok(Self { version })
    }

    fn packed_footprint() -> usize {
        mem::size_of::<u16>()
    }
}

impl std::fmt::Display for LogHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}
