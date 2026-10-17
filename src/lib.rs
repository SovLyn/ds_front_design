//! Deep Space Atlas — a procedurally drawn cosmic field guide.
//!
//! The crate is split into an egui application layer (`app`, `ui`,
//! `transition`), a content layer (`content`), and a WebGPU renderer
//! (`render`, `scenes`). Only the renderer knows about pixels; only the
//! content layer knows about text.

pub mod app;
pub mod content;
pub mod render;
pub mod scenes;
pub mod transition;
pub mod ui;
