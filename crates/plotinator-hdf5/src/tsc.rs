use std::{io, path::Path};

use chrono::{DateTime, Utc};
use plotinator_log_if::{
    hdf5::SkytemHdf5,
    prelude::{PlotLabels, Plotable, RawPlot},
};
use serde::{Deserialize, Serialize};

use crate::tsc::{gps_marks::GpsMarkRecords, gps_pvt::GpsPvtRecords, hm::HmData};

mod gps_marks;
mod gps_pvt;
mod hm;

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
    fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let h5 = hdf5::File::open(path)?;
        log::info!("Reading TSC datasets");
        let hm = HmData::from_hdf5(&h5)?;
        let gps_marks = GpsMarkRecords::from_hdf5(&h5)?;
        let gps_pvts = GpsPvtRecords::from_hdf5(&h5)?;

        let Some(first_timestamp) = gps_marks.first_timestamp() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "gps marks dataset is empty",
            ));
        };

        log::info!("Creating GPS marks plots");
        let (mut plots, mut metadata) = gps_marks.build_plots_and_metadata();
        log::info!("Creating GPS PVT plots");
        plots.extend(gps_pvts.build_plots());
        log::info!("Creating HM plots");
        let gps_time = gps_marks.timestamps();
        let (hm_plots, hm_metadata) = hm.build_plots_and_metadata(&gps_time);
        plots.extend(hm_plots);
        metadata.extend(hm_metadata);

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
    use crate::tsc::{gps_marks::GpsMarkRecords, hm::HmData};
    use plotinator_test_util::test_file_defs::tsc::*;
    use testresult::TestResult;

    #[test]
    fn test_read_tsc() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;

        let gps_marks = GpsMarkRecords::from_hdf5(&h5file)?;
        let hm = HmData::from_hdf5(&h5file)?;

        let gps_time = gps_marks.timestamps();

        let hm_series = hm.create_time_series(&gps_time, [0, 0, 0, 0, 0]);

        insta::assert_debug_snapshot!(hm_series);

        Ok(())
    }
}
