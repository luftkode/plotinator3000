use egui_plot::PlotBounds;
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plots::plot_data::{cooked_plot::CookedPlot, plot_labels::StoredPlotLabels};

use super::util;

pub mod cooked_plot;
pub mod plot_labels;

#[derive(Default, Deserialize, Serialize)]
pub struct PlotData {
    max_bounds: Option<PlotBounds>,
    plots: Vec<CookedPlot>,
    plot_labels: Vec<StoredPlotLabels>,
    next_auto_color_idx: usize,
}

impl PlotData {
    pub fn plots(&self) -> &[CookedPlot] {
        &self.plots
    }

    pub fn plots_as_mut(&mut self) -> &mut Vec<CookedPlot> {
        &mut self.plots
    }

    pub fn plot_labels(&self) -> &[StoredPlotLabels] {
        &self.plot_labels
    }

    pub fn plot_labels_as_mut(&mut self) -> &mut Vec<StoredPlotLabels> {
        &mut self.plot_labels
    }

    pub fn add_plot_labels(&mut self, plot_labels: StoredPlotLabels) {
        self.plot_labels.push(plot_labels);
    }

    /// Returns a borrowed iterator over the labels of all plots.
    ///
    /// The label is on the form `"<name> #<log_id>"`.
    pub fn plot_labels_iter(&self) -> impl Iterator<Item = &str> {
        self.plots.iter().map(|p| p.label())
    }

    /// Returns whether [`PlotData`] contains a plot with the label `plot_label`.
    ///
    /// The label is on the form `"<name> #<log_id>"`
    pub fn contains_plot(&self, plot_label: &str) -> bool {
        self.plots
            .iter()
            .any(|p: &CookedPlot| p.label() == plot_label)
    }

    /// Adds a plot to the [`PlotData`] collection if another plot with the same label doesn't already exist
    #[plotinator_proc_macros::log_time]
    pub fn add_plot(&mut self, raw_plot: &RawPlotCommon, log_id: u16, descriptive_name: &str) {
        // Crash in development but just emit an error message in release mode
        debug_assert!(
            raw_plot.points().len() > 1,
            "got raw_plot with less than 2 points. Datasets that contain less than 2 points should be removed by a parser before being passed to the plotter!"
        );
        if raw_plot.points().len() < 2 {
            eprintln!(
                "Error: Got raw_plot with less than 2 points. Datasets that contain less than 2 points should be removed by a parser before being passed to the plotter!"
            );
            return;
        }

        self.inner_add_plot(raw_plot, log_id, descriptive_name);
    }

    pub fn inner_add_plot(
        &mut self,
        raw_plot: &RawPlotCommon,
        log_id: u16,
        descriptive_name: &str,
    ) {
        let new_plot = CookedPlot::new(raw_plot, log_id, descriptive_name.to_owned());
        self.add_cooked(new_plot);
    }

    pub fn add_cooked(&mut self, plot: CookedPlot) {
        log::info!("Adding plot: {}", plot.name());
        self.plots.push(plot);
        self.calc_max_bounds();
    }

    pub fn calc_max_bounds(&mut self) {
        let mut max_bounds: Option<PlotBounds> = None;
        for p in &self.plots {
            if let Some(max_bounds) = &mut max_bounds {
                max_bounds.merge(&p.get_max_bounds());
            } else {
                max_bounds = Some(p.get_max_bounds());
            }
        }
        if let Some(max_bounds) = &mut max_bounds {
            // finally extend each bound by 10%
            let margin_fraction = egui::Vec2::splat(0.1);
            max_bounds.add_relative_margin_x(margin_fraction);
            max_bounds.add_relative_margin_y(margin_fraction);
        }
        self.max_bounds = max_bounds;
    }

    pub fn get_max_bounds(&self) -> Option<PlotBounds> {
        self.max_bounds
    }
}
