use std::time::{Duration, SystemTime};

/// State for "playing" the plot, meaning scrolling the plot(s) at real time pace.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct PlayState {
    is_playing: bool,               // Whether the plot is playing
    start_time: Option<SystemTime>, // Store the time when the animation started
    elapsed_time: Duration,
    elapsed_last_plot_update: f64,
}

impl PlayState {
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn toggle_play_pause(&mut self) {
        self.is_playing = !self.is_playing;
        if self.is_playing() {
            self.update_start_time();
        } else {
            self.accumulate_paused_time();
        }
    }

    fn update_start_time(&mut self) {
        self.start_time = Some(SystemTime::now());
    }

    fn accumulate_paused_time(&mut self) {
        if let Some(start) = self.start_time {
            // Add the time played since the last "start"
            self.elapsed_time += start.elapsed().unwrap_or_default();
            self.start_time = None; // Stop tracking the current time
        }
    }

    /// Update the play time and return a [String] with the formatted updated time
    pub fn calculate_play_time(&mut self) -> String {
        let seconds_elapsed = if self.is_playing() {
            if let Some(start) = self.start_time {
                // Calculate time passed since the current play session started
                let time_since_last_start = start.elapsed().unwrap_or_default();
                let total_elapsed_time = self.elapsed_time + time_since_last_start;

                total_elapsed_time.as_secs_f64()
            } else {
                Duration::default().as_secs_f64()
            }
        } else {
            self.elapsed_time.as_secs_f64()
        };
        format!("{:.2}s", seconds_elapsed)
    }

    /// If the play timer is running, returns the elapsed time in milliseconds from the last time it was updated (previous frame)
    pub fn play_timer_elapsed_update(&mut self) -> Option<f64> {
        if self.is_playing {
            self.start_time.and_then(|start_time| {
                let current_elapsed = start_time.elapsed().unwrap_or_default();
                let total_elapsed = self.elapsed_time + current_elapsed;
                let elapsed_since_last_update =
                    total_elapsed.as_millis() as f64 - self.elapsed_last_plot_update;

                self.elapsed_last_plot_update = total_elapsed.as_millis() as f64;

                if elapsed_since_last_update > 0.0 {
                    Some(elapsed_since_last_update)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }
}
