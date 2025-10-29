plotinator_macros::non_wasm_modules!(
    pub(crate) mod util;
    pub(crate) mod stream_descriptor;
    pub mod bifrost;
    pub mod frame_altimeters;
    pub mod frame_inclinometers;
    pub mod frame_magnetometer;
    pub mod frame_gps;
    pub mod njord_ins;
    pub mod njord_altimeter;
    pub mod tsc;
);

#[cfg(not(target_arch = "wasm32"))]
pub use {
    bifrost::BifrostLoopCurrent, frame_altimeters::FrameAltimeters, frame_gps::FrameGps,
    frame_inclinometers::FrameInclinometers, frame_magnetometer::FrameMagnetometer,
    njord_altimeter::NjordAltimeter, njord_ins::NjordIns, tsc::Tsc,
};

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
