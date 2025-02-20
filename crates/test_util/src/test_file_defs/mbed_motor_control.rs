use paste::paste;
use std::path::PathBuf;

macro_rules! define_mbed_log {
    ($name:ident, $path:expr) => {
        define_binary_test_file!($name, concat!("mbed_motor_control/", $path));
    };
}

define_mbed_log!(mbed_pid_v1, "v1/20240926_121708/pid_20240926_121708_00.bin");
define_mbed_log!(
    mbed_status_v1,
    "v1/20240926_121708/status_20240926_121708_00.bin"
);
define_mbed_log!(mbed_pid_v2, "v2/20241014_080729/pid_20241014_080729_00.bin");
define_mbed_log!(
    mbed_status_v2,
    "v2/20241014_080729/status_20241014_080729_00.bin"
);
define_mbed_log!(mbed_pid_v3, "v3/short_start/pid_20241029_133931_00.bin");
define_mbed_log!(
    mbed_status_v3,
    "v3/short_start/status_20241029_133931_00.bin"
);
define_mbed_log!(mbed_pid_v4, "v4/pid_20250120_092446_00.bin");
define_mbed_log!(mbed_pid_v5_regular, "v5/regular/pid_20250210_085126_00.bin");
define_mbed_log!(
    mbed_pid_v5_configuring,
    "v5/configuring/pid_20250205_121547_00.bin"
);
define_mbed_log!(mbed_status_v5, "v5/status_20250120_092446_00.bin");
define_mbed_log!(
    mbed_status_v6_regular,
    "v6/regular/status_20250210_085126_00.bin"
);
define_mbed_log!(
    mbed_status_v6_configuring,
    "v6/configuring/status_20250205_121547_00.bin"
);

define_mbed_log!(mbed_pid_v6_regular, "v6/regular/pid_20250220_134638_00.bin");
