//! Small painting primitives shared by the page and the chrome.

use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontId, Galley, Mesh, Painter, Rect, Shape};

/// Lay out one run of text with a fixed measure, line height and tracking.
pub fn layout_text(
    ctx: &egui::Context,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    line_height: Option<f32>,
    letter_spacing: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width.max(1.0);
    job.halign = Align::LEFT;
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font,
            color,
            line_height,
            extra_letter_spacing: letter_spacing,
            ..Default::default()
        },
    );
    ctx.fonts_mut(|fonts| fonts.layout_job(job))
}

/// A two-stop gradient rectangle. egui interpolates vertex colours, so a pair
/// of triangles is enough for a smooth scrim.
pub fn gradient_rect(painter: &Painter, rect: Rect, start: Color32, end: Color32, vertical: bool) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let mut mesh = Mesh::default();
    let (a, b) = (start, end);
    if vertical {
        mesh.colored_vertex(rect.left_top(), a);
        mesh.colored_vertex(rect.right_top(), a);
        mesh.colored_vertex(rect.right_bottom(), b);
        mesh.colored_vertex(rect.left_bottom(), b);
    } else {
        mesh.colored_vertex(rect.left_top(), a);
        mesh.colored_vertex(rect.right_top(), b);
        mesh.colored_vertex(rect.right_bottom(), b);
        mesh.colored_vertex(rect.left_bottom(), a);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}
