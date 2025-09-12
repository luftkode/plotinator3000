use chrono::{DateTime, TimeZone as _, Utc};
use plotinator_log_if::prelude::*;
use std::io::{self, BufRead as _};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NjordInsPPP {
    raw_plots: Vec<RawPlot>,
    first_timestamp: DateTime<Utc>,
    metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
struct NjordInsPPPRow {
    timestamp: DateTime<Utc>,
    fix_type: f64,
    latitude: f64,
    longitude: f64,
    height: f64,
    lat_sd: f64,
    lon_sd: f64,
    height_sd: f64,
    vel_n: f64,
    vel_e: f64,
    vel_d: f64,
    vel_n_sd: f64,
    vel_e_sd: f64,
    vel_d_sd: f64,
    roll: f64,
    pitch: f64,
    heading: f64,
    roll_sd: f64,
    pitch_sd: f64,
    heading_sd: f64,
    acc_bias_x: f64,
    acc_bias_y: f64,
    acc_bias_z: f64,
    acc_bias_x_sd: f64,
    acc_bias_y_sd: f64,
    acc_bias_z_sd: f64,
    gyro_bias_x: f64,
    gyro_bias_y: f64,
    gyro_bias_z: f64,
    gyro_bias_x_sd: f64,
    gyro_bias_y_sd: f64,
    gyro_bias_z_sd: f64,
    gps_sats: f64,
    glonass_sats: f64,
    beidou_sats: f64,
    galileo_sats: f64,
}

/// A small helper to manage stateful parsing of fields from a single CSV row.
struct FieldParser<'a> {
    fields: &'a [&'a str],
    current_index: usize,
}

impl<'a> FieldParser<'a> {
    fn new(fields: &'a [&'a str]) -> Self {
        Self {
            fields,
            current_index: 1, // Start after "Human Timestamp"
        }
    }

    fn next<T>(&mut self) -> io::Result<T>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        let val = self
            .fields
            .get(self.current_index)
            .unwrap_or(&"")
            .parse::<T>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        self.current_index += 1;
        Ok(val)
    }
}

impl LogEntry for NjordInsPPPRow {
    fn from_reader(reader: &mut impl std::io::BufRead) -> std::io::Result<(Self, usize)> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "End of file"));
        }

        let fields: Vec<&str> = line.trim().split(',').collect();
        if fields.len() < 40 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Insufficient columns in CSV row",
            ));
        }

        let mut p = FieldParser::new(&fields);

        let unix_time: i64 = p.next()?;
        let microseconds: i64 = p.next()?;
        let timestamp = combine_timestamps(unix_time, microseconds);

        Ok((
            Self {
                timestamp,
                fix_type: p.next()?,
                latitude: p.next()?,
                longitude: p.next()?,
                height: p.next()?,
                lat_sd: p.next()?,
                lon_sd: p.next()?,
                height_sd: p.next()?,
                vel_n: p.next()?,
                vel_e: p.next()?,
                vel_d: p.next()?,
                vel_n_sd: p.next()?,
                vel_e_sd: p.next()?,
                vel_d_sd: p.next()?,
                roll: p.next()?,
                pitch: p.next()?,
                heading: p.next()?,
                roll_sd: p.next()?,
                pitch_sd: p.next()?,
                heading_sd: p.next()?,
                acc_bias_x: p.next()?,
                acc_bias_y: p.next()?,
                acc_bias_z: p.next()?,
                acc_bias_x_sd: p.next()?,
                acc_bias_y_sd: p.next()?,
                acc_bias_z_sd: p.next()?,
                gyro_bias_x: p.next()?,
                gyro_bias_y: p.next()?,
                gyro_bias_z: p.next()?,
                gyro_bias_x_sd: p.next()?,
                gyro_bias_y_sd: p.next()?,
                gyro_bias_z_sd: p.next()?,
                gps_sats: p.next()?,
                glonass_sats: p.next()?,
                beidou_sats: p.next()?,
                galileo_sats: p.next()?,
            },
            bytes_read,
        ))
    }

    fn timestamp_ns(&self) -> f64 {
        self.timestamp.timestamp_nanos_opt().expect("time overflow") as f64
    }
}

impl SkytemLog for NjordInsPPP {}

impl GitMetadata for NjordInsPPP {
    fn project_version(&self) -> Option<String> {
        None
    }
    fn git_short_sha(&self) -> Option<String> {
        None
    }
    fn git_branch(&self) -> Option<String> {
        None
    }
    fn git_repo_status(&self) -> Option<String> {
        None
    }
}

fn combine_timestamps(unix_time: i64, microseconds: i64) -> DateTime<Utc> {
    let combined = unix_time * 1_000_000_000 + microseconds * 1_000;
    Utc.timestamp_nanos(combined)
}

const EXPECTED_HEADER: &str = "Human Timestamp,Unix Time,Microseconds,Fix Type,Latitude,Longitude,Height,Latitude SD,Longitude SD,Height SD,Velocity North,Velocity East,Velocity Down,Velocity North SD,Velocity East SD,Velocity Down SD,Roll,Pitch,Heading,Roll SD,Pitch SD,Heading SD,Accelerometer Bias X,Accelerometer Bias Y,Accelerometer Bias Z,Accelerometer Bias X SD,Accelerometer Bias Y SD,Accelerometer Bias Z SD,Gyroscope Bias X,Gyroscope Bias Y,Gyroscope Bias Z,Gyroscope Bias X SD,Gyroscope Bias Y SD,Gyroscope Bias Z SD,GPS Satellites,GLONASS Satellites,BeiDou Satellites,Galileo Satellites,Differential GPS Satellites,Differential Glonass Satellites,Differential BeiDou Satellites,Differential Galileo Satellites,Dual Antenna Fix,Horizontal Separation,Vertical Separation,SBAS Satellites,Differential SBAS Satellites,Zero Velocity Update,Base to Rover North,Base to Rover East,Base to Rover Down,Base to Rover North SD,Base to Rover East SD,Base to Rover Down SD,Moving Base Fix Type,Event 1 Flag,Event 2 Flag";

impl Parseable for NjordInsPPP {
    const DESCRIPTIVE_NAME: &str = "Njord INS (PPP corrected)";

    #[allow(
        clippy::too_many_lines,
        reason = "As simple as can be. There's just a lot of columns"
    )]
    fn from_reader(reader: &mut impl std::io::BufRead) -> std::io::Result<(Self, usize)> {
        let mut header = String::new();
        let header_bytes_read = reader.read_line(&mut header)?;
        if header.trim() != EXPECTED_HEADER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("CSV header mismatch, expected: {EXPECTED_HEADER}"),
            ));
        }

        let (entries, entries_bytes_read) = parse_to_vec::<NjordInsPPPRow>(reader);
        let row_len = entries.len();

        let first_timestamp = entries.first().map(|e| e.timestamp).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "No data rows found in log")
        })?;

        let raw_plots = vec![
            // Attitude
            RawPlot::new(
                "Pitch °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.pitch),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Roll °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.roll),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Heading °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.heading),
                ExpectedPlotRange::OneToOneHundred,
            ),
            // Position
            RawPlot::new(
                "Latitude °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.latitude),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Longitude °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.longitude),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Height [m]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.height),
                ExpectedPlotRange::OneToOneHundred,
            ),
            // Velocity
            RawPlot::new(
                "Velocity North [m/s]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.vel_n),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Velocity East [m/s]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.vel_e),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Velocity Down [m/s]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.vel_d),
                ExpectedPlotRange::OneToOneHundred,
            ),
            // Standard Deviations
            RawPlot::new(
                "Latitude SD [m]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.lat_sd),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Longitude SD [m]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.lon_sd),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Height SD [m]".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.height_sd),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Roll SD °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.roll_sd),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Pitch SD °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.pitch_sd),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Heading SD °".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.heading_sd),
                ExpectedPlotRange::OneToOneHundred,
            ),
            // Biases
            RawPlot::new(
                "Accelerometer Bias X".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.acc_bias_x),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Accelerometer Bias Y".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.acc_bias_y),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Accelerometer Bias Z".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.acc_bias_z),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Gyroscope Bias X".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.gyro_bias_x),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Gyroscope Bias Y".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.gyro_bias_y),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Gyroscope Bias Z".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.gyro_bias_z),
                ExpectedPlotRange::OneToOneHundred,
            ),
            // GNSS
            RawPlot::new(
                "Fix Type".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.fix_type),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPS Satellites".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.gps_sats),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GLONASS Satellites".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.glonass_sats),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "BeiDou Satellites".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.beidou_sats),
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Galileo Satellites".into(),
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.galileo_sats),
                ExpectedPlotRange::OneToOneHundred,
            ),
        ];

        let total_bytes_read = header_bytes_read + entries_bytes_read;

        Ok((
            Self {
                raw_plots,
                first_timestamp,
                metadata: vec![("Dataset length".into(), row_len.to_string())],
            },
            total_bytes_read,
        ))
    }

    fn is_buf_valid(buf: &[u8]) -> bool {
        let mut reader = io::BufReader::new(buf);
        let mut header = String::new();
        if reader.read_line(&mut header).is_ok() {
            header.trim() == EXPECTED_HEADER
        } else {
            false
        }
    }
}

impl Plotable for NjordInsPPP {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }
    fn first_timestamp(&self) -> DateTime<Utc> {
        self.first_timestamp
    }
    fn descriptive_name(&self) -> &str {
        Self::DESCRIPTIVE_NAME
    }
    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }
    fn metadata(&self) -> Option<Vec<(String, String)>> {
        Some(self.metadata.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    const TEST_2_ROWS: &str = r#"Human Timestamp,Unix Time,Microseconds,Fix Type,Latitude,Longitude,Height,Latitude SD,Longitude SD,Height SD,Velocity North,Velocity East,Velocity Down,Velocity North SD,Velocity East SD,Velocity Down SD,Roll,Pitch,Heading,Roll SD,Pitch SD,Heading SD,Accelerometer Bias X,Accelerometer Bias Y,Accelerometer Bias Z,Accelerometer Bias X SD,Accelerometer Bias Y SD,Accelerometer Bias Z SD,Gyroscope Bias X,Gyroscope Bias Y,Gyroscope Bias Z,Gyroscope Bias X SD,Gyroscope Bias Y SD,Gyroscope Bias Z SD,GPS Satellites,GLONASS Satellites,BeiDou Satellites,Galileo Satellites,Differential GPS Satellites,Differential Glonass Satellites,Differential BeiDou Satellites,Differential Galileo Satellites,Dual Antenna Fix,Horizontal Separation,Vertical Separation,SBAS Satellites,Differential SBAS Satellites,Zero Velocity Update,Base to Rover North,Base to Rover East,Base to Rover Down,Base to Rover North SD,Base to Rover East SD,Base to Rover Down SD,Moving Base Fix Type,Event 1 Flag,Event 2 Flag
Tue Sep 09 18:37:32 CEST 2025,1757435852,60122,0,56.16089116138246,10.037770381937543,73.88893217076051,0.0038976466174703915,0.0029626628693197985,0.006090692312154209,-0.01181018737237851,-0.007867264187908507,0.004649026723151484,0.02324845065728795,0.02123606035625654,0.02533486231942722,-22.546345193354732,1.0883468942416055,293.7360522244404,0.17802519672798603,0.1813104963224544,0.7797202141260063,0.05232010939934284,0.04152445705865526,-0.050844753052972036,0.005864476873871117,0.007274185847221772,0.0024995735763677293,0.20439258839962995,0.09112531794196868,-0.025753284272798296,0.006779249411791834,0.00693205364414753,0.005840468393505123,0,0,0,0,0,0,0,0,0,4.0434749574595264E-4,1.3501383364200592E-5,0,0,0,-2963.7535666315816,-8827.15350052854,-43.93726188596338,0,0,0,0,0,0
Tue Sep 09 18:37:32 CEST 2025,1757435852,80242,0,56.16089115927095,10.037770379401547,73.88883762837278,0.0035813104392463085,0.0026806454183981787,0.005751248197028137,-0.01215548184143171,-0.007924811390479958,0.0047504851703400255,0.02274096355910464,0.02066690165873292,0.025111764097004625,-22.546600143317384,1.0897117108317118,293.7357925781802,0.17797662417238783,0.1812639579877422,0.7796308562295896,0.05232006941995765,0.04152442694905534,-0.05084477765025472,0.005864465275108637,0.007274171858246212,0.002499569416929948,0.20439249894879927,0.09112522988877947,-0.02575287812937469,0.006779239410370064,0.006932043615748095,0.005840467336356259,0,0,0,0,0,0,0,0,0,4.099859875968871E-4,4.146154969930649E-5,0,0,1,-2963.7538013700396,-8827.153658453375,-43.93735675420612,0,0,0,0,0,0"#;

    #[test]
    fn test_read_csv() {
        let mut reader = BufReader::new(TEST_2_ROWS.as_bytes());
        let (njord_ins, read_bytes) = NjordInsPPP::from_reader(&mut reader).unwrap();

        insta::assert_debug_snapshot!(njord_ins);
        insta::assert_debug_snapshot!(read_bytes);
    }
}
