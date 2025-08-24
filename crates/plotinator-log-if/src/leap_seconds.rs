// LEAP_SECONDS: Newest -> Oldest
//
// If a new leap second is added, it needs to go in this list, otherwise new GPS/Unix timestamp conversions are erroneous
//
// Since the earth's rotation is accelerating, there might not be a new leap second for a very long time.
//
// Source: https://data.iana.org/time-zones/tzdb/leap-seconds.list
const LEAP_SECONDS: &[(i64, i64)] = &[
    (1483228800, 18), // Jan 1, 2017
    (1435708800, 17), // July 1, 2015
    (1341100800, 16), // July 1, 2012
    (1230768000, 15), // Jan 1, 2009
    (1136073600, 14), // Jan 1, 2006
];

/// Converts GPS Week + Time-of-Week to a precise Unix timestamp in nanoseconds.
///
/// # Arguments
///
/// * `wn` - GPS week number (`GpsWeek`)
/// * `tow_ms` - Time-of-Week in milliseconds (`TowMs`)
/// * `tow_sub_ms` - Sub-millisecond part of TOW (`TowSubMs`)
///
/// # Returns
///
/// A `UnixNs` timestamp corrected for leap seconds.
///
/// # Example
///
/// ```
/// # use plotinator_log_if::leap_seconds::*;
/// let week = GpsWeek(1930);
/// let tow = TowMs(345_600_000); // 4th day in ms
/// let sub_ms = TowSubMs(500_000);
/// let unix_ns = gps_to_unix_ns(week, tow, sub_ms).0;
/// assert_eq!(unix_ns, 1_483_574_382_000_500_000);
/// ```
pub fn gps_to_unix_ns(
    GpsWeek(wn): GpsWeek,
    TowMs(tow_ms): TowMs,
    TowSubMs(tow_sub_ms): TowSubMs,
) -> UnixNs {
    const GPS_EPOCH_UNIX_SECONDS: i64 = 315964800; // GPS epoch relative to Unix epoch (Jan 6, 1980)
    const SECONDS_PER_WEEK: i64 = 604_800;

    // Convert GPS time to nanoseconds since GPS epoch
    let week_ns = wn as i64 * SECONDS_PER_WEEK * 1_000_000_000;
    let tow_ns = tow_ms as i64 * 1_000_000 + tow_sub_ms as i64;
    let gps_ns = week_ns + tow_ns;

    // Convert to Unix timestamp (still in GPS time scale)
    let gps_in_unix_ns = GPS_EPOCH_UNIX_SECONDS * 1_000_000_000 + gps_ns;

    // Get the leap second offset for this time
    let leap_offset = get_leap_second_offset_ns(UnixNs(gps_in_unix_ns));

    // SUBTRACT leap seconds because GPS is ahead of UTC
    UnixNs(gps_in_unix_ns - leap_offset.0)
}

/// Returns the cumulative leap second offset in nanoseconds for a given Unix timestamp.
///
/// # Arguments
///
/// * `unix_ts` - A `UnixNs` timestamp (nanoseconds since Unix epoch) without leap second correction.
///
/// # Returns
///
/// A `UnixNs` representing the total leap second offset (in nanoseconds) that had occurred by the given timestamp.
///
/// # Example
///
/// ```
/// # use plotinator_log_if::leap_seconds::*;
/// let ts = UnixNs(1483228800 * 1_000_000_000); // Jan 1, 2017
/// let offset = get_leap_second_offset_ns(ts);
/// assert_eq!(offset.0, 18_000_000_000);
/// ```
pub fn get_leap_second_offset_ns(UnixNs(unix_ts): UnixNs) -> UnixNs {
    for &(leap_ts_s, cumulative_offset) in LEAP_SECONDS {
        if unix_ts >= leap_ts_s * 1_000_000_000 {
            return UnixNs(cumulative_offset * 1_000_000_000);
        }
    }
    UnixNs(0)
}

/// Newtype representing a GPS week number.
#[derive(Debug, Clone, Copy)]
pub struct GpsWeek(pub u16);

/// Newtype representing Time-of-Week in milliseconds.
#[derive(Debug, Clone, Copy)]
pub struct TowMs(pub u32);

/// Newtype representing fractional milliseconds within a TOW.
///
/// The actual units is nanoseconds
#[derive(Debug, Clone, Copy)]
pub struct TowSubMs(pub u32);

/// Newtype representing a Unix timestamp in nanoseconds.
#[derive(Debug, Clone, Copy)]
pub struct UnixNs(pub i64);
