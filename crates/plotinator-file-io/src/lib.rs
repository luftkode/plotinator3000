mod custom_files;
pub mod dropped_files;
pub mod file_dialog;
pub mod loaded_files;

pub enum FileParseMessage {
    Info(String),
    Warn(String),
    Error(String),
}
