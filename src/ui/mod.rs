//! The interface layer: typography, scrims, the chapter rail and the small
//! amount of chrome that sits above the WebGPU scene.
//!
//! Everything here is painted with egui shapes. The background is a single
//! paint callback owned by `render`; the UI never talks to wgpu.

pub mod fonts;
pub mod page;
pub mod theme;
mod widgets;

use egui::{
    Color32, Context, CornerRadius, CursorIcon, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

use crate::content::{self, PAGE_COUNT};
use crate::transition::{self, PageTransition, TextVisual};

pub use page::UiState;

use self::theme::Metrics;

/// One frame of state handed to the UI by the app.
pub struct FrameInput {
    pub current: usize,
    pub transition: PageTransition,
    pub pointer: Vec2,
    pub reduced_motion: bool,
    pub boot: f32,
    pub now: f32,
    /// Seconds since the current page became the target.
    pub page_started: f32,
    pub mobile: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    GoTo(usize),
    ToggleMotion,
}

pub struct Outcome {
    pub action: Option<Action>,
}

impl Outcome {
    fn none() -> Self {
        Self { action: None }
    }
    fn set(&mut self, action: Action) {
        if self.action.is_none() {
            self.action = Some(action);
        }
    }
}

pub fn draw(ui: &mut egui::Ui, input: &FrameInput, state: &mut UiState) -> Outcome {
    let ctx = ui.ctx().clone();
    let mut outcome = Outcome::none();

    {
        let screen = ui.max_rect();
        let m = Metrics::new(screen, input.mobile);
        let painter = ui.painter().clone();
        let boot = transition::smoothstep(0.0, 0.92, input.boot);
        let target = input.current.min(PAGE_COUNT - 1);
        let accent = content::page(target).accent;

        paint_scrim(&painter, &m, boot, accent);

        if input.transition.is_active() {
            let t = input.transition.t();
            let out = input.transition.outgoing();
            let inc = input.transition.incoming();
            let out_page = input.transition.from;
            let in_page = input.transition.to;
            page::draw(
                &ctx,
                &painter,
                state,
                &m,
                out_page,
                1.0,
                TextVisual {
                    alpha: out.alpha * boot,
                    offset_y: out.offset_y,
                    scale: out.scale,
                },
            );
            page::draw(
                &ctx,
                &painter,
                state,
                &m,
                in_page,
                transition::smoothstep(0.28, 1.0, t),
                TextVisual {
                    alpha: inc.alpha * boot,
                    offset_y: inc.offset_y,
                    scale: inc.scale,
                },
            );
        } else {
            let enter = transition::smoothstep(0.0, 0.88, input.page_started);
            let paint = page::draw(
                &ctx,
                &painter,
                state,
                &m,
                target,
                enter,
                TextVisual {
                    alpha: boot,
                    offset_y: 0.0,
                    scale: 1.0,
                },
            );
            if let Some(rect) = paint.cta {
                let response = ui.interact(rect, Id::unique(("cta", target)), Sense::click());
                if response.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if response.clicked() {
                    outcome.set(Action::GoTo((target + 1).min(PAGE_COUNT - 1)));
                }
            }
        }

        // Chrome fades with the boot sequence and dips slightly mid-transition.
        let transition_dip = if input.transition.is_active() {
            1.0 - 0.28 * (input.transition.t() * std::f32::consts::PI).sin()
        } else {
            1.0
        };
        let chrome = boot * transition_dip;

        outcome.merge(paint_top_bar(&ctx, ui, &m, input, chrome, target));
        outcome.merge(paint_rail(&ctx, ui, &m, input, chrome, target));
        paint_footer(&ctx, &painter, &m, input, chrome, target);
        paint_hint(&ctx, &painter, &m, input, chrome, target);
    }

    outcome
}

impl Outcome {
    fn merge(&mut self, other: Outcome) {
        if let Some(action) = other.action {
            self.set(action);
        }
    }
}

fn paint_scrim(painter: &egui::Painter, m: &Metrics, boot: f32, accent: [u8; 3]) {
    let dark = Color32::from_rgba_unmultiplied(3, 5, 10, (238.0 * boot) as u8);
    let clear = Color32::from_rgba_unmultiplied(3, 5, 10, 0);
    if m.mobile {
        // A deeper reader's scrim: the scene stays visible above the text, but
        // the title and body always have a quiet field to sit on.
        let fade = Rect::from_min_max(Pos2::new(0.0, m.h * 0.16), Pos2::new(m.w, m.h * 0.38));
        widgets::gradient_rect(painter, fade, clear, dark, true);
        let panel = Rect::from_min_max(Pos2::new(0.0, m.h * 0.38), Pos2::new(m.w, m.h));
        painter.rect_filled(panel, 0, dark);
        painter.line_segment(
            [Pos2::new(0.0, m.h * 0.38), Pos2::new(m.w, m.h * 0.38)],
            egui::Stroke::new(1.0, theme::hairline((boot * 18.0) as u8)),
        );
    } else {
        let rect = Rect::from_min_max(
            Pos2::new(0.0, 0.0),
            Pos2::new((m.margin + m.content_w + 150.0).min(m.w), m.h),
        );
        widgets::gradient_rect(painter, rect, dark, clear, false);
    }

    // A quiet wash behind the top bar keeps the wordmark readable.
    let top = Rect::from_min_size(Pos2::ZERO, Vec2::new(m.w, m.topbar + 52.0));
    widgets::gradient_rect(
        painter,
        top,
        Color32::from_rgba_unmultiplied(
            3,
            5,
            10,
            ((if m.mobile { 205.0 } else { 150.0 }) * boot) as u8,
        ),
        clear,
        true,
    );

    // A barely-there accent breath in the bottom-left corner.
    let glow = Rect::from_min_size(
        Pos2::new(0.0, m.h - 210.0),
        Vec2::new(m.margin + 240.0, 210.0),
    );
    widgets::gradient_rect(
        painter,
        glow,
        Color32::from_rgba_unmultiplied(accent[0], accent[1], accent[2], (10.0 * boot) as u8),
        Color32::TRANSPARENT,
        true,
    );
}

fn paint_top_bar(
    ctx: &Context,
    ui: &mut egui::Ui,
    m: &Metrics,
    input: &FrameInput,
    alpha: f32,
    target: usize,
) -> Outcome {
    let mut outcome = Outcome::none();
    let painter = ui.painter();
    let x = m.margin;
    let y = m.topbar;

    // Wordmark, clickable back to the cover.
    painter.circle_filled(
        Pos2::new(x + 2.0, y + 1.0),
        2.4,
        theme::accent(content::page(0).accent, (alpha * 220.0) as u8),
    );
    let word = widgets::layout_text(
        ctx,
        "DEEP SPACE ATLAS",
        fonts::body(10.2),
        Color32::WHITE,
        230.0,
        Some(13.0),
        3.1,
    );
    painter.galley_with_override_text_color(
        Pos2::new(x + 12.0, y - 6.0),
        word,
        theme::white((alpha * 200.0) as u8),
    );
    let word_rect = Rect::from_min_size(Pos2::new(x, y - 12.0), Vec2::new(170.0, 24.0));
    let word_resp = ui.interact(word_rect, Id::unique("wordmark"), Sense::click());
    if word_resp.hovered() {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
    if word_resp.clicked() {
        outcome.set(Action::GoTo(0));
    }

    if !m.mobile {
        let subtitle = widgets::layout_text(
            ctx,
            "深空图鉴 · A PROCEDURAL ATLAS",
            fonts::body(8.4),
            Color32::WHITE,
            250.0,
            Some(11.0),
            1.8,
        );
        painter.galley_with_override_text_color(
            Pos2::new(x + 12.0, y + 11.0),
            subtitle,
            theme::fade(theme::FAINT, alpha),
        );
    }

    let counter = if m.mobile {
        format!("{:02}/07", target)
    } else {
        format!("{:02} — 07", target)
    };
    let counter_galley = widgets::layout_text(
        ctx,
        &counter,
        fonts::body(9.6),
        Color32::WHITE,
        120.0,
        Some(12.0),
        1.8,
    );
    let counter_w = counter_galley.size().x;

    // Mobile: a compact two-character toggle at the far right, counter beside it.
    // Desktop: the pill sits to the left of the counter.
    let (pill, counter_left) = if m.mobile {
        let pill = Rect::from_min_size(
            Pos2::new(m.w - m.margin - 30.0, y - 13.0),
            Vec2::new(30.0, 26.0),
        );
        (pill, m.w - m.margin - 30.0 - 12.0 - counter_w)
    } else {
        let pill = Rect::from_min_size(
            Pos2::new(m.w - m.margin - counter_w - 26.0 - 72.0, y - 13.0),
            Vec2::new(72.0, 26.0),
        );
        (pill, m.w - m.margin - counter_w)
    };

    painter.galley_with_override_text_color(
        Pos2::new(counter_left, y - 6.0),
        counter_galley,
        theme::fade(theme::MUTED, alpha * 0.9),
    );

    let label = if input.reduced_motion {
        if m.mobile { "静" } else { "静止" }
    } else if m.mobile {
        "动"
    } else {
        "动态"
    };
    let resp = ui.interact(pill, Id::unique("motion-toggle"), Sense::click());
    let hovered = resp.hovered();
    let clicked = resp.clicked();
    if hovered {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
    if clicked {
        outcome.set(Action::ToggleMotion);
    }
    let _ = resp.on_hover_text(if m.mobile {
        "减少动态 / Reduce motion"
    } else {
        "减少动态  ·  M"
    });
    painter.rect_filled(
        pill,
        CornerRadius::same(13),
        theme::white((alpha * if hovered { 16.0 } else { 8.0 }) as u8),
    );
    painter.rect_stroke(
        pill,
        CornerRadius::same(13),
        Stroke::new(1.0, theme::hairline((alpha * 38.0) as u8)),
        StrokeKind::Inside,
    );
    let active = if input.reduced_motion {
        theme::MUTED
    } else {
        theme::accent(content::page(target).accent, 255)
    };
    painter.circle_filled(
        Pos2::new(
            pill.left() + if m.mobile { 10.0 } else { 15.0 },
            pill.center().y,
        ),
        2.2,
        theme::fade(active, alpha),
    );
    let pill_text = widgets::layout_text(
        ctx,
        label,
        fonts::body(if m.mobile { 10.0 } else { 9.4 }),
        Color32::WHITE,
        50.0,
        Some(12.0),
        0.6,
    );
    let tx = pill.left() + if m.mobile { 17.0 } else { 24.0 };
    painter.galley_with_override_text_color(
        Pos2::new(tx, pill.center().y - pill_text.size().y * 0.5),
        pill_text,
        theme::white((alpha * 200.0) as u8),
    );

    outcome
}

fn paint_rail(
    ctx: &Context,
    ui: &mut egui::Ui,
    m: &Metrics,
    input: &FrameInput,
    alpha: f32,
    target: usize,
) -> Outcome {
    let mut outcome = Outcome::none();
    let painter = ui.painter();
    let current = if input.transition.is_active() && input.transition.t() < 0.5 {
        input.transition.from
    } else {
        target
    };

    if m.mobile {
        // A horizontal chapter rail under the top bar.
        let y = m.topbar + 34.0;
        let x0 = m.margin;
        let x1 = m.w - m.margin;
        painter.line_segment(
            [Pos2::new(x0, y), Pos2::new(x1, y)],
            Stroke::new(1.0, theme::hairline((alpha * 26.0) as u8)),
        );
        let step = (x1 - x0) / (PAGE_COUNT - 1) as f32;
        for i in 0..PAGE_COUNT {
            let x = x0 + step * i as f32;
            let hit = Rect::from_center_size(Pos2::new(x, y), Vec2::new(28.0, 24.0));
            let resp = ui.interact(hit, Id::unique(("rail-m", i)), Sense::click());
            if resp.clicked() {
                outcome.set(Action::GoTo(i));
            }
            let active = i == current;
            let hovered = resp.hovered();
            let col = if active {
                theme::accent(content::page(i).accent, (alpha * 240.0) as u8)
            } else {
                theme::white((alpha * if hovered { 130.0 } else { 46.0 }) as u8)
            };
            painter.circle_filled(Pos2::new(x, y), if active { 3.4 } else { 1.8 }, col);
            if active {
                painter.line_segment(
                    [Pos2::new(x - 7.0, y), Pos2::new(x + 7.0, y)],
                    Stroke::new(1.0, col),
                );
            }
        }
        return outcome;
    }

    let x = m.rail_x;
    let top = m.rail_top;
    let bottom = m.rail_bottom;
    painter.line_segment(
        [Pos2::new(x, top), Pos2::new(x, bottom)],
        Stroke::new(1.0, theme::hairline((alpha * 26.0) as u8)),
    );
    let step = (bottom - top) / (PAGE_COUNT - 1) as f32;
    for i in 0..PAGE_COUNT {
        let y = top + step * i as f32;
        let hit = Rect::from_center_size(Pos2::new(x - 12.0, y), Vec2::new(78.0, 26.0));
        let resp = ui.interact(hit, Id::unique(("rail", i)), Sense::click());
        if resp.hovered() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        if resp.clicked() {
            outcome.set(Action::GoTo(i));
        }
        let active = i == current;
        let hovered = resp.hovered();
        let col = if active {
            theme::accent(content::page(i).accent, (alpha * 235.0) as u8)
        } else {
            theme::white((alpha * if hovered { 150.0 } else { 48.0 }) as u8)
        };
        let len = if active { 20.0 } else { 10.0 };
        painter.line_segment(
            [Pos2::new(x - len, y), Pos2::new(x, y)],
            Stroke::new(1.0, col),
        );
        if active {
            painter.circle_filled(Pos2::new(x + 5.0, y), 2.1, col);
        }
        let label = widgets::layout_text(
            ctx,
            content::page(i).number,
            fonts::body(8.4),
            Color32::WHITE,
            24.0,
            Some(10.0),
            1.1,
        );
        let lx = x - 28.0 - label.size().x;
        painter.galley_with_override_text_color(
            Pos2::new(lx, y - label.size().y * 0.5),
            label,
            theme::white((alpha * if active { 180.0 } else { 60.0 }) as u8),
        );
    }

    outcome
}

fn paint_footer(
    ctx: &Context,
    painter: &egui::Painter,
    m: &Metrics,
    input: &FrameInput,
    alpha: f32,
    target: usize,
) {
    if m.mobile {
        return;
    }
    let page = content::page(target);
    let left = widgets::layout_text(
        ctx,
        &format!("{} / 07", page.number),
        fonts::body(8.4),
        Color32::WHITE,
        80.0,
        Some(10.0),
        1.4,
    );
    painter.galley_with_override_text_color(
        Pos2::new(m.margin, m.h - 30.0),
        left,
        theme::white((alpha * 120.0) as u8),
    );
    let right = widgets::layout_text(
        ctx,
        "PROCEDURAL · REAL-TIME",
        fonts::body(7.8),
        Color32::WHITE,
        220.0,
        Some(10.0),
        1.5,
    );
    painter.galley_with_override_text_color(
        Pos2::new(m.w - m.margin - right.size().x, m.h - 30.0),
        right,
        theme::fade(theme::FAINT, alpha * 0.9),
    );
    let _ = (input, page);
}

fn paint_hint(
    ctx: &Context,
    painter: &egui::Painter,
    m: &Metrics,
    input: &FrameInput,
    alpha: f32,
    target: usize,
) {
    if m.mobile || input.transition.is_active() {
        return;
    }
    let page = content::page(target);
    if page.cta.is_some() {
        return;
    }
    let pulse = if input.reduced_motion {
        0.55
    } else {
        0.45 + 0.16 * (input.now * 2.1).sin()
    };
    let hint = if target == PAGE_COUNT - 1 {
        "SCROLL UP  /  返回"
    } else {
        "SCROLL  /  滚动"
    };
    let text = widgets::layout_text(
        ctx,
        hint,
        fonts::body(8.2),
        Color32::WHITE,
        170.0,
        Some(10.0),
        2.2,
    );
    let x = (m.w - text.size().x) * 0.5;
    let y = m.h - 34.0;
    painter.galley_with_override_text_color(
        Pos2::new(x, y),
        text,
        theme::white((alpha * 95.0) as u8),
    );
    let line_y = y + 18.0;
    painter.line_segment(
        [
            Pos2::new(m.w * 0.5, line_y),
            Pos2::new(m.w * 0.5, line_y + 18.0),
        ],
        Stroke::new(1.0, theme::white((alpha * 30.0) as u8)),
    );
    let dot_y = line_y + 4.0 + 9.0 * (1.0 - pulse);
    painter.circle_filled(
        Pos2::new(m.w * 0.5, dot_y),
        1.4,
        theme::white((alpha * 120.0 * pulse) as u8),
    );
}
