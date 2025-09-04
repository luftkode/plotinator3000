macro_rules! define_bifrost_current_file {
    ($name:ident, $path:expr) => {
        define_binary_test_file!($name, concat!("hdf5/bifrost_current/", $path));
    };
}

define_bifrost_current_file!(bifrost_current, "20240930_100137_bifrost.h5");
