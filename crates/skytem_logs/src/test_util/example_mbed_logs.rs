macro_rules! mbed_log {
    ($file:expr) => {
        concat!("../../../../test_data/mbed_motor_control/", $file)
    };
}

pub const MBED_MOTOR_CONTROL_PID_V1: &str =
    mbed_log!("v1/20240926_121708/pid_20240926_121708_00.bin");

const TEST_DATA_V1: &str =
    "../../test_data/mbed_motor_control/v1/20240926_121708/pid_20240926_121708_00.bin";
const TEST_DATA_V2: &str =
    "../../test_data/mbed_motor_control/v2/20241014_080729/pid_20241014_080729_00.bin";
const TEST_DATA_V4: &str = "../../test_data/mbed_motor_control/v4/pid_20250120_092446_00.bin";
