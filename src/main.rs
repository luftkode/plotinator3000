use std::io::{self};

use std::path::Path;

use clap::Parser;
use config::{Config, LogType};
use logs::pid::PidLogEntry;
use logs::status::StatusLogEntry;
use logs::{parse_and_display, LogEntry, LogHeader};

pub mod config;
pub mod logs;
pub mod util;

fn main() -> io::Result<()> {
    let args = Config::parse();
    let path = Path::new(&args.file_path);
    let buf = std::fs::read(path)?;
    println!("File contents len: {}", buf.len());

    let mut pos = 0;
    let header = LogHeader::from_buf(&mut buf.as_slice())?;
    println!("{header}");
    pos += LogHeader::packed_footprint();

    match args.log_type {
        LogType::Status => {
            parse_and_display::<StatusLogEntry>(&mut &buf[pos..]);
        }
        LogType::Pid => parse_and_display::<PidLogEntry>(&mut &buf[pos..]),
    }

    Ok(())
}
