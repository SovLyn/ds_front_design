# Contributing to Deep Space Atlas

谢谢你想为这部图鉴添一笔。This project is intentionally small: a Rust/egui
interface, a single procedural WebGPU background module, and a content layer.
The rules below keep it readable.

## Getting started

```bash
rustup update stable
rustup target add wasm32-unknown-unknown
cargo install trunk          # or download a Trunk 0.21+ binary
trunk serve                  # http://127.0.0.1:8080/
```

The same app runs natively for quick checks:

```bash
cargo run --release
```

You need a WebGPU-capable browser (Chrome/Edge 113+, Safari 26+) and a secure
context (`https://` or `localhost`). If WebGPU is unavailable you will see the
fallback page; that is expected.

## The four layers

Keep these boundaries intact:

1. **`content`** owns text and figures. It knows nothing about pixels.
2. **`scenes`** owns the scene identity, palette, camera and phase.
3. **`render`** owns wgpu, the WGSL shader and the uniform block. It never knows
   about page text.
4. **`ui` / `transition` / `app`** own layout, input and timing. They never talk
   to wgpu directly; they hand a `SceneFrame` to `BackgroundCallback`.

The only bridge from the interface into the renderer is
`render::BackgroundCallback`. If you need pixels in a new place, add a field to
`SceneFrame`/`FrameUniform` and document it in `docs/ARCHITECTURE.md`.

## Code style

- `cargo fmt --all` before every commit.
- `cargo clippy --all-targets -- -D warnings` should be clean. If a lint is a
  false positive, use a narrow `#[allow(...)]` with a comment.
- No `unwrap()` in library code where a failure is plausible. `expect()` is
  acceptable for truly impossible invariants (a missing canvas during wasm
  startup is handled with a fallback instead).
- Prefer small, named functions over long `ui` closures.
- Comments explain *why*, not *what*. The shader has a few unusually detailed
  comments because the math is not obvious.
- Keep the UI copy in `content`; do not hard-code chapter text in `ui`.

## Adding or editing a chapter

1. Edit `src/content/mod.rs`.
2. Use 2–4 paragraphs and 2–4 `Stat`s. Values should be rounded, sourced and
   free of false precision. Write in short, declarative Chinese; the English
   kicker/subtitle is a quiet counterpart, not a translation of every sentence.
3. The page block auto-fits by trying scales from 1.0 down to 0.70 and a compact
   variant. Check the longest paragraph at 390 px width; if it still overflows,
   shorten the copy rather than shrinking type further.
4. The cover label `NN — 07` and the rail are derived from `PAGE_COUNT`. If you
   add a ninth page, update the literal `07` in the counter strings and the
   progress label logic.

## Adding a scene

1. Add a `SceneId` variant and its `mode()` index in `src/scenes/mod.rs`.
2. Add a `SceneSpec` to `SCENES` (linear RGB palette, camera pose, drift, phase).
3. Add a `scene_*` function in `src/render/shaders/atlas.wgsl`, dispatch it from
   `scene_at`, and keep the same uniform interface.
4. Add the matching `Page` in `content`.
5. Run the headless preview and open the PNGs:

```bash
cargo run --release --example preview -- 1600 900
```

6. Check the browser console. Chrome is stricter than native naga about
   uniformity: do not call `fwidth`/`dpdx` inside a non-uniform branch. Prefer
   analytic widths for orbit lines.

## Fonts

The repository ships subsetted Noto CJK faces. Only the glyphs used by the
current copy are present, which keeps the bundle small. If you add text:

```bash
python -m pip install fonttools brotli
```

Download the variable fonts from the Google Fonts repository:

- `https://github.com/google/fonts/tree/main/ofl/notosanssc`
- `https://github.com/google/fonts/tree/main/ofl/notoserifsc`

Generate a character set from the Rust sources plus the UI labels, then:

```python
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

src = "NotoSerifSC[wght].ttf"
font = TTFont(src)
instantiateVariableFont(font, {"wght": 300}, inplace=True)
font.save("NotoSerifSC-Light-full.ttf")
```

```bash
pyftsubset NotoSerifSC-Light-full.ttf \
  --text-file=chars.txt \
  --output-file=assets/fonts/NotoSerifSC-Light.ttf \
  --layout-features='*' --no-hinting --desubroutinize
```

Repeat for the 400 serif and the 400 sans. The OFL text must stay in
`assets/fonts/OFL.txt`. Do not commit the multi-megabyte variable fonts.

## Documentation

- User/contributor docs: `README.md`, `CONTRIBUTE.md`, `TODO.md`,
  `CHANGELOG.md`, `LICENSE`, `docs/*.md` (except `docs/AGENTS.md`).
- Agent-only guides (`AGENTS.md`, `src/**/AGENTS.md`, `docs/AGENTS.md`) are
  intentionally gitignored. Do not add them to version control.
- If you change a public command, a module boundary, the uniform layout or the
  transition timing, update `README.md` and `docs/ARCHITECTURE.md` in the same
  change.

## Testing

There is no test harness for the visuals. The practical gate is:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo check --target wasm32-unknown-unknown
trunk build --release
cargo run --release --example preview -- 1600 900
```

Then open `trunk serve` in a WebGPU browser and check:

- all eight chapters and the cover CTA;
- wheel, arrow keys, `PageUp`/`PageDown`, `Home`/`End`, the rail, and a touch
  swipe (or DevTools touch emulation);
- transitions at ~0.3 s and ~0.6 s (no text overlap, no black flash);
- the `M` toggle and the OS reduced-motion setting;
- 390 × 844 and 1440 × 900, including a 2× DPR canvas;
- the fallback with `navigator.gpu` removed before load;
- the browser console: no WGSL validation errors and no panics.

`examples/preview.rs` is the fastest visual loop for shader work. It uses a real
wgpu device and the same composite as the web build.

## Performance and accessibility expectations

- Keep the scene pass at one full-screen draw and bloom at quarter resolution.
- Do not introduce per-frame allocations in the hot path (the page galley cache
  exists for this reason).
- Do not add nested scroll areas or hijack horizontal scrolling.
- Keep keyboard and touch navigation working; do not require a mouse.
- Respect `prefers-reduced-motion`; do not add flashing or rapid hue cycling.
- Maintain text contrast over bright scenes with the existing scrim rather than
  turning the scene dark globally.

## Pull request checklist

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo check --target wasm32-unknown-unknown`
- [ ] `trunk build --release`
- [ ] checked at 1440 × 900 and 390 × 844
- [ ] checked the browser console for WGSL / wgpu errors
- [ ] README/docs updated if behaviour or commands changed
- [ ] no `AGENTS.md` or build output in the diff
- [ ] fonts subset and OFL credited if copy changed

## License

By contributing you agree that your changes are released under the MIT License
in `LICENSE`. Bundled fonts remain under the SIL Open Font License 1.1.
