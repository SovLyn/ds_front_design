//! Reduced-motion preference, with a manual override.

use super::platform;

pub struct MotionPreference {
    pub reduced: bool,
    manual: bool,
    last_poll: f32,
}

impl MotionPreference {
    pub fn new(now: f32) -> Self {
        Self {
            reduced: platform::prefers_reduced_motion(),
            manual: false,
            last_poll: now,
        }
    }

    /// Re-read the OS setting, unless the reader has made an explicit choice.
    pub fn poll(&mut self, now: f32) {
        if now - self.last_poll < 1.0 {
            return;
        }
        self.last_poll = now;
        if !self.manual {
            self.reduced = platform::prefers_reduced_motion();
        }
    }

    pub fn toggle(&mut self) {
        self.reduced = !self.reduced;
        self.manual = true;
    }
}
