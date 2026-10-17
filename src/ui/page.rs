//! Layout and painting of one chapter of the atlas.
//!
//! Text is laid out into galleys once per (page, measure, scale) and cached.
//! Painting then only positions those galleys, which keeps the per-frame cost
//! low even while two pages cross-fade.

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, Context, Galley, Painter, Pos2, Rect, Vec2};

use crate::content::{self, Page};
use crate::transition::{TextVisual, stagger};

use super::theme::{self, Metrics};
use super::{fonts, widgets};

const SCALES: [f32; 6] = [1.0, 0.94, 0.88, 0.82, 0.76, 0.70];

pub struct StatCell {
    value: Arc<Galley>,
    label: Arc<Galley>,
    note: Option<Arc<Galley>>,
}

pub struct PageBlock {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    kicker: Arc<Galley>,
    title: Arc<Galley>,
    title_en: Arc<Galley>,
    lead: Arc<Galley>,
    paragraphs: Vec<Arc<Galley>>,
    stats: Vec<StatCell>,
    stat_columns: usize,
    stat_cell_w: f32,
    stat_row_h: f32,
    cta: Option<Arc<Galley>>,
    paragraph_gap: f32,
    stats_gap: f32,
    cta_gap: f32,
}

impl PageBlock {
    fn build(ctx: &Context, m: &Metrics, page: &Page, scale: f32, compact: bool) -> Self {
        let width = m.content_w;
        let white = Color32::WHITE;

        let title_size = m.title * scale;
        let kicker_size = m.kicker * scale;
        let en_size = m.title_en * scale;
        let lead_size = m.lead * scale;
        let body_size = m.body * scale;
        let body_lh = m.body_lh * scale;
        let stat_value_size = m.stat_value * scale;
        let stat_label_size = m.stat_label * scale;
        let stat_note_size = m.stat_note * scale;

        let kicker = widgets::layout_text(
            ctx,
            page.kicker,
            fonts::body(kicker_size),
            white,
            width,
            Some(kicker_size * 1.25),
            kicker_size * 0.20,
        );
        let title = widgets::layout_text(
            ctx,
            page.title,
            fonts::display_light(title_size),
            white,
            width,
            Some(title_size * 1.04),
            title_size * 0.015,
        );
        let title_en = if compact {
            widgets::layout_text(ctx, "", fonts::body(en_size), white, width, None, 0.0)
        } else {
            widgets::layout_text(
                ctx,
                page.title_en,
                fonts::body(en_size),
                white,
                width,
                Some(en_size * 1.35),
                en_size * 0.17,
            )
        };
        let lead = widgets::layout_text(
            ctx,
            page.lead,
            fonts::display(lead_size),
            white,
            width * 0.94,
            Some(lead_size * 1.58),
            lead_size * 0.01,
        );

        let paragraphs = page
            .paragraphs
            .iter()
            .map(|text| {
                widgets::layout_text(
                    ctx,
                    text,
                    fonts::body(body_size),
                    white,
                    width,
                    Some(body_lh),
                    0.0,
                )
            })
            .collect::<Vec<_>>();

        // Four figures in a single row only works when every value is short.
        // Otherwise a 2x2 grid keeps the numbers at the same optical size.
        let long_values = page.stats.iter().any(|stat| stat.value.chars().count() > 8);
        let stat_columns = if m.mobile || long_values {
            2
        } else {
            page.stats.len().clamp(1, 4)
        };
        let column_gap = if m.mobile { 18.0 } else { 20.0 };
        let stat_cell_w = (width - column_gap * (stat_columns.saturating_sub(1) as f32))
            / stat_columns.max(1) as f32;
        let mut stats = Vec::with_capacity(page.stats.len());
        let mut stat_row_h: f32 = 0.0;
        for stat in page.stats {
            // Long figures shrink until they fit on one line; a data strip
            // with wrapped numbers reads as a bug, not as a design.
            let mut value_size = stat_value_size;
            let mut value = widgets::layout_text(
                ctx,
                stat.value,
                fonts::display(value_size),
                white,
                stat_cell_w,
                Some(value_size * 1.18),
                0.0,
            );
            while value.size().y > stat_value_size * 1.30 && value_size > 12.0 {
                value_size *= 0.90;
                value = widgets::layout_text(
                    ctx,
                    stat.value,
                    fonts::display(value_size),
                    white,
                    stat_cell_w,
                    Some(value_size * 1.18),
                    0.0,
                );
            }
            let label = widgets::layout_text(
                ctx,
                stat.label,
                fonts::body(stat_label_size),
                white,
                stat_cell_w,
                Some(stat_label_size * 1.35),
                stat_label_size * 0.06,
            );
            let note = if compact {
                None
            } else {
                Some(widgets::layout_text(
                    ctx,
                    stat.note,
                    fonts::body(stat_note_size),
                    white,
                    stat_cell_w,
                    Some(stat_note_size * 1.35),
                    stat_note_size * 0.12,
                ))
            };
            stat_row_h = stat_row_h.max(
                value.size().y
                    + 5.0
                    + label.size().y
                    + note.as_ref().map_or(0.0, |n| 3.0 + n.size().y),
            );
            stats.push(StatCell { value, label, note });
        }

        let cta = page.cta.map(|label| {
            widgets::layout_text(
                ctx,
                label,
                fonts::body(13.0 * scale),
                white,
                190.0,
                Some(18.0 * scale),
                13.0 * scale * 0.10,
            )
        });

        let paragraph_gap = 13.0 * scale.max(0.8);
        let stats_gap = 25.0 * scale.max(0.8);
        let cta_gap = 22.0 * scale.max(0.8);

        let mut height = kicker.size().y + 11.0 * scale;
        height += title.size().y + 5.0 * scale;
        if !compact {
            height += title_en.size().y + 19.0 * scale;
        }
        height += 1.0 + 17.0 * scale;
        height += lead.size().y + 18.0 * scale;
        height += paragraphs.iter().map(|p| p.size().y).sum::<f32>();
        if paragraphs.len() > 1 {
            height += paragraph_gap * (paragraphs.len() - 1) as f32;
        }
        height += stats_gap;
        let rows = stats.len().div_ceil(stat_columns.max(1));
        height += stat_row_h * rows as f32;
        if rows > 1 {
            height += 16.0 * (rows - 1) as f32;
        }
        if cta.is_some() {
            height += cta_gap + 42.0 * scale.max(0.8);
        }

        Self {
            width,
            height,
            scale,
            kicker,
            title,
            title_en,
            lead,
            paragraphs,
            stats,
            stat_columns,
            stat_cell_w,
            stat_row_h,
            cta,
            paragraph_gap,
            stats_gap,
            cta_gap,
        }
    }
}

/// Cached galleys for every page currently on screen.
#[derive(Default)]
pub struct UiState {
    blocks: HashMap<(usize, u32, u16, bool), Arc<PageBlock>>,
    cached_width: f32,
}

impl UiState {
    fn block(&mut self, ctx: &Context, m: &Metrics, page: &Page, index: usize) -> Arc<PageBlock> {
        if (self.cached_width - m.content_w).abs() > 1.0 {
            self.blocks.clear();
            self.cached_width = m.content_w;
        }

        let width_key = (m.content_w * 0.5) as u32;
        let room = m.text_room();

        for compact in [false, true] {
            for scale in SCALES {
                let key = (index, width_key, (scale * 100.0) as u16, compact);
                let block = self
                    .blocks
                    .entry(key)
                    .or_insert_with(|| Arc::new(PageBlock::build(ctx, m, page, scale, compact)))
                    .clone();
                if block.height <= room {
                    if self.blocks.len() > 96 {
                        // Keep the cache bounded across window resizes.
                        self.blocks.retain(|k, _| k.0 == index);
                    }
                    return block;
                }
            }
        }

        // Extremely short viewport: use the smallest compact block rather than
        // clipping the type. The page remains readable, just denser.
        let key = (index, width_key, 70, true);
        self.blocks
            .entry(key)
            .or_insert_with(|| Arc::new(PageBlock::build(ctx, m, page, 0.70, true)))
            .clone()
    }
}

pub struct PagePaint {
    pub block: Rect,
    pub cta: Option<Rect>,
}

fn paint_shape(
    painter: &Painter,
    x: f32,
    y: f32,
    galley: &Arc<Galley>,
    color: Color32,
    scale: f32,
    pivot: Pos2,
) {
    let mut shape =
        egui::Shape::galley_with_override_text_color(Pos2::new(x, y), galley.clone(), color);
    if (scale - 1.0).abs() > 0.0005 {
        let transform = egui::emath::TSTransform::new(pivot.to_vec2() * (1.0 - scale), scale);
        shape.transform(transform);
    }
    painter.add(shape);
}

/// Draw a page and return the rects the chrome may want to interact with.
pub fn draw(
    ctx: &Context,
    painter: &Painter,
    state: &mut UiState,
    m: &Metrics,
    index: usize,
    enter: f32,
    visual: TextVisual,
) -> PagePaint {
    let page = content::page(index);
    let block = state.block(ctx, m, page, index);
    let accent = page.accent;

    let origin_x = m.margin;
    let block_top = m.block_origin_y(block.height);
    let pivot = Pos2::new(origin_x + block.width * 0.5, block_top + block.height * 0.5);
    let scale = visual.scale.clamp(0.96, 1.04);
    let mut y = block_top + visual.offset_y;
    let base_alpha = visual.alpha.clamp(0.0, 1.0);

    let paint = |x: f32, y: f32, galley: &Arc<Galley>, color: Color32| {
        paint_shape(painter, x, y, galley, color, scale, pivot);
    };

    let element = |delay: f32| {
        let k = stagger(enter, delay);
        (base_alpha * k, (1.0 - k) * 13.0)
    };

    // Eyebrow.
    let (a, dy) = element(0.0);
    paint(
        origin_x,
        y + dy,
        &block.kicker,
        theme::accent(accent, (a * 235.0) as u8),
    );
    y += block.kicker.size().y + 11.0 * block.scale;

    // Title.
    let (a, dy) = element(0.05);
    paint(
        origin_x,
        y + dy,
        &block.title,
        theme::white((a * 255.0) as u8),
    );
    y += block.title.size().y + 5.0 * block.scale;

    // English subtitle.
    if block.title_en.size().y > 0.0 {
        let (a, dy) = element(0.11);
        paint(
            origin_x,
            y + dy,
            &block.title_en,
            theme::fade(theme::MUTED, a),
        );
        y += block.title_en.size().y + 19.0 * block.scale;
    }

    // Rule.
    let (a, dy) = element(0.17);
    let rule_y = y + dy;
    painter.line_segment(
        [
            Pos2::new(origin_x, rule_y),
            Pos2::new(origin_x + 52.0, rule_y),
        ],
        egui::Stroke::new(1.0, theme::hairline((a * 70.0) as u8)),
    );
    painter.circle_filled(
        Pos2::new(origin_x + 60.0, rule_y),
        1.6,
        theme::accent(accent, (a * 190.0) as u8),
    );
    y += 1.0 + 17.0 * block.scale;

    // Lead.
    let (a, dy) = element(0.22);
    paint(
        origin_x,
        y + dy,
        &block.lead,
        theme::white((a * 238.0) as u8),
    );
    y += block.lead.size().y + 18.0 * block.scale;

    // Paragraphs.
    for (i, paragraph) in block.paragraphs.iter().enumerate() {
        let (a, dy) = element(0.28 + i as f32 * 0.05);
        paint(origin_x, y + dy, paragraph, theme::white((a * 200.0) as u8));
        y += paragraph.size().y + block.paragraph_gap;
    }
    if !block.paragraphs.is_empty() {
        y -= block.paragraph_gap;
    }

    y += block.stats_gap;

    // Stats.
    let stats_top = y;
    painter.line_segment(
        [
            Pos2::new(origin_x, stats_top),
            Pos2::new(origin_x + block.width, stats_top),
        ],
        egui::Stroke::new(1.0, theme::hairline(22)),
    );
    let rows = block.stats.len().div_ceil(block.stat_columns.max(1));
    for (i, stat) in block.stats.iter().enumerate() {
        let row = i / block.stat_columns;
        let col = i % block.stat_columns;
        let x = origin_x + col as f32 * (block.stat_cell_w + if m.mobile { 18.0 } else { 20.0 });
        let sy = stats_top + 13.0 + row as f32 * (block.stat_row_h + 16.0);
        let (a, dy) = element(0.40 + i as f32 * 0.03);
        paint(x, sy + dy, &stat.value, theme::white((a * 245.0) as u8));
        let mut ny = sy + stat.value.size().y + 5.0;
        paint(x, ny + dy, &stat.label, theme::fade(theme::MUTED, a));
        ny += stat.label.size().y;
        if let Some(note) = &stat.note {
            paint(x, ny + 3.0 + dy, note, theme::fade(theme::FAINT, a));
        }
    }
    let stats_height = block.stat_row_h * rows as f32
        + if rows > 1 {
            16.0 * (rows - 1) as f32
        } else {
            0.0
        };
    y = stats_top + 13.0 + stats_height;

    // Cover call to action.
    let mut cta_rect = None;
    if let Some(cta) = &block.cta {
        y += block.cta_gap;
        let raw_rect = Rect::from_min_size(
            Pos2::new(origin_x, y),
            Vec2::new(198.0 * block.scale.max(0.8), 42.0 * block.scale.max(0.8)),
        );
        let hovered = ctx
            .input(|i| i.pointer.hover_pos())
            .map(|p| raw_rect.contains(p))
            .unwrap_or(false);
        let (a, dy) = element(0.50);
        let raw_rect = raw_rect.translate(Vec2::new(0.0, dy));
        let transform = egui::emath::TSTransform::new(pivot.to_vec2() * (1.0 - scale), scale);
        let rect = transform.mul_rect(raw_rect);
        let fill = if hovered {
            theme::accent(page.accent, (a * 30.0) as u8)
        } else {
            theme::white((a * 10.0) as u8)
        };
        painter.rect_filled(rect, egui::CornerRadius::same(2), fill);
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(2),
            egui::Stroke::new(
                1.0,
                theme::accent(page.accent, (a * if hovered { 210.0 } else { 115.0 }) as u8),
            ),
            egui::StrokeKind::Inside,
        );
        paint(
            raw_rect.left() + 18.0,
            raw_rect.center().y - cta.size().y * 0.5,
            cta,
            theme::white((a * 235.0) as u8),
        );
        painter.text(
            Pos2::new(rect.right() - 18.0 * scale, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "→",
            fonts::body(15.0 * block.scale.max(0.8) * scale),
            theme::accent(page.accent, (a * 220.0) as u8),
        );
        cta_rect = Some(rect);
        y = rect.bottom();
    }

    PagePaint {
        block: Rect::from_min_size(
            Pos2::new(origin_x, m.block_origin_y(block.height) + visual.offset_y),
            Vec2::new(
                block.width,
                y - (m.block_origin_y(block.height) + visual.offset_y),
            ),
        ),
        cta: cta_rect,
    }
}
