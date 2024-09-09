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
        if self.is_playing {
            self.start_time = Some(SystemTime::now());
        } else {
            self.accumulate_paused_time();
        }
    }

    fn accumulate_paused_time(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.elapsed_time += start.elapsed().unwrap_or_default();
        }
    }

    fn total_elapsed(&self) -> Duration {
        self.start_time
            .map(|start| self.elapsed_time + start.elapsed().unwrap_or_default())
            .unwrap_or(self.elapsed_time)
    }

    /// Update the play time and return a formatted string with the updated time
    pub fn calculate_play_time(&self) -> String {
        format!("{:.2}s", self.total_elapsed().as_secs_f64())
    }

    /// If the play timer is running, returns the elapsed time in milliseconds from the last update (previous frame)
    pub fn play_timer_elapsed_update(&mut self) -> Option<f64> {
        if self.is_playing {
            let elapsed_since_last_update =
                self.total_elapsed().as_millis() as f64 - self.elapsed_last_plot_update;

            self.elapsed_last_plot_update = self.total_elapsed().as_millis() as f64;

            (elapsed_since_last_update > 0.0).then_some(elapsed_since_last_update)
        } else {
            None
        }
    }
}
