use paste::paste;
use std::path::PathBuf;

define_binary_test_file!(
    frame_magnetometer,
    "frame_magnetometer/20250813_063418_frame-magnetometer.h5"
);

define_utf8_test_file!(
    frame_magnetometer_sps,
    "frame_magnetometer/frame-magnetometer.sps"
);
