# Changelog

All notable changes to Deep Space Atlas are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
semantic versioning once it leaves the 0.x series.

## [0.1.0] — 2026-10-17

### Added

- Eight-chapter vertical atlas: prologue, star, planetary system, nebula, galaxy,
  quasar, black hole, epilogue.
- Rust + eframe/egui application with a WebGPU background drawn by a single
  custom WGSL shader.
- Procedural scenes: starfield, stellar surface, orbit lines and planets,
  molecular cloud, spiral galaxy, quasar disk and jets, lensed black hole,
  quiet epilogue field.
- Offscreen HDR scene pass, quarter-resolution bloom, ACES-like composite,
  vignette, display-space grain and conditional sRGB encoding.
- Wheel, keyboard, touch-swipe, rail-click and cover-CTA navigation; accumulated
  wheel threshold and cooldown to prevent multi-page jumps.
- State-machine page transitions with staggered text exit/entry and a
  cross-blended scene; reduced-motion transitions at ~220 ms.
- Responsive layout with galley caching, auto-fit scales, a 2×2 statistic grid
  for long figures, desktop scrim and mobile reader scrim.
- Reduced-motion detection (`prefers-reduced-motion`) with a manual `M` / pill
  toggle.
- Adaptive render scale and a mobile quality tier.
- Friendly no-WebGPU / no-JS fallback page and a DOM boot sequence.
- Bundled, subsetted Noto Serif SC / Noto Sans SC faces under the SIL OFL 1.1.
- Headless wgpu preview example (`examples/preview.rs`) for shader iteration.
- User documentation (`README.md`, `CONTRIBUTE.md`, `TODO.md`,
  `docs/ARCHITECTURE.md`, `docs/DESIGN.md`, `docs/DEPLOY.md`) and local-only
  `AGENTS.md` guides.

### Notes

- Initial release; no stable public API.
- Tested with Rust 1.95, Chrome 155 / D3D12, and the headless Vulkan preview.
- WebGPU is required; the app intentionally does not fall back to WebGL2.
