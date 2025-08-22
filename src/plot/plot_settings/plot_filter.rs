use egui::RichText;
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

    /// Returns whether a plot with the given name should be highlighted (is hovered)
    pub fn should_highlight(&self, plot_name: &str) -> bool {
        self.plots
            .iter()
            .find(|p| p.name() == plot_name)
            .is_some_and(|p| p.is_hovered())
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
    pub fn show(&mut self, ui: &mut egui::Ui, plots: &plotinator_plot_util::Plots) {
        let mut enable_all = false;
        let mut disable_all = false;

        // Header with global controls and stats
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Show all").strong()).clicked() {
                enable_all = true;
            }
            if ui.button(RichText::new("Hide all").strong()).clicked() {
                disable_all = true;
            }
            ui.separator();
            let shown_count = self.plots.iter().filter(|p| p.show()).count();
            let total_count = self.plots.len();
            ui.label(format!("Shown: {shown_count}/{total_count} plot types"));
        });

        ui.separator();

        if enable_all {
            self.set_show_all();
        } else if disable_all {
            self.set_hide_all();
        }

        // Helper function to count occurrences of a plot name across all datasets
        let count_plot_occurrences = |plot_name: &str| -> usize {
            let mut count = 0;

            // Count in percentage plots
            count += plots
                .percentage()
                .plots()
                .iter()
                .filter(|p| p.name() == plot_name)
                .count();

            // Count in hundreds plots
            count += plots
                .one_to_hundred()
                .plots()
                .iter()
                .filter(|p| p.name() == plot_name)
                .count();

            // Count in thousands plots
            count += plots
                .thousands()
                .plots()
                .iter()
                .filter(|p| p.name() == plot_name)
                .count();

            count
        };

        // Scrollable area for plot toggles
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for plot in &mut self.plots {
                    let dataset_count = count_plot_occurrences(plot.name());
                    let plot_name = plot.name().to_owned();

                    ui.horizontal(|ui| {
                        plot.hovered = ui.toggle_value(&mut plot.show, plot_name).hovered();
                        if dataset_count > 0 {
                            ui.label(
                                RichText::new(format!("({dataset_count})"))
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                    });
                }
            });
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PlotNameShow {
    name: String,
    show: bool,
    // On hover: Highlight the line plots that match the name
    hovered: bool,
}

impl PlotNameShow {
    pub fn new(name: String, show: bool) -> Self {
        Self {
            name,
            show,
            hovered: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn show(&self) -> bool {
        self.show
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    pub fn set_show(&mut self, show: bool) {
        self.show = show;
    }
}
