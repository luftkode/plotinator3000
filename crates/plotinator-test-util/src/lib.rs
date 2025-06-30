#![allow(
    clippy::disallowed_types,
    reason = "This is test utilities so things like PathBuf is fine, we won't deploy this code anywhere"
)]
pub mod test_file_defs;

pub use {
    std::fs,
    std::io,
    test_file_defs::{
        bifrost_current::*, legacy_generator::*, mbed_motor_control::*, njord_ins::*, wasp200::*,
    },
    testresult::TestResult,
};
