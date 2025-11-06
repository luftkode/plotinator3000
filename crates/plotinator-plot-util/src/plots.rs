use egui_plot::PlotBounds;
use plotinator_ui_util::ExpectedPlotRange;
use serde::{Deserialize, Serialize};

pub mod plot_data;
mod util;

use plot_data::PlotData;

use crate::{CookedPlot, StoredPlotLabels};

#[derive(Default, Debug, PartialEq, Deserialize, Serialize, Clone, Copy)]
pub struct MaxPlotBounds {
    pub percentage: Option<PlotBounds>,
    pub hundreds: Option<PlotBounds>,
    pub thousands: Option<PlotBounds>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Plots {
    pub percentage: PlotData,
    pub one_to_hundred: PlotData,
    pub thousands: PlotData,
}

impl Plots {
    /// necessary because the raw plot points are not serializable
    /// so they are skipped and initialized as None. So this
    /// generates them from the `raw_points` (only needed once per session)
    pub fn build_plots(&mut self) {
        self.percentage
            .plots_as_mut()
            .iter_mut()
            .for_each(|p| p.build_raw_plot_points());
        self.one_to_hundred
            .plots_as_mut()
            .iter_mut()
            .for_each(|p| p.build_raw_plot_points());
        self.thousands
            .plots_as_mut()
            .iter_mut()
            .for_each(|p| p.build_raw_plot_points());
    }

    pub fn add_plots(&mut self, plots: Vec<CookedPlot>) {
        for p in plots {
            match p.expected_range() {
                ExpectedPlotRange::Percentage => self.percentage.add_cooked(p),
                ExpectedPlotRange::Hundreds => self.one_to_hundred.add_cooked(p),
                ExpectedPlotRange::Thousands => self.thousands.add_cooked(p),
            }
        }
    }

    pub fn add_plot_labels(&mut self, plot_labels: Vec<StoredPlotLabels>) {
        for l in plot_labels {
            match l.expected_range {
                ExpectedPlotRange::Percentage => self.percentage.add_plot_labels(l),
                ExpectedPlotRange::Hundreds => self.one_to_hundred.add_plot_labels(l),
                ExpectedPlotRange::Thousands => self.thousands.add_plot_labels(l),
            }
        }
    }

    pub fn total_data_points(&self) -> u64 {
        let mut total_points: u64 = 0;
        for p in self.percentage().plots() {
            total_points += p.get_raw().len() as u64;
        }
        for p in self.one_to_hundred().plots() {
            total_points += p.get_raw().len() as u64;
        }
        for p in self.thousands().plots() {
            total_points += p.get_raw().len() as u64;
        }

        total_points
    }

    /// Return an iterator over all cooked plots across `percentage`,
    /// `one_to_hundred`, and `thousands`.
    pub fn individual_plots(&self) -> impl Iterator<Item = &CookedPlot> {
        self.percentage
            .plots()
            .iter()
            .chain(self.one_to_hundred.plots().iter())
            .chain(self.thousands.plots().iter())
    }

    pub fn percentage(&self) -> &PlotData {
        &self.percentage
    }

    pub fn percentage_mut(&mut self) -> &mut PlotData {
        &mut self.percentage
    }

    pub fn one_to_hundred(&self) -> &PlotData {
        &self.one_to_hundred
    }

    pub fn one_to_hundred_mut(&mut self) -> &mut PlotData {
        &mut self.one_to_hundred
    }

    pub fn thousands(&self) -> &PlotData {
        &self.thousands
    }

    pub fn thousands_mut(&mut self) -> &mut PlotData {
        &mut self.thousands
    }

    pub fn calc_all_plot_max_bounds(&mut self, max_bounds: &mut MaxPlotBounds) {
        self.percentage.calc_max_bounds();
        max_bounds.percentage = self.percentage.get_max_bounds();
        self.one_to_hundred.calc_max_bounds();
        max_bounds.hundreds = self.one_to_hundred.get_max_bounds();
        self.thousands.calc_max_bounds();
        max_bounds.thousands = self.thousands.get_max_bounds();
    }
}
