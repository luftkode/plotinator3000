use egui_plot::PlotBounds;
use serde::{Deserialize, Serialize};

use super::PlotType;

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct YAxisLock {
    pub lock_y_axis: bool,
    bounds_percentage: BoundsLock,
    bounds_hundreds: BoundsLock,
    bounds_thousands: BoundsLock,
}

impl YAxisLock {
    pub fn handle<F>(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi<'_>,
        plot_type: PlotType,
        between_bounds_update_fn: F,
    ) where
        F: FnOnce(&mut egui_plot::PlotUi<'_>),
    {
        // Note to developer: This function might look needlessly complicated but remember that `plot_bounds()` returns the bounds from the previous frame
        // so we basically need to keep the state of the previous 2 frames to make sure we enforce the y-axis lock but stay compatible with
        // zooming and scrolling while we have linked axes between plots.
        if self.lock_y_axis {
            // When the lock is enabled we get the bounds from the previous frame and set the locked Y-min/max based on those values
            let mut plot_bounds = plot_ui.plot_bounds();
            let y_range = plot_bounds.range_y();
            self.set_y_lock_if_not_set((*y_range.start(), *y_range.end()), plot_type);

            if let Some(prev_bounds) = self.get_bounds(plot_type) {
                // If the bounds are not the same as the one we stored last frame, then we set the y-values to the locked values
                if prev_bounds != plot_bounds {
                    if let Some((y_locked_min, y_locked_max)) = self.get_locked(plot_type) {
                        let bounds_locked_y =
                            PlotBounds::from_min_max([0.0, y_locked_min], [0.0, y_locked_max]);
                        plot_bounds.set_y(&bounds_locked_y);
                        plot_ui.set_plot_bounds(plot_bounds);
                    }
                }
            }
        }
        between_bounds_update_fn(plot_ui);

        if !self.lock_y_axis {
            self.reset_locked();
        }
        // Store the plot bounds
        self.set_bounds(plot_type, plot_ui.plot_bounds());
    }

    fn get_locked(&self, plot_type: PlotType) -> Option<(f64, f64)> {
        match plot_type {
            PlotType::Percentage => self.bounds_percentage.get_locked(),
            PlotType::Hundreds => self.bounds_hundreds.get_locked(),
            PlotType::Thousands => self.bounds_thousands.get_locked(),
        }
    }

    /// If the locked Y-values are not already set, set them to `y_min_max`
    fn set_y_lock_if_not_set(&mut self, y_min_max: (f64, f64), plot_type: PlotType) {
        if !self.is_y_lock_set(plot_type) {
            match plot_type {
                PlotType::Percentage => {
                    self.bounds_percentage.lock(y_min_max);
                }
                PlotType::Hundreds => {
                    self.bounds_hundreds.lock(y_min_max);
                }
                PlotType::Thousands => {
                    self.bounds_thousands.lock(y_min_max);
                }
            }
        }
    }

    fn is_y_lock_set(&self, plot_type: PlotType) -> bool {
        match plot_type {
            PlotType::Percentage => self.bounds_percentage.is_y_lock_set(),
            PlotType::Hundreds => self.bounds_hundreds.is_y_lock_set(),
            PlotType::Thousands => self.bounds_thousands.is_y_lock_set(),
        }
    }

    fn reset_locked(&mut self) {
        self.bounds_percentage.reset();
        self.bounds_hundreds.reset();
        self.bounds_thousands.reset();
    }

    fn get_bounds(&self, plot_type: PlotType) -> Option<PlotBounds> {
        match plot_type {
            PlotType::Percentage => self.bounds_percentage.bounds(),
            PlotType::Hundreds => self.bounds_hundreds.bounds(),
            PlotType::Thousands => self.bounds_thousands.bounds(),
        }
    }

    fn set_bounds(&mut self, plot_type: PlotType, bounds: PlotBounds) {
        match plot_type {
            PlotType::Percentage => self.bounds_percentage.update_bounds(bounds),
            PlotType::Hundreds => self.bounds_hundreds.update_bounds(bounds),
            PlotType::Thousands => self.bounds_thousands.update_bounds(bounds),
        }
    }
}

/// Store the state of an axis lock, needs to store the current bounds such that it can be "behind" an additional frame
/// which makes it compatible with zoom/scroll/etc so it lets zooming etc. update the plots and waits another frame to enforce the bounds
/// (the update logic is implemented e.g. in the [`YAxisLock`] )
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct BoundsLock {
    current_bounds: Option<PlotBounds>,
    lock_min_max: Option<(f64, f64)>,
}

impl BoundsLock {
    /// Returns true if there is a lock set
    pub fn is_y_lock_set(&self) -> bool {
        self.lock_min_max.is_some()
    }

    /// Update the bounds
    pub fn update_bounds(&mut self, new_bounds: PlotBounds) {
        self.current_bounds = Some(new_bounds);
    }

    /// Get the bounds
    pub fn bounds(&self) -> Option<PlotBounds> {
        self.current_bounds
    }

    /// Reset the lock
    pub fn reset(&mut self) {
        self.lock_min_max = None;
    }

    /// Lock the bounds of the axis
    pub fn lock(&mut self, min_max: (f64, f64)) {
        self.lock_min_max = Some(min_max);
    }

    /// Get the locked bounds
    pub fn get_locked(&self) -> Option<(f64, f64)> {
        self.lock_min_max
    }
}
