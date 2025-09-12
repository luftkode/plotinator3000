#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

/// Used for file dialog filters
pub const FILE_FILTER_NAME: &str = "Known Logs and Plotinator3000 files";
pub const FILE_FILTER_EXTENSIONS: &[&str] = &[
    "p3k", "sps", "h5", "hdf5", "hdf", "bin", "log", "zip", "csv", "txt",
];
