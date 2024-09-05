#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod plot;
use std::io;

pub use app::App;
use logs::{parse_and_display, pid::PidLogEntry, status::StatusLogEntry, LogEntry, LogHeader};
pub mod logs;
pub mod util;

pub enum LogType {
    Status,
    Pid,
}

fn print_log() -> io::Result<()> {
    // let args = Config::parse();
    // let path = Path::new(&args.file_path);
    let buf = std::fs::read("dsa")?;
    println!("File contents len: {}", buf.len());

    let mut pos = 0;
    let header = LogHeader::from_buf(&mut buf.as_slice())?;
    println!("{header}");
    pos += LogHeader::packed_footprint();

    let logtype = LogType::Status;

    match logtype {
        LogType::Status => {
            parse_and_display::<StatusLogEntry>(&mut &buf[pos..]);
        }
        LogType::Pid => parse_and_display::<PidLogEntry>(&mut &buf[pos..]),
    }

    Ok(())
}
