use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, ValueEnum, Clone, Copy)]
pub enum LogType {
    Status,
    Pid,
}
/// Simple program to print the filename from a given file path
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// File path to extract the filename from
    pub file_path: String,
    /// Log type: 'status' for Status log, 'pid' for PID log
    #[arg(short, long)]
    pub log_type: LogType,
}
