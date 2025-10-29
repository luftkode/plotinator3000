macro_rules! define_njord_altimeter_sps {
    ($name:ident, $path:expr) => {
        define_utf8_test_file!($name, concat!("njord_altimeter/", $path));
    };
}

macro_rules! define_njord_altimeter_h5 {
    ($name:ident, $path:expr) => {
        define_binary_test_file!($name, concat!("njord_altimeter/", $path));
    };
}

define_njord_altimeter_h5!(
    njord_altimeter_wasp200,
    "20250506_154920_njord_altimeter_wasp200.h5"
);

define_njord_altimeter_sps!(
    njord_altimeter_wasp200_sps,
    "20250506_154920_njord_altimeter_wasp200.sps"
);

define_njord_altimeter_h5!(
    njord_altimeter_wasp200_sf20,
    "20251029_105246_njord-altimeter_wasp200_sf20.h5"
);
