# Architecture

This document describes the implementation as it exists in `0.1.0`. It is the
place for details that would bloat the README: the uniform contract, the paint
callback lifecycle, the transition formulas and the reasons behind the
web-specific startup code.

## Goals

- A reading experience, not a dashboard: full-viewport chapters, one vertical
  navigation axis, no nested scrolling.
- A genuinely procedural background: no bundled photographs or texture atlases.
- A clean boundary between text, layout, input and GPU code.
- A credible WebGPU pipeline: HDR scene target, bloom, tone mapping, adaptive
  resolution, and a friendly failure mode.
- A static, self-contained deployment produced by `trunk build --release`.

Non-goals: a general-purpose engine, a CMS, audio, 3D model loading, a WebGL2
fallback, or arbitrary page counts without touching the display labels.

## Layer contract

```text
content  ── text, figures, accents ─────────────┐
scenes   ── scene id, palette, camera, phase ──┤
                                                ▼
app      ── state, clock, input, transition ── SceneFrame
                                                │
                                                ▼
render   ── wgpu pipelines, WGSL, uniform ─── BackgroundCallback
                                                │
ui       ── scrim, galleys, rail, actions ◄────┘  (paint order only)
```

- `content` and `scenes` are pure data/description.
- `render` never reads `content` or `ui`; it consumes `SceneFrame`.
- `ui` never touches wgpu; it paints egui shapes.
- `app` is the only module that knows all four, and the only one that constructs
  `SceneFrame` and `FrameInput`.
- The only type crossing into egui's wgpu renderer is `BackgroundCallback`, which
  implements `egui_wgpu::CallbackTrait`.

## Module map

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | Native `run_native`; wasm WebGPU guard, canvas lookup, `WebRunner` |
| `src/lib.rs` | Public module map |
| `src/app/mod.rs` | `AtlasApp`, clock, boot fade, pointer smoothing, adaptive scale, paint-callback insertion |
| `src/app/input.rs` | `Navigation`: wheel accumulation, keys, touch swipe |
| `src/app/motion.rs` | `MotionPreference`: OS polling + manual override |
| `src/app/platform.rs` | wasm-only `navigator.gpu`, secure context, `matchMedia`, DOM labels, ready classes |
| `src/content/mod.rs` | `Page`, `Stat`, `PAGES`, `PAGE_COUNT` |
| `src/scenes/mod.rs` | `SceneId`, `CameraPose`, `SceneSpec`, `SCENES`, `scene_for` |
| `src/transition/mod.rs` | `PageTransition`, easing, text visuals, stagger |
| `src/ui/mod.rs` | Scrim, top bar, rail, hint, footer, actions |
| `src/ui/page.rs` | Galley layout/cache, fit scales, page painting, stats, CTA |
| `src/ui/fonts.rs` | Font registration and family helpers |
| `src/ui/theme.rs` | Palette, metrics, responsive type scale |
| `src/ui/widgets.rs` | `LayoutJob` helper, gradient mesh, text paint |
| `src/render/gpu.rs` | `SceneRenderer`, pipelines, targets, callback |
| `src/render/uniform.rs` | `SceneFrame`, `FrameUniform`, size assert |
| `src/render/shaders/atlas.wgsl` | All scene modes, bloom and composite |

## Frame lifecycle

### Native

`eframe::run_native` creates a wgpu render state. `AtlasApp::new` inserts a
`SceneRenderer` into `render_state.renderer.write().callback_resources`.

### Web

`main` checks `navigator.gpu` and `window.isSecureContext` through
`js_sys::Reflect`. If either is missing it adds `no-webgpu` to the document
element and returns without starting eframe. Otherwise:

1. Find `#atlas-canvas`.
2. `eframe::WebRunner::new()`.
3. `start(canvas, WebOptions { renderer: Wgpu, .. }, app_creator)`.
4. Leak the runner (`mem::forget`) so its animation loop and app runner live for
   the page lifetime.

`index.html` also performs a pre-wasm capability check so the fallback appears
before the bundle downloads, and it re-asserts the canvas backing-store size at
`clientWidth × devicePixelRatio` after layout (see [High-DPI](#high-dpi)).

### Per frame

1. `App::ui` receives the root egui `Ui`.
2. Advance the clock by `stable_dt` (clamped to 4 ms..100 ms). The clock is
   **app-owned**, not `InputState::time`, so it cannot stall if the backend
   reports a zero time.
3. Poll reduced motion once per second; read input; update the transition.
4. Build a `SceneFrame` from `content::page(current)` and, if transitioning,
   the `from`/`to` pages.
5. Append `Callback::new_paint_callback(viewport, BackgroundCallback { frame,
   render_scale })` to the root painter **first**.
6. `ui::draw` paints the scrim and type on top of the callback shape.
7. Handle `ui::Action` (GoTo / ToggleMotion) and update the accessible canvas
   label and `data-motion` attribute.
8. Adapt the render scale; request the next repaint after 16 ms.

The callback order matters: the callback paints the entire viewport into the
egui render pass, so any shape added before it would be overwritten. The UI
painter draws after it in the same shape list.

### Callback stages

`BackgroundCallback::prepare`:

- Reads `SceneRenderer` from `CallbackResources` (`get_mut`).
- Calls `prepare(device, queue, encoder, screen_size_in_pixels, &frame)`.
- Returns an empty `Vec<CommandBuffer>` because the work was encoded into
  egui's encoder.

`BackgroundCallback::paint`:

- Reads the renderer immutably.
- Calls `paint(&mut RenderPass<'static>)`, which draws the composite.

The callback itself is `Send + Sync`; on wasm, `CallbackResources` does not
require `Send + Sync`, so wgpu resources can live there comfortably.

## GPU resources

`SceneRenderer` owns:

- `target_format` (the egui surface format).
- A 240-byte uniform buffer and its bind group.
- A filtering sampler.
- Five pipelines: scene, downsample, blur H, blur V, composite.
- `Option<Targets>`: scene texture/view, bloom A/B textures/views, four bind
  groups (down, blur H, blur V, composite).

Formats and sizes:

- Scene target: `Rgba16Float`, dimensions
  `clamp(screen_px × render_scale, 64..screen_px)`.
- Bloom: scene / 4, minimum 8 px.
- All intermediates `Rgba16Float`; the composite writes `target_format`.

Resize: `ensure_targets` early-returns only when both the scene and bloom sizes
match. Otherwise it recreates textures, views and **all** bind groups. Old
views/bind groups are dropped with the old `Targets` value.

Bind groups:

| Pass | Layout bindings |
| --- | --- |
| Scene | 0 uniform |
| Downsample / blur H / blur V | 0 uniform, 1 texture, 3 sampler |
| Composite | 0 uniform, 1 scene, 2 bloom, 3 sampler |

The shared WGSL module declares bindings 1 and 2; the blur passes bind the same
view into both slots so a single module can serve every entry point without
unbound-binding validation errors.

Pass order:

```text
scene → bloom_a (downsample/bright)
bloom_a → bloom_b (blur H)
bloom_b → bloom_a (blur V)
composite(scene, bloom_a) → target_format
```

## Uniform contract

`SceneFrame` is the CPU-side frame. `FrameUniform` is `#[repr(C)]`, `Pod`, and
asserted at compile time to be 240 bytes (15 × `vec4`).

| Row | Contents |
| --- | --- |
| `res` | scene width, height, aspect, time |
| `modes` | mode A, mode B, blend, quality |
| `col_a` / `col_b` | linear RGB + energy |
| `cam_a` / `cam_b` | yaw, pitch, distance, tan(half-FOV) |
| `accent_a` / `accent_b` | linear RGB + accent energy |
| `pointer` | x, y, unused drift, motion |
| `post` | bloom threshold 0.85, bloom strength 0.34, grain 0.030, vignette 0.60 |
| `phase` | phase A, phase B, seed 19, veil coefficient |
| `screen` | output width, height, 1/w, 1/h |
| `texel` | 1/scene w, 1/scene h, 1/bloom w, 1/bloom h |
| `composition` | centre x, centre y, mobile scale, mobile flag |
| `flags` | encode sRGB, reduced motion, boot fade, unused |

`SceneFrame::uniform` fills sizes, `1/` values and the sRGB flag. The app sets
`veil = 0.24` during a transition and `0.0` otherwise; the shader computes
`1 - veil * sin(blend * π)`.

## Scene dispatch

`fs_scene` calls `scene_at(mode, camera, color, accent, uv, phase)`. Each mode
is a function in `atlas.wgsl`; all eight share `hash/noise/fbm`, `camera_ray`,
`origin_distance`, `sphere_t` and the post helpers.

| Index | Mode | Technique |
| --- | --- | --- |
| 0 | Prologue | Sparse cube-face stars, one dim dust cloud, hairline arc |
| 1 | Star | Ray-sphere photosphere, multi-scale Worley convection, sunspots, faculae, corona, prominences |
| 2 | System | Analytic sphere hits, tilted orbit planes, anti-aliased rings by analytic width, procedural planet albedo |
| 3 | Nebula | Domain-warped fbm, ridged filaments, dust absorption, ionisation rim, star grid |
| 4 | Galaxy | Inclined thin-disk plane sampling, logarithmic spiral arms, dust, knots, resolved stars, bulge |
| 5 | Quasar | Plane disk with 3D noise streaks, Doppler beaming, collimated knots, point core |
| 6 | Black hole | Impact-parameter lensing, shadow, photon ring, direct + weakly bent disk, infall streams |
| 7 | Epilogue | Quiet starfield, violet haze, a single last light |

The scene pass renders **two** modes during a transition and mixes them by
`smoothstep(blend)`; each mode keeps its own camera and palette.

## Transition state machine

`PageTransition` is a small struct: `from`, `to`, `elapsed`, `duration`,
`direction`, `active`.

- Start: ignored if `from == to`; duration is 0.82 s, or 0.22 s when reduced
  motion is requested.
- Update: clamp `elapsed`; when it reaches `duration`, `active = false` and
  `from = to`.
- `t()` is linear progress; `blend()` is smootherstep for the scene.
- Outgoing text: `smoothstep(0, 0.46, t)` → alpha `1-k`, offset `-26·k`,
  scale `1 − 0.012k`.
- Incoming text: `smoothstep(0.32, 1, t)` → alpha `k`, offset `+30·(1-k)`,
  scale `0.988 + 0.012k`.
- The UI chrome dips by `0.28·sin(πt)`; the shader veil uses the same sine.
- Element stagger: each text block has a delay; `stagger(enter, delay)` is a
  smoothstep from `delay` to `delay + 0.52`.

The incoming page’s idle entrance begins at the transition start, so once the
transition ends the text is already at full alpha.

## Input

`Navigation::update(ctx, now, dt, blocked)` returns at most one request.

- **Keys** — `ArrowUp/Down/Left/Right`, `PageUp/PageDown`, `Space`, `Home`,
  `End`; `M` toggles motion even while a transition is running.
- **Wheel** — uses `smooth_scroll_delta.y`; an accumulator resets after 0.18 s
  of quiet; a request fires above 42 units; a 0.62 s cooldown follows. One
  gesture cannot skip pages.
- **Touch** — tracks the first touch id, start y/time, last y/time. A swipe of
  > 46 px, or > 18 px with velocity > 520 px/s, triggers one page. `End` resets
  the touch; `Cancel` clears it.
- **Blocked** — while a transition is active, the wheel and touch state are
  cleared and navigation is ignored; `M` still works.

Mouse dragging is intentionally not a navigation gesture; the page is a reading
surface, and text selection / clicking the chrome must remain possible.

## Layout and typography

`Metrics::new(rect, mobile_hint)` chooses desktop or mobile:

- Desktop: margin `clamp(w·0.055, 48, 96)`, content width `clamp(w·0.335, 360,
  500)`, title `clamp(w·0.052, 52, 82)`, body 15.2 / 27.
- Mobile: margin 24, content `w − 48`, title `clamp(w·0.118, 36, 54)`, body
  13.8 / 23.6.
- Mobile if `hint || w < 760 || (w < 1000 && h > w·1.12)`.

`PageBlock::build` lays out every element with `LayoutJob` into cached galleys:

- The display family (Light serif) is used for titles; Regular serif for lead
  and statistics; sans for body, labels and English microtype.
- Galleys use `Color32::WHITE` and are painted with
  `galley_with_override_text_color`, so alpha can animate without re-layout.
- The cache key is `(page index, width/2, scale·100, compact)`; the cache is
  cleared when the content width changes by more than 1 pt.
- Fit: try scales `[1.0, 0.94, 0.88, 0.82, 0.76, 0.70]`, then the same scales
  in compact mode (no English subtitle / stat notes). The first block whose
  measured height fits `text_room` wins.
- Statistics use four columns only when every value is ≤ 8 characters; long
  figures switch to a 2 × 2 grid so all numbers keep the same optical size. A
  per-value shrink loop is a final fallback.

Desktop block placement is vertically centred and clamped below the top bar.
Mobile placement is bottom-aligned with 52 px of breathing room. The mobile
scrim is a gradient from 16% to 38% height followed by a solid reader panel;
the desktop scrim is a horizontal gradient behind the text column.

## High-DPI

eframe derives the egui viewport from `canvas.width / pixels_per_point`. In some
emulated / high-DPI contexts the ResizeObserver reports the CSS size rather than
the device-pixel size, which would make egui believe the viewport is 195 × 422
on a 390 × 844 phone. `index.html` re-asserts the backing store at
`clientWidth × devicePixelRatio` at load, on resize/orientation, and at 0.4 s /
1.2 s / 2.5 s after load. eframe then sees the intended 390 × 844 points and
renders at 2×.

The scene render scale is independent of DPR; it is applied to the physical
canvas size inside the renderer.

## Adaptive performance

- Start: 0.82 desktop, 0.62 mobile (0.72 on ≤ 4-core desktops).
- Frame time is smoothed: `frame_ms = 0.92·frame_ms + 0.08·dt_ms`.
- Lower the scale by 0.06 after 40 frames slower than 24 ms, while > 0.55.
- Raise it by 0.03 after 110 frames faster than 14 ms, while < 0.90.
- Quality uniform: 0.55 mobile; 0.70 desktop when scale < 0.66; otherwise 1.0.
- The shader uses `quality` to skip the fine star-surface octave, thin the
  galaxy’s resolved star field and reduce nebula sparkle on mobile.

## Observability

- `log` macros are forwarded to the browser console by `eframe::WebLogger`.
- `platform::mark_ready` adds `atlas-ready`; the boot overlay fades out.
- `platform::mark_no_webgpu` adds `no-webgpu`; the static fallback appears.
- The canvas `aria-label` is updated with the current chapter number/title.
- `<html data-motion="full|reduced">` exposes the manual/OS state for debugging
  and CSS.

## Testing

- `examples/preview.rs` creates a wgpu device without a surface, renders each
  scene at a full render scale, reads back with 256-byte row alignment and saves
  PNGs. It is the primary shader feedback loop.
- Native `cargo run` exercises the same eframe/egui path.
- Browser checks (Chrome 155 / D3D12) cover all chapters, wheel, keys, touch,
  rail clicks, CTA, motion toggle, mid-transition frames, mobile layout, DPR,
  and the no-WebGPU fallback.

## Invariants

1. Exactly one full-screen canvas; no DOM UI framework.
2. No nested scroll containers; the page never scrolls its own content.
3. Scene rendering happens in `prepare`; `paint` only composites.
4. `FrameUniform` stays 15 `vec4` rows; update the compile-time assert and this
   table together.
5. Every page has a `SceneId` that exists in `SCENES` and a mode handled by
   `scene_at`.
6. AGENTS files are local-only; user docs remain tracked.
7. Fonts remain subsetted and OFL-credited.
8. WebGPU is required; the fallback is a first-class surface, not an error page.
