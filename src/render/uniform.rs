//! CPU-side frame description and the exact GPU uniform layout.

use bytemuck::{Pod, Zeroable};

use crate::scenes::CameraPose;

/// High-level parameters for one rendered frame of the background.
#[derive(Clone, Copy, Debug)]
pub struct SceneFrame {
    pub time: f32,
    pub mode_a: u32,
    pub mode_b: u32,
    /// 0 = scene A, 1 = scene B.
    pub blend: f32,
    pub color_a: [f32; 3],
    pub color_b: [f32; 3],
    pub energy_a: f32,
    pub energy_b: f32,
    pub accent_a: [f32; 3],
    pub accent_b: [f32; 3],
    pub accent_energy_a: f32,
    pub accent_energy_b: f32,
    pub camera_a: CameraPose,
    pub camera_b: CameraPose,
    pub phase_a: f32,
    pub phase_b: f32,
    /// Pointer parallax in the range -1..1.
    pub pointer: [f32; 2],
    /// 1.0 for full motion, lower when reduced motion is requested.
    pub motion: f32,
    /// 0.0 (mobile) .. 1.0 (desktop) — drives particle density in the shader.
    pub quality: f32,
    pub reduced_motion: bool,
    /// Extra darkening during a transition.
    pub veil: f32,
    /// 0..1 entrance fade.
    pub boot: f32,
    /// Where the scene's visual centre sits in normalized screen coordinates.
    pub composition: [f32; 2],
    pub mobile: bool,
}

impl Default for SceneFrame {
    fn default() -> Self {
        Self {
            time: 0.0,
            mode_a: 0,
            mode_b: 0,
            blend: 0.0,
            color_a: [0.5, 0.6, 1.0],
            color_b: [0.5, 0.6, 1.0],
            energy_a: 0.5,
            energy_b: 0.5,
            accent_a: [1.0, 0.7, 0.4],
            accent_b: [1.0, 0.7, 0.4],
            accent_energy_a: 0.5,
            accent_energy_b: 0.5,
            camera_a: CameraPose::new(0.0, 0.0, 4.0, 0.5),
            camera_b: CameraPose::new(0.0, 0.0, 4.0, 0.5),
            phase_a: 0.0,
            phase_b: 0.0,
            pointer: [0.0, 0.0],
            motion: 1.0,
            quality: 1.0,
            reduced_motion: false,
            veil: 1.0,
            boot: 1.0,
            composition: [0.5, 0.5],
            mobile: false,
        }
    }
}

impl SceneFrame {
    /// Build the GPU uniform block. Sizes are in physical pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn uniform(
        &self,
        scene_size: [u32; 2],
        bloom_size: [u32; 2],
        output_size: [u32; 2],
        encode_srgb: bool,
    ) -> FrameUniform {
        let sw = scene_size[0].max(1) as f32;
        let sh = scene_size[1].max(1) as f32;
        let bw = bloom_size[0].max(1) as f32;
        let bh = bloom_size[1].max(1) as f32;
        let ow = output_size[0].max(1) as f32;
        let oh = output_size[1].max(1) as f32;

        let ca = self.camera_a;
        let cb = self.camera_b;

        FrameUniform {
            res: [sw, sh, sw / sh, self.time],
            modes: [
                self.mode_a as f32,
                self.mode_b as f32,
                self.blend,
                self.quality,
            ],
            col_a: [
                self.color_a[0],
                self.color_a[1],
                self.color_a[2],
                self.energy_a,
            ],
            col_b: [
                self.color_b[0],
                self.color_b[1],
                self.color_b[2],
                self.energy_b,
            ],
            cam_a: [ca.yaw, ca.pitch, ca.distance, ca.fov],
            cam_b: [cb.yaw, cb.pitch, cb.distance, cb.fov],
            accent_a: [
                self.accent_a[0],
                self.accent_a[1],
                self.accent_a[2],
                self.accent_energy_a,
            ],
            accent_b: [
                self.accent_b[0],
                self.accent_b[1],
                self.accent_b[2],
                self.accent_energy_b,
            ],
            pointer: [self.pointer[0], self.pointer[1], 0.0, self.motion],
            post: [0.85, 0.34, 0.030, 0.60],
            phase: [self.phase_a, self.phase_b, 19.0, self.veil],
            screen: [ow, oh, 1.0 / ow, 1.0 / oh],
            texel: [1.0 / sw, 1.0 / sh, 1.0 / bw, 1.0 / bh],
            layout: [
                self.composition[0],
                self.composition[1],
                if self.mobile { 0.88 } else { 1.0 },
                if self.mobile { 1.0 } else { 0.0 },
            ],
            flags: [
                if encode_srgb { 1.0 } else { 0.0 },
                if self.reduced_motion { 1.0 } else { 0.0 },
                self.boot,
                0.0,
            ],
        }
    }
}

/// Mirrors `Uniforms` in `shaders/atlas.wgsl` (all members are vec4 aligned).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FrameUniform {
    pub res: [f32; 4],
    pub modes: [f32; 4],
    pub col_a: [f32; 4],
    pub col_b: [f32; 4],
    pub cam_a: [f32; 4],
    pub cam_b: [f32; 4],
    pub accent_a: [f32; 4],
    pub accent_b: [f32; 4],
    pub pointer: [f32; 4],
    pub post: [f32; 4],
    pub phase: [f32; 4],
    pub screen: [f32; 4],
    pub texel: [f32; 4],
    pub layout: [f32; 4],
    pub flags: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<FrameUniform>() == 15 * 16);
