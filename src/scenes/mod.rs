//! Scene descriptions: each page of the atlas maps to one procedural scene.

use std::f32::consts::PI;

/// The eight scenes drawn by the WebGPU background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneId {
    Prologue,
    Star,
    System,
    Nebula,
    Galaxy,
    Quasar,
    BlackHole,
    Epilogue,
}

impl SceneId {
    /// Index used by the shader dispatcher.
    pub const fn mode(self) -> u32 {
        match self {
            Self::Prologue => 0,
            Self::Star => 1,
            Self::System => 2,
            Self::Nebula => 3,
            Self::Galaxy => 4,
            Self::Quasar => 5,
            Self::BlackHole => 6,
            Self::Epilogue => 7,
        }
    }
}

/// A camera pose in the renderer's spherical staging rig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Tangent of half the vertical field of view.
    pub fov: f32,
}

impl CameraPose {
    pub const fn new(yaw: f32, pitch: f32, distance: f32, fov: f32) -> Self {
        Self {
            yaw,
            pitch,
            distance,
            fov,
        }
    }

    /// Interpolate pose and keep the orbit on the short arc.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let mut delta = other.yaw - self.yaw;
        while delta > PI {
            delta -= 2.0 * PI;
        }
        while delta < -PI {
            delta += 2.0 * PI;
        }
        Self {
            yaw: self.yaw + delta * t,
            pitch: self.pitch + (other.pitch - self.pitch) * t,
            distance: self.distance + (other.distance - self.distance) * t,
            fov: self.fov + (other.fov - self.fov) * t,
        }
    }

    /// Slow orbital drift. The atlas never snaps its camera.
    pub fn drift(self, time: f32, rate: f32) -> Self {
        Self {
            yaw: self.yaw + time * rate,
            pitch: self.pitch + (time * rate * 0.37).sin() * 0.035,
            ..self
        }
    }
}

/// Everything the background needs for one page.
#[derive(Clone, Copy, Debug)]
pub struct SceneSpec {
    pub id: SceneId,
    /// Linear RGB primary light.
    pub color: [f32; 3],
    /// Linear RGB secondary light.
    pub accent: [f32; 3],
    pub energy: f32,
    pub accent_energy: f32,
    pub camera: CameraPose,
    pub drift: f32,
    pub phase: f32,
}

#[allow(clippy::too_many_arguments)] // a scene spec is clearer as one row
const fn scene(
    id: SceneId,
    color: [f32; 3],
    accent: [f32; 3],
    energy: f32,
    accent_energy: f32,
    camera: CameraPose,
    drift: f32,
    phase: f32,
) -> SceneSpec {
    SceneSpec {
        id,
        color,
        accent,
        energy,
        accent_energy,
        camera,
        drift,
        phase,
    }
}

/// The visual score. Each page is a different constellation of light.
pub const SCENES: [SceneSpec; 8] = [
    // Prologue — almost nothing, just depth.
    scene(
        SceneId::Prologue,
        [0.42, 0.58, 1.0],
        [1.0, 0.62, 0.38],
        0.55,
        0.45,
        CameraPose::new(-0.45, 0.10, 4.0, 0.62),
        0.004,
        3.7,
    ),
    // Star — a single furnace.
    scene(
        SceneId::Star,
        [1.0, 0.70, 0.40],
        [1.0, 0.84, 0.58],
        1.0,
        0.9,
        CameraPose::new(0.22, 0.12, 7.2, 0.52),
        0.012,
        8.1,
    ),
    // System — a quiet clockwork.
    scene(
        SceneId::System,
        [1.0, 0.82, 0.58],
        [0.55, 0.78, 0.95],
        0.9,
        0.6,
        CameraPose::new(0.62, 0.36, 12.5, 0.52),
        0.018,
        21.3,
    ),
    // Nebula — gas, dust and new light.
    scene(
        SceneId::Nebula,
        [0.86, 0.58, 0.92],
        [0.38, 0.82, 0.90],
        0.85,
        0.8,
        CameraPose::new(0.15, 0.04, 4.0, 0.75),
        0.005,
        42.0,
    ),
    // Galaxy — a spiral of suns.
    scene(
        SceneId::Galaxy,
        [1.0, 0.86, 0.66],
        [0.56, 0.68, 1.0],
        1.0,
        0.75,
        CameraPose::new(0.58, 0.72, 22.0, 0.50),
        0.010,
        63.0,
    ),
    // Quasar — the brightest engine.
    scene(
        SceneId::Quasar,
        [0.88, 0.94, 1.0],
        [1.0, 0.72, 0.40],
        1.0,
        0.75,
        CameraPose::new(0.32, 0.52, 18.0, 0.44),
        0.006,
        91.0,
    ),
    // Black hole — where light stops.
    scene(
        SceneId::BlackHole,
        [1.0, 0.74, 0.46],
        [0.52, 0.68, 1.0],
        1.0,
        0.9,
        CameraPose::new(0.16, 0.28, 19.5, 0.44),
        0.008,
        117.0,
    ),
    // Epilogue — the quiet returns.
    scene(
        SceneId::Epilogue,
        [0.52, 0.64, 0.86],
        [1.0, 0.72, 0.48],
        0.55,
        0.5,
        CameraPose::new(-0.18, 0.06, 4.0, 0.62),
        0.003,
        131.0,
    ),
];

pub fn scene_for(id: SceneId) -> &'static SceneSpec {
    &SCENES[id.mode() as usize]
}

/// Interpolate two palettes through a transition.
pub fn lerp_scene(a: &SceneSpec, b: &SceneSpec, t: f32) -> SceneSpec {
    SceneSpec {
        id: if t < 0.5 { a.id } else { b.id },
        color: lerp3(a.color, b.color, t),
        accent: lerp3(a.accent, b.accent, t),
        energy: a.energy + (b.energy - a.energy) * t,
        accent_energy: a.accent_energy + (b.accent_energy - a.accent_energy) * t,
        camera: a.camera.lerp(b.camera, t),
        drift: a.drift + (b.drift - a.drift) * t,
        phase: a.phase + (b.phase - a.phase) * t,
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
