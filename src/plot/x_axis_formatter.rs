use std::ops::RangeInclusive;

use chrono::{DateTime, Timelike as _};
use egui_plot::{GridInput, GridMark};

// Constants
const NANOS_PER_SEC: u64 = 1_000_000_000;
const NANOS_PER_MILLI: u64 = 1_000_000;
const NANOS_PER_MICRO: u64 = 1_000;
const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;
const HOURS_PER_DAY: u64 = 24;
const DAYS_PER_MONTH: u64 = 30; // Approximate
const DAYS_PER_YEAR: u64 = 365; // Approximate

/// Represents a time unit with conversion to nanoseconds and nice step values
#[derive(Debug)]
struct TimeUnit {
    /// Name of the unit
    #[allow(
        dead_code,
        reason = "It's just like a comment, and might be useful for logging/debugging"
    )]
    name: &'static str,
    /// How many nanoseconds in one unit
    nanos_per_unit: f64,
    /// Nice step values in this unit
    steps: &'static [u64],
}

/// Available time units from nanoseconds to years
const TIME_UNITS: &[TimeUnit] = &[
    TimeUnit {
        name: "nanosecond",
        nanos_per_unit: 1.,
        steps: &[1, 2, 5, 10, 20, 50, 100, 200, 500],
    },
    TimeUnit {
        name: "microsecond",
        nanos_per_unit: NANOS_PER_MICRO as f64,
        steps: &[1, 2, 5, 10, 20, 50, 100, 200, 500],
    },
    TimeUnit {
        name: "millisecond",
        nanos_per_unit: NANOS_PER_MILLI as f64,
        steps: &[1, 2, 5, 10, 20, 50, 100, 200, 500],
    },
    TimeUnit {
        name: "second",
        nanos_per_unit: NANOS_PER_SEC as f64,
        steps: &[1, 2, 5, 10, 15, 30],
    },
    TimeUnit {
        name: "minute",
        nanos_per_unit: (NANOS_PER_SEC * SECONDS_PER_MINUTE) as f64,
        steps: &[1, 2, 5, 10, 15, 30],
    },
    TimeUnit {
        name: "hour",
        nanos_per_unit: (NANOS_PER_SEC * SECONDS_PER_MINUTE * MINUTES_PER_HOUR) as f64,
        steps: &[1, 2, 3, 6, 12],
    },
    TimeUnit {
        name: "day",
        nanos_per_unit: (NANOS_PER_SEC * SECONDS_PER_MINUTE * MINUTES_PER_HOUR * HOURS_PER_DAY)
            as f64,
        steps: &[1, 2, 7, 14],
    },
    TimeUnit {
        name: "month",
        nanos_per_unit: (NANOS_PER_SEC
            * SECONDS_PER_MINUTE
            * MINUTES_PER_HOUR
            * HOURS_PER_DAY
            * DAYS_PER_MONTH) as f64,
        steps: &[1, 2, 3, 6],
    },
    TimeUnit {
        name: "year",
        nanos_per_unit: (NANOS_PER_SEC
            * SECONDS_PER_MINUTE
            * MINUTES_PER_HOUR
            * HOURS_PER_DAY
            * DAYS_PER_YEAR) as f64,
        steps: &[1, 2, 5, 10],
    },
];

/// Generate grid marks for the time (X) axis
#[allow(
    clippy::needless_pass_by_value,
    reason = "That's the callback signature, and since GridInput is just 3xf64 it's probably a performance win to just copy it instead of hanging onto pointers"
)]
pub fn x_grid(input: GridInput) -> Vec<GridMark> {
    let (min_ns, max_ns) = input.bounds;
    let range_ns = max_ns - min_ns;

    const TARGET_GRID_COUNT_MIN: f64 = 4.0;

    // Find the appropriate time unit and step size
    let (unit, step) = find_appropriate_time_unit(range_ns, TARGET_GRID_COUNT_MIN);

    generate_grid_marks(min_ns, max_ns, unit, step)
}

/// Find the most appropriate time unit and step size for the given range
fn find_appropriate_time_unit(range_ns: f64, target_count_min: f64) -> (&'static TimeUnit, u64) {
    // Start from the largest unit (year) and work downwards
    for unit in TIME_UNITS.iter().rev() {
        let unit_range = range_ns / unit.nanos_per_unit;

        // Start from the largest step
        for &step in unit.steps.iter().rev() {
            let grid_count = unit_range / step as f64;
            // As soon as we get more grids than minimum, we return it
            if grid_count >= target_count_min {
                return (unit, step);
            }
        }
    }

    // Fallback to nanoseconds with step 500 (realistically the minimum)
    (&TIME_UNITS[0], 500)
}

/// Generate grid marks using the selected time unit and step size
fn generate_grid_marks(min_ns: f64, max_ns: f64, unit: &TimeUnit, step: u64) -> Vec<GridMark> {
    // Typically 3-12 marks, preallocate 12 to avoid reallocating in 99.9% of cases
    let mut marks = Vec::with_capacity(12);

    // Calculate step size in nanoseconds
    let step_ns = step as f64 * unit.nanos_per_unit;

    // Find the first mark position (round up to the next step)
    let first_mark = (min_ns / step_ns).ceil() as u64;

    // NOTE: Use a loop counter to avoid accumulating floating point precision errors
    let mut i = first_mark;
    let mut value = i as f64 * step_ns;
    while value < max_ns {
        marks.push(GridMark {
            value,
            step_size: step_ns,
        });

        i += 1;
        value = i as f64 * step_ns;
    }

    marks
}

enum AxisRange {
    Over2Days,
    Over10Minutes,
    Over4Seconds,
    Over10MilliSeconds,
    Over10MicroSeconds,
    Under,
}

impl AxisRange {
    const TWO_DAYS: f64 = NANOS_PER_SEC as f64
        * SECONDS_PER_MINUTE as f64
        * MINUTES_PER_HOUR as f64
        * HOURS_PER_DAY as f64
        * 2.0;
    const TEN_MINUTES: f64 = NANOS_PER_SEC as f64 * SECONDS_PER_MINUTE as f64 * 10.0;
    const THREE_SECONDS: f64 = NANOS_PER_SEC as f64 * 4.0;
    const TEN_MILLISECONDS: f64 = NANOS_PER_MILLI as f64 * 10.0;
    const TEN_MICROSECONDS: f64 = NANOS_PER_MICRO as f64 * 10.0;

    fn from_ns(range_ns: f64) -> Self {
        if range_ns > Self::TWO_DAYS {
            Self::Over2Days
        } else if range_ns > Self::TEN_MINUTES {
            Self::Over10Minutes
        } else if range_ns > Self::THREE_SECONDS {
            Self::Over4Seconds
        } else if range_ns > Self::TEN_MILLISECONDS {
            Self::Over10MilliSeconds
        } else if range_ns > Self::TEN_MICROSECONDS {
            Self::Over10MicroSeconds
        } else {
            Self::Under
        }
    }
}

/// Format time values based on the current range
pub fn format_time(mark: GridMark, range: &RangeInclusive<f64>) -> String {
    let ns = mark.value;
    let range_ns = *range.end() - *range.start();

    // Convert to seconds and get timestamp
    let sec = ns / NANOS_PER_SEC as f64;
    let ns_remainder = sec.fract() * NANOS_PER_SEC as f64;
    let Some(dt) = DateTime::from_timestamp(sec as i64, ns_remainder as u32) else {
        // Will happen if the user zooms out where the X-axis is extended >100 years
        log::warn!("Timestamp value out of range: {sec}");
        return "out of range".to_owned();
    };

    match AxisRange::from_ns(range_ns) {
        AxisRange::Over2Days => {
            if dt.hour() == 0 && dt.minute() == 0 {
                // Midnight: show the date
                dt.format("%Y-%m-%d")
            } else {
                dt.format("%m-%d %H:%M")
            }
        }
        AxisRange::Over10Minutes => {
            if dt.hour() == 0 && dt.minute() == 0 {
                // Midnight: show the date
                dt.format("%Y-%m-%d")
            } else {
                dt.format("%H:%M")
            }
        }
        AxisRange::Over4Seconds => {
            if dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 {
                // Midnight: show the date
                dt.format("%Y-%m-%d")
            } else {
                dt.format("%H:%M:%S")
            }
        }
        AxisRange::Over10MilliSeconds => dt.format("%H:%M:%S.%3fms"),
        AxisRange::Over10MicroSeconds => dt.format("%S.%6fus"),
        AxisRange::Under => dt.format(".%9fns"),
    }
    .to_string()
}
