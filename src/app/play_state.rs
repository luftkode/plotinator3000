use std::time::{Duration, SystemTime};

/// State for managing the playback of the plot, simulating real-time scrolling.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct PlayState {
    is_playing: bool,
    start_time: Option<SystemTime>,
    elapsed: Duration,   // Accumulated play time
    last_update_ms: f64, // Time in milliseconds at the last update
}

impl PlayState {
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Toggles between play and pause modes.
    pub fn toggle(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.play();
        }
    }

    fn play(&mut self) {
        self.start_time = Some(SystemTime::now());
        self.is_playing = true;
    }

    fn pause(&mut self) {
        self.update_elapsed();
        self.is_playing = false;
    }

    /// Updates elapsed time when pausing.
    fn update_elapsed(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.elapsed += start.elapsed().unwrap_or_default();
        }
    }

    /// Computes total elapsed time, including active play time.
    fn total_elapsed(&self) -> Duration {
        self.start_time
            .map(|start| self.elapsed + start.elapsed().unwrap_or_default())
            .unwrap_or(self.elapsed)
    }

    /// Returns total play time as a formatted string (e.g., "12.34s").
    pub fn formatted_time(&self) -> String {
        format!("{:.2}s", self.total_elapsed().as_secs_f64())
    }

    /// Returns time in milliseconds since the last update if playing.
    pub fn time_since_update(&mut self) -> Option<f64> {
        if self.is_playing {
            let elapsed_ms = self.total_elapsed().as_millis() as f64;
            let delta_ms = elapsed_ms - self.last_update_ms;

            self.last_update_ms = elapsed_ms;

            (delta_ms > 0.0).then_some(delta_ms)
        } else {
            None
        }
    }
}
