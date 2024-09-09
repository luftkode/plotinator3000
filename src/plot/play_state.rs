use std::time::{Duration, SystemTime};

/// Represents the state of the playback (either playing or paused).
#[derive(Default, PartialEq, serde::Deserialize, serde::Serialize)]
enum PlaybackState {
    Playing {
        start_time: SystemTime,
    },
    #[default]
    Paused,
}

/// State for managing real-time playback of the plot.
#[derive(Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlayState {
    state: PlaybackState,
    elapsed: Duration,   // Accumulated play time
    last_update_ms: f64, // Time in milliseconds at the last update
}

impl PlayState {
    /// Checks if the state is currently playing.
    pub fn is_playing(&self) -> bool {
        matches!(self.state, PlaybackState::Playing { .. })
    }

    /// Toggles between playing and paused states.
    pub fn toggle(&mut self) {
        match self.state {
            PlaybackState::Paused => self.play(),
            PlaybackState::Playing { .. } => self.pause(),
        }
    }

    /// Starts playing, recording the start time.
    fn play(&mut self) {
        self.state = PlaybackState::Playing {
            start_time: SystemTime::now(),
        };
    }

    /// Pauses the playback, updating the elapsed time.
    fn pause(&mut self) {
        if let PlaybackState::Playing { start_time } = self.state {
            self.elapsed += start_time.elapsed().unwrap_or_default();
            self.state = PlaybackState::Paused;
        }
    }

    /// Resets the playback to the beginning, clearing elapsed time and stopping playback.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.last_update_ms = 0.0;
        self.state = PlaybackState::Paused;
    }

    /// Computes total elapsed time, including active play time if playing.
    fn total_elapsed(&self) -> Duration {
        match self.state {
            PlaybackState::Playing { start_time } => {
                self.elapsed + start_time.elapsed().unwrap_or_default()
            }
            PlaybackState::Paused => self.elapsed,
        }
    }

    /// Returns total play time as a formatted string (e.g., "12.34s").
    pub fn formatted_time(&self) -> String {
        format!("{:.2}s", self.total_elapsed().as_secs_f64())
    }

    /// Returns time in milliseconds since the last update if playing.
    pub fn time_since_update(&mut self) -> Option<f64> {
        if let PlaybackState::Playing { .. } = self.state {
            let elapsed_ms = self.total_elapsed().as_millis() as f64;
            let delta_ms = elapsed_ms - self.last_update_ms;

            self.last_update_ms = elapsed_ms;

            (delta_ms > 0.0).then_some(delta_ms)
        } else {
            None
        }
    }
}
