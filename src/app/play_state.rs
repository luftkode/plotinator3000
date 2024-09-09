use std::time::{Duration, SystemTime};

/// State for managing the playback of the plot, simulating real-time scrolling.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct PlayState {
    is_playing: bool,
    start_time: Option<SystemTime>,
    elapsed_time: Duration,     // Total accumulated play time
    last_plot_update_time: f64, // Time at the last plot update in milliseconds
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
            self.update_elapsed_time();
        }
    }

    fn update_elapsed_time(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.elapsed_time += start.elapsed().unwrap_or_default();
        }
    }

    fn current_elapsed_time(&self) -> Duration {
        self.start_time.map_or(self.elapsed_time, |start| {
            self.elapsed_time + start.elapsed().unwrap_or_default()
        })
    }

    /// Returns the total play time formatted as a string in seconds (e.g., "12.34s")
    pub fn formatted_play_time(&self) -> String {
        format!("{:.2}s", self.current_elapsed_time().as_secs_f64())
    }

    /// Returns the time in milliseconds since the last update, if currently playing.
    pub fn time_since_last_update(&mut self) -> Option<f64> {
        if self.is_playing {
            let total_elapsed_ms = self.current_elapsed_time().as_millis() as f64;
            let time_delta = total_elapsed_ms - self.last_plot_update_time;

            self.last_plot_update_time = total_elapsed_ms;

            (time_delta > 0.0).then_some(time_delta)
        } else {
            None
        }
    }
}
