use std::{
    fmt,
    io::{self, BufReader},
};

use chrono::{DateTime, Utc};
use entries::NavSysSpsEntry;
use header::NavSysSpsHeader;
use log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

mod entries;
mod header;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NavSysSps {
    header: NavSysSpsHeader,
    entries: Vec<NavSysSpsEntry>,
    raw_plots: Vec<RawPlot>,
}

impl NavSysSps {
    fn build_raw_plots(entries: &[NavSysSpsEntry]) -> Vec<RawPlot> {
        let mut raw_he1_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_he2_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl1_points_pitch: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl2_points_pitch: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl1_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl2_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_mag1_points: Vec<[f64; 2]> = Vec::new();

        for entry in entries {
            match entry {
                NavSysSpsEntry::HE1(e) => {
                    raw_he1_points_altitude.push([e.timestamp_ns(), e.altitude_m()]);
                }
                NavSysSpsEntry::HE2(e) => {
                    raw_he2_points_altitude.push([e.timestamp_ns(), e.altitude_m()]);
                }
                NavSysSpsEntry::TL1(e) => {
                    raw_tl1_points_pitch.push([e.timestamp_ns(), e.pitch_angle_degrees()]);
                    raw_tl1_points_roll.push([e.timestamp_ns(), e.roll_angle_degrees()]);
                }
                NavSysSpsEntry::TL2(e) => {
                    raw_tl2_points_pitch.push([e.timestamp_ns(), e.pitch_angle_degrees()]);
                    raw_tl2_points_roll.push([e.timestamp_ns(), e.roll_angle_degrees()]);
                }
                NavSysSpsEntry::GP1(e) => {
                    let ts = e.timestamp_ns();
                    raw_gp1_points_latitude.push([ts, e.latitude()]);
                    raw_gp1_points_longitude.push([ts, e.longitude()]);
                    raw_gp1_points_gps_time_delta_ms.push([ts, e.gps_time_delta_ms()]);
                    raw_gp1_points_num_satellites.push([ts, e.num_satellites().into()]);
                    raw_gp1_points_speed_kmh.push([ts, e.speed_kmh().into()]);
                    raw_gp1_points_hdop.push([ts, e.hdop().into()]);
                    raw_gp1_points_vdop.push([ts, e.vdop().into()]);
                    raw_gp1_points_pdop.push([ts, e.pdop().into()]);
                    raw_gp1_points_altitude.push([ts, e.altitude_above_mean_sea().into()]);
                }
                NavSysSpsEntry::GP2(e) => {
                    let ts = e.timestamp_ns();
                    raw_gp2_points_latitude.push([ts, e.latitude()]);
                    raw_gp2_points_longitude.push([ts, e.longitude()]);
                    raw_gp2_points_gps_time_delta_ms.push([ts, e.gps_time_delta_ms()]);
                    raw_gp2_points_num_satellites.push([ts, e.num_satellites().into()]);
                    raw_gp2_points_speed_kmh.push([ts, e.speed_kmh().into()]);
                    raw_gp2_points_hdop.push([ts, e.hdop().into()]);
                    raw_gp2_points_vdop.push([ts, e.vdop().into()]);
                    raw_gp2_points_pdop.push([ts, e.pdop().into()]);
                    raw_gp2_points_altitude.push([ts, e.altitude_above_mean_sea().into()]);
                }
                NavSysSpsEntry::MA1(e) => {
                    raw_mag1_points.push([e.timestamp_ns(), e.field_nanotesla()]);
                }
            }
        }
        vec![
            RawPlot::new(
                "HE1 Altitude [M]".into(),
                raw_he1_points_altitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "HE2 Altitude [M]".into(),
                raw_he2_points_altitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "TL1 Pitch".into(),
                raw_tl1_points_pitch,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL2 Pitch".into(),
                raw_tl2_points_pitch,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL1 Roll".into(),
                raw_tl1_points_roll,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL2 Roll".into(),
                raw_tl2_points_roll,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 Latitude".into(),
                raw_gp1_points_latitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP2 Latitude".into(),
                raw_gp2_points_latitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP1 Longitude".into(),
                raw_gp1_points_longitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP2 Longitude".into(),
                raw_gp2_points_longitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP1 Time delta [ms]".into(),
                raw_gp1_points_gps_time_delta_ms,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP2 Time delta [ms]".into(),
                raw_gp2_points_gps_time_delta_ms,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP1 Satelittes".into(),
                raw_gp1_points_num_satellites,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 Satelittes".into(),
                raw_gp2_points_num_satellites,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 Speed [km/h]".into(),
                raw_gp1_points_speed_kmh,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 Speed [km/h]".into(),
                raw_gp2_points_speed_kmh,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 HDOP".into(),
                raw_gp1_points_hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 HDOP".into(),
                raw_gp2_points_hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 VDOP".into(),
                raw_gp1_points_vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 VDOP".into(),
                raw_gp2_points_vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 PDOP".into(),
                raw_gp1_points_pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 PDOP".into(),
                raw_gp2_points_pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 Altitude [m]".into(),
                raw_gp1_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 Altitude [m]".into(),
                raw_gp2_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "MA1 B-field [nT]".into(),
                raw_mag1_points,
                ExpectedPlotRange::Thousands,
            ),
        ]
    }
}

impl fmt::Display for NavSysSps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

impl Plotable for NavSysSps {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.header.first_timestamp()
    }

    fn descriptive_name(&self) -> &str {
        Self::DESCRIPTIVE_NAME
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        let mut metadata: Vec<(String, String)> = vec![
            ("Version".into(), self.header.version().to_string()),
            (
                "NavSys Software rev.".into(),
                self.header.software_revision().to_owned(),
            ),
            (
                "TiltSensor ID".into(),
                self.header.tilt_sensor_id().to_owned(),
            ),
        ];

        Some(metadata)
    }
}

impl SkytemLog for NavSysSps {
    type Entry = NavSysSpsEntry;

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl Parseable for NavSysSps {
    const DESCRIPTIVE_NAME: &str = "NavSys Sps";

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut total_bytes_read = 0;
        let (header, bytes_read) = NavSysSpsHeader::from_reader(reader)?;
        total_bytes_read += bytes_read;

        let (entries, bytes_read) = parse_to_vec(reader);
        total_bytes_read += bytes_read;

        let raw_plots = Self::build_raw_plots(&entries);

        Ok((
            Self {
                header,
                entries,
                raw_plots,
            },
            total_bytes_read,
        ))
    }

    fn is_buf_valid(buf: &[u8]) -> bool {
        let mut reader = BufReader::new(buf);
        NavSysSpsHeader::from_reader(&mut reader).is_ok()
    }
}

impl GitMetadata for NavSysSps {
    fn project_version(&self) -> Option<String> {
        Some(self.header.software_revision().to_owned())
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
