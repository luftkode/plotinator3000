use std::{
    env,
    fs::{self, File},
    io,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use axoupdater::AxoupdateResult;
use egui::{mutex::Mutex, RichText, ScrollArea};

use crate::APP_NAME;

const DISABLE_UPDATES_FILE: &str = "logviewer_disable_updates";
const BYPASS_UPDATES_ENV_VAR: &str = "LOGVIEWER_BYPASS_UPDATES";

const FORCE_UPGRADE: bool = true;

/// Check for the environment variable to bypass updates
pub fn bypass_updates() -> io::Result<bool> {
    if let Ok(value) = env::var(BYPASS_UPDATES_ENV_VAR) {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            log::info!("Update bypassed due to environment variable.");
            return Ok(true);
        }
    }
    Ok(false)
}

fn create_disable_update_file() -> io::Result<()> {
    let exe_path = std::env::current_exe().expect("Could not find executable");
    let exe_dir = exe_path
        .parent()
        .expect("Could not find executable directory");
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    File::create(disable_updates_file).expect("Failed to create disable updates file");
    log::info!("Updates disabled");
    Ok(())
}

fn remove_disable_update_file() -> io::Result<()> {
    let exe_path = std::env::current_exe().expect("Could not find executable");
    let exe_dir = exe_path
        .parent()
        .expect("Could not find executable directory");
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    fs::remove_file(disable_updates_file).expect("Failed to remove disable updates file");
    log::info!("Updates re-enabled");
    Ok(())
}

pub fn is_updates_disabled() -> io::Result<bool> {
    // Get the path of the executable
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find executable directory",
        )
    })?;

    // Check if the logviewer_disable_updates file exists
    let disable_updates_file = exe_dir.join(DISABLE_UPDATES_FILE);
    if disable_updates_file.exists() {
        log::warn!("Update bypassed due to presence of '{DISABLE_UPDATES_FILE}' file.");
        return Ok(true);
    }
    Ok(false)
}

pub fn is_update_available() -> AxoupdateResult<bool> {
    if axoupdater::AxoUpdater::new_for(APP_NAME)
        .load_receipt()?
        .always_update(FORCE_UPGRADE) // Set to test it
        .is_update_needed_sync()?
    {
        log::warn!("{APP_NAME} is outdated; please upgrade!");
        Ok(true)
    } else {
        log::info!("{APP_NAME} is up to date");
        Ok(false)
    }
}

const START_UPDATE_PROGRESS: f32 = 10.0;
const LOAD_METADATA_PROGRESS: f32 = 30.0;
const WAIT_FOR_COUNTDOWN_PROGRESS: f32 = 40.0;
const UPDATE_DONE_PROGRESS: f32 = 100.0;

const COUNTDOWN_FOR_UPGRADE_SECS: u8 = 5;

#[derive(Debug)]
enum UpdateStep {
    UnInit,
    Initial,
    LoadMetadata,
    WaitingForCountdown(u8),
    InstallUpdate,
    Completed(String),
    Cancelled,
}
impl UpdateStep {
    fn update_progress(&self) -> f32 {
        match self {
            UpdateStep::UnInit | UpdateStep::Initial => 0.0,
            UpdateStep::LoadMetadata => START_UPDATE_PROGRESS,
            Self::WaitingForCountdown(countdown) => {
                LOAD_METADATA_PROGRESS + (COUNTDOWN_FOR_UPGRADE_SECS - countdown) as f32 * 2.
            }
            UpdateStep::InstallUpdate => {
                WAIT_FOR_COUNTDOWN_PROGRESS + (COUNTDOWN_FOR_UPGRADE_SECS as f32 * 10.)
            }
            UpdateStep::Completed(_) => UPDATE_DONE_PROGRESS,
            Self::Cancelled => 0.0,
        }
    }

    fn next_progress(&self) -> f32 {
        match self {
            UpdateStep::UnInit => 0.0,
            UpdateStep::Initial => START_UPDATE_PROGRESS,
            UpdateStep::LoadMetadata => LOAD_METADATA_PROGRESS,
            Self::WaitingForCountdown(countdown) => {
                (COUNTDOWN_FOR_UPGRADE_SECS - countdown) as f32 * 10.
            }
            UpdateStep::InstallUpdate => UPDATE_DONE_PROGRESS,
            UpdateStep::Completed(_) => UPDATE_DONE_PROGRESS,
            Self::Cancelled => 0.0,
        }
    }

    fn description(&self) -> String {
        match self {
            UpdateStep::UnInit => "Waiting for update agent...\n".to_owned(),
            UpdateStep::Initial => "Starting update...\n".to_owned(),
            UpdateStep::LoadMetadata => "Loading update metadata...\n".to_owned(),
            Self::WaitingForCountdown(countdown) => {
                if *countdown == 0 {
                    "0...\n".to_owned()
                } else {
                    format!("{countdown}... ")
                }
            }
            UpdateStep::InstallUpdate => "Retrieving update...\n".to_owned(),
            UpdateStep::Completed(description) => description.to_owned(),
            Self::Cancelled => "Update Cancelled!\n".to_owned(),
        }
    }
}

fn run_update_process(
    sender: mpsc::Sender<UpdateStep>,
    countdown: Arc<AtomicU8>,
    update_cancelled: Arc<AtomicBool>,
) -> AxoupdateResult<bool> {
    sender
        .send(UpdateStep::Initial)
        .expect("Failed sending update to gui");

    let mut updater = axoupdater::AxoUpdater::new_for(APP_NAME);
    updater.disable_installer_output();

    sender
        .send(UpdateStep::LoadMetadata)
        .expect("Failed sending update to gui");
    updater.load_receipt()?;

    updater.always_update(FORCE_UPGRADE);

    // wait for countdown
    while countdown.load(Ordering::SeqCst) != 0 && !update_cancelled.load(Ordering::SeqCst) {
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
        return Ok(false);
    }
    sender
        .send(UpdateStep::InstallUpdate)
        .expect("Failed sending update to gui");

    if let Some(result) = updater.run_sync()? {
        let msg = format!(
            "Updated to: {APP_NAME} v{}\nInstalled at {}",
            result.new_version, result.install_prefix
        );
        sender
            .send(UpdateStep::Completed(msg))
            .expect("Failed sending update to gui");
        Ok(true)
    } else {
        sender
            .send(UpdateStep::Completed(
                "The newest version is already installed!\n".to_string(),
            ))
            .expect("Failed sending update to gui");
        Ok(false)
    }
}

pub fn show_simple_update_window() -> eframe::Result<bool> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        //centered: true,
        ..Default::default()
    };

    let is_updated = Arc::new(AtomicBool::new(false));
    let update_clone = is_updated.clone();

    // Channel for log messages and progress updates
    let (tx, rx) = mpsc::channel::<UpdateStep>();

    // Shared log output and progress state
    let log_output = Arc::new(Mutex::new(String::new()));
    let progress_value = Arc::new(Mutex::new(0.0));
    let current_update_step = Arc::new(Mutex::new(UpdateStep::UnInit));

    let countdown = Arc::new(AtomicU8::new(COUNTDOWN_FOR_UPGRADE_SECS));
    let update_cancelled = Arc::new(AtomicBool::new(false));

    // Run the update in a separate thread
    let updater_thread = thread::Builder::new()
        .name("Updater thread".to_owned())
        .spawn({
            let update_clone = update_clone.clone();
            let countdown = countdown.clone();
            let update_cancelled = update_cancelled.clone();
            move || {
                if let Ok(did_update) = run_update_process(tx, countdown, update_cancelled) {
                    update_clone.store(did_update, Ordering::Relaxed);
                }
            }
        })
        .expect("Failed spawning updater thread");

    eframe::run_simple_native("Update Available", options, move |ctx, _frame| {
        // Process messages from the channel
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
            let tmp_curr_update_step = current_update_step.lock();
            let mut progress = progress_value.lock();
            let target = tmp_curr_update_step.next_progress();

            // Slow down increment as it approaches the target
            if *progress < target {
                let distance = target - *progress;
                let increment = distance / 100.0; // Decrease increment as we approach the target
                *progress += increment;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new(format!("Updating {}", APP_NAME)).size(24.0));
                ui.add_space(10.0);

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
                    if countdown_val != 0 {
                        ui.label(
                            RichText::new(format!("Updating in {countdown_val}s...")).strong(),
                        );
                        if ui
                            .button(RichText::new("Disable Updates (can be enabled later)"))
                            .clicked()
                        {
                            update_cancelled.store(true, Ordering::SeqCst);
                            // Create the disable updates file
                            create_disable_update_file().expect("Failed to disable updates");
                        }
                    }
                }

                // Show a "Close" button once the update is done
                if update_clone.load(Ordering::Relaxed) {
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

                // Display the log output in a scrollable area
                ScrollArea::vertical()
                    .auto_shrink([true; 2])
                    .show(ui, |ui| {
                        let log = log_output.lock();
                        ui.label(log.as_str());
                    });
            });
        });
        // Keep the UI updated with new log messages and progress
        ctx.request_repaint();
    })?;

    if let Err(e) = updater_thread.join() {
        log::error!("{e:?}");
    }

    Ok(is_updated.load(Ordering::Relaxed))
}

/// Returns true if updates are re-enabled
pub fn show_simple_updates_are_disabled_window() -> eframe::Result<bool> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        //centered: true,
        ..Default::default()
    };

    let re_enable_updates_local = Arc::new(AtomicBool::new(false));
    let re_enable_updates = re_enable_updates_local.clone();

    eframe::run_simple_native(APP_NAME, options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("⚠").size(30.0));
                ui.add_space(10.0);

                if re_enable_updates.load(Ordering::SeqCst) {
                    ui.label(
                        RichText::new("Restart to run the updater")
                            .size(18.)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    if ui
                        .button(RichText::new("Close").strong().size(18.0))
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else {
                    ui.label(
                        RichText::new(format!("Updates are currently disabled"))
                            .strong()
                            .size(18.),
                    );
                    ui.add_space(10.0);
                    if ui
                        .button(RichText::new("Re-enable updates").strong().size(18.0))
                        .clicked()
                    {
                        // Remove the disable updates file
                        remove_disable_update_file().expect("Failed to disable updates");
                        re_enable_updates.store(true, Ordering::SeqCst);
                    }
                }

                // Show a "Continue" button to open the GUI
                ui.add_space(10.0);
                if ui
                    .button(RichText::new("Continue...").strong().size(18.0))
                    .clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    })?;

    Ok(re_enable_updates_local.load(Ordering::SeqCst))
}
