use crate::app;

/// Get the current time with support for wasm and native.
fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .expect("no global `window` exists")
            .performance()
            .expect("should have performance on window")
            .now()
            / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs_f64()
    }
}

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
enum PlaybackState {
    Playing { start_time: f64 },
    Paused { pause_time: f64 },
}

impl Default for PlaybackState {
    fn default() -> Self {
        PlaybackState::Paused { pause_time: now() }
    }
}

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlayState {
    state: PlaybackState,
    elapsed: f64,
    last_update: f64,
}

impl Default for PlayState {
    fn default() -> Self {
        Self {
            state: PlaybackState::default(),
            elapsed: 0.0,
            last_update: now(),
        }
    }
}

impl PlayState {
    pub fn handle_playback_button_press(
        &mut self,
        playback_button_event: app::PlayBackButtonEvent,
    ) {
        match playback_button_event {
            app::PlayBackButtonEvent::PlayPause => self.toggle(),
            app::PlayBackButtonEvent::Reset => self.reset(),
        }
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, PlaybackState::Playing { .. })
    }

    pub fn toggle(&mut self) {
        match self.state {
            PlaybackState::Paused { .. } => self.play(),
            PlaybackState::Playing { .. } => self.pause(),
        }
    }

    fn play(&mut self) {
        let now = now();
        if let PlaybackState::Paused { pause_time } = self.state {
            self.last_update += now - pause_time;
        }
        self.state = PlaybackState::Playing { start_time: now };
    }

    fn pause(&mut self) {
        if let PlaybackState::Playing { start_time } = self.state {
            let now = now();
            self.elapsed += now - start_time;
            self.state = PlaybackState::Paused { pause_time: now };
        }
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.last_update = now();
        self.state = PlaybackState::Paused { pause_time: now() };
    }

    fn total_elapsed(&self) -> f64 {
        match self.state {
            PlaybackState::Playing { start_time } => self.elapsed + (now() - start_time),
            PlaybackState::Paused { .. } => self.elapsed,
        }
    }

    pub fn formatted_time(&self) -> String {
        format!("{:.2}s", self.total_elapsed())
    }

    pub fn time_since_update(&mut self) -> Option<f64> {
        if let PlaybackState::Playing { .. } = self.state {
            let now = now();
            let delta = now - self.last_update;
            self.last_update = now;
            Some(delta)
        } else {
            None
        }
    }
}