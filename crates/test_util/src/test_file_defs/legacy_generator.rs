use paste::paste;
use std::path::PathBuf;
macro_rules! define_legacy_generator_log {
    ($name:ident, $path:expr) => {
        define_utf8_test_file!($name, concat!("generator/", $path));
    };
}

define_legacy_generator_log!(legacy_generator_log, "20230124_134738_Gen.log");
