use serde::{Deserialize, Serialize};

pub mod plot_data;

use plot_data::PlotData;

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
}
