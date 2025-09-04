plotinator_macros::non_wasm_modules!(
    pub(crate) mod util;
    pub(crate) mod stream_descriptor;
    pub mod bifrost;
    pub mod wasp200;
    pub mod frame_altimeters;
    pub mod frame_inclinometers;
    pub mod frame_magnetometer;
    pub mod frame_gps;
    pub mod njord_ins;
    pub mod tsc;
);

// File extensions we recognize as hdf5 files.
const POSSIBLE_HDF5_EXTENSIONS_CASE_INSENSITIVE: [&str; 3] = ["h5", "hdf5", "hdf"];

pub fn path_has_hdf5_extension(path: &std::path::Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    for possible_extension in POSSIBLE_HDF5_EXTENSIONS_CASE_INSENSITIVE {
        if extension.eq_ignore_ascii_case(possible_extension) {
            return true;
        }
    }
    false
}
