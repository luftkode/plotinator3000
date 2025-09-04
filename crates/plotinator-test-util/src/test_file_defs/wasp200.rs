macro_rules! define_wasp200_sps {
    ($name:ident, $path:expr) => {
        define_utf8_test_file!($name, concat!("wasp200/", $path));
    };
}

macro_rules! define_wasp200_h5 {
    ($name:ident, $path:expr) => {
        define_binary_test_file!($name, concat!("wasp200/", $path));
    };
}

define_wasp200_h5!(wasp200, "20250506_154920_wasp200.h5");

define_wasp200_sps!(wasp200_sps, "20250506_154920_wasp200.sps");
