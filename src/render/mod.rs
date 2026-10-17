//! Render layer: WebGPU resources, shader entry points and frame uniforms.

mod gpu;
mod uniform;

pub use gpu::{BackgroundCallback, SceneRenderer};
pub use uniform::{FrameUniform, SceneFrame};
