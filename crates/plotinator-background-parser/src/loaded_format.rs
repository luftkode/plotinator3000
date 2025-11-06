use std::sync::atomic::{AtomicU16, Ordering};

use plotinator_log_if::{prelude::*, rawplot::path_data::GeoSpatialDataset};
use plotinator_plot_util::{CookedPlot, StoredPlotLabels};
use plotinator_supported_formats::{ParseInfo, SupportedFormat};
use plotinator_ui_log_settings::LoadedLogSettings;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

/// Get the next unique ID for a log
///
// This is how all logs get their log_id, and how each plot for each log gets their log_id
#[must_use]
fn next_log_id() -> u16 {
    static LOG_ID_COUNTER: AtomicU16 = AtomicU16::new(1);
    LOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
pub struct LoadedSupportedFormat {
    id: u16,
    format: Option<SupportedFormat>,
    cooked_plots: Option<Vec<CookedPlot>>,
    cooked_labels: Option<Vec<StoredPlotLabels>>,
    settings: Option<LoadedLogSettings>,
    // This is for the map
    geo_spatial_data: Option<Vec<GeoSpatialDataset>>,
}

impl LoadedSupportedFormat {
    pub fn new(format: SupportedFormat) -> Self {
        Self {
            id: next_log_id(),
            format: Some(format),
            cooked_plots: None,
            cooked_labels: None,
            settings: None,
            geo_spatial_data: None,
        }
    }

    pub fn format_name(&self) -> &str {
        self.format
            .as_ref()
            .expect("unsound condition")
            .descriptive_name()
    }

    pub fn parse_info(&self) -> Option<ParseInfo> {
        self.format
            .as_ref()
            .expect("unsound condition")
            .parse_info()
    }

    pub fn take_supported_format(&mut self) -> SupportedFormat {
        self.format
            .take()
            .expect("attempted to take support format tiwce")
    }

    pub fn take_settings(&mut self) -> LoadedLogSettings {
        self.settings
            .take()
            .expect("attempted to take settings twice")
    }

    pub fn take_cooked_plots(&mut self) -> Vec<CookedPlot> {
        self.cooked_plots
            .take()
            .expect("attempted to take cooked plots twice")
    }

    pub fn take_cooked_labels(&mut self) -> Vec<StoredPlotLabels> {
        self.cooked_labels
            .take()
            .expect("attempted to take plot labels twice")
    }

    pub fn take_geo_spatial_data(&mut self) -> Vec<GeoSpatialDataset> {
        self.geo_spatial_data
            .take()
            .expect("attempted to take geospatial data twice")
    }

    pub fn cook_all(&mut self) {
        self.cooked_plots = Some(self.cook_plots());
        self.cooked_labels = Some(self.cook_labels());
        self.settings = Some(self.make_loaded_log_settings());
        self.geo_spatial_data = Some(
            self.format
                .as_ref()
                .expect("unsound condition")
                .geo_spatial_data(),
        );
    }

    fn make_loaded_log_settings(&self) -> LoadedLogSettings {
        LoadedLogSettings::new(
            self.id,
            self.format
                .as_ref()
                .expect("unsound condition")
                .descriptive_name()
                .to_owned(),
            self.format
                .as_ref()
                .expect("unsound condition")
                .first_timestamp(),
            self.format.as_ref().expect("unsound condition").metadata(),
            self.format
                .as_ref()
                .expect("unsound condition")
                .parse_info(),
        )
    }

    fn cook_plots(&self) -> Vec<CookedPlot> {
        const PARALLEL_THRESHOLD: usize = 200_000;

        // Compute the average number of points across all plots
        let plot_point_counts: Vec<usize> = self
            .format
            .as_ref()
            .expect("unsound condition")
            .raw_plots()
            .iter()
            .map(|p| match p {
                RawPlot::Generic { common } => common.points().len(),
                RawPlot::GeoSpatialDataset(geo) => geo.len(),
            })
            .collect();

        let avg_plot_points_count = if plot_point_counts.is_empty() {
            0
        } else {
            plot_point_counts.iter().sum::<usize>() / plot_point_counts.len()
        };

        if avg_plot_points_count > PARALLEL_THRESHOLD {
            log::info!(
                "Processing new plots in parallel (average point count {avg_plot_points_count} exceeds threshold {PARALLEL_THRESHOLD})"
            );
            cook_plots_par(self.format.as_ref().expect("unsound condition"), self.id)
        } else {
            cook_plots_seq(self.format.as_ref().expect("unsound condition"), self.id)
        }
    }

    pub fn cook_labels(&self) -> Vec<StoredPlotLabels> {
        let mut labels = vec![];

        if let Some(plot_labels) = self.format.as_ref().expect("unsound condition").labels() {
            for l in plot_labels {
                let owned_label_points = l.label_points().to_owned();
                let stored_labels =
                    StoredPlotLabels::new(owned_label_points, self.id, l.expected_range());
                labels.push(stored_labels);
            }
        }

        labels
    }
}

fn cook_plots_seq(format: &SupportedFormat, data_id: u16) -> Vec<CookedPlot> {
    let mut cooked_plots = Vec::with_capacity(format.raw_plots().len());

    for raw_plot in format.raw_plots() {
        match raw_plot {
            RawPlot::Generic { common } => {
                let cooked =
                    CookedPlot::new(&common, data_id, format.descriptive_name().to_owned());
                cooked_plots.push(cooked);
            }
            RawPlot::GeoSpatialDataset(geo_data) => {
                for common in geo_data.raw_plots_common() {
                    let cooked =
                        CookedPlot::new(&common, data_id, format.descriptive_name().to_owned());
                    cooked_plots.push(cooked);
                }
            }
        }
    }
    cooked_plots
}

fn cook_plots_par(format: &SupportedFormat, id: u16) -> Vec<CookedPlot> {
    // Extract all GeoSpatial plots into owned Vec
    let geo_plots: Vec<RawPlotCommon> = format
        .raw_plots()
        .iter()
        .filter_map(|rp| match rp {
            RawPlot::GeoSpatialDataset(geo_data) => Some(geo_data.raw_plots_common()),
            RawPlot::Generic { .. } => None,
        })
        .flatten()
        .collect();

    let cooked_plots: Vec<CookedPlot> = geo_plots
        .par_iter()
        .chain(format.raw_plots().par_iter().filter_map(|rp| match rp {
            RawPlot::Generic { common } => Some(common),
            RawPlot::GeoSpatialDataset(_) => None,
        }))
        .map(|rpc| CookedPlot::new(rpc, id, format.descriptive_name().to_owned()))
        .collect();

    cooked_plots
}
