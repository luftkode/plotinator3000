use std::{io, path::Path};

use chrono::{DateTime, Utc};
use hdf5::Dataset;
use plotinator_log_if::{hdf5::SkytemHdf5, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    stream_descriptor::StreamDescriptor,
    util::{
        assert_description_in_attrs, log_all_attributes, open_dataset,
        read_any_attribute_to_string, read_string_attribute,
    },
};

impl SkytemHdf5 for NjordIns {
    #[allow(
        clippy::too_many_lines,
        reason = "Just adding quick Njord INS support... This needs a refactor, when the dataformat is more stable for example"
    )]
    fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let hdf5_file = hdf5::File::open(&path)?;

        let system_status_dataset = open_dataset(
            &hdf5_file,
            Self::SYSTEM_STATUS_DATASET,
            Self::EXPECT_DIMENSION,
        )?;
        assert_description_in_attrs(&system_status_dataset)?;

        let raw_byte_stream_dataset = open_dataset(
            &hdf5_file,
            Self::RAW_BYTE_STREAM_DATASET,
            Self::EXPECT_DIMENSION,
        )?;

        let unix_time_dataset =
            open_dataset(&hdf5_file, Self::UNIX_TIME_DATASET, Self::EXPECT_DIMENSION)?;

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
        let microseconds_dataset = open_dataset(
            &hdf5_file,
            Self::MICROSECONDS_DATASET,
            Self::EXPECT_DIMENSION,
        )?;

        log_all_attributes(&system_status_dataset);
        log_all_attributes(&raw_byte_stream_dataset);
        log_all_attributes(&unix_time_dataset);
        log_all_attributes(&position_dataset);
        log_all_attributes(&orientation_dataset);
        log_all_attributes(&filter_status_dataset);
        log_all_attributes(&microseconds_dataset);

        let unix_time: ndarray::Array2<i64> = unix_time_dataset.read()?;
        let microseconds: ndarray::Array2<i64> = microseconds_dataset.read()?;
        let timestamps = combine_timestamps(&unix_time, &microseconds);
        let first_timestamp = *timestamps.first().expect("No timestamps in dataset");
        // convert to f64 once and for all
        let timestamps: Vec<f64> = timestamps.into_iter().map(|ts| ts as f64).collect();

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
        let _system_status_unit =
            read_any_attribute_to_string(&system_status_dataset.attr("unit")?)?;
        let system_status: ndarray::Array2<f32> = system_status_dataset.read()?;
        let dataset_len = system_status.len(); // all datasets are the same length

        log::info!(
            "Got NJORD INS system status with {} samples",
            system_status.len()
        );
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

        for (row, ts) in system_status.rows().into_iter().zip(&timestamps) {
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
        let filter_status: ndarray::Array2<f32> = filter_status_dataset.read()?;
        let mut orientation_filter_initialized = Vec::with_capacity(dataset_len);
        let mut navigation_filter_initialized = Vec::with_capacity(dataset_len);
        let mut heading_initialized = Vec::with_capacity(dataset_len);
        let mut utc_time_initialized = Vec::with_capacity(dataset_len);
        // Not meaningful as of this writing
        //let mut gnss_fix_status = Vec::with_capacity(dataset_len);
        let mut event_1_occurred = Vec::with_capacity(dataset_len);
        let mut event_2_occurred = Vec::with_capacity(dataset_len);
        let mut internal_gnss_enabled = Vec::with_capacity(dataset_len);
        // Seems to be missing
        // let mut dual_antenna_heading_active = Vec::with_capacity(dataset_len);
        let mut velocity_heading_enabled = Vec::with_capacity(dataset_len);
        let mut atmospheric_altitude_enabled = Vec::with_capacity(dataset_len);
        let mut external_position_active = Vec::with_capacity(dataset_len);
        let mut external_velocity_active = Vec::with_capacity(dataset_len);
        let mut external_heading_active = Vec::with_capacity(dataset_len);

        for (row, ts) in filter_status.rows().into_iter().zip(&timestamps) {
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

        // Stream of orientation data with roll, pitch, and heading converted to degrees
        let _orientation_unit = read_any_attribute_to_string(&orientation_dataset.attr("unit")?)?;
        let orientation: ndarray::Array2<f64> = orientation_dataset.read()?;
        log::info!(
            "Got NJORD INS orientation with shape {:?}",
            orientation.shape()
        );
        let mut rolls = Vec::with_capacity(dataset_len);
        let mut pitches = Vec::with_capacity(dataset_len);
        let mut headings = Vec::with_capacity(dataset_len);
        for (row, ts) in orientation.rows().into_iter().zip(&timestamps) {
            let roll = row[0];
            let pitch = row[1];
            let heading = row[2];

            rolls.push([*ts, roll]);
            pitches.push([*ts, pitch]);
            headings.push([*ts, heading]);
        }

        // Stream of position data with latitude, longitude (converted to degrees), and height (m)
        let _position_unit = read_any_attribute_to_string(&position_dataset.attr("unit")?)?;
        let position: ndarray::Array2<f64> = position_dataset.read()?;
        let mut latitudes = Vec::with_capacity(dataset_len);
        let mut longitudes = Vec::with_capacity(dataset_len);
        let mut heights = Vec::with_capacity(dataset_len);
        for (row, ts) in position.rows().into_iter().zip(&timestamps) {
            let latitude = row[0];
            let longitude = row[1];
            let height = row[2] / 100.; // Convert from cm to M

            latitudes.push([*ts, latitude]);
            longitudes.push([*ts, longitude]);
            heights.push([*ts, height]);
        }

        let metadata = Self::extract_metadata(
            &system_status_dataset,
            &filter_status_dataset,
            &position_dataset,
            &orientation_dataset,
        )?;

        let raw_plots = vec![
            // Position data
            RawPlot::new(
                "Latitude".to_owned(),
                latitudes,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Longitude".to_owned(),
                longitudes,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Height [m]".to_owned(),
                heights,
                ExpectedPlotRange::OneToOneHundred,
            ),
            // Orientation data
            RawPlot::new("Roll".to_owned(), rolls, ExpectedPlotRange::OneToOneHundred),
            RawPlot::new(
                "Pitch".to_owned(),
                pitches,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "Heading".to_owned(),
                headings,
                ExpectedPlotRange::OneToOneHundred,
            ),
            // System Status data (all boolean flags)
            RawPlot::new(
                "System Failures [bool]".to_owned(),
                system_failures,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Accelerometer Sensor Failures [bool]".to_owned(),
                accelerometer_sensor_failures,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Gyroscope Sensor Failures [bool]".to_owned(),
                gyroscope_sensor_failures,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Magnetometer Sensor Failures [bool]".to_owned(),
                magnetometer_sensor_failures,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Pressure Sensor Failures [bool]".to_owned(),
                pressure_sensor_failures,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "GNSS Failures [bool]".to_owned(),
                gnss_failures,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Accelerometer Over Range [bool]".to_owned(),
                accelerometer_over_range,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Gyroscope Over Range [bool]".to_owned(),
                gyroscope_over_range,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Magnetometer Over Range [bool]".to_owned(),
                magnetometer_over_range,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Pressure Over Range [bool]".to_owned(),
                pressure_over_range,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Minimum Temperature Alarm [bool]".to_owned(),
                minimum_temperature_alarm,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Maximum Temperature Alarm [bool]".to_owned(),
                maximum_temperature_alarm,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "High Voltage Alarm [bool]".to_owned(),
                high_voltage_alarm,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "GNSS Antenna Connection [bool]".to_owned(),
                gnss_antenna_connection,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Data Output Overflow Alarm [bool]".to_owned(),
                data_output_overflow_alarm,
                ExpectedPlotRange::Percentage,
            ),
            // Filter Status data (boolean flags)
            RawPlot::new(
                "Orientation Filter Initialized [bool]".to_owned(),
                orientation_filter_initialized,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Navigation Filter Initialized [bool]".to_owned(),
                navigation_filter_initialized,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Heading Initialized [bool]".to_owned(),
                heading_initialized,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "UTC Time Initialized [bool]".to_owned(),
                utc_time_initialized,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Event 1 Occurred [bool]".to_owned(),
                event_1_occurred,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Event 2 Occurred [bool]".to_owned(),
                event_2_occurred,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Internal GNSS Enabled [bool]".to_owned(),
                internal_gnss_enabled,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Velocity Heading Enabled [bool]".to_owned(),
                velocity_heading_enabled,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "Atmospheric Altitude Enabled [bool]".to_owned(),
                atmospheric_altitude_enabled,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "External Position Active [bool]".to_owned(),
                external_position_active,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "External Velocity Active [bool]".to_owned(),
                external_velocity_active,
                ExpectedPlotRange::Percentage,
            ),
            RawPlot::new(
                "External Heading Active [bool]".to_owned(),
                external_heading_active,
                ExpectedPlotRange::Percentage,
            ),
        ];

        Ok(Self {
            starting_timestamp_utc: DateTime::from_timestamp_nanos(first_timestamp).to_utc(),
            dataset_description: "Njord INS Dataset".to_owned(),
            raw_plots,
            metadata,
        })
    }
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
    const SYSTEM_STATUS_DATASET: &str = "system_status";
    const RAW_BYTE_STREAM_DATASET: &str = "raw_byte_stream";
    const UNIX_TIME_DATASET: &str = "unix_time";
    const MICROSECONDS_DATASET: &str = "microseconds";
    const FILTER_STATUS_DATASET: &str = "filter_status";
    const POSITION_DATASET: &str = "position";
    const ORIENTATION_DATASET: &str = "orientation";
    const EXPECT_DIMENSION: usize = 2;

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
        assert_eq!(njord_ins.raw_plots.len(), 33);
        assert_eq!(njord_ins.raw_plots[0].points().len(), 4840);

        Ok(())
    }
}
