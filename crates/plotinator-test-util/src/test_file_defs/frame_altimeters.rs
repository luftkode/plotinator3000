macro_rules! define_frame_altimeters_h5 {
    ($name:ident, $path:expr) => {
        define_binary_test_file!($name, concat!("frame_altimeters/", $path));
    };
}

define_frame_altimeters_h5!(frame_altimeters, "20250613_092324_frame-altimeters.h5");
