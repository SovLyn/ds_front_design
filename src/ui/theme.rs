//! Shared spacing, type scale and colours.

use egui::{Color32, Rect};

pub const INK: Color32 = Color32::from_rgb(4, 6, 11);
pub const TEXT: Color32 = Color32::from_rgb(232, 236, 244);
pub const MUTED: Color32 = Color32::from_rgb(136, 146, 166);
pub const FAINT: Color32 = Color32::from_rgb(86, 94, 112);

pub fn hairline(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
}

pub fn white(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
}

pub fn fade(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

pub fn accent(rgb: [u8; 3], alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], alpha)
}

/// Everything the page layout needs to know about the viewport.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub w: f32,
    pub h: f32,
    pub mobile: bool,
    pub margin: f32,
    pub content_w: f32,
    pub title: f32,
    pub title_en: f32,
    pub kicker: f32,
    pub lead: f32,
    pub body: f32,
    pub body_lh: f32,
    pub stat_value: f32,
    pub stat_label: f32,
    pub stat_note: f32,
    pub topbar: f32,
    pub rail_x: f32,
    pub rail_top: f32,
    pub rail_bottom: f32,
}

impl Metrics {
    pub fn new(rect: Rect, mobile_hint: bool) -> Self {
        let w = rect.width().max(1.0);
        let h = rect.height().max(1.0);
        let mobile = mobile_hint || w < 760.0 || (w < 1000.0 && h > w * 1.12);

        if mobile {
            Self {
                w,
                h,
                mobile,
                margin: 24.0,
                content_w: (w - 48.0).max(240.0),
                title: (w * 0.118).clamp(36.0, 54.0),
                title_en: 10.5,
                kicker: 9.5,
                lead: 16.8,
                body: 13.8,
                body_lh: 23.6,
                stat_value: 19.5,
                stat_label: 8.6,
                stat_note: 7.6,
                topbar: 26.0,
                rail_x: w - 18.0,
                rail_top: h * 0.24,
                rail_bottom: h * 0.76,
            }
        } else {
            Self {
                w,
                h,
                mobile,
                margin: (w * 0.055).clamp(48.0, 96.0),
                content_w: (w * 0.335).clamp(360.0, 500.0),
                title: (w * 0.052).clamp(52.0, 82.0),
                title_en: 12.5,
                kicker: 10.5,
                lead: 19.2,
                body: 15.2,
                body_lh: 27.0,
                stat_value: 21.0,
                stat_label: 8.8,
                stat_note: 7.8,
                topbar: 38.0,
                rail_x: w - 34.0,
                rail_top: h * 0.28,
                rail_bottom: h * 0.72,
            }
        }
    }

    /// Space available for the text block, below the top bar.
    pub fn text_room(&self) -> f32 {
        if self.mobile {
            self.h - self.topbar - 58.0
        } else {
            self.h - self.topbar - 76.0
        }
    }

    /// Default top of the text block. Actual pages centre or bottom-align it.
    pub fn block_origin_y(&self, block_height: f32) -> f32 {
        if self.mobile {
            (self.h - block_height - 52.0).max(self.topbar + 18.0)
        } else {
            let centred = (self.h - block_height) * 0.5 + 8.0;
            centred.clamp(
                self.topbar + 22.0,
                (self.h - block_height - 30.0).max(self.topbar + 22.0),
            )
        }
    }
}
