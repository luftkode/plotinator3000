use serde::{Deserialize, Serialize};

pub mod plot_data;

use plot_data::{PlotData, PlotWithName};

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Plots {
    percentage: PlotData,
    one_to_hundred: PlotData,
    thousands: PlotData,
}

impl Plots {
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
fn calc_plot_x_min_max(plots: &[PlotWithName], x_min_max: &mut Option<(f64, f64)>) {
    for plot in plots {
        if plot.raw_plot.len() < 2 {
            continue;
        }
        let Some(first_x) = plot.raw_plot.first().and_then(|f| f.first()) else {
            continue;
        };
        let Some(last_x) = plot.raw_plot.last().and_then(|l| l.first()) else {
            continue;
        };
        if let Some((current_x_min, current_x_max)) = x_min_max {
            if first_x < current_x_min {
                *current_x_min = *first_x;
            }
            if last_x > current_x_max {
                *current_x_max = *last_x;
            }
        } else {
            x_min_max.replace((*first_x, *last_x));
        }
    }
}
