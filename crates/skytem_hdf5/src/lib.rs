#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod util;

#[cfg(not(target_arch = "wasm32"))]
pub mod bifrost;

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
