use std::{path::Path, time::Instant};

use anyhow::bail;
use chrono::{DateTime, Utc};
use plotinator_log_if::{
    hdf5::SkytemHdf5,
    prelude::{PlotLabels, Plotable},
    rawplot::RawPlot,
};
use serde::{Deserialize, Serialize};

use crate::tsc::{
    gps_marks::GpsMarkRecords, gps_pvt::GpsPvtRecords, hm::HmData, metadata::RootMetadata,
};

mod gps_marks;
mod gps_pvt;
mod hm;
mod metadata;

pub(crate) const TSC_LEGEND_NAME: &str = "TSC";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tsc {
    first_timestamp: DateTime<Utc>,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl Tsc {
    const DESCRIPTIVE_NAME: &str = "TSC";
}

impl SkytemHdf5 for Tsc {
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let total_start = Instant::now();
        let start_reading = Instant::now();
        let h5 = hdf5::File::open(path)?;
        log::info!("== Reading TSC datasets");

        // Read top level metadata
        let root_metadata = RootMetadata::parse_from_tsc(&h5)?;
        let mut metadata = root_metadata.metadata_strings();

        let start = Instant::now();
        let hm = HmData::from_hdf5(&h5)?;
        log::info!("Read HmData in {:.1?}", start.elapsed());

        let start = Instant::now();
        let gps_marks = GpsMarkRecords::from_hdf5(&h5)?;
        log::info!("Read GpsMarks in {:.1?}", start.elapsed());
        let start = Instant::now();
        let gps_pvts = GpsPvtRecords::from_hdf5(&h5)?;
        log::info!("Read GpsPvt in {:.1?}", start.elapsed());
        log::info!(
            "== Finished reading datasets in {:.1?}",
            start_reading.elapsed()
        );

        let Some(first_timestamp) = gps_marks.first_timestamp() else {
            bail!("gps marks dataset is empty");
        };

        let start_building_plots = Instant::now();
        log::info!("== Building plots");
        let start = Instant::now();
        log::info!("Creating GPS marks plots");
        let (mut plots, mut gps_metadata) = gps_marks.build_plots_and_metadata();
        metadata.append(&mut gps_metadata);
        log::info!("Created GPS marks plots in {:.1?}", start.elapsed());
        log::info!("Creating GPS PVT plots");
        let start = Instant::now();
        plots.extend(gps_pvts.build_plots());
        log::info!("Created GPS PVT plots in {:.1?}", start.elapsed());
        log::info!("Creating HM plots");
        let start = Instant::now();
        let gps_time = gps_marks.timestamps();
        let (hm_plots, mut hm_metadata) = hm.build_plots_and_metadata(&gps_time, &root_metadata)?;
        plots.extend(hm_plots);
        metadata.append(&mut hm_metadata);
        log::info!("Created HM plots in {:.1?}", start.elapsed());
        log::info!(
            "== Finishing build plots in {:.1?}",
            start_building_plots.elapsed()
        );
        log::info!(
            "== == Total TSC::from_path duration: {:.1?} == ==",
            total_start.elapsed()
        );

        plots.retain(|p| match p {
            RawPlot::GeoSpatialDataset(_) => true,
            RawPlot::Generic { common } | RawPlot::Boolean { common } => {
                if common.points().len() < 2 {
                    log::warn!("Discarding plot with less than 2 points: {}", common.name());
                    false
                } else {
                    true
                }
            }
        });

        Ok(Self {
            first_timestamp,
            raw_plots: plots,
            metadata,
        })
    }
}

impl Plotable for Tsc {
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
    use crate::tsc::{gps_marks::GpsMarkRecords, hm::HmData, metadata::RootMetadata};
    use plotinator_log_if::rawplot::RawPlot;
    use plotinator_test_util::test_file_defs::tsc::*;
    use testresult::TestResult;

    #[test]
    fn test_read_tsc_gps_marks_timestamps() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;

        let gps_marks = GpsMarkRecords::from_hdf5(&h5file)?;
        let gps_time = gps_marks.timestamps();
        let first_10_gps_time: Vec<f64> = gps_time.into_iter().take(10).collect();
        insta::assert_debug_snapshot!(first_10_gps_time);

        Ok(())
    }

    #[test]
    fn test_read_bfield_and_zero_pos() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let root_metadata = RootMetadata::parse_from_tsc(&h5file)?;
        let gps_marks = GpsMarkRecords::from_hdf5(&h5file)?;
        let hm_data = HmData::from_hdf5(&h5file)?;

        let gps_timestamps = gps_marks.timestamps();
        let (plots, _metadata) =
            hm_data.build_plots_and_metadata(&gps_timestamps, &root_metadata)?;

        let zero_pos_first_10: Vec<[f64; 2]> = match plots.first().unwrap() {
            RawPlot::Generic { common } => common.points().iter().copied().take(10).collect(),
            _ => unreachable!(),
        };
        let bfield_first_10: Vec<[f64; 2]> = match &plots[1] {
            RawPlot::Generic { common } => common.points().iter().copied().take(10).collect(),
            _ => unreachable!(),
        };

        insta::assert_debug_snapshot!(zero_pos_first_10);
        insta::assert_debug_snapshot!(bfield_first_10);

        Ok(())
    }
}
