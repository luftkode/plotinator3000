use std::{io, path::Path};

use chrono::{DateTime, Utc};
use hdf5::Dataset;
use ndarray::Array2;
use plotinator_log_if::{hdf5::SkytemHdf5, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    stream_descriptor::StreamDescriptor,
    util::{
        self, assert_description_in_attrs, log_all_attributes, open_dataset,
        read_any_attribute_to_string, read_string_attribute,
    },
};

const LEGEND_NAME: &str = "Njord-INS";
const RAW_PLOT_NAME_SUFFIX: &str = "(Njord-INS)";

impl SkytemHdf5 for NjordIns {
    #[allow(
        clippy::too_many_lines,
        reason = "Just adding quick Njord INS support... This needs a refactor, when the dataformat is more stable for example"
    )]
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let hdf5_file = hdf5::File::open(&path)?;

        let system_status_dataset = open_dataset(
            &hdf5_file,
            Self::SYSTEM_STATUS_DATASET,
            Self::EXPECT_DIMENSION,
        )?;
        assert_description_in_attrs(&system_status_dataset)?;

        let position_dataset =
            open_dataset(&hdf5_file, Self::POSITION_DATASET, Self::EXPECT_DIMENSION)?;

        let orientation_dataset = open_dataset(
            &hdf5_file,
            Self::ORIENTATION_DATASET,
            Self::EXPECT_DIMENSION,
        )?;

        let filter_status_dataset = open_dataset(
            &hdf5_file,
            Self::FILTER_STATUS_DATASET,
            Self::EXPECT_DIMENSION,
        )?;

        log_all_attributes(&system_status_dataset);
        log_all_attributes(&position_dataset);
        log_all_attributes(&orientation_dataset);
        log_all_attributes(&filter_status_dataset);

        let (first_timestamp, timestamps, delta_t_samples_opt, rawplot_offset_opt) =
            Self::read_dataset_time(&hdf5_file)?;

        let _system_status_unit =
            read_any_attribute_to_string(&system_status_dataset.attr("unit")?)?;

        let system_status: ndarray::Array2<f32> = system_status_dataset.read()?;
        let filter_status: ndarray::Array2<f32> = filter_status_dataset.read()?;
        let dataset_len = system_status.len(); // all datasets are the same length

        log::info!(
            "Got NJORD INS system status with {} samples",
            system_status.len()
        );

        let mut system_status_plots: Option<Vec<RawPlot>> = None;
        let mut filter_status_plots: Option<Vec<RawPlot>> = None;
        let mut orientation_position_plots: Option<Vec<RawPlot>> = None;

        // Use scoped threads to parallelize the major processing steps
        rayon::scope(|s| {
            // System status processing in one thread
            s.spawn(|_| {
                system_status_plots = Some(process_system_status(
                    &system_status,
                    &timestamps,
                    dataset_len,
                ));
            });

            // Filter status processing in another thread
            s.spawn(|_| {
                filter_status_plots = Some(process_filter_status(
                    &filter_status,
                    &timestamps,
                    dataset_len,
                ));
            });

            s.spawn(|_| {
                match process_orientation_and_position(
                    &orientation_dataset,
                    &position_dataset,
                    &timestamps,
                    dataset_len,
                ) {
                    Ok(p) => orientation_position_plots = Some(p),
                    Err(e) => {
                        log::error!("Failed to process Njord INS orientation and position: {e}")
                    }
                }
            });
        });

        let metadata = Self::extract_metadata(
            &system_status_dataset,
            &filter_status_dataset,
            &position_dataset,
            &orientation_dataset,
        )?;

        let mut raw_plots: Vec<RawPlot> = vec![];

        if let Some(delta_t_samples) = delta_t_samples_opt {
            raw_plots.push(delta_t_samples.into());
        }

        if let Some(offset_plot) = rawplot_offset_opt {
            raw_plots.push(offset_plot.into());
        }
        if let Some(plots) = orientation_position_plots {
            raw_plots.extend(plots);
        }
        if let Some(plots) = system_status_plots {
            raw_plots.extend(plots);
        }
        if let Some(plots) = filter_status_plots {
            raw_plots.extend(plots);
        }

        Ok(Self {
            starting_timestamp_utc: DateTime::from_timestamp_nanos(first_timestamp).to_utc(),
            dataset_description: "Njord INS Dataset".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

#[allow(clippy::too_many_lines, reason = "Long but simple")]
fn process_system_status(
    system_status: &ndarray::Array2<f32>,
    timestamps: &[f64],
    dataset_len: usize,
) -> Vec<RawPlot> {
    // This dataset represents the System Status packet which is 16 bits
    // Bit 	Description
    // 0 	System Failure
    // 1 	Accelerometer Sensor Failure
    // 2 	Gyroscope Sensor Failure
    // 3 	Magnetometer Sensor Failure
    // 4 	Pressure Sensor Failure
    // 5 	GNSS Failure
    // 6 	Accelerometer Over Range
    // 7 	Gyroscope Over Range
    // 8 	Magnetometer Over Range
    // 9 	Pressure Over Range
    // 10 	Minimum Temperature Alarm
    // 11 	Maximum Temperature Alarm
    // 12
    // 13 	High Voltage Alarm
    // 14 	GNSS Antenna Connection (open or short circuit, primary or secondary antenna) . NOTE: This bit is not applicable to units with a Trimble BD992 GNSS Receiver
    // 15 	Data Output Overflow Alarm
    let mut system_failures = Vec::with_capacity(dataset_len);
    let mut accelerometer_sensor_failures = Vec::with_capacity(dataset_len);
    let mut gyroscope_sensor_failures = Vec::with_capacity(dataset_len);
    let mut magnetometer_sensor_failures = Vec::with_capacity(dataset_len);
    let mut pressure_sensor_failures = Vec::with_capacity(dataset_len);
    let mut gnss_failures = Vec::with_capacity(dataset_len);
    let mut accelerometer_over_range = Vec::with_capacity(dataset_len);
    let mut gyroscope_over_range = Vec::with_capacity(dataset_len);
    let mut magnetometer_over_range = Vec::with_capacity(dataset_len);
    let mut pressure_over_range = Vec::with_capacity(dataset_len);
    let mut minimum_temperature_alarm = Vec::with_capacity(dataset_len);
    let mut maximum_temperature_alarm = Vec::with_capacity(dataset_len);
    let mut high_voltage_alarm = Vec::with_capacity(dataset_len);
    let mut gnss_antenna_connection = Vec::with_capacity(dataset_len);
    let mut data_output_overflow_alarm = Vec::with_capacity(dataset_len);

    for (row, ts) in system_status.rows().into_iter().zip(timestamps) {
        system_failures.push([*ts, row[0] as f64]);
        accelerometer_sensor_failures.push([*ts, row[1] as f64]);
        gyroscope_sensor_failures.push([*ts, row[2] as f64]);
        magnetometer_sensor_failures.push([*ts, row[3] as f64]);
        pressure_sensor_failures.push([*ts, row[4] as f64]);
        gnss_failures.push([*ts, row[5] as f64]);
        accelerometer_over_range.push([*ts, row[6] as f64]);
        gyroscope_over_range.push([*ts, row[7] as f64]);
        magnetometer_over_range.push([*ts, row[8] as f64]);
        pressure_over_range.push([*ts, row[9] as f64]);
        minimum_temperature_alarm.push([*ts, row[10] as f64]);
        maximum_temperature_alarm.push([*ts, row[11] as f64]);
        // Bit 12 is reserved/unused
        high_voltage_alarm.push([*ts, row[13] as f64]);
        gnss_antenna_connection.push([*ts, row[14] as f64]);
        data_output_overflow_alarm.push([*ts, row[15] as f64]);
    }

    vec![
        RawPlotCommon::new(
            "System Failures [bool]".to_owned(),
            system_failures,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Accelerometer Sensor Failures [bool]".to_owned(),
            accelerometer_sensor_failures,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Gyroscope Sensor Failures [bool]".to_owned(),
            gyroscope_sensor_failures,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Magnetometer Sensor Failures [bool]".to_owned(),
            magnetometer_sensor_failures,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Pressure Sensor Failures [bool]".to_owned(),
            pressure_sensor_failures,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "GNSS Failures [bool]".to_owned(),
            gnss_failures,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Accelerometer Over Range [bool]".to_owned(),
            accelerometer_over_range,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Gyroscope Over Range [bool]".to_owned(),
            gyroscope_over_range,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Magnetometer Over Range [bool]".to_owned(),
            magnetometer_over_range,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Pressure Over Range [bool]".to_owned(),
            pressure_over_range,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Minimum Temperature Alarm [bool]".to_owned(),
            minimum_temperature_alarm,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Maximum Temperature Alarm [bool]".to_owned(),
            maximum_temperature_alarm,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "High Voltage Alarm [bool]".to_owned(),
            high_voltage_alarm,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "GNSS Antenna Connection [bool]".to_owned(),
            gnss_antenna_connection,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Data Output Overflow Alarm [bool]".to_owned(),
            data_output_overflow_alarm,
            ExpectedPlotRange::Percentage,
        )
        .into(),
    ]
}

fn process_filter_status(
    filter_status: &ndarray::Array2<f32>,
    timestamps: &[f64],
    dataset_len: usize,
) -> Vec<RawPlot> {
    // This dataset represents the Filter Status packet
    // Filter Status
    // This field contains 16 bits that indicate the status of the filters. These are boolean fields with a zero indicating false and one indicating true.
    // Bit 	Description
    // 0 	Orientation Filter Initialised
    // 1 	Navigation Filter Initialised
    // 2 	Heading Initialised
    // 3 	UTC Time Initialised
    // 4 	GNSS Fix Status (see the next table)
    // 5
    // 6
    // 7 	Event 1 Occurred
    // 8 	Event 2 Occurred
    // 9 	Internal GNSS Enabled
    // 10 	Dual Antenna Heading Active
    // 11 	Velocity Heading Enabled
    // 12 	Atmospheric Altitude Enabled
    // 13 	External Position Active
    // 14 	External Velocity Active
    // 15 	External Heading Active
    //
    // GNSS Fix Status
    // Value 	Bit 6 	Bit 5 	Bit 4 	Description
    // 0 	0 	0 	0 	No GNSS fix
    // 1 	0 	0 	1 	2D GNSS fix
    // 2 	0 	1 	0 	3D GNSS fix
    // 3 	0 	1 	1 	SBAS GNSS fix
    // 4 	1 	0 	0 	Differential GNSS fix
    // 5 	1 	0 	1 	PPP GNSS fix
    // 6 	1 	1 	0 	RTK Float GNSS fix
    // 7 	1 	1 	1 	RTK Fixed GNSS fix
    let mut orientation_filter_initialized = Vec::with_capacity(dataset_len);
    let mut navigation_filter_initialized = Vec::with_capacity(dataset_len);
    let mut heading_initialized = Vec::with_capacity(dataset_len);
    let mut utc_time_initialized = Vec::with_capacity(dataset_len);
    let mut event_1_occurred = Vec::with_capacity(dataset_len);
    let mut event_2_occurred = Vec::with_capacity(dataset_len);
    let mut internal_gnss_enabled = Vec::with_capacity(dataset_len);
    let mut velocity_heading_enabled = Vec::with_capacity(dataset_len);
    let mut atmospheric_altitude_enabled = Vec::with_capacity(dataset_len);
    let mut external_position_active = Vec::with_capacity(dataset_len);
    let mut external_velocity_active = Vec::with_capacity(dataset_len);
    let mut external_heading_active = Vec::with_capacity(dataset_len);

    for (row, ts) in filter_status.rows().into_iter().zip(timestamps) {
        orientation_filter_initialized.push([*ts, row[0] as f64]);
        navigation_filter_initialized.push([*ts, row[1] as f64]);
        heading_initialized.push([*ts, row[2] as f64]);
        utc_time_initialized.push([*ts, row[3] as f64]);

        // GNSS Fix Status is just set to true so we ignore it, until it has been changed in Njord INS
        // let gnss_fix_value = (row[4] as u8) | ((row[5] as u8) << 1) | ((row[6] as u8) << 2);
        // gnss_fix_status.push([*ts, gnss_fix_value as f64]);

        event_1_occurred.push([*ts, row[5] as f64]);
        event_2_occurred.push([*ts, row[6] as f64]);
        internal_gnss_enabled.push([*ts, row[7] as f64]);
        // MISSING
        //dual_antenna_heading_active.push([*ts, row[10] as f64]);
        velocity_heading_enabled.push([*ts, row[8] as f64]);
        atmospheric_altitude_enabled.push([*ts, row[9] as f64]);
        external_position_active.push([*ts, row[10] as f64]);
        external_velocity_active.push([*ts, row[11] as f64]);
        external_heading_active.push([*ts, row[12] as f64]);
    }

    vec![
        RawPlotCommon::new(
            "Orientation Filter Initialized [bool]".to_owned(),
            orientation_filter_initialized,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Navigation Filter Initialized [bool]".to_owned(),
            navigation_filter_initialized,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Heading Initialized [bool]".to_owned(),
            heading_initialized,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "UTC Time Initialized [bool]".to_owned(),
            utc_time_initialized,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Event 1 Occurred [bool]".to_owned(),
            event_1_occurred,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Event 2 Occurred [bool]".to_owned(),
            event_2_occurred,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Internal GNSS Enabled [bool]".to_owned(),
            internal_gnss_enabled,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Velocity Heading Enabled [bool]".to_owned(),
            velocity_heading_enabled,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "Atmospheric Altitude Enabled [bool]".to_owned(),
            atmospheric_altitude_enabled,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "External Position Active [bool]".to_owned(),
            external_position_active,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "External Velocity Active [bool]".to_owned(),
            external_velocity_active,
            ExpectedPlotRange::Percentage,
        )
        .into(),
        RawPlotCommon::new(
            "External Heading Active [bool]".to_owned(),
            external_heading_active,
            ExpectedPlotRange::Percentage,
        )
        .into(),
    ]
}

fn process_orientation_and_position(
    orientation_dataset: &Dataset,
    position_dataset: &Dataset,
    timestamps: &[f64],
    dataset_len: usize,
) -> anyhow::Result<Vec<RawPlot>> {
    let orientation: Array2<f64> = orientation_dataset.read()?;
    let position: Array2<f64> = position_dataset.read()?;

    let mut rolls = Vec::with_capacity(dataset_len);
    let mut pitches = Vec::with_capacity(dataset_len);
    let mut headings = Vec::with_capacity(dataset_len);

    let mut latitudes = Vec::with_capacity(dataset_len);
    let mut longitudes = Vec::with_capacity(dataset_len);
    let mut heights = Vec::with_capacity(dataset_len);

    for row in position.rows().into_iter() {
        let latitude = row[0];
        let longitude = row[1];
        let height = row[2] / 100.; // Convert from cm to M

        latitudes.push(latitude);
        longitudes.push(longitude);
        heights.push(height);
    }

    for (row, ts) in orientation.rows().into_iter().zip(timestamps) {
        let roll = row[0];
        let pitch = row[1];
        let heading = row[2];

        rolls.push([*ts, roll]);
        pitches.push([*ts, pitch]);
        headings.push(heading);
    }

    let geo_data = GeoSpatialDataBuilder::new(LEGEND_NAME.to_owned())
        .timestamp(timestamps)
        .lat(&latitudes)
        .lon(&longitudes)
        .heading(&headings)
        .altitude(&heights)
        .build()?
        .into();

    Ok(vec![
        RawPlotCommon::new(
            format!("Roll° ({LEGEND_NAME})"),
            rolls,
            ExpectedPlotRange::OneToOneHundred,
        )
        .into(),
        RawPlotCommon::new(
            format!("Pitch° ({LEGEND_NAME})"),
            pitches,
            ExpectedPlotRange::OneToOneHundred,
        )
        .into(),
        geo_data,
    ])
}

fn combine_timestamps(
    unix_time: &ndarray::Array2<i64>,
    microseconds: &ndarray::Array2<i64>,
) -> Vec<i64> {
    unix_time
        .iter()
        .zip(microseconds.iter())
        .map(|(&sec, &micros)| sec * 1_000_000_000 + micros * 1_000)
        .collect()
}

impl Plotable for NjordIns {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.starting_timestamp_utc
    }

    fn descriptive_name(&self) -> &str {
        &self.dataset_description
    }

    fn labels(&self) -> Option<&[plotinator_log_if::prelude::PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        Some(self.metadata.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NjordIns {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl NjordIns {
    // Before we logged system time
    const UNIX_TIME_DATASET: &str = "unix_time";
    const MICROSECONDS_DATASET: &str = "microseconds";
    // After we started logging system time
    const GPS_UNIX_TIME_DATASET: &str = "gps_unix_time";
    const SYSTEM_TIMESTAMP: &str = "timestamp";

    const SYSTEM_STATUS_DATASET: &str = "system_status";
    const FILTER_STATUS_DATASET: &str = "filter_status";
    const POSITION_DATASET: &str = "position";
    const ORIENTATION_DATASET: &str = "orientation";
    const EXPECT_DIMENSION: usize = 2;

    fn poc_read_dataset_time(
        hdf5_file: &hdf5::File,
    ) -> anyhow::Result<(i64, Vec<f64>, Option<RawPlotCommon>)> {
        let unix_time_dataset =
            open_dataset(hdf5_file, Self::UNIX_TIME_DATASET, Self::EXPECT_DIMENSION)?;
        let microseconds_dataset = open_dataset(
            hdf5_file,
            Self::MICROSECONDS_DATASET,
            Self::EXPECT_DIMENSION,
        )?;

        let unix_time: ndarray::Array2<i64> = unix_time_dataset.read()?;
        let microseconds: ndarray::Array2<i64> = microseconds_dataset.read()?;
        let timestamps: Vec<i64> = combine_timestamps(&unix_time, &microseconds);
        let first_timestamp = *timestamps.first().expect("No timestamps in dataset");
        let time_between_samples =
            util::gen_time_between_samples_rawplot(&timestamps, RAW_PLOT_NAME_SUFFIX);
        // convert to f64 once and for all
        let timestamps: Vec<f64> = timestamps.into_iter().map(|ts| ts as f64).collect();
        Ok((first_timestamp, timestamps, time_between_samples))
    }

    fn with_sys_time_read_dataset_time(
        hdf5_file: &hdf5::File,
    ) -> anyhow::Result<(i64, Vec<f64>, Option<RawPlotCommon>, RawPlotCommon)> {
        let sys_time = open_dataset(hdf5_file, Self::SYSTEM_TIMESTAMP, Self::EXPECT_DIMENSION)?;
        let sys_time: Vec<i64> = sys_time.read_raw()?;
        let first_timestamp = *sys_time.first().expect("No timestamps in dataset");
        let time_between_samples =
            util::gen_time_between_samples_rawplot(&sys_time, RAW_PLOT_NAME_SUFFIX);
        let plot_timestamps: Vec<f64> = sys_time.iter().map(|ts| *ts as f64).collect();
        let gps_unix_time = open_dataset(
            hdf5_file,
            Self::GPS_UNIX_TIME_DATASET,
            Self::EXPECT_DIMENSION,
        )?;

        let gps_unix_time: ndarray::Array2<i64> = gps_unix_time.read()?;

        let mut time_offset = Vec::with_capacity(sys_time.len());

        for ((gps_ts, sys_ts), plot_ts) in gps_unix_time.iter().zip(sys_time).zip(&plot_timestamps)
        {
            let delta_ns = (sys_ts - *gps_ts) as f64;
            let delta_ms = delta_ns / 1e6;
            time_offset.push([*plot_ts, delta_ms]);
        }

        Ok((
            first_timestamp,
            plot_timestamps,
            time_between_samples,
            RawPlotCommon::new(
                format!("Δt GPS/System [ms] {RAW_PLOT_NAME_SUFFIX}"),
                time_offset,
                ExpectedPlotRange::OneToOneHundred,
            ),
        ))
    }

    // Returns the time that the dataset should be aligned with
    #[allow(
        clippy::type_complexity,
        reason = "we'll stop supporting this sensor soon enough"
    )]
    fn read_dataset_time(
        hdf5_file: &hdf5::File,
    ) -> anyhow::Result<(i64, Vec<f64>, Option<RawPlotCommon>, Option<RawPlotCommon>)> {
        if let Ok((first_timestamp, plot_timestamps, delta_t_samples)) =
            Self::poc_read_dataset_time(hdf5_file)
        {
            Ok((first_timestamp, plot_timestamps, delta_t_samples, None))
        } else {
            let (first_ts, plot_ts, delta_t_samples, rawplot_offset) =
                Self::with_sys_time_read_dataset_time(hdf5_file)?;
            Ok((first_ts, plot_ts, delta_t_samples, Some(rawplot_offset)))
        }
    }

    fn extract_metadata(
        system_status_dataset: &Dataset,
        filter_status_dataset: &Dataset,
        position_dataset: &Dataset,
        orientation_dataset: &Dataset,
    ) -> io::Result<Vec<(String, String)>> {
        let mut metadata = vec![];

        let datasets = &[
            ("System Status", system_status_dataset),
            ("Filter Status", filter_status_dataset),
            ("Position", position_dataset),
            ("Orientation", orientation_dataset),
        ];

        for (name, dataset) in datasets {
            let description = read_string_attribute(&dataset.attr("description")?)?;
            let descriptor = StreamDescriptor::try_from(*dataset)?;
            metadata.push((format!("{name} Description"), description));
            metadata.extend_from_slice(&descriptor.to_metadata());
        }

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::test_file_defs::njord_ins::*;
    use testresult::TestResult;

    #[test]
    fn test_read_njord_ins() -> TestResult {
        let njord_ins = NjordIns::from_path(njord_ins())?;
        assert_eq!(njord_ins.metadata.len(), 44);
        assert_eq!(njord_ins.raw_plots.len(), 31);
        match &njord_ins.raw_plots[1] {
            RawPlot::Generic { common } => assert_eq!(common.points().len(), 4840),
            _ => unreachable!(),
        };

        Ok(())
    }
}
