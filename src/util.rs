use std::{
    io::{self, Read},
    time::Duration,
};

pub fn parse_timestamp(timestamp: u32) -> String {
    let duration = Duration::from_millis(timestamp as u64);
    let hours = (duration.as_secs() % 86400) / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;
    let seconds = duration.as_secs() % 60;
    let milliseconds = duration.subsec_millis();

    format!(
        "{:02}:{:02}:{:02}.{:03}",
        hours, minutes, seconds, milliseconds
    )
}

pub fn read_u32(bytes: &mut &[u8]) -> io::Result<u32> {
    let mut buf = [0; 4];
    bytes.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub fn read_f32(bytes: &mut &[u8]) -> io::Result<f32> {
    let mut buf = [0; 4];
    bytes.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}
