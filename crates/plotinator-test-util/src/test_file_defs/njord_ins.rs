use paste::paste;
use std::path::PathBuf;

macro_rules! define_njord_ins_h5 {
    ($name:ident, $path:expr) => {
        define_binary_test_file!($name, concat!("njord_ins/", $path));
    };
}

define_njord_ins_h5!(njord_ins, "20250630_113745_certus.h5");
