//! Wheel, keyboard and touch navigation.
//!
//! The wheel is deliberately conservative: a large accumulated threshold plus
//! a cooldown means one gesture can never skip three chapters. Touch uses a
//! vertical displacement (and a small velocity allowance) rather than a live
//! scroll offset, so the page always settles on an integer index.

use egui::{Event, Key, TouchId, TouchPhase};

use crate::content::PAGE_COUNT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Next,
    Prev,
    Index(usize),
    ToggleMotion,
}

pub struct Navigation {
    wheel: f32,
    cooldown: f32,
    last_wheel: f32,
    touch: Option<TouchId>,
    touch_start_y: f32,
    touch_start_t: f32,
    touch_last_y: f32,
    touch_last_t: f32,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            wheel: 0.0,
            cooldown: 0.0,
            last_wheel: -10.0,
            touch: None,
            touch_start_y: 0.0,
            touch_start_t: 0.0,
            touch_last_y: 0.0,
            touch_last_t: 0.0,
        }
    }
}

impl Navigation {
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        now: f32,
        dt: f32,
        blocked: bool,
    ) -> Option<Request> {
        self.cooldown = (self.cooldown - dt).max(0.0);

        if ctx.input(|i| i.key_pressed(Key::M)) {
            return Some(Request::ToggleMotion);
        }

        if blocked {
            self.wheel = 0.0;
            self.touch = None;
            return None;
        }

        let keys = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowDown)
                    || i.key_pressed(Key::PageDown)
                    || i.key_pressed(Key::ArrowRight)
                    || i.key_pressed(Key::Space),
                i.key_pressed(Key::ArrowUp)
                    || i.key_pressed(Key::PageUp)
                    || i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
            )
        });
        if keys.2 {
            return Some(Request::Index(0));
        }
        if keys.3 {
            return Some(Request::Index(PAGE_COUNT - 1));
        }
        if keys.0 {
            return Some(Request::Next);
        }
        if keys.1 {
            return Some(Request::Prev);
        }

        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            if now - self.last_wheel > 0.18 {
                self.wheel = 0.0;
            }
            self.last_wheel = now;
            self.wheel += scroll;
            if self.cooldown <= 0.0 && self.wheel.abs() > 42.0 {
                let next = self.wheel < 0.0;
                self.wheel = 0.0;
                self.cooldown = 0.62;
                return Some(if next { Request::Next } else { Request::Prev });
            }
        }

        let mut request = None;
        ctx.input(|i| {
            for event in &i.events {
                let Event::Touch { phase, pos, id, .. } = event else {
                    continue;
                };
                match phase {
                    TouchPhase::Start => {
                        self.touch = Some(*id);
                        self.touch_start_y = pos.y;
                        self.touch_start_t = now;
                        self.touch_last_y = pos.y;
                        self.touch_last_t = now;
                    }
                    TouchPhase::Move => {
                        if self.touch == Some(*id) {
                            self.touch_last_y = pos.y;
                            self.touch_last_t = now;
                        }
                    }
                    TouchPhase::End => {
                        if self.touch == Some(*id) {
                            let dy = pos.y - self.touch_start_y;
                            let duration = (now - self.touch_start_t).max(0.001);
                            let velocity = (self.touch_last_y - self.touch_start_y) / duration;
                            let enough =
                                dy.abs() > 46.0 || (dy.abs() > 18.0 && velocity.abs() > 520.0);
                            if enough {
                                request = Some(if dy < 0.0 {
                                    Request::Next
                                } else {
                                    Request::Prev
                                });
                                self.cooldown = 0.5;
                            }
                            self.touch = None;
                        }
                    }
                    TouchPhase::Cancel => {
                        if self.touch == Some(*id) {
                            self.touch = None;
                        }
                    }
                }
            }
        });
        request
    }
}
