use egui::{Response, RichText};
use plotinator_plot_util::PlotValues;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PlotNameFilter {
    plots: Vec<PlotNameShow>,
}

impl PlotNameFilter {
    pub fn add_plot(&mut self, plot_name_show: PlotNameShow) {
        self.plots.push(plot_name_show);
        // sort in alphabetical order
        self.plots.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    }

    /// Returns whether the filter already contains a plot with the given name
    pub fn contains_name(&self, plot_name: &str) -> bool {
        self.plots.iter().any(|p| p.name() == plot_name)
    }

    /// Takes in a slice of [`PlotValues`] and a function that filters based on log id
    /// and returns an iterator that yields all the [`PlotValues`] that should be shown
    ///
    /// The id filter `fn_show_id` should return true if the given log should be shown according to the ID filter
    pub fn filter_plot_values<'pv, IF>(
        &'pv self,
        plot_data: &'pv [PlotValues],
        fn_show_id: IF,
    ) -> impl Iterator<Item = &'pv PlotValues>
    where
        IF: Fn(u16) -> bool,
    {
        plot_data.iter().filter(move |pv| {
            self.plots
                .iter()
                .find(|pf| pf.name() == pv.name())
                .is_some_and(|pf| pf.show() && fn_show_id(pv.log_id()))
        })
    }

    pub fn set_show_all(&mut self) {
        for p in &mut self.plots {
            p.set_show(true);
        }
    }

    pub fn set_hide_all(&mut self) {
        for p in &mut self.plots {
            p.set_show(false);
        }
    }

    /// Shows the window where users can toggle plot visibility based on plot labels
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let mut enable_all = false;
        let mut disable_all = false;
        egui::Grid::new("global_filter_settings").show(ui, |ui| {
            if ui
                .button(RichText::new("Show all").strong().heading())
                .clicked()
            {
                enable_all = true;
            }
            if ui
                .button(RichText::new("Hide all").strong().heading())
                .clicked()
            {
                disable_all = true;
            }
        });
        if enable_all {
            self.set_show_all();
        } else if disable_all {
            self.set_hide_all();
        }

        for plot in &mut self.plots {
            plot.show_as_toggle_value(ui);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PlotNameShow {
    name: String,
    show: bool,
}

impl PlotNameShow {
    pub fn new(name: String, show: bool) -> Self {
        Self { name, show }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn show(&self) -> bool {
        self.show
    }

    pub fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    pub fn show_as_toggle_value(&mut self, ui: &mut egui::Ui) -> Response {
        ui.toggle_value(&mut self.show, self.name.clone())
    }
}
