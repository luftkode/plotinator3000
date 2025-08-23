use std::collections::BTreeMap;

use egui::RichText;
use plotinator_plot_util::PlotValues;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PlotNameFilter {
    plots: Vec<PlotNameShow>,
    // Cache for grouped plots - not serialized since it's derived data
    #[serde(skip)]
    grouped_plots_cache: BTreeMap<String, Vec<usize>>,
    // Track if plots have been modified to invalidate cache
    #[serde(skip)]
    cache_valid: bool,
}

impl PlotNameFilter {
    pub fn add_plot(&mut self, plot_name_show: PlotNameShow) {
        self.plots.push(plot_name_show);
        // sort in alphabetical order
        self.plots.sort_unstable_by(|a, b| a.name().cmp(b.name()));
        // Invalidate cache since plots have changed
        self.invalidate_cache();
    }

    /// Returns whether the filter already contains a plot with the given name and association
    pub fn contains(&self, plot_name: &str, associated_descriptive_name: &str) -> bool {
        self.plots.iter().any(|p| {
            p.name() == plot_name && p.associated_descriptive_name == associated_descriptive_name
        })
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
                .find(|pf| {
                    pf.name() == pv.name()
                        && pf.associated_descriptive_name == pv.associated_descriptive_name()
                })
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

    /// Invalidate the cache when the structure of plots changes
    fn invalidate_cache(&mut self) {
        self.grouped_plots_cache.clear();
        self.cache_valid = false;
    }

    /// Build (if necessary) and retrieve the cached grouped plots
    fn get_grouped_plots(&mut self) -> &BTreeMap<String, Vec<usize>> {
        if !self.cache_valid {
            for (index, plot) in self.plots.iter().enumerate() {
                self.grouped_plots_cache
                    .entry(plot.associated_descriptive_name.clone())
                    .or_default()
                    .push(index);
            }

            self.cache_valid = true;
        }

        &self.grouped_plots_cache
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

        // Get the cached grouped plots
        let grouped_plots = self.get_grouped_plots().clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (descriptive_name, plot_indices) in grouped_plots {
                    // Create a CollapsingHeader for each group.
                    let header_text = format!("{descriptive_name} ({})", plot_indices.len());
                    egui::CollapsingHeader::new(RichText::new(header_text).strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            // Render the toggle buttons for each plot within the group.
                            for &plot_index in &plot_indices {
                                let plot = &mut self.plots[plot_index];
                                let dataset_count = count_plot_occurrences(plot.name());
                                let plot_name = plot.name().to_owned();

                                ui.horizontal(|ui| {
                                    plot.hovered =
                                        ui.toggle_value(&mut plot.show, plot_name).hovered();
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
            });
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PlotNameShow {
    name: String,
    // The descriptive name of the log it came from
    associated_descriptive_name: String,
    show: bool,
    // On hover: Highlight the line plots that match the name
    hovered: bool,
}

impl PlotNameShow {
    pub fn new(name: String, show: bool, associated_descriptive_name: String) -> Self {
        Self {
            name,
            show,
            associated_descriptive_name,
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
