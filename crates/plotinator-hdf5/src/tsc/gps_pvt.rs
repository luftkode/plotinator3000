use chrono::{DateTime, Utc};
use hdf5::H5Type;
use ndarray::Array1;
use plotinator_log_if::prelude::*;
use plotinator_ui_util::ExpectedPlotRange;

use crate::tsc::TSC_LEGEND_NAME;

/// Wrapper around all the [`GpsPvtRecord`]s from a TSC.h5 file
pub(crate) struct GpsPvtRecords {
    inner: Array1<GpsPvtRecord>,
}

impl GpsPvtRecords {
    const DATASET_NAME: &str = "GPS_PVT";

    pub fn from_hdf5(h5: &hdf5::File) -> hdf5::Result<Self> {
        let dataset = h5.dataset(Self::DATASET_NAME)?;
        let gps_pvts = dataset.read::<GpsPvtRecord, ndarray::Ix1>()?;
        Ok(Self { inner: gps_pvts })
    }

    // Return a vector of timestamps (unix UTC nanoseconds)
    //
    pub fn timestamps(&self) -> Vec<Option<f64>> {
        self.inner
            .iter()
            .map(|g| g.unix_timestamp_ns().map(|ts| ts as f64))
            .collect()
    }

    #[allow(clippy::too_many_lines, reason = "Long but simple")]
    pub fn build_plots(&self) -> Vec<RawPlot> {
        let time = self.timestamps();

        let mut timestamps = Vec::with_capacity(time.len());
        let mut height = Vec::with_capacity(time.len());
        let mut gspeed = Vec::with_capacity(time.len());
        let mut heading = Vec::with_capacity(time.len());
        let mut lat_deg = Vec::with_capacity(time.len());
        let mut lon_deg = Vec::with_capacity(time.len());

        let mut h_msl = Vec::with_capacity(time.len());
        let mut h_elips = Vec::with_capacity(time.len());
        let mut numsv = Vec::with_capacity(time.len());
        let mut hacc = Vec::with_capacity(time.len());
        let mut vacc = Vec::with_capacity(time.len());
        let mut sacc = Vec::with_capacity(time.len());
        let mut pdop = Vec::with_capacity(time.len());
        let mut vel_n = Vec::with_capacity(time.len());
        let mut vel_e = Vec::with_capacity(time.len());
        let mut vel_d = Vec::with_capacity(time.len());
        let mut mag_dec = Vec::with_capacity(time.len());

        // Valid flags
        let mut valid_date = Vec::with_capacity(time.len());
        let mut valid_time = Vec::with_capacity(time.len());
        let mut valid_fully_resolved = Vec::with_capacity(time.len());
        let mut valid_mag = Vec::with_capacity(time.len());

        // Status flags
        let mut gnss_fix_ok = Vec::with_capacity(time.len());
        let mut diff_soln = Vec::with_capacity(time.len());
        let mut head_veh_valid = Vec::with_capacity(time.len());

        // Additional flags
        let mut confirmed_avail = Vec::with_capacity(time.len());
        let mut confirmed_date = Vec::with_capacity(time.len());
        let mut confirmed_time = Vec::with_capacity(time.len());

        let mut default_hide_valid_date = true;
        let mut default_hide_valid_time = true;
        let mut default_hide_valid_fully_resolved = true;
        let mut default_hide_valid_mag = true;
        let mut default_hide_gnss_fix_ok = true;
        let mut default_hide_diff_soln = true;
        let mut default_hide_head_veh_valid = true;
        let mut default_hide_confirmed_avail = true;
        let mut default_hide_confirmed_date = true;
        let mut default_hide_confirmed_time = true;

        let mut prev_valid_date: Option<f64> = None;
        let mut prev_valid_time: Option<f64> = None;
        let mut prev_valid_fully_resolved: Option<f64> = None;
        let mut prev_valid_mag: Option<f64> = None;
        let mut prev_gnss_fix_ok: Option<f64> = None;
        let mut prev_diff_soln: Option<f64> = None;
        let mut prev_head_veh_valid: Option<f64> = None;
        let mut prev_confirmed_avail: Option<f64> = None;
        let mut prev_confirmed_date: Option<f64> = None;
        let mut prev_confirmed_time: Option<f64> = None;

        for (e, &timestamp_opt) in self.inner.iter().zip(&time) {
            let Some(t) = timestamp_opt else {
                log::warn!("Ignoring NAV-PVT data from invalid timestamp");
                continue;
            };
            timestamps.push(t);
            height.push(e.height_meters());
            gspeed.push(e.ground_speed_kmh());
            heading.push(e.head_mot as f64 * 1e-5); // degrees
            lat_deg.push(e.latitude_degrees());
            lon_deg.push(e.longitude_degrees());

            numsv.push([t, e.num_sv as f64]);
            h_msl.push([t, e.height_msl_meters()]);
            h_elips.push([t, e.height_meters()]);
            hacc.push([t, e.h_acc as f64 * 1e-3]); // mm -> m
            vacc.push([t, e.v_acc as f64 * 1e-3]); // mm -> m
            sacc.push([t, e.s_acc as f64 * 3.6e-3]); // mm/s -> km/h
            pdop.push([t, e.p_dop as f64 * 0.01]);
            vel_n.push([t, e.vel_n as f64 * 3.6e-3]); // mm/s -> km/h
            vel_e.push([t, e.vel_e as f64 * 3.6e-3]); // mm/s -> km/h
            vel_d.push([t, e.vel_d as f64 * 3.6e-3]); // mm/s -> km/h
            mag_dec.push([t, e.mag_dec as f64 * 1e-2]); // degrees * 1e-2 -> degrees

            // Extract valid flags
            let v_valid_date = e.flag_valid(GpsPvtRecord::VALID_DATE);
            let v_valid_time = e.flag_valid(GpsPvtRecord::VALID_TIME);
            let v_valid_fully_resolved = e.flag_valid(GpsPvtRecord::VALID_FULLY_RESOLVED);
            let v_valid_mag = e.flag_valid(GpsPvtRecord::VALID_MAG);
            // Extract status flags
            let v_gnss_fix_ok = e.flag_status(GpsPvtRecord::FLAGS_GNSS_FIX_OK);
            let v_diff_soln = e.flag_status(GpsPvtRecord::FLAGS_DIFF_SOLN);
            let v_head_veh_valid = e.flag_status(GpsPvtRecord::FLAGS_HEAD_VEH_VALID);
            // Extract additional flags
            let v_confirmed_avail = e.flag_additional(GpsPvtRecord::FLAGS2_CONFIRMED_AVAILABLE);
            let v_confirmed_date = e.flag_additional(GpsPvtRecord::FLAGS2_CONFIRMED_DATE);
            let v_confirmed_time = e.flag_additional(GpsPvtRecord::FLAGS2_CONFIRMED_TIME);

            valid_date.push([t, v_valid_date]);
            valid_time.push([t, v_valid_time]);
            valid_fully_resolved.push([t, v_valid_fully_resolved]);
            valid_mag.push([t, v_valid_mag]);
            gnss_fix_ok.push([t, v_gnss_fix_ok]);
            diff_soln.push([t, v_diff_soln]);
            head_veh_valid.push([t, v_head_veh_valid]);
            confirmed_avail.push([t, v_confirmed_avail]);
            confirmed_date.push([t, v_confirmed_date]);
            confirmed_time.push([t, v_confirmed_time]);

            macro_rules! detect_change {
                ($prev:ident, $curr:expr, $flag:ident) => {{
                    if $flag {
                        if let Some(p) = $prev {
                            if (p - $curr).abs() > 0.1 {
                                $flag = false;
                            }
                        }
                        $prev = Some($curr);
                    }
                }};
            }

            detect_change!(prev_valid_date, v_valid_date, default_hide_valid_date);
            detect_change!(prev_valid_time, v_valid_time, default_hide_valid_time);
            detect_change!(
                prev_valid_fully_resolved,
                v_valid_fully_resolved,
                default_hide_valid_fully_resolved
            );
            detect_change!(prev_valid_mag, v_valid_mag, default_hide_valid_mag);
            detect_change!(prev_gnss_fix_ok, v_gnss_fix_ok, default_hide_gnss_fix_ok);
            detect_change!(prev_diff_soln, v_diff_soln, default_hide_diff_soln);
            detect_change!(
                prev_head_veh_valid,
                v_head_veh_valid,
                default_hide_head_veh_valid
            );
            detect_change!(
                prev_confirmed_avail,
                v_confirmed_avail,
                default_hide_confirmed_avail
            );
            detect_change!(
                prev_confirmed_date,
                v_confirmed_date,
                default_hide_confirmed_date
            );
            detect_change!(
                prev_confirmed_time,
                v_confirmed_time,
                default_hide_confirmed_time
            );
        }

        let geo_data: Option<RawPlot> = GeoSpatialDataBuilder::new(TSC_LEGEND_NAME.to_owned())
            .timestamp(&timestamps)
            .lat(&lat_deg)
            .lon(&lon_deg)
            .altitude_from_gnss(height)
            .speed(&gspeed)
            .heading(&heading)
            .build_into_rawplot()
            .expect("invalid builder");

        #[rustfmt::skip]
        let plots = RawPlotBuilder::new(TSC_LEGEND_NAME)
            .add(numsv, DataType::other_unitless("Satellites", ExpectedPlotRange::Hundreds, false))
            .add(h_msl, DataType::AltitudeMSL)
            .add(h_elips, DataType::AltitudeEllipsoidal)
            .add(hacc, DataType::other_distance("Horizontal acc", true))
            .add(vacc, DataType::other_distance("Vertical acc", true))
            .add(sacc, DataType::other_velocity("Speed acc", true))
            .add(pdop, DataType::other_unitless("PDOP", ExpectedPlotRange::Hundreds, true))
            .add(vel_n, DataType::other_velocity("Velocity North", true))
            .add(vel_e, DataType::other_velocity("Velocity East", true))
            .add(vel_d, DataType::other_velocity("Velocity Down", true))
            .add(mag_dec, DataType::other_degrees("Magnetic Decl.", true))
            .add(valid_date, DataType::bool("Valid Date", default_hide_valid_date))
            .add(valid_time, DataType::bool("Valid Time", default_hide_valid_time))
            .add(valid_fully_resolved, DataType::bool("Valid fully resolved", default_hide_valid_fully_resolved))
            .add(valid_mag, DataType::bool("Valid Magnetic Declination", default_hide_valid_mag))
            .add(gnss_fix_ok, DataType::bool("GNSS Fix OK", default_hide_gnss_fix_ok))
            .add(diff_soln, DataType::bool("Differential Solution", default_hide_diff_soln))
            .add(head_veh_valid, DataType::bool("Head Vehicle Valid", default_hide_head_veh_valid))
            .add(confirmed_avail, DataType::bool("Confirmed Available", default_hide_confirmed_avail))
            .add(confirmed_date, DataType::bool("Confirmed Date", default_hide_confirmed_date))
            .add(confirmed_time, DataType::bool("Confirmed Time", default_hide_confirmed_time));
        let mut plots = plots.build();

        if let Some(geo_data) = geo_data {
            plots.push(geo_data);
        }
        plots
    }
}
/// UBX-NAV-PVT message payload structure
#[derive(H5Type, Debug)]
#[repr(C)]
struct GpsPvtRecord {
    /// GPS time of week of navigation epoch (milliseconds)
    #[hdf5(rename = "iTOW")]
    i_tow: u32,

    /// Year (UTC)
    year: u16,
    /// Month (UTC), range 1-12
    month: u8,
    /// Day of month (UTC), range 1-31
    day: u8,
    /// Hour of day (UTC), range 0-23
    hour: u8,
    /// Minute of hour (UTC), range 0-59
    min: u8,
    /// Seconds of minute (UTC), range 0-60 (60 for leap seconds)
    sec: u8,

    /// Validity flags
    /// Bit 0 (`VALID_DATE)`: valid UTC date
    /// Bit 1 (`VALID_TIME)`: valid UTC time of day
    /// Bit 2 (`VALID_FULLY_RESOLVED)`: UTC time fully resolved (no seconds uncertainty)
    /// Bit 3 (`VALID_MAG)`: valid magnetic declination
    #[hdf5(rename = "validFlag")]
    valid_flag: u8,

    /// Time accuracy estimate (nanoseconds)
    #[hdf5(rename = "tAcc")]
    t_acc: u32,

    /// Fraction of second, range -1e9..1e9 (UTC) (nanoseconds)
    nano: i32,

    /// GNSS fix type, range 0-5
    /// 0: No fix
    /// 1: Dead reckoning only
    /// 2: 2D-fix (signal from only 3 SVs, constant altitude assumed)
    /// 3: 3D-fix
    /// 4: GNSS + dead reckoning combined
    /// 5: Time only fix (high precision devices)
    #[hdf5(rename = "fixType")]
    fix_type: u8,

    /// Fix status flags
    /// Bit 0 (`FLAGS_GNSS_FIX_OK)`: valid fix (within DOP & accuracy masks)
    /// Bit 1 (`FLAGS_DIFF_SOLN)`: DGPS used
    /// Bits 2-4 (`FLAGS_PSM_MASK)`: Power save mode state
    /// Bit 5 (`FLAGS_HEAD_VEH_VALID)`: heading of vehicle is valid
    /// Bits 6-7 (`FLAGS_CARRIER_PHASE_MASK)`: Carrier phase solution status
    flags: u8,

    /// Additional flags
    /// Bit 5 (`FLAGS2_CONFIRMED_AVAILABLE)`: UTC Date/Time validity confirmation available
    /// Bit 6 (`FLAGS2_CONFIRMED_DATE)`: UTC Date validity confirmed
    /// Bit 7 (`FLAGS2_CONFIRMED_TIME)`: UTC Time of Day validity confirmed
    flags2: u8,

    /// Number of satellites used in navigation solution
    #[hdf5(rename = "numSV")]
    num_sv: u8,

    /// Longitude (degrees * 1e-7)
    lon: i32,
    /// Latitude (degrees * 1e-7)
    lat: i32,
    /// Height above ellipsoid (millimeters)
    height: i32,
    /// Height above mean sea level (millimeters)
    #[hdf5(rename = "hMSL")]
    h_msl: i32,

    /// Horizontal accuracy estimate (millimeters)
    #[hdf5(rename = "hAcc")]
    h_acc: u32,
    /// Vertical accuracy estimate (millimeters)
    #[hdf5(rename = "vAcc")]
    v_acc: u32,

    /// NED north velocity (millimeters/second)
    #[hdf5(rename = "velN")]
    vel_n: i32,
    /// NED east velocity (millimeters/second)
    #[hdf5(rename = "velE")]
    vel_e: i32,
    /// NED down velocity (millimeters/second)
    #[hdf5(rename = "velD")]
    vel_d: i32,

    /// Ground speed (2-D) (millimeters/second)
    #[hdf5(rename = "gSpeed")]
    g_speed: i32,

    /// Heading of motion (2-D) (degrees * 1e-5)
    #[hdf5(rename = "headMot")]
    head_mot: i32,

    /// Speed accuracy estimate (millimeters/second)
    #[hdf5(rename = "sAcc")]
    s_acc: u32,

    /// Heading accuracy estimate (both motion & vehicle) (degrees * 1e-5)
    #[hdf5(rename = "headAcc")]
    head_acc: u32,

    /// Position DOP (dilution of precision) (1 / 0.01)
    #[hdf5(rename = "pDOP")]
    p_dop: u16,

    reserved01: u8,
    reserved02: u8,
    reserved03: u8,
    reserved04: u8,
    reserved05: u8,
    reserved06: u8,

    /// Heading of vehicle (2-D) (degrees * 1e-5)
    /// Vehicle orientation, different from headMot (direction of motion)
    #[hdf5(rename = "headVeh")]
    head_veh: i32,

    /// Magnetic declination (degrees * 1e-2)
    #[hdf5(rename = "magDec")]
    mag_dec: i16,
    /// Magnetic declination accuracy (degrees * 1e-2)
    #[hdf5(rename = "magAcc")]
    mag_acc: u16,
}

impl GpsPvtRecord {
    // Valid flag constants
    const VALID_DATE: u8 = 1;
    const VALID_TIME: u8 = 2;
    const VALID_FULLY_RESOLVED: u8 = 4;
    const VALID_MAG: u8 = 8;

    // Status flag constants
    const FLAGS_GNSS_FIX_OK: u8 = 1;
    const FLAGS_DIFF_SOLN: u8 = 2;
    const FLAGS_HEAD_VEH_VALID: u8 = 32;

    // Additional flag constants
    const FLAGS2_CONFIRMED_AVAILABLE: u8 = 32;
    const FLAGS2_CONFIRMED_DATE: u8 = 64;
    const FLAGS2_CONFIRMED_TIME: u8 = 128;

    fn unix_timestamp_utc(&self) -> Option<DateTime<Utc>> {
        let Some(naive_date) =
            chrono::NaiveDate::from_ymd_opt(self.year as i32, self.month as u32, self.day as u32)
        else {
            log::error!("Invalid date: {self:?}");
            return None;
        };

        let Some(naive_datetime) =
            naive_date.and_hms_opt(self.hour as u32, self.min as u32, self.sec as u32)
        else {
            log::error!("Invalid time: {self:?}");
            return None;
        };

        let mut utc_datetime = naive_datetime.and_utc();

        // Apply nanosecond correction (could be timing offset/correction)
        if self.nano != 0 {
            utc_datetime += chrono::Duration::nanoseconds(self.nano as i64);
        }

        Some(utc_datetime)
    }

    /// Convert PVT record to precise Unix timestamp in nanoseconds since Unix epoch
    fn unix_timestamp_ns(&self) -> Option<i64> {
        self.unix_timestamp_utc()
            .and_then(|ts| ts.timestamp_nanos_opt())
    }

    /// Get latitude in degrees
    pub fn latitude_degrees(&self) -> f64 {
        self.lat as f64 * 1e-7
    }

    /// Get longitude in degrees
    pub fn longitude_degrees(&self) -> f64 {
        self.lon as f64 * 1e-7
    }

    /// Get height above ellipsoid in meters
    pub fn height_meters(&self) -> f64 {
        self.height as f64 * 1e-3
    }

    /// Get height above mean sea level in meters
    pub fn height_msl_meters(&self) -> f64 {
        self.h_msl as f64 * 1e-3
    }

    /// Get ground speed in km/h
    pub fn ground_speed_kmh(&self) -> f64 {
        self.g_speed as f64 * 3.6e-3
    }

    /// Extract flag from `valid_flag` field as float (0.0 or 1.0)
    fn flag_valid(&self, mask: u8) -> f64 {
        if self.valid_flag & mask != 0 {
            1.0
        } else {
            0.0
        }
    }

    /// Extract flag from flags field as float (0.0 or 1.0)
    fn flag_status(&self, mask: u8) -> f64 {
        if self.flags & mask != 0 { 1.0 } else { 0.0 }
    }

    /// Extract flag from flags2 field as float (0.0 or 1.0)
    fn flag_additional(&self, mask: u8) -> f64 {
        if self.flags2 & mask != 0 { 1.0 } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use plotinator_test_util::test_file_defs::tsc::*;
    use testresult::TestResult;

    #[test]
    fn read_gps_pvt() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let gps_pvt = GpsPvtRecords::from_hdf5(&h5file)?;
        let gps_pvt = gps_pvt.inner;
        for g in &gps_pvt {
            eprintln!("g: {g:?}");
            eprintln!("{:?}", g.unix_timestamp_utc());
            eprintln!(
                "Lat: {:.7}°, Lon: {:.7}°",
                g.latitude_degrees(),
                g.longitude_degrees()
            );
            eprintln!(
                "Height: {:.3}m, Speed: {:.3}m/s",
                g.height_meters(),
                g.ground_speed_kmh()
            );
        }

        insta::assert_debug_snapshot!(gps_pvt);
        Ok(())
    }

    #[test]
    fn read_gps_pvt_from_stub() -> TestResult {
        let h5file = hdf5::File::open(tsc_stub())?;
        let gps_pvt = GpsPvtRecords::from_hdf5(&h5file)?;
        let gps_pvt = gps_pvt.inner;
        for g in &gps_pvt {
            eprintln!("g: {g:?}");
            eprintln!("{:?}", g.unix_timestamp_utc());
            eprintln!(
                "Lat: {:.7}°, Lon: {:.7}°",
                g.latitude_degrees(),
                g.longitude_degrees()
            );
            eprintln!(
                "Height: {:.3}m, Speed: {:.3}m/s",
                g.height_meters(),
                g.ground_speed_kmh()
            );
        }

        insta::assert_debug_snapshot!(gps_pvt);
        Ok(())
    }

    #[test]
    fn test_timestamp_conversion() {
        let record = GpsPvtRecord {
            i_tow: 345600000, // 4 days into week in milliseconds
            year: 2022,
            month: 3,
            day: 10,
            hour: 15,
            min: 30,
            sec: 45,
            valid_flag: 0x07, // Valid date/time
            t_acc: 100,
            nano: 123456789, // 123.456789 milliseconds additional nanoseconds
            fix_type: 3,
            flags: 0,
            flags2: 0,
            num_sv: 12,
            lon: 550_000_000, // 55.0 degrees * 1e7
            lat: 125_000_000, // 12.5 degrees * 1e7
            height: 100_000,  // 100m * 1e3
            h_msl: 50_000,    // 50m * 1e3
            h_acc: 2000,
            v_acc: 3000,
            vel_n: 1000,     // 1 m/s * 1e3
            vel_e: 500,      // 0.5 m/s * 1e3
            vel_d: -200,     // -0.2 m/s * 1e3
            g_speed: 2000,   // 2 m/s * 1e3
            head_mot: 90000, // 90 degrees * 1e3
            s_acc: 100,
            head_acc: 5000,
            p_dop: 180, // 1.8 * 100
            reserved01: 0,
            reserved02: 0,
            reserved03: 0,
            reserved04: 0,
            reserved05: 0,
            reserved06: 0,
            head_veh: 90000,
            mag_dec: 0,
            mag_acc: 0,
        };

        // Verify the expected datetime
        assert_eq!(
            record.unix_timestamp_utc().unwrap(),
            NaiveDate::from_ymd_opt(2022, 3, 10)
                .unwrap()
                .and_hms_nano_opt(15, 30, 45, 123456789)
                .unwrap()
                .and_utc()
        );
    }
}
