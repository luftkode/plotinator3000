macro_rules! test_file {
    ($path:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_data/", $path)
    };
}

macro_rules! define_binary_test_file {
    ($name:ident, $path:expr) => {
        paste::paste! {
            pub const [<$name:upper _PATH>]: &str = concat!(
                test_file!($path),
            );

            pub const [<$name:upper _BYTES>]: &[u8] = include_bytes!(concat!(
                test_file!($path),
            ));

            pub fn [<$name>]() -> std::path::PathBuf {
                std::path::PathBuf::from([<$name:upper _PATH>])
                    .canonicalize()
                    .expect("Failed to canonicalize path: {[<$name:upper _PATH>]}")
            }
        }
    };
}

macro_rules! define_utf8_test_file {
    ($name:ident, $path:expr) => {
        paste::paste! {
            pub const [<$name:upper _PATH>]: &str = concat!(
                test_file!($path),
            );

            pub const [<$name:upper _BYTES>]: &[u8] = include_bytes!(concat!(
                test_file!($path),
            ));

            pub const [<$name:upper _STR>]: &str = include_str!(concat!(
                test_file!($path),
            ));

            pub fn [<$name>]() -> std::path::PathBuf {
                std::path::PathBuf::from([<$name:upper _PATH>])
                    .canonicalize()
                    .expect("Failed to canonicalize path: {[<$name:upper _PATH>]}")
            }
        }
    };
}

pub mod bifrost_current;
pub mod frame_altimeters;
pub mod frame_gps;
pub mod frame_inclinometers;
pub mod frame_magnetometer;
pub mod legacy_generator;
pub mod mbed_motor_control;
pub mod navsys_kitchen_sink;
pub mod njord_ins;
pub mod tsc;
pub mod wasp200;
