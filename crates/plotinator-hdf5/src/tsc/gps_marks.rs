use chrono::{DateTime, TimeZone as _, Utc};
use hdf5::H5Type;
use ndarray::{ArrayBase, Dim, OwnedRepr};
use plotinator_log_if::{
    leap_seconds::{GpsWeek, TowMs, TowSubMs, gps_to_unix_ns},
    prelude::{ExpectedPlotRange, RawPlotCommon},
    rawplot::RawPlot,
};

type GpsMarks = ArrayBase<OwnedRepr<GpsMarkRecord>, Dim<[usize; 1]>>;

/// Wrapper around all the [`GpsMarkRecord`]s from a TSC.h5 file
pub(crate) struct GpsMarkRecords {
    inner: GpsMarks,
}

impl GpsMarkRecords {
    const DATASET_NAME: &str = "GPS_marks";

    pub fn from_hdf5(h5: &hdf5::File) -> hdf5::Result<Self> {
        let dataset = h5.dataset(Self::DATASET_NAME)?;
        let gps_marks = dataset.read::<GpsMarkRecord, ndarray::Ix1>()?;

        Ok(Self { inner: gps_marks })
    }

    // Return a vector of timestamps (unix UTC nanoseconds)
    pub fn timestamps(&self) -> Vec<f64> {
        self.inner
            .iter()
            .map(|g| g.unix_timestamp_ns() as f64)
            .collect()
    }

    pub fn first_timestamp(&self) -> Option<DateTime<Utc>> {
        self.inner.first().map(|e| e.unix_timestamp_utc())
    }

    #[allow(clippy::too_many_lines, reason = "Long but simple")]
    pub fn build_plots_and_metadata(&self) -> (Vec<RawPlot>, Vec<(String, String)>) {
        let time = self.timestamps();

        let mut metadata = vec![("GPS Marks".to_owned(), time.len().to_string())];

        let mut pulse_width = Vec::with_capacity(time.len());
        let mut acc_est = Vec::with_capacity(time.len());
        let mut count = Vec::with_capacity(time.len());

        // flag series
        let mut mode_running = Vec::with_capacity(time.len());
        let mut run = Vec::with_capacity(time.len());
        let mut new_falling_edge = Vec::with_capacity(time.len());
        let mut timebase_gnss = Vec::with_capacity(time.len());
        let mut timebase_utc = Vec::with_capacity(time.len());
        let mut utc_avail = Vec::with_capacity(time.len());
        let mut time_valid = Vec::with_capacity(time.len());
        let mut new_rising_edge = Vec::with_capacity(time.len());

        let mut step_counter = StepCounter::new();
        let mut delta_computer = DeltaComputer::new();

        for (e, &t) in self.inner.iter().zip(&time) {
            step_counter.record_count(e.count, t);
            delta_computer.record_timestamp(e.unix_timestamp_ns(), t, e.count);

            pulse_width.push([t, e.pulse_width_us()]);
            acc_est.push([t, e.acc_est as f64]);
            count.push([t, e.count as f64]);

            mode_running.push([t, e.flag(GpsMarkRecord::FLAGS_MODE_RUNNING)]);
            run.push([t, e.flag(GpsMarkRecord::FLAGS_RUN)]);
            new_falling_edge.push([t, e.flag(GpsMarkRecord::FLAGS_NEWFALLINGEDGE)]);
            timebase_gnss.push([t, e.flag(GpsMarkRecord::FLAGS_TIMEBASE_GNSS)]);
            timebase_utc.push([t, e.flag(GpsMarkRecord::FLAGS_TIMEBASE_UTC)]);
            utc_avail.push([t, e.flag(GpsMarkRecord::FLAGS_UTC_AVAIL)]);
            time_valid.push([t, e.flag(GpsMarkRecord::FLAGS_TIME_VALID)]);
            new_rising_edge.push([t, e.flag(GpsMarkRecord::FLAGS_NEWRISINGEDGE)]);
        }

        // Add step counter metadata
        metadata.push((
            "Count step differences".to_owned(),
            step_counter.to_metadata_string(),
        ));
        let (all_stats, normal_stats) = delta_computer.to_metadata_string();
        metadata.push(("Timestamp Δt stats [s]".to_owned(), all_stats));
        metadata.push((
            "Timestamp Δt stats (without faults) [s]".to_owned(),
            normal_stats,
        ));
        (
            vec![
                RawPlotCommon::new(
                    "Pulse width [µs]".to_owned(),
                    pulse_width,
                    ExpectedPlotRange::Thousands,
                )
                .into(),
                RawPlotCommon::new(
                    "Accuracy estimate [ns]".to_owned(),
                    acc_est,
                    ExpectedPlotRange::OneToOneHundred,
                )
                .into(),
                RawPlotCommon::new("Count".to_owned(), count, ExpectedPlotRange::Thousands).into(),
                RawPlotCommon::new(
                    "Mode running [bool]".to_owned(),
                    mode_running,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new("Run [bool]".to_owned(), run, ExpectedPlotRange::Percentage)
                    .into(),
                RawPlotCommon::new(
                    "New falling edge [bool]".to_owned(),
                    new_falling_edge,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new(
                    "Timebase GNSS [bool]".to_owned(),
                    timebase_gnss,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new(
                    "Timebase UTC [bool]".to_owned(),
                    timebase_utc,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new(
                    "UTC available [bool]".to_owned(),
                    utc_avail,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new(
                    "Time valid [bool]".to_owned(),
                    time_valid,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new(
                    "New rising edge [bool]".to_owned(),
                    new_rising_edge,
                    ExpectedPlotRange::Percentage,
                )
                .into(),
                RawPlotCommon::new(
                    "Timestamp Δt [s]".to_owned(),
                    delta_computer.take_plot_points(),
                    ExpectedPlotRange::OneToOneHundred,
                )
                .into(),
            ],
            metadata,
        )
    }
}

/// Tracks step differences in GPS mark counts with detailed information
struct StepCounter {
    events: Vec<StepEvent>,
    prev_count: Option<u16>,
    prev_time: Option<f64>,
}

/// Represents a step event with detailed information
struct StepEvent {
    step_delta: u16,
    timestamp_ns: i64,
    time_delta_s: f64,
    count_before: u16,
    count_after: u16,
}

impl StepCounter {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            prev_count: None,
            prev_time: None,
        }
    }

    fn record_count(&mut self, count: u16, timestamp_ns: f64) {
        if let (Some(prev_count), Some(prev_time_ns)) = (self.prev_count, self.prev_time) {
            let step_delta = count.wrapping_sub(prev_count);

            if step_delta != 1 {
                let time_delta_s = (timestamp_ns - prev_time_ns) / 1_000_000_000.0;
                self.events.push(StepEvent {
                    step_delta,
                    timestamp_ns: timestamp_ns as i64,
                    time_delta_s,
                    count_before: prev_count,
                    count_after: count,
                });
            }
        }

        self.prev_count = Some(count);
        self.prev_time = Some(timestamp_ns);
    }

    fn to_metadata_string(&self) -> String {
        if self.events.is_empty() {
            return "All steps were 1".to_owned();
        }

        let mut result = String::new();
        result.push_str(&format!(
            "Found {} non-1 step event{plural}:\n",
            self.events.len(),
            plural = if self.events.len() == 1 { "" } else { "s" }
        ));

        for event in &self.events {
            let dt = Utc.timestamp_nanos(event.timestamp_ns);
            let time_str = dt.format("%H:%M:%S%.3f").to_string();

            result.push_str(&format!(
                "Step={} at {time_str} [time delta: {:.6}s, count: {} -> {}]\n",
                event.step_delta, event.time_delta_s, event.count_before, event.count_after
            ));
        }

        result
    }
}

/// Computes deltas between consecutive GPS marks
struct DeltaComputer {
    deltas: Vec<[f64; 2]>, // [timestamp, delta_s]
    prev_ns: Option<i64>,
    all_stats: Vec<f64>,     // all deltas
    normal_stats: Vec<f64>,  // deltas without faults (Δcount = 1)
    prev_count: Option<u16>, // to detect faults
}

impl DeltaComputer {
    fn new() -> Self {
        Self {
            deltas: Vec::new(),
            prev_ns: None,
            all_stats: Vec::new(),
            normal_stats: Vec::new(),
            prev_count: None,
        }
    }

    fn record_timestamp(&mut self, curr_ns: i64, curr_time: f64, curr_count: u16) {
        if let Some(prev) = self.prev_ns {
            let delta_s = (curr_ns - prev) as f64 / 1e9;
            self.deltas.push([curr_time, delta_s]);
            self.all_stats.push(delta_s);

            // Add to normal_stats only if step increment is 1
            if let Some(prev_count) = self.prev_count {
                if curr_count.wrapping_sub(prev_count) == 1 {
                    self.normal_stats.push(delta_s);
                }
            }
        }

        self.prev_ns = Some(curr_ns);
        self.prev_count = Some(curr_count);
    }

    fn take_plot_points(self) -> Vec<[f64; 2]> {
        self.deltas
    }

    fn stats_to_string(stats: &[f64]) -> String {
        if stats.is_empty() {
            return "No deltas".to_owned();
        }
        let min = stats.iter().copied().fold(f64::INFINITY, f64::min);
        let max = stats.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let avg = stats.iter().sum::<f64>() / stats.len() as f64;
        let mean_diff_sq_sum: f64 = stats.iter().map(|x| (x - avg).powi(2)).sum();
        let stddev = (mean_diff_sq_sum / stats.len() as f64).sqrt();
        format!("min={min:.6}, max={max:.6}, avg={avg:.6}, σ={stddev:.6}")
    }

    fn to_metadata_string(&self) -> (String, String) {
        let all = Self::stats_to_string(&self.all_stats);
        let normal = Self::stats_to_string(&self.normal_stats);
        (all, normal)
    }
}

/// TIM-TM2 (0x0D, 0x03)
/// Time mark data
///
/// Description for details.
///
/// Supported on:
///  - u-blox 8 / u-blox M8 with protocol version 22 (only with Timing Products)
///
/// `CLASS_ID` = 13, `MESSAGE_ID` = 3
///
/// Flag constants:
/// - `FLAGS_MODE_RUNNING` = 1     // single = 0, running = 1
/// - `FLAGS_RUN` = 2              // armed = 0, stopped = 1
/// - `FLAGS_NEWFALLINGEDGE` = 4   // new falling edge detected
/// - `FLAGS_TIMEBASE_GNSS` = 8    // 0 = time base is receiver time, 1 = time base is GNSS Time (according to the configuration in CFG-TP5 for tpldx= 0)
/// - `FLAGS_TIMEBASE_UTC` = 16    // Time Base is UTC (the variant according to the configuration in CFG-NAV5
/// - `FLAGS_UTC_AVAIL` = 32       // 0 = utc not available, 1 = utc available
/// - `FLAGS_TIME_VALID` = 64      // 0 = time is not valid, 1 time is valid
/// - `FLAGS_NEWRISINGEDGE` = 128  // new rising edge detected
#[derive(H5Type, Debug)]
#[repr(C)]
struct GpsMarkRecord {
    /// Channel (i.e. EXTINT) upon which the pulse was measured
    ch: u8,

    /// Bitmask [newRisingEdge, time, utc, timeBase, , newFallingEdge, run, mode]
    flags: u8,

    /// Rising edge count
    count: u16,

    /// Week number of last rising edge
    #[hdf5(rename = "wnR")]
    wn_r: u16,

    /// Week number of last falling edge
    #[hdf5(rename = "wnF")]
    wn_f: u16,

    /// Time of Week of rising edge (milliseconds)
    #[hdf5(rename = "towMsR")]
    tow_ms_r: u32,

    /// Millisecond fraction of `ToW` of rising edge in nanoseconds
    #[hdf5(rename = "towSubMsR")]
    tow_sub_ms_r: u32,

    /// Time of Week of falling edge (milliseconds)
    #[hdf5(rename = "towMsF")]
    tow_ms_f: u32,

    /// Millisecond fraction of `ToW` of falling edge in nanoseconds
    #[hdf5(rename = "towSubMsF")]
    tow_sub_ms_f: u32,

    /// Accuracy estimate
    #[hdf5(rename = "accEst")]
    acc_est: u32,
}

impl GpsMarkRecord {
    const FLAGS_MODE_RUNNING: u8 = 1;
    const FLAGS_RUN: u8 = 2;
    const FLAGS_NEWFALLINGEDGE: u8 = 4;
    const FLAGS_TIMEBASE_GNSS: u8 = 8;
    const FLAGS_TIMEBASE_UTC: u8 = 16;
    const FLAGS_UTC_AVAIL: u8 = 32;
    const FLAGS_TIME_VALID: u8 = 64;
    const FLAGS_NEWRISINGEDGE: u8 = 128;

    fn flag(&self, mask: u8) -> f64 {
        if self.flags & mask != 0 { 1.0 } else { 0.0 }
    }

    fn unix_timestamp_utc(&self) -> DateTime<Utc> {
        Utc.timestamp_nanos(self.unix_timestamp_ns())
    }

    /// Convert GPS week + TOW to precise Unix timestamp in nanoseconds since Unix epoch
    /// All calculations done in integers to avoid floating point accumulation errors
    fn unix_timestamp_ns(&self) -> i64 {
        gps_to_unix_ns(
            GpsWeek(self.wn_r),
            TowMs(self.tow_ms_r),
            TowSubMs(self.tow_sub_ms_r),
        )
        .0
    }

    /// Calculate pulse width in microseconds (integer arithmetic until final conversion)
    pub fn pulse_width_us(&self) -> f64 {
        const MS_TO_NS: i64 = 1_000_000;
        const NS_TO_US: f64 = 1.0 / 1_000.0;

        // Compute deltas in each unit
        let week_delta = (self.wn_f as i64 - self.wn_r as i64) * 604_800; // seconds
        let ms_delta = self.tow_ms_f as i64 - self.tow_ms_r as i64; // milliseconds
        let sub_ns_delta = self.tow_sub_ms_f as i64 - self.tow_sub_ms_r as i64; // nanoseconds

        // Combine everything into nanoseconds
        let total_ns = week_delta * 1_000_000_000   // weeks -> ns
                    + ms_delta * MS_TO_NS // ms -> ns
                    + sub_ns_delta; // ns

        // Convert to microseconds
        total_ns as f64 * NS_TO_US
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use plotinator_test_util::test_file_defs::tsc::*;
    use testresult::TestResult;

    #[test]
    fn read_gps_marks() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let gps_marks = GpsMarkRecords::from_hdf5(&h5file)?;
        let gps_marks = gps_marks.inner;
        for g in &gps_marks {
            eprintln!("g: {g:?}");
            eprintln!("{}", g.unix_timestamp_utc());
        }

        insta::assert_debug_snapshot!(gps_marks);
        Ok(())
    }

    #[test]
    fn test_timestamp_conversion_simple() {
        let record = GpsMarkRecord {
            ch: 1,
            flags: 0,
            count: 100,
            wn_r: 2200, // Example GPS week
            wn_f: 0,
            tow_ms_r: 123_000, // 123 s
            tow_sub_ms_r: 0,   // 0 nanoseconds
            tow_ms_f: 0,
            tow_sub_ms_f: 0,
            acc_est: 0,
        };

        let dt = Utc.timestamp_opt(1646524905, 0).unwrap();
        let readable_dt = NaiveDate::from_ymd_opt(2022, 3, 6) // YYYY-MM-DD
            .unwrap()
            .and_hms_nano_opt(0, 1, 45, 0) // HH:MM:SS + nanos
            .unwrap()
            .and_utc();
        assert_eq!(dt, readable_dt, "Reference timestamps don't match");
        assert_eq!(
            record.unix_timestamp_utc(),
            readable_dt,
            "invalid conversion"
        );
    }

    #[test]
    fn test_timestamp_conversion_nanos() {
        const NANO_SEC_PART: u32 = 814_123_456;
        let record = GpsMarkRecord {
            ch: 1,
            flags: 0,
            count: 100,
            wn_r: 2381, // Example GPS week
            wn_f: 2381,
            tow_ms_r: 62_625_000,        // 62625000 ms
            tow_sub_ms_r: NANO_SEC_PART, // 814_123_456 nanoseconds
            tow_ms_f: 0,
            tow_sub_ms_f: 0,
            acc_est: 0,
        };

        let dt = Utc.timestamp_nanos(1_756_056_207_814_123_456);
        let readable_dt = NaiveDate::from_ymd_opt(2025, 8, 24) // YYYY-MM-DD
            .unwrap()
            .and_hms_nano_opt(17, 23, 27, NANO_SEC_PART) // HH:MM:SS + nanos
            .unwrap()
            .and_utc();
        assert_eq!(dt, readable_dt, "Reference timestamps don't match");
        assert_eq!(
            record.unix_timestamp_utc(),
            readable_dt,
            "invalid conversion"
        );
    }
}
