use super::loaded_logs;
use egui::{Color32, RichText, Ui};
use egui_phosphor::regular;
use egui_plot::PlotBounds;
use plotinator_ui_log_settings::LoadedLogSettings;

// One shot flags for modifying all logs and/or log groups
#[derive(PartialEq)]
enum OneShotDoAll {
    Collapse,
    Expand,
    Show,
    Hide,
}

#[derive(Default, PartialEq)]
pub struct LogGroupUIState {
    flag: Option<OneShotDoAll>,
    collapsed_log_groups: Vec<String>,
}

impl LogGroupUIState {
    pub(crate) fn expand_all(&mut self) {
        self.flag = Some(OneShotDoAll::Expand);
    }
    pub(crate) fn collapse_all(&mut self) {
        self.flag = Some(OneShotDoAll::Collapse);
    }
    pub(crate) fn show_all(&mut self) {
        self.flag = Some(OneShotDoAll::Show);
    }
    pub(crate) fn hide_all(&mut self) {
        self.flag = Some(OneShotDoAll::Hide);
    }

    fn handle_flags(&mut self, loaded_log_settings: &mut [LoadedLogSettings]) {
        let Some(flag) = self.flag.take() else {
            return;
        };
        match flag {
            OneShotDoAll::Collapse => self.flag = Some(OneShotDoAll::Collapse),
            OneShotDoAll::Expand => self.collapsed_log_groups.clear(),
            OneShotDoAll::Show => Self::set_all_visibility(loaded_log_settings, true),
            OneShotDoAll::Hide => Self::set_all_visibility(loaded_log_settings, false),
        }
    }

    fn set_all_visibility(loaded_log_settings: &mut [LoadedLogSettings], show: bool) {
        for f in loaded_log_settings.iter_mut() {
            *f.show_log_mut() = show;
        }
    }

    fn clear_flag(&mut self) {
        self.flag = None;
    }
}

/// Renders the UI for grouped log files.
pub(crate) fn show_log_groups(
    ui: &mut Ui,
    loaded_log_settings: &mut [LoadedLogSettings],
    state: &mut LogGroupUIState,
    selected_box: Option<PlotBounds>,
) {
    state.handle_flags(loaded_log_settings);
    let mut i = 0;
    while i < loaded_log_settings.len() {
        let name = loaded_log_settings[i].descriptive_name().to_owned();
        let group_end = loaded_log_settings[i..]
            .iter()
            .position(|s| s.descriptive_name() != name)
            .map_or(loaded_log_settings.len(), |pos| i + pos);

        let group_settings = &mut loaded_log_settings[i..group_end];
        let group_size = group_settings.len();

        if group_size == 1 {
            show_single_log(ui, &mut group_settings[0], selected_box);
        } else {
            show_group_header(ui, group_settings, &name, state);

            if !state.collapsed_log_groups.contains(&name) {
                for setting in group_settings {
                    loaded_logs::show_log_settings_ui(ui, setting, selected_box);
                    ui.end_row();
                }
            }
        }

        i = group_end;

        if i < loaded_log_settings.len() {
            ui.separator();
            ui.separator();
            ui.separator();
            ui.separator();
            ui.end_row();
        }
    }

    state.clear_flag();
}

/// Renders a single log without grouping overhead - more concise display
fn show_single_log(
    ui: &mut Ui,
    log_setting: &mut LoadedLogSettings,
    selected_box: Option<PlotBounds>,
) {
    *log_setting.cursor_hovering_on_mut() = false;
    loaded_logs::show_log_settings_ui(ui, log_setting, selected_box);
    ui.end_row();
}

/// Renders the header for a log group, including collapse/expand and group actions.
fn show_group_header(
    ui: &mut Ui,
    group_settings: &mut [LoadedLogSettings],
    name: &String,
    state: &mut LogGroupUIState,
) {
    if state.flag == Some(OneShotDoAll::Collapse) {
        state.collapsed_log_groups.push(name.to_owned());
    }
    let is_collapsed = state.collapsed_log_groups.contains(name);
    let mut group_hovered = false;

    // Column 1: Collapse/Expand Button with group count
    let count_text = if group_settings.len() > 1 {
        format!(" ({})", group_settings.len())
    } else {
        String::new()
    };

    let icon = if is_collapsed {
        RichText::new(format!("{} {name}{count_text}", regular::CARET_RIGHT))
    } else {
        RichText::new(format!("{} {name}{count_text}", regular::CARET_DOWN)).strong()
    };

    let button_collapse = ui.button(icon);
    if button_collapse.clicked() {
        if is_collapsed {
            state.collapsed_log_groups.retain(|g| g != name);
        } else {
            state.collapsed_log_groups.push(name.to_owned());
        }
    }
    if button_collapse.hovered() {
        group_hovered = true;
    }

    // Columns 2 & 3: Empty space for alignment
    if ui.label("").hovered() {
        group_hovered = true;
    }
    if ui.label("").hovered() {
        group_hovered = true;
    }

    // Column 4: Group action buttons (visibility and deletion)
    ui.horizontal(|ui| {
        if show_group_visibility_toggle(ui, group_settings).hovered() {
            group_hovered = true;
        }
        if show_group_delete_toggle(ui, group_settings).hovered() {
            group_hovered = true;
        }
    });

    for settings in group_settings {
        *settings.cursor_hovering_on_mut() = group_hovered;
    }

    ui.end_row();
}

/// Renders the visibility toggle button for a group.
fn show_group_visibility_toggle(
    ui: &mut Ui,
    group_settings: &mut [LoadedLogSettings],
) -> egui::Response {
    let any_shown = group_settings.iter().any(|s| s.show_log());
    let (icon, hover_text) = if any_shown {
        (
            RichText::new(regular::EYE).color(Color32::GREEN),
            "Hide all in group",
        )
    } else {
        (RichText::new(regular::EYE_SLASH), "Show all in group")
    };

    let response = ui.button(icon).on_hover_text(hover_text);
    if response.clicked() {
        let new_visibility = !any_shown;
        for setting in group_settings.iter_mut() {
            *setting.show_log_mut() = new_visibility;
        }
    }
    response
}

/// Renders the delete/restore toggle button for a group.
fn show_group_delete_toggle(
    ui: &mut Ui,
    group_settings: &mut [LoadedLogSettings],
) -> egui::Response {
    let all_marked_for_deletion = group_settings.iter().all(|s| s.marked_for_deletion());
    let (color, hover_text) = if all_marked_for_deletion {
        (Color32::RED, "Restore all in group")
    } else {
        (Color32::YELLOW, "Delete all in group")
    };

    let icon = RichText::new(regular::TRASH).color(color);
    let response = ui.button(icon).on_hover_text(hover_text);

    if response.clicked() {
        let new_marked_state = !all_marked_for_deletion;
        for setting in group_settings.iter_mut() {
            setting.mark_for_deletion(new_marked_state);
        }
    }
    response
}
