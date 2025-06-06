use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::Duration,
};

use egui::{Color32, Context, RichText, ScrollArea, mutex::Mutex};
use semver::Version;

pub(super) mod error_window;
#[cfg(target_os = "windows")]
pub(super) mod pre_admin_window;
pub(super) mod updates_disabled;

use crate::APP_NAME;

use super::PlotinatorUpdater;

const START_UPDATE_PROGRESS: f32 = 10.0;
const LOAD_METADATA_PROGRESS: f32 = 30.0;
const WAIT_FOR_COUNTDOWN_PROGRESS: f32 = 40.0;
const UPDATE_DONE_PROGRESS: f32 = 100.0;

const COUNTDOWN_FOR_UPGRADE_SECS: u8 = 5;

/// Messages sent from the thread that performs the update, to the GUI thread that displays update progress
#[derive(Debug)]
enum UpdateStep {
    UnInit,
    Initial,
    LoadMetadata,
    WaitingForCountdown(u8),
    UpdateNowClicked,
    InstallUpdate,
    Completed(String),
    Cancelled,
    Error(String),
}

impl UpdateStep {
    fn update_progress(&self) -> f32 {
        match self {
            Self::Cancelled | Self::UnInit | Self::Initial | Self::Error(_) => 0.0,
            Self::LoadMetadata => START_UPDATE_PROGRESS,
            Self::WaitingForCountdown(countdown) => {
                LOAD_METADATA_PROGRESS + (COUNTDOWN_FOR_UPGRADE_SECS - countdown) as f32 * 2.
            }
            Self::UpdateNowClicked | Self::InstallUpdate => {
                WAIT_FOR_COUNTDOWN_PROGRESS + (COUNTDOWN_FOR_UPGRADE_SECS as f32 * 10.)
            }
            Self::Completed(_) => UPDATE_DONE_PROGRESS,
        }
    }

    fn next_progress(&self) -> f32 {
        match self {
            Self::Cancelled | Self::UnInit | Self::Error(_) => 0.0,
            Self::Initial => START_UPDATE_PROGRESS,
            Self::LoadMetadata => LOAD_METADATA_PROGRESS,
            Self::WaitingForCountdown(countdown) => {
                (COUNTDOWN_FOR_UPGRADE_SECS - countdown) as f32 * 10.
            }
            Self::Completed(_) | Self::UpdateNowClicked | Self::InstallUpdate => {
                UPDATE_DONE_PROGRESS
            }
        }
    }

    fn description(&self) -> String {
        match self {
            Self::UnInit => "Waiting for update agent...\n".to_owned(),
            Self::Initial => "Starting update...\n".to_owned(),
            Self::LoadMetadata => "Loading update metadata...\n".to_owned(),
            Self::WaitingForCountdown(countdown) => {
                if *countdown == 0 {
                    "0...\n".to_owned()
                } else {
                    format!("{countdown}... ")
                }
            }
            Self::UpdateNowClicked => "Now!\n".to_owned(),
            Self::InstallUpdate => "Retrieving update...\n".to_owned(),
            Self::Completed(description) => description.to_owned(),
            Self::Cancelled => "Update Cancelled!\n".to_owned(),
            Self::Error(e) => e.to_owned(),
        }
    }
}

/// Runs in a separate thread and performs update steps
#[allow(
    clippy::result_large_err,
    reason = "This function is only called once, so performance doesn't really suffer, Besides this lint is due to the axoupdater library, not really our fault"
)]
fn perform_update(
    version: Version,
    sender: &mpsc::Sender<UpdateStep>,
    countdown: &AtomicU8,
    update_cancelled: &AtomicBool,
    update_now_clicked: &AtomicBool,
    error_occurred: &Mutex<Option<String>>,
) -> bool {
    sender
        .send(UpdateStep::Initial)
        .expect("Failed sending update to gui");

    let mut updater = match PlotinatorUpdater::new(version) {
        Ok(updater) => updater,
        Err(e) => {
            sender
                .send(UpdateStep::Error(e.to_string()))
                .expect("Failed sending update to gui");
            *error_occurred.lock() = Some(e.to_string());
            return false;
        }
    };

    sender
        .send(UpdateStep::LoadMetadata)
        .expect("Failed sending update to gui");

    // At this point we force upgrade otherwise axoupdater will look for an install receipt and prevent us from updating (we don't use an install receipt.)
    updater.always_update(true);

    // wait for countdown
    while countdown.load(Ordering::SeqCst) != 0
        && !update_cancelled.load(Ordering::SeqCst)
        && !update_now_clicked.load(Ordering::SeqCst)
    {
        let prev_val = countdown.fetch_sub(1, Ordering::SeqCst);
        sender
            .send(UpdateStep::WaitingForCountdown(prev_val - 1))
            .expect("Failed sending update to gui");
        std::thread::sleep(Duration::from_secs(1));
    }
    if update_cancelled.load(Ordering::SeqCst) {
        sender
            .send(UpdateStep::Cancelled)
            .expect("Failed sending update to gui");
        return false;
    }
    if update_now_clicked.load(Ordering::SeqCst) {
        sender
            .send(UpdateStep::UpdateNowClicked)
            .expect("Failed sending update to gui");
    }
    sender
        .send(UpdateStep::InstallUpdate)
        .expect("Failed sending update to gui");

    match updater.run() {
        Ok(result) => {
            if let Some(update_result) = result {
                let msg = format!(
                    "Updated to: {app_name} v{new_version}\nInstalled at {install_prefix}",
                    app_name = APP_NAME,
                    new_version = update_result.new_version,
                    install_prefix = update_result.install_prefix
                );
                sender
                    .send(UpdateStep::Completed(msg))
                    .expect("Failed sending update to gui");
                true
            } else {
                sender
                    .send(UpdateStep::Completed(
                        "The newest version is already installed!\n".to_owned(),
                    ))
                    .expect("Failed sending update to gui");
                false
            }
        }
        Err(e) => {
            sender
                .send(UpdateStep::Error(e.to_string()))
                .expect("Failed sending update to gui");
            *error_occurred.lock() = Some(e.to_string());
            false
        }
    }
}

pub(super) fn show_simple_update_window(version: Version) -> eframe::Result<bool> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(crate::APP_ICON).expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    let is_updated = Arc::new(AtomicBool::new(false));
    let is_updated_clone: Arc<AtomicBool> = is_updated.clone();

    // Channel for log messages and progress updates
    let (tx, rx) = mpsc::channel::<UpdateStep>();

    // Shared log output and progress state
    let log_output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let progress_value: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    let current_update_step: Arc<Mutex<UpdateStep>> = Arc::new(Mutex::new(UpdateStep::UnInit));

    let countdown: Arc<AtomicU8> = Arc::new(AtomicU8::new(COUNTDOWN_FOR_UPGRADE_SECS));
    let update_cancelled: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let update_now_clicked: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let error_occurred: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // Run the update in a separate thread
    let updater_thread = thread::Builder::new()
        .name("Updater thread".to_owned())
        .spawn({
            let update_clone = is_updated_clone.clone();
            let countdown = countdown.clone();
            let update_cancelled = update_cancelled.clone();
            let update_now_clicked = update_now_clicked.clone();
            let error_occurred = error_occurred.clone();
            move || {
                let did_update = perform_update(
                    version,
                    &tx,
                    &countdown,
                    &update_cancelled,
                    &update_now_clicked,
                    &error_occurred,
                );

                update_clone.store(did_update, Ordering::Relaxed);
            }
        })
        .expect("Failed spawning updater thread");

    eframe::run_simple_native("Update Available", options, move |ctx, _frame| {
        // Process messages from the channel
        process_updater_thread_messages(
            &rx,
            &log_output,
            &update_cancelled,
            &progress_value,
            &current_update_step,
        );
        ui_show_update_window_central_panel(
            ctx,
            &log_output,
            &update_cancelled,
            &progress_value,
            &countdown,
            &update_now_clicked,
            &is_updated_clone,
            &error_occurred,
        );
        // Keep the UI updated with new log messages and progress
        ctx.request_repaint();
    })?;

    if let Err(e) = updater_thread.join() {
        log::error!("{e:?}");
    }

    Ok(is_updated.load(Ordering::Relaxed))
}

#[allow(
    clippy::too_many_arguments,
    reason = "No time and hopefully won't touch this code again for a long time. All these atomic variables etc. are really shared state between the updater GUI and the updater thread. They should be encapsulated in a struct"
)]
fn ui_show_update_window_central_panel(
    ctx: &Context,
    log_output: &Mutex<String>,
    update_cancelled: &AtomicBool,
    progress_value: &Mutex<f32>,
    countdown: &AtomicU8,
    update_now_clicked: &AtomicBool,
    is_updated: &AtomicBool,
    error_occurred: &Mutex<Option<String>>,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            if is_updated.load(Ordering::Relaxed) {
                ui.heading(
                    RichText::new("Update complete!")
                        .size(24.0)
                        .color(Color32::GREEN),
                );
            } else {
                ui.heading(RichText::new(format!("Updating {APP_NAME}")).size(24.0));
            }
            ui.add_space(10.0);

            if let Some(err) = error_occurred.lock().as_deref() {
                ui.label(
                    RichText::new(format!("Error performing update: {err}"))
                        .size(18.0)
                        .strong()
                        .color(Color32::RED),
                );
                ui.add_space(10.0);
                ui.label("Please report this error at the link below");
                ui.add(egui::Hyperlink::from_label_and_url(
                    "Plotinator3000 issues",
                    "https://github.com/luftkode/plotinator3000/issues",
                ));
                ui.add_space(20.0);
            } else {
                // Show the progress bar
                ui.add(egui::ProgressBar::new(*progress_value.lock() / 100.0));
                ui.add_space(20.0);

                if update_cancelled.load(Ordering::SeqCst) {
                    if ui
                        .button(RichText::new("Continue...").strong().size(18.0))
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else {
                    // Show the countdown or disable updates button
                    let countdown_val = countdown.load(Ordering::SeqCst);
                    if countdown_val != 0 && !is_updated.load(Ordering::SeqCst) {
                        if update_now_clicked.load(Ordering::SeqCst) {
                            ui.label(RichText::new("Updating now!".to_owned()).strong());
                        } else {
                            ui.label(
                                RichText::new(format!("Updating in {countdown_val}s...")).strong(),
                            );
                            ui.add_space(5.0);
                            if ui.button(RichText::new("Update now!").strong()).clicked() {
                                update_now_clicked.store(true, Ordering::SeqCst);
                            }
                            ui.add_space(10.0);
                            if ui
                                .button(RichText::new("Disable Updates (can be enabled later)"))
                                .clicked()
                            {
                                update_cancelled.store(true, Ordering::SeqCst);
                                // Create the disable updates file
                                super::create_disable_update_file()
                                    .expect("Failed to disable updates");
                            }
                        }
                    }
                }
            }

            // Show a "Close" button once the update is done
            if is_updated.load(Ordering::Relaxed) {
                ui_show_update_done_close_button(ctx, ui);
            }

            // Display the log output in a scrollable area
            ScrollArea::vertical()
                .auto_shrink([true; 2])
                .show(ui, |ui| {
                    let log = log_output.lock();
                    ui.label(log.as_str());
                });
        });
    });
}

fn ui_show_update_done_close_button(ctx: &Context, ui: &mut egui::Ui) {
    ui.label(RichText::new("Restart to use the new version").strong());
    ui.add_space(10.0);
    if ui
        .button(RichText::new("Close").strong().size(18.0))
        .clicked()
        || ui.input(|i| i.key_pressed(egui::Key::Enter))
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    ui.add_space(20.0);
}

fn process_updater_thread_messages(
    rx: &Receiver<UpdateStep>,
    log_output: &Mutex<String>,
    update_cancelled: &AtomicBool,
    progress_value: &Mutex<f32>,
    current_update_step: &Mutex<UpdateStep>,
) {
    while let Ok(update_msg) = rx.try_recv() {
        if matches!(update_msg, UpdateStep::Completed(_)) {
            log_output.lock().clear();
        }
        log_output.lock().push_str(&update_msg.description());
        if !update_cancelled.load(Ordering::SeqCst) {
            *progress_value.lock() = update_msg.update_progress();
        }
        *current_update_step.lock() = update_msg;
    }
    if !update_cancelled.load(Ordering::SeqCst) {
        let progress_target = current_update_step.lock().next_progress();
        let mut progress = progress_value.lock();

        // Slow down increment as it approaches the target
        if *progress < progress_target {
            let distance = progress_target - *progress;
            let increment = distance / 100.0; // Decrease increment as we approach the target
            *progress += increment;
        }
    }
}
