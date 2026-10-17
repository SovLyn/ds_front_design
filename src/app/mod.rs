//! The eframe application: state, input, timing and the paint callback.
//!
//! The app owns the current page, the running transition and the adaptive
//! render scale. It hands a `SceneFrame` to the WebGPU renderer once per frame
//! and a `FrameInput` to the egui interface.

mod input;
mod motion;
pub mod platform;

use std::time::Duration;

use eframe::{egui, egui_wgpu};
use egui::Vec2;

use crate::content::{self, PAGE_COUNT};
use crate::render::{BackgroundCallback, SceneFrame, SceneRenderer};
use crate::scenes;
use crate::transition::{self, PageTransition};
use crate::ui::{self, FrameInput, UiState};

use self::input::{Navigation, Request};
use self::motion::MotionPreference;

pub struct AtlasApp {
    current: usize,
    transition: PageTransition,
    boot: f32,
    page_started: f32,
    pointer: Vec2,
    pointer_target: Vec2,
    motion: MotionPreference,
    navigation: Navigation,
    ui: UiState,
    mobile: bool,
    render_scale: f32,
    quality: f32,
    frame_ms: f32,
    slow_frames: u32,
    fast_frames: u32,
    ready: bool,
    clock: f32,
    logged_frame: bool,
    last_label: usize,
    last_motion: bool,
}

impl AtlasApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::fonts::install(&cc.egui_ctx);

        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = egui::Color32::TRANSPARENT;
        style.visuals.window_fill = egui::Color32::TRANSPARENT;
        style.visuals.override_text_color = None;
        style.animation_time = 0.12;
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);

        if let Some(render_state) = &cc.wgpu_render_state {
            let renderer = SceneRenderer::new(&render_state.device, render_state.target_format);
            render_state
                .renderer
                .write()
                .callback_resources
                .insert(renderer);
        } else {
            log::error!("no wgpu render state: the background canvas cannot start");
        }

        let screen = cc.egui_ctx.content_rect();
        let mobile = platform::is_mobile_layout(screen.width(), screen.height());
        let cores = platform::hardware_concurrency();
        let render_scale = if mobile {
            0.62
        } else if cores <= 4 {
            0.72
        } else {
            0.82
        };

        let now = cc.egui_ctx.input(|i| i.time) as f32;
        Self {
            current: 0,
            transition: PageTransition::default(),
            boot: 0.0,
            page_started: 0.0,
            pointer: Vec2::ZERO,
            pointer_target: Vec2::ZERO,
            motion: MotionPreference::new(now),
            navigation: Navigation::default(),
            ui: UiState::default(),
            mobile,
            render_scale,
            quality: if mobile { 0.55 } else { 1.0 },
            frame_ms: 16.7,
            slow_frames: 0,
            fast_frames: 0,
            ready: false,
            clock: 0.0,
            logged_frame: false,
            last_label: usize::MAX,
            last_motion: false,
        }
    }

    fn navigate(&mut self, target: usize) {
        let target = target.min(PAGE_COUNT - 1);
        if target == self.current || self.transition.is_active() {
            return;
        }
        self.transition
            .start(self.current, target, self.motion.reduced);
        self.current = target;
        self.page_started = 0.0;
    }

    fn handle_request(&mut self, request: Request) {
        match request {
            Request::Next => self.navigate(self.current + 1),
            Request::Prev => self.navigate(self.current.saturating_sub(1)),
            Request::Index(index) => self.navigate(index),
            Request::ToggleMotion => self.motion.toggle(),
        }
    }

    fn scene_frame(&self, time: f32) -> SceneFrame {
        let target = content::page(self.current);
        let target_spec = scenes::scene_for(target.scene);
        let reduced = self.motion.reduced;
        let boot = transition::smoothstep(0.0, 1.0, self.boot);
        let composition = if self.mobile {
            [0.50, 0.30]
        } else {
            [0.65, 0.50]
        };

        if self.transition.is_active() {
            let from = content::page(self.transition.from);
            let to = content::page(self.transition.to);
            let a = scenes::scene_for(from.scene);
            let b = scenes::scene_for(to.scene);
            SceneFrame {
                time,
                mode_a: a.id.mode(),
                mode_b: b.id.mode(),
                blend: self.transition.blend(),
                color_a: a.color,
                color_b: b.color,
                energy_a: a.energy,
                energy_b: b.energy,
                accent_a: a.accent,
                accent_b: b.accent,
                accent_energy_a: a.accent_energy,
                accent_energy_b: b.accent_energy,
                camera_a: a.camera.drift(time, a.drift),
                camera_b: b.camera.drift(time, b.drift),
                phase_a: a.phase,
                phase_b: b.phase,
                pointer: [self.pointer.x * 0.6, self.pointer.y * 0.6],
                motion: if reduced { 0.0 } else { 1.0 },
                quality: self.quality,
                reduced_motion: reduced,
                veil: 0.24,
                boot,
                composition,
                mobile: self.mobile,
            }
        } else {
            let spec = target_spec;
            SceneFrame {
                time,
                mode_a: spec.id.mode(),
                mode_b: spec.id.mode(),
                blend: 0.0,
                color_a: spec.color,
                color_b: spec.color,
                energy_a: spec.energy,
                energy_b: spec.energy,
                accent_a: spec.accent,
                accent_b: spec.accent,
                accent_energy_a: spec.accent_energy,
                accent_energy_b: spec.accent_energy,
                camera_a: spec.camera.drift(time, spec.drift),
                camera_b: spec.camera.drift(time, spec.drift),
                phase_a: spec.phase,
                phase_b: spec.phase,
                pointer: [self.pointer.x * 0.6, self.pointer.y * 0.6],
                motion: if reduced { 0.0 } else { 1.0 },
                quality: self.quality,
                reduced_motion: reduced,
                veil: 0.0,
                boot,
                composition,
                mobile: self.mobile,
            }
        }
    }

    fn adapt(&mut self, dt: f32) {
        if (dt - 1.0 / 60.0).abs() < 0.004 || self.clock < 1.5 {
            return;
        }
        self.frame_ms = self.frame_ms * 0.92 + dt * 1000.0 * 0.08;

        if dt > 0.024 {
            self.slow_frames = self.slow_frames.saturating_add(1);
            self.fast_frames = 0;
        } else if dt < 0.014 {
            self.fast_frames = self.fast_frames.saturating_add(1);
            self.slow_frames = 0;
        } else {
            self.slow_frames = 0;
            self.fast_frames = 0;
        }

        if self.slow_frames > 40 && self.render_scale > 0.55 {
            self.render_scale = (self.render_scale - 0.06).max(0.55);
            self.slow_frames = 0;
            log::debug!("lowering render scale to {:.2}", self.render_scale);
        } else if self.fast_frames > 110 && self.render_scale < 0.90 {
            self.render_scale = (self.render_scale + 0.03).min(0.90);
            self.fast_frames = 0;
            log::debug!("raising render scale to {:.2}", self.render_scale);
        }
        self.quality = if self.mobile {
            0.55
        } else if self.render_scale < 0.66 {
            0.70
        } else {
            1.0
        };
    }
}

impl eframe::App for AtlasApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 0.1);
        self.clock += dt;
        let now = self.clock;
        let elapsed = now;

        if !self.logged_frame {
            self.logged_frame = true;
            log::debug!("atlas first frame: dt={dt:.4}, clock={now:.3}");
            log::debug!(
                "initial layout: viewport={:?} content={:?} ppp={} mobile={}",
                ctx.viewport_rect(),
                ctx.content_rect(),
                ctx.pixels_per_point(),
                self.mobile
            );
        }

        self.boot = (elapsed / 1.25).clamp(0.0, 1.0);
        self.page_started += dt;
        self.transition.update(dt);
        self.motion.poll(now);

        let viewport = ctx.viewport_rect();
        self.mobile = platform::is_mobile_layout(viewport.width(), viewport.height());

        let hover = ctx.input(|i| i.pointer.hover_pos());
        if let Some(pos) = hover {
            let nx = (pos.x / viewport.width().max(1.0)) * 2.0 - 1.0;
            let ny = (pos.y / viewport.height().max(1.0)) * 2.0 - 1.0;
            self.pointer_target = Vec2::new(nx.clamp(-1.0, 1.0), ny.clamp(-1.0, 1.0));
        } else {
            self.pointer_target = Vec2::ZERO;
        }
        let follow = (dt * 5.0).clamp(0.0, 1.0);
        self.pointer += (self.pointer_target - self.pointer) * follow;

        let request = self
            .navigation
            .update(&ctx, now, dt, self.transition.is_active());
        if let Some(request) = request {
            self.handle_request(request);
        }

        let time = now;
        let frame = self.scene_frame(time);
        let callback = egui_wgpu::Callback::new_paint_callback(
            viewport,
            BackgroundCallback {
                frame,
                render_scale: self.render_scale,
            },
        );
        ui.painter().add(callback);

        let input = FrameInput {
            current: self.current,
            transition: self.transition,
            pointer: self.pointer,
            reduced_motion: self.motion.reduced,
            boot: self.boot,
            now,
            page_started: self.page_started,
            mobile: self.mobile,
        };
        if self.last_label != self.current {
            let page = content::page(self.current);
            platform::set_page_label(page.number, page.title);
            self.last_label = self.current;
        }
        if self.last_motion != self.motion.reduced {
            platform::set_motion(self.motion.reduced);
            self.last_motion = self.motion.reduced;
        }

        let outcome = ui::draw(ui, &input, &mut self.ui);
        if let Some(action) = outcome.action {
            match action {
                ui::Action::GoTo(index) => self.navigate(index),
                ui::Action::ToggleMotion => self.motion.toggle(),
            }
        }

        self.adapt(dt);

        if !self.ready && elapsed > 0.22 {
            platform::mark_ready();
            self.ready = true;
        }

        ctx.request_repaint_after(Duration::from_millis(16));
        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.008, 0.012, 0.024, 1.0]
    }
}
