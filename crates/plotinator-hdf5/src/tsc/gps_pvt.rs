use chrono::{DateTime, Utc};
use hdf5::H5Type;
use ndarray::{ArrayBase, Dim, OwnedRepr};
use plotinator_log_if::prelude::{ExpectedPlotRange, RawPlot};

type GpsPvts = ArrayBase<OwnedRepr<GpsPvtRecord>, Dim<[usize; 1]>>;

/// Wrapper around all the [`GpsPvtRecord`]s from a TSC.h5 file
pub(crate) struct GpsPvtRecords {
    inner: GpsPvts,
}

impl GpsPvtRecords {
    const DATASET_NAME: &str = "GPS_PVT";

    pub fn from_hdf5(h5: &hdf5::File) -> hdf5::Result<Self> {
        let dataset = h5.dataset(Self::DATASET_NAME)?;
        let gps_pvts = dataset.read::<GpsPvtRecord, ndarray::Ix1>()?;
        Ok(Self { inner: gps_pvts })
    }

    // Return a vector of timestamps (unix UTC nanoseconds)
    pub fn timestamps(&self) -> Vec<f64> {
        self.inner
            .iter()
            .map(|g| g.unix_timestamp_ns() as f64)
            .collect()
    }

    #[allow(clippy::too_many_lines, reason = "Long but simple")]
    pub fn build_plots(&self) -> Vec<RawPlot> {
        let time = self.timestamps();

        let mut numsv = Vec::with_capacity(time.len());
        let mut height = Vec::with_capacity(time.len());
        let mut h_msl = Vec::with_capacity(time.len());
        let mut gspeed = Vec::with_capacity(time.len());
        let mut heading = Vec::with_capacity(time.len());
        let mut hacc = Vec::with_capacity(time.len());
        let mut vacc = Vec::with_capacity(time.len());
        let mut sacc = Vec::with_capacity(time.len());
        let mut pdop = Vec::with_capacity(time.len());
        let mut lat_deg = Vec::with_capacity(time.len());
        let mut lon_deg = Vec::with_capacity(time.len());
        let mut vel_n = Vec::with_capacity(time.len());
        let mut vel_e = Vec::with_capacity(time.len());
        let mut vel_d = Vec::with_capacity(time.len());
        let mut mag_dec = Vec::with_capacity(time.len());

        for (e, &t) in self.inner.iter().zip(&time) {
            numsv.push([t, e.num_sv as f64]);
            height.push([t, e.height_meters()]);
            h_msl.push([t, e.height_msl_meters()]);
            gspeed.push([t, e.ground_speed_ms()]);
            heading.push([t, e.head_mot as f64 * 1e-5]); // degrees
            hacc.push([t, e.h_acc as f64 * 1e-3]); // mm → m
            vacc.push([t, e.v_acc as f64 * 1e-3]); // mm → m
            sacc.push([t, e.s_acc as f64 * 1e-3]); // mm/s → m/s
            pdop.push([t, e.p_dop as f64 * 0.01]);
            lat_deg.push([t, e.latitude_degrees()]);
            lon_deg.push([t, e.longitude_degrees()]);
            vel_n.push([t, e.vel_n as f64 * 1e-3]); // mm/s → m/s
            vel_e.push([t, e.vel_e as f64 * 1e-3]); // mm/s → m/s
            vel_d.push([t, e.vel_d as f64 * 1e-3]); // mm/s → m/s
            mag_dec.push([t, e.mag_dec as f64 * 1e-2]); // degrees * 1e-2 → degrees
        }

        vec![
            RawPlot::new(
                "Satellites".to_owned(),
                numsv,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Height [ellipsoid, m]".to_owned(),
                height,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "Height [MSL, m]".to_owned(),
                h_msl,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "Ground speed [m/s]".to_owned(),
                gspeed,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Heading [deg]".to_owned(),
                heading,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Horizontal accuracy [m]".to_owned(),
                hacc,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "Vertical accuracy [m]".to_owned(),
                vacc,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "Speed accuracy [m/s]".to_owned(),
                sacc,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Position DOP".to_owned(),
                pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            // New RawPlot instances
            RawPlot::new(
                "Latitude [deg]".to_owned(),
                lat_deg,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Longitude [deg]".to_owned(),
                lon_deg,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Velocity North [m/s]".to_owned(),
                vel_n,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Velocity East [m/s]".to_owned(),
                vel_e,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Velocity Down [m/s]".to_owned(),
                vel_d,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Magnetic Declination [deg]".to_owned(),
                mag_dec,
                ExpectedPlotRange::OneToOneHundred, // Typically within ±30 degrees
            ),
        ]
    }
}

#[derive(H5Type, Debug)]
#[repr(C)]
struct GpsPvtRecord {
    /// GPS time of week of navigation epoch (milliseconds)
    #[hdf5(rename = "iTOW")]
    i_tow: u32,
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    min: u8,
    sec: u8,
    #[hdf5(rename = "validFlag")]
    valid_flag: u8,
    /// Time accuracy estimate (nanoseconds)
    #[hdf5(rename = "tAcc")]
    t_acc: u32,
    /// Fraction of second, range -1e9..1e9 (UTC) (nanoseconds)
    nano: i32,
    /// GNSS fix Type
    #[hdf5(rename = "fixType")]
    fix_type: u8,
    /// Fix status flags
    flags: u8,
    /// Additional flags
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
    /// Heading accuracy estimate (degrees * 1e-5)
    #[hdf5(rename = "headAcc")]
    head_acc: u32,

    /// Position DOP (dilution of precision) * 0.01
    #[hdf5(rename = "pDOP")]
    p_dop: u16,

    reserved01: u8,
    reserved02: u8,
    reserved03: u8,
    reserved04: u8,
    reserved05: u8,
    reserved06: u8,

    /// Heading of vehicle (2-D) (degrees * 1e-5)
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
    fn unix_timestamp_utc(&self) -> DateTime<Utc> {
        // Build datetime from explicit UTC fields - this must succeed or crash
        let naive_date =
            chrono::NaiveDate::from_ymd_opt(self.year as i32, self.month as u32, self.day as u32)
                .expect("Invalid date in GPS PVT record");

        let naive_datetime = naive_date
            .and_hms_opt(self.hour as u32, self.min as u32, self.sec as u32)
            .expect("Invalid time in GPS PVT record");

        let mut utc_datetime = naive_datetime.and_utc();

        // Apply nanosecond correction (could be timing offset/correction)
        if self.nano != 0 {
            utc_datetime += chrono::Duration::nanoseconds(self.nano as i64);
        }

        utc_datetime
    }

    /// Convert PVT record to precise Unix timestamp in nanoseconds since Unix epoch
    /// Uses only the explicit UTC date/time fields - no fallbacks, crashes on invalid data
    fn unix_timestamp_ns(&self) -> i64 {
        self.unix_timestamp_utc()
            .timestamp_nanos_opt()
            .expect("invalid time")
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

    /// Get ground speed in m/s
    pub fn ground_speed_ms(&self) -> f64 {
        self.g_speed as f64 * 1e-3
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
            eprintln!("{}", g.unix_timestamp_utc());
            eprintln!(
                "Lat: {:.7}°, Lon: {:.7}°",
                g.latitude_degrees(),
                g.longitude_degrees()
            );
            eprintln!(
                "Height: {:.3}m, Speed: {:.3}m/s",
                g.height_meters(),
                g.ground_speed_ms()
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
            record.unix_timestamp_utc(),
            NaiveDate::from_ymd_opt(2022, 3, 10)
                .unwrap()
                .and_hms_nano_opt(15, 30, 45, 123456789)
                .unwrap()
                .and_utc()
        );
    }
}
