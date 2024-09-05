use clap::Parser;
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

/// Simple program to print the filename from a given file path
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File path to extract the filename from
    file_path: String,
}

fn parse_timestamp(timestamp: u32) -> String {
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

fn read_u32(bytes: &mut &[u8]) -> io::Result<u32> {
    let mut buf: [u8; 4] = [0; 4];
    bytes.read_exact(&mut buf[..4])?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32(bytes: &mut &[u8]) -> io::Result<f32> {
    let mut buf: [u8; 4] = [0; 4];
    bytes.read_exact(&mut buf[..4])?;
    Ok(f32::from_le_bytes(buf))
}

#[derive(Debug)]
struct LogHeader {
    version: u16,
}

#[derive(Debug)]
struct StatusLogEntry {
    timestamp_ms: String,
    engine_temp: f32,
    fan_on: bool,
    vbat: f32,
    setpoint: f32,
    motor_state: u8,
}

impl StatusLogEntry {
    pub fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let timestamp_ms_raw = read_u32(&mut &bytes[..])?;
        let timestamp_ms = parse_timestamp(timestamp_ms_raw);
        let engine_temp = read_f32(&mut &bytes[4..])?;
        let fan_on: bool = bytes[8] == 0;
        let vbat = read_f32(&mut &bytes[9..])?;
        let setpoint = read_f32(&mut &bytes[13..])?;
        let motor_state = bytes[17];
        Ok(Self {
            timestamp_ms,
            engine_temp,
            fan_on,
            vbat,
            setpoint,
            motor_state,
        })
    }
}
impl std::fmt::Display for StatusLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {} {} {}",
            self.timestamp_ms,
            self.engine_temp,
            self.fan_on,
            self.vbat,
            self.setpoint,
            self.motor_state
        )
    }
}

#[derive(Debug)]
struct PidLogEntry {
    timestamp_ms: String,
    rpm: f32,
    pid_err: f32,
    servo_duty_cycle: f32,
}

impl PidLogEntry {
    pub fn from_buf(bytes: &mut &[u8]) -> io::Result<PidLogEntry> {
        let timestamp_ms_raw = read_u32(&mut &bytes[..])?;
        let timestamp_ms = parse_timestamp(timestamp_ms_raw);
        let rpm = read_f32(&mut &bytes[4..])?;
        let pid_err = read_f32(&mut &bytes[8..])?;
        let servo_duty_cycle = read_f32(&mut &bytes[12..])?;

        Ok(Self {
            timestamp_ms,
            rpm,
            pid_err,
            servo_duty_cycle,
        })
    }
}

impl std::fmt::Display for PidLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {}",
            self.timestamp_ms, self.rpm, self.pid_err, self.servo_duty_cycle
        )
    }
}

fn main() -> io::Result<()> {
    // let args = Args::parse();
    // let path = Path::new(&args.file_path);
    let buf = std::fs::read("status_20240905_133820_00.bin")?;
    println!("File contents len: {}", buf.len());

    let mut pos = 0;

    let header = LogHeader {
        version: u16::from_le_bytes([buf[0], buf[1]]),
    };
    pos += 2;

    println!("{header:?}");

    loop {
        match StatusLogEntry::from_buf(&mut &buf[pos..]) {
            Ok(e) => println!("{e}"),
            Err(_) => {
                eprintln!("End of buffer at {pos}");
                break;
            }
        }
        pos += 18;
    }

    Ok(())
}
