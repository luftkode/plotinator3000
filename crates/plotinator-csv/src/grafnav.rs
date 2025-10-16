use anyhow::bail;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone as _, Utc};
use plotinator_log_if::{
    prelude::*,
    rawplot::{DataType, RawPlotBuilder},
};
use plotinator_ui_util::ExpectedPlotRange;
use std::io::{self, BufRead as _};

const LEGEND_NAME: &str = "GrafNav-PPP";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrafNavPPP {
    raw_plots: Vec<RawPlot>,
    first_timestamp: DateTime<Utc>,
    metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
struct GrafNavPPPRow {
    seq_num: i32,
    timestamp: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    h_msl: f64,
    northing: f64,
    easting: f64,
    quality: i32,
    undulation: f64,
    pdop: f64,
    h_ell: f64,
    num_satellites: i32,
    cog: f64, // course over ground (heading)
    v_east: f64,
    v_north: f64,
    v_up: f64,
    hz_speed: f64,
}

impl LogEntry for GrafNavPPPRow {
    fn from_reader(reader: &mut impl std::io::BufRead) -> std::io::Result<(Self, usize)> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "End of file"));
        }

        let line = line.trim();
        if line.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Empty line"));
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 17 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Insufficient columns in row: expected 17, got {}",
                    fields.len()
                ),
            ));
        }

        let seq_num: i32 = fields[0].parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid seq_num: {e}"))
        })?;

        // Parse date (YYYY/MM/DD)
        let date_str = fields[1];
        let date = NaiveDate::parse_from_str(date_str, "%Y/%m/%d").map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid date format: {e}"),
            )
        })?;

        // Parse time (HH:MM:SS.SS)
        let time_str = fields[2];
        let time = NaiveTime::parse_from_str(time_str, "%H:%M:%S%.f").map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid time format: {e}"),
            )
        })?;

        let timestamp = Utc.from_utc_datetime(&date.and_time(time));

        let parse_field = |index: usize, name: &str| -> io::Result<f64> {
            fields[index].parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Invalid {name}: {e}"))
            })
        };

        let parse_int_field = |index: usize, name: &str| -> io::Result<i32> {
            fields[index].parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Invalid {name}: {e}"))
            })
        };

        Ok((
            Self {
                seq_num,
                timestamp,
                latitude: parse_field(3, "latitude")?,
                longitude: parse_field(4, "longitude")?,
                h_msl: parse_field(5, "h_msl")?,
                northing: parse_field(6, "northing")?,
                easting: parse_field(7, "easting")?,
                quality: parse_int_field(8, "quality")?,
                undulation: parse_field(9, "undulation")?,
                pdop: parse_field(10, "pdop")?,
                h_ell: parse_field(11, "h_ell")?,
                num_satellites: parse_int_field(12, "num_satellites")?,
                cog: parse_field(13, "cog")?,
                v_east: parse_field(14, "v_east")?,
                v_north: parse_field(15, "v_north")?,
                v_up: parse_field(16, "v_up")?,
                hz_speed: parse_field(17, "hz_speed")?,
            },
            bytes_read,
        ))
    }

    fn timestamp_ns(&self) -> f64 {
        self.timestamp.timestamp_nanos_opt().expect("time overflow") as f64
    }
}

impl SkytemLog for GrafNavPPP {}

impl GitMetadata for GrafNavPPP {
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

fn parse_metadata_line(line: &str) -> Option<(String, String)> {
    if let Some(colon_pos) = line.find(':') {
        let key = line[..colon_pos].trim().to_owned();
        let value = line[colon_pos + 1..].trim().to_owned();
        Some((key, value))
    } else {
        None
    }
}

impl Parseable for GrafNavPPP {
    const DESCRIPTIVE_NAME: &str = "GrafNav PPP";

    #[allow(clippy::too_many_lines, reason = "Long but simple")]
    fn from_reader(reader: &mut impl std::io::BufRead) -> anyhow::Result<(Self, usize)> {
        let mut total_bytes_read = 0;
        let mut metadata = Vec::new();
        let mut line = String::new();

        // Parse metadata section
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;
            total_bytes_read += bytes_read;

            if bytes_read == 0 {
                bail!(
                    "Not a valid '{}': Unexpected end of file while reading metadata",
                    Self::DESCRIPTIVE_NAME
                );
            }

            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Check if this is the start of the column header (starts with "SeqNum")
            if trimmed.starts_with("SeqNum") {
                break;
            }

            // Parse metadata lines
            if let Some((key, value)) = parse_metadata_line(trimmed) {
                metadata.push((key, value));
            }
        }

        // Skip the units line (second header line)
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        total_bytes_read += bytes_read;

        // Parse data rows
        let (entries, entries_bytes_read) = parse_to_vec::<GrafNavPPPRow>(reader);
        total_bytes_read += entries_bytes_read;

        let row_len = entries.len();

        let first_timestamp = entries.first().map(|e| e.timestamp).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "No data rows found in log")
        })?;

        metadata.push(("Dataset length".into(), row_len.to_string()));

        let mut timestamps = Vec::with_capacity(entries.len());
        let mut latitude = Vec::with_capacity(entries.len());
        let mut longitude = Vec::with_capacity(entries.len());
        let mut altitude = Vec::with_capacity(entries.len());
        let mut speed = Vec::with_capacity(entries.len());
        let mut heading = Vec::with_capacity(entries.len());

        for e in &entries {
            timestamps.push(e.timestamp_ns());
            latitude.push(e.latitude);
            longitude.push(e.longitude);
            altitude.push(e.h_ell);
            speed.push(e.hz_speed);
            heading.push(e.cog);
        }

        let geo_data: Option<RawPlot> = GeoSpatialDataBuilder::new(LEGEND_NAME)
            .timestamp(&timestamps)
            .lon(&longitude)
            .lat(&latitude)
            .altitude_from_gnss(altitude)
            .speed(&speed)
            .heading(&heading)
            .build_into_rawplot()
            .expect("invalid builder");

        let mut raw_plots = RawPlotBuilder::new(LEGEND_NAME)
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.h_msl),
                DataType::AltitudeMSL,
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.h_ell),
                DataType::AltitudeEllipsoidal,
            )
            // UTM Coordinates
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.northing),
                DataType::UtmNorthing,
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.easting),
                DataType::UtmEasting,
            )
            // Velocity
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.v_east),
                DataType::other_velocity("East", true),
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.v_north),
                DataType::other_velocity("North", true),
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.v_up),
                DataType::other_velocity("Up", true),
            )
            // Quality indicators
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.quality as f64),
                DataType::other_unitless("Quality Factor", ExpectedPlotRange::Hundreds, false),
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.pdop),
                DataType::other_unitless("PDOP", ExpectedPlotRange::Hundreds, true),
            )
            .add(
                plot_points_from_log_entry(
                    &entries,
                    |e| e.timestamp_ns(),
                    |e| e.num_satellites as f64,
                ),
                DataType::other_unitless("Satellites", ExpectedPlotRange::Hundreds, false),
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.undulation),
                DataType::Other {
                    name: "Undulation".into(),
                    unit: Some("m".into()),
                    plot_range: ExpectedPlotRange::Hundreds,
                    default_hidden: true,
                },
            )
            .add(
                plot_points_from_log_entry(&entries, |e| e.timestamp_ns(), |e| e.seq_num as f64),
                DataType::other_unitless("Sequence Number", ExpectedPlotRange::Hundreds, true),
            )
            .build();

        if let Some(geo_data) = geo_data {
            raw_plots.push(geo_data);
        }

        Ok((
            Self {
                raw_plots,
                first_timestamp,
                metadata,
            },
            total_bytes_read,
        ))
    }
    fn is_buf_valid(buf: &[u8]) -> Result<(), String> {
        let mut reader = io::BufReader::new(buf);
        let mut line = String::new();

        // Skip empty lines at the beginning
        loop {
            line.clear();
            if let Err(e) = reader.read_line(&mut line) {
                return Err(format!(
                    "Not a valid '{}', failed to read line while skipping empty lines: {e}",
                    Self::DESCRIPTIVE_NAME
                ));
            }

            let trimmed = line.trim();
            if !trimmed.is_empty() {
                break;
            }
        }

        // First non-empty line should start with "Project:"
        if !line.trim().starts_with("Project:") {
            return Err(format!(
                "Not a valid '{}', First non-empty line should start with \"Project:\"",
                Self::DESCRIPTIVE_NAME
            ));
        }

        // Second line should start with "Program:" and contain "GrafNav Version"
        line.clear();
        if let Err(e) = reader.read_line(&mut line) {
            return Err(format!(
                "Not a valid '{}', failed to read line while reading second non-empty line: {e}",
                Self::DESCRIPTIVE_NAME
            ));
        }

        let trimmed = line.trim();
        if trimmed.starts_with("Program:") && trimmed.contains("GrafNav Version") {
            Ok(())
        } else {
            Err(format!(
                "Not a valid '{}', First non-empty line should start with \"Project:\" and contain \"GrafNav Version\", got: {trimmed}",
                Self::DESCRIPTIVE_NAME
            ))
        }
    }
}

impl Plotable for GrafNavPPP {
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
    use plotinator_test_util::test_file_defs::csv::GRAFNAV_GP1_PPP_BYTES;

    use super::*;
    use std::io::BufReader;

    const TEST_2_ROWS: &str = r#"
    Project:     20250909_05_GP1_PPP

    Program:     GrafNav Version 9.00.2207
    Profile:     20120305_Skytem
    Source:      GNSS Epochs(PPP Combined)
    ProcessInfo: 20250909_05_GP1_PPP by Unknown on 9/12/2025 at 10:12:48
    Datum:       WGS84 (2025.690)
    Remote:      Antenna height 0.000 m, to L1PC [Generic(NONE)]
    UTC Offset:  18 s
    Geoid:       EGM96-World.wpg [] (Absolute correction)
    Map projection Info:
      UTM Zone:    32
    Column/Variable Contents, Units and Description:
      01: SeqNum                                      Sequence number that increments by user defined value. Jumps of more than one indicate missing epochs
      02: UTCDate      Year/Month/Day                 Date of epoch or feature (UTC time)
      03: UTCTime      HH:MM:SS.SS                    Time of epoch or feature - Receiver time frame (UTC)
      04: Latitude     Decimal Degrees (signed)       North/South Geographic coordinate
      05: Longitude    Decimal Degrees (signed)       East/West Geographic coordinate
      06: H-MSL        Metres                         Height above the geoid
      07: Northing     Metres                         North (y) coordinate in Universal Transverse Mercator grid
      08: Easting      Metres                         East (x) coordinate in Universal Transverse Mercator grid
      09: Q                                           Quality factor where 1 is best and 6 is worst
      10: Undulation   Metres                         Height of the ellipsoid above the geoid
      11: PDOP                                        Position Dilution of Precision, which is a measure of X, Y, Z position geometry
      12: H-Ell        Metres                         Height above the current ellipsoid
      13: NS                                          Number of total satellites (GPS+GLONASS+BeiDou+Galileo+QZSS) used in solution
      14: COG          Decimal Degrees (signed)       Direction that antenna is travelling
      15: VEast        Kilometers per Hour            East local level velocity
      16: VNorth       Kilometers per Hour            North local level velocity
      17: VUp          Kilometers per Hour            Up local level velocity
      18: HzSpeed      Kilometers per Hour            Combined effect of local level north and east speed
    SeqNum    UTCDate     UTCTime       Latitude      Longitude        H-MSL     Northing      Easting Q   Undulation   PDOP        H-Ell NS            COG     VEast    VNorth       VUp HzSpeed
                (YMD)       (HMS)          (deg)          (deg)          (m)          (m)          (m)            (m)  (dop)          (m)             (deg)    (km/h)    (km/h)    (km/h) (km/h)
    1      2025/09/09 16:37:22.00  56.1609498630  10.0376824505       34.200  6224477.873   564447.430 3       39.195   1.43       73.394 15   0.0000000000    -0.065     0.084     0.024 0.106
    2      2025/09/09 16:37:23.00  56.1609498800  10.0376825120       34.193  6224477.875   564447.434 3       39.195   1.43       73.388 15   0.0000000000     0.073     0.008    -0.114 0.074"#;

    #[test]
    fn test_read_csv() {
        let mut reader = BufReader::new(TEST_2_ROWS.as_bytes());
        let (grafnavppp, read_bytes) = GrafNavPPP::from_reader(&mut reader).unwrap();
        insta::with_settings!({filters => vec![
            (r"color: #\w+,", "color: [RANDOMIZED COLOR]"),
        ]}, {
            insta::assert_debug_snapshot!(grafnavppp);
        });
        insta::assert_debug_snapshot!(read_bytes);
    }

    #[test]
    fn test_read_csv_test_data() {
        let mut reader = BufReader::new(GRAFNAV_GP1_PPP_BYTES);
        let (grafnavppp, read_bytes) = GrafNavPPP::from_reader(&mut reader).unwrap();
        insta::assert_debug_snapshot!(grafnavppp.raw_plots.len());
        insta::assert_debug_snapshot!(read_bytes);
    }
}
