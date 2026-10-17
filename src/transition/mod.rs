//! The page state machine.
//!
//! The app never scrolls a container. It owns one current page and, while a
//! transition is running, the outgoing and incoming page indices. The renderer
//! receives the same pair so the background can cross-blend in lockstep with
//! the type.

/// Smooth, slightly theatrical easing. `t` is clamped by the caller.
pub fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// A small hermite ramp used for staggered text entrances.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < 1e-6 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How the type on one page should be drawn this frame.
#[derive(Clone, Copy, Debug)]
pub struct TextVisual {
    pub alpha: f32,
    pub offset_y: f32,
    /// A very small scale around the block centre; enough to feel the page
    /// settle without re-flowing the type.
    pub scale: f32,
}

impl TextVisual {
    pub const VISIBLE: Self = Self {
        alpha: 1.0,
        offset_y: 0.0,
        scale: 1.0,
    };
}

/// A running page transition.
#[derive(Clone, Copy, Debug)]
pub struct PageTransition {
    pub from: usize,
    pub to: usize,
    pub elapsed: f32,
    pub duration: f32,
    pub direction: f32,
    active: bool,
}

impl Default for PageTransition {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            elapsed: 0.0,
            duration: 0.82,
            direction: 1.0,
            active: false,
        }
    }
}

impl PageTransition {
    pub fn start(&mut self, from: usize, to: usize, reduced_motion: bool) {
        if from == to {
            return;
        }
        self.from = from;
        self.to = to;
        self.elapsed = 0.0;
        self.duration = if reduced_motion { 0.22 } else { 0.82 };
        self.direction = if to > from { 1.0 } else { -1.0 };
        self.active = true;
    }

    pub fn update(&mut self, dt: f32) {
        if !self.active {
            return;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
        if self.elapsed >= self.duration {
            self.active = false;
            self.from = self.to;
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Linear progress in `0..=1`.
    pub fn t(&self) -> f32 {
        if self.active {
            (self.elapsed / self.duration.max(0.001)).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Eased progress used for the scene cross-blend.
    pub fn blend(&self) -> f32 {
        smootherstep(self.t())
    }

    /// The outgoing page leaves during the first half of the transition.
    pub fn outgoing(&self) -> TextVisual {
        if !self.active {
            return TextVisual::VISIBLE;
        }
        let t = self.t();
        let k = smoothstep(0.0, 0.46, t);
        TextVisual {
            alpha: 1.0 - k,
            offset_y: -26.0 * k * self.direction,
            scale: 1.0 - 0.012 * k,
        }
    }

    /// The incoming page arrives a little after the outgoing one leaves.
    pub fn incoming(&self) -> TextVisual {
        if !self.active {
            return TextVisual::VISIBLE;
        }
        let t = self.t();
        let k = smoothstep(0.32, 1.0, t);
        TextVisual {
            alpha: k,
            offset_y: 30.0 * (1.0 - k) * self.direction,
            scale: 0.988 + 0.012 * k,
        }
    }

    /// A transient darkening between the two pages.
    pub fn veil(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        (self.t() * std::f32::consts::PI).sin() * 0.24
    }
}

/// A tiny stagger helper for the elements of one page.
pub fn stagger(enter: f32, delay: f32) -> f32 {
    smoothstep(delay, delay + 0.52, enter)
}
