use serde::{Deserialize, Serialize};

pub mod plot_data;
mod util;

use plot_data::{PlotData, PlotValues};

#[derive(Default, Deserialize, Serialize)]
pub struct Plots {
    pub percentage: PlotData,
    pub one_to_hundred: PlotData,
    pub thousands: PlotData,
}

impl Plots {
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

    pub fn calc_all_plot_x_min_max(&self, x_min_max: &mut Option<(f64, f64)>) {
        calc_plot_x_min_max(self.percentage().plots(), x_min_max);
        calc_plot_x_min_max(self.one_to_hundred().plots(), x_min_max);
        calc_plot_x_min_max(self.thousands().plots(), x_min_max);
    }
}

// Go through each plot and find the minimum and maximum x-value (timestamp) and save it in `x_min_max`
fn calc_plot_x_min_max(plots: &[PlotValues], x_min_max: &mut Option<(f64, f64)>) {
    for plot in plots {
        if plot.total_data_points() < 2 {
            continue;
        }
        let first_x = plot.first_timestamp();
        let last_x = plot.last_timestamp();

        if let Some((current_x_min, current_x_max)) = x_min_max {
            if first_x < *current_x_min {
                *current_x_min = first_x;
            }
            if last_x > *current_x_max {
                *current_x_max = last_x;
            }
        } else {
            x_min_max.replace((first_x, last_x));
        }
    }
}
