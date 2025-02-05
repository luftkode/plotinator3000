use std::io;

use byteorder::{LittleEndian, ReadBytesExt};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

use super::MbedConfig;

#[derive(Debug, CopyGetters, PartialEq, Deserialize, Serialize, Clone, Copy)]
#[repr(packed)]
pub(crate) struct MbedConfigV4 {
    #[getset(get_copy = "pub")]
    pid_cfg: PidConfig,
    #[getset(get_copy = "pub")]
    general_cfg: GeneralConfig,
}

impl MbedConfig for MbedConfigV4 {
    fn raw_size() -> usize {
        size_of::<Self>()
    }

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<Self> {
        let pid_cfg = PidConfig::from_reader(reader)?;
        let general_cfg = GeneralConfig::from_reader(reader)?;

        Ok(Self {
            pid_cfg,
            general_cfg,
        })
    }

    fn field_value_pairs(&self) -> Vec<(String, String)> {
        let mut fvp = self.general_cfg.field_value_pairs();
        fvp.append(&mut self.pid_cfg.field_value_pairs());
        fvp
    }
}

#[derive(Debug, CopyGetters, PartialEq, Deserialize, Serialize, Clone, Copy)]
#[repr(packed)]
pub(crate) struct GeneralConfig {
    #[getset(get_copy = "pub")]
    t_run: u8,
    #[getset(get_copy = "pub")]
    t_fan_on: u8,
    #[getset(get_copy = "pub")]
    t_fan_off: u8,

    #[getset(get_copy = "pub")]
    initial_throttle_percent: f32,
    #[getset(get_copy = "pub")]
    initial_throttle_step_size_percent: f32,

    #[getset(get_copy = "pub")]
    rpm_idle: u16,
    #[getset(get_copy = "pub")]
    rpm_standby: u16,
    #[getset(get_copy = "pub")]
    rpm_running: u16,

    #[getset(get_copy = "pub")]
    time_in_idle: u8,
    #[getset(get_copy = "pub")]
    time_shutdown: u16,
    #[getset(get_copy = "pub")]
    time_wait_for_cap: u16,

    #[getset(get_copy = "pub")]
    vbat_ready: f32,
    #[getset(get_copy = "pub")]
    servo_min: u16,
    #[getset(get_copy = "pub")]
    servo_max: u16,

    #[getset(get_copy = "pub")]
    high_res_sample_period_ms: u16,

    #[getset(get_copy = "pub")]
    low_res_sample_period_ms: u16,
}

impl MbedConfig for GeneralConfig {
    fn raw_size() -> usize {
        size_of::<Self>()
    }

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<Self> {
        let t_run = reader.read_u8()?;
        let t_fan_on = reader.read_u8()?;
        let t_fan_off = reader.read_u8()?;
        let initial_throttle_percent = reader.read_f32::<LittleEndian>()?;
        let initial_throttle_step_size_percent = reader.read_f32::<LittleEndian>()?;
        let rpm_idle = reader.read_u16::<LittleEndian>()?;
        let rpm_standby = reader.read_u16::<LittleEndian>()?;
        let rpm_running = reader.read_u16::<LittleEndian>()?;
        let time_in_idle = reader.read_u8()?;
        let time_shutdown = reader.read_u16::<LittleEndian>()?;
        let time_wait_for_cap = reader.read_u16::<LittleEndian>()?;
        let vbat_ready = reader.read_f32::<LittleEndian>()?;
        let servo_min = reader.read_u16::<LittleEndian>()?;
        let servo_max = reader.read_u16::<LittleEndian>()?;
        let high_res_sample_period_ms = reader.read_u16::<LittleEndian>()?;
        let low_res_sample_period_ms = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            t_run,
            t_fan_on,
            t_fan_off,
            initial_throttle_percent,
            initial_throttle_step_size_percent,
            rpm_idle,
            rpm_standby,
            rpm_running,
            time_in_idle,
            time_shutdown,
            time_wait_for_cap,
            vbat_ready,
            servo_min,
            servo_max,
            high_res_sample_period_ms,
            low_res_sample_period_ms,
        })
    }

    fn field_value_pairs(&self) -> Vec<(String, String)> {
        vec![
            ("T_RUN".to_owned(), self.t_run.to_string()),
            ("T_FAN_ON".to_owned(), self.t_fan_on.to_string()),
            ("T_FAN_OFF".to_owned(), self.t_fan_off.to_string()),
            (
                "INITIAL_THROTTLE_PERCENT".to_owned(),
                self.initial_throttle_percent().to_string(),
            ),
            (
                "INITIAL_THROTTLE_STEP_SIZE_PERCENT".to_owned(),
                self.initial_throttle_percent().to_string(),
            ),
            ("RPM_IDLE".to_owned(), self.rpm_idle().to_string()),
            ("RPM_STANDBY".to_owned(), self.rpm_standby().to_string()),
            ("RPM_RUNNING".to_owned(), self.rpm_running().to_string()),
            ("TIME_IN_IDLE".to_owned(), self.time_in_idle().to_string()),
            ("TIME_SHUTDOWN".to_owned(), self.time_shutdown().to_string()),
            (
                "TIME_WAIT_FOR_CAP".to_owned(),
                self.time_wait_for_cap().to_string(),
            ),
            ("VBAT_READY".to_owned(), self.vbat_ready().to_string()),
            ("SERVO_MIN".to_owned(), self.servo_min().to_string()),
            ("SERVO_MAX".to_owned(), self.servo_max().to_string()),
            (
                "HIGH_RES_SAMPLE_PERIOD_MS".to_owned(),
                self.high_res_sample_period_ms().to_string(),
            ),
            (
                "LOW_RES_SAMPLE_PERIOD_MS".to_owned(),
                self.low_res_sample_period_ms().to_string(),
            ),
        ]
    }
}

#[derive(Debug, CopyGetters, PartialEq, Deserialize, Serialize, Clone, Copy)]
#[repr(packed)]
pub(crate) struct PidConfig {
    #[getset(get_copy = "pub")]
    kp_idle: f32,
    #[getset(get_copy = "pub")]
    ki_idle: f32,
    #[getset(get_copy = "pub")]
    kd_idle: f32,

    #[getset(get_copy = "pub")]
    kp_standby: f32,
    #[getset(get_copy = "pub")]
    ki_standby: f32,
    #[getset(get_copy = "pub")]
    kd_standby: f32,

    #[getset(get_copy = "pub")]
    kp_running: f32,
    #[getset(get_copy = "pub")]
    ki_running: f32,
    #[getset(get_copy = "pub")]
    kd_running: f32,
}

impl MbedConfig for PidConfig {
    fn raw_size() -> usize {
        size_of::<Self>()
    }

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<Self> {
        let kp_idle = reader.read_f32::<LittleEndian>()?;
        let ki_idle = reader.read_f32::<LittleEndian>()?;
        let kd_idle = reader.read_f32::<LittleEndian>()?;

        let kp_standby = reader.read_f32::<LittleEndian>()?;
        let ki_standby = reader.read_f32::<LittleEndian>()?;
        let kd_standby = reader.read_f32::<LittleEndian>()?;

        let kp_running = reader.read_f32::<LittleEndian>()?;
        let ki_running = reader.read_f32::<LittleEndian>()?;
        let kd_running = reader.read_f32::<LittleEndian>()?;

        Ok(Self {
            kp_idle,
            ki_idle,
            kd_idle,
            kp_standby,
            ki_standby,
            kd_standby,
            kp_running,
            ki_running,
            kd_running,
        })
    }

    fn field_value_pairs(&self) -> Vec<(String, String)> {
        let pid_vals_idle = (
            "Kp, Ki, Kd: IDLE".to_owned(),
            format!("{}, {}, {}", self.kp_idle(), self.ki_idle(), self.kd_idle()),
        );
        let pid_vals_standby = (
            "Kp, Ki, Kd: STANDBY".to_owned(),
            format!(
                "{}, {}, {}",
                self.kp_standby(),
                self.ki_standby(),
                self.kd_standby()
            ),
        );
        let pid_vals_running = (
            "Kp, Ki, Kd: RUNNING".to_owned(),
            format!(
                "{}, {}, {}",
                self.kp_running(),
                self.ki_running(),
                self.kd_running()
            ),
        );
        vec![pid_vals_idle, pid_vals_standby, pid_vals_running]
    }
}
