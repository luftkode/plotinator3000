use std::{fmt::Display, io};

pub mod pid;
pub mod status;

pub fn parse_unique_description(raw_uniq_desc: [u8; 128]) -> String {
    String::from_utf8_lossy(&raw_uniq_desc)
        .trim_end_matches(char::from(0))
        .to_owned()
}

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
    fn timestamp_ms(&self) -> u32;
}
