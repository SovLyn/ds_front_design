# TODO / Known limits

This is a living list, not a roadmap promise. Items marked **decision** are
intentional trade-offs recorded so that future work does not “fix” them by
accident.

## Visuals

- [ ] **Star surface** — the Worley convection cells read as a little too
      regular at large sizes. Try a multi-scale cellular noise with stronger
      domain warp, or a small hand-tuned sunspot mask.
- [ ] **Quasar disk** — the inner disk can still clip to white at 1440p. Reduce
      the Doppler beam or move the exposure into a separate ACES curve.
- [ ] **Galaxy core** — the transition from the warm bulge to the arms leaves a
      slightly dark ring. A smoother inner-disk component would help.
- [ ] **Epilogue** — the single warm glow is abstract; a small inclined galaxy
      or a distant cluster would give the last page more narrative.
- [ ] **Black-hole lensed arc** — the upper arc is an artistic approximation.
      A physically derived bending function would be more accurate, at the cost
      of a more expensive shader.
- [ ] **System rings** — the Saturn-style ring is visible only at some camera
      angles / sizes. Increase the ring extent or make the outer planet larger.
- [ ] **Nebula** — could use a few more filamentary tendrils and a darker outer
      field; the current cloud is deliberately soft.

## Performance

- [ ] Profile the wasm build on mid-range Android and iOS. The current adaptive
      controller lowers the render scale, but the bloom passes and the 27-cell
      Worley star surface deserve a dedicated budget.
- [ ] Consider enabling Trunk's `wasm-opt` (`data-wasm-opt="z"`) in the release
      pipeline; Binaryen is not currently a build prerequisite, so it is left
      optional.
- [ ] Add a low-power mode that lowers the scene to 30 fps and disables bloom on
      very small devices.
- [ ] The font subsets are ~700 KB together; a CJK variable font with a smaller
      glyph set or a system-font fallback could reduce the initial payload.

## Interface

- [ ] **Deep links** — the current page is not reflected in the URL. Add a hash
      route (`#/black-hole`) and handle `hashchange` without breaking the state
      machine.
- [ ] **Focus rings** — the rail, CTA and motion pill are pointer-first. Add
      visible keyboard focus and `Enter`/`Space` activation for the CTA.
- [ ] **Screen reader narration** — the canvas `aria-label` announces the
      chapter, but the paragraphs are not exposed as a DOM text alternative.
      A visually hidden chapter summary would improve this.
- [ ] **Localization** — Chinese is primary, English is a display voice. A full
      English edition would need a content locale layer and a second font
      subset.
- [ ] **Page count label** — the cover’s `NN — 07` and the rail assume eight
      chapters. If the count changes, update the literals and the progress
      labels (they are derived from `PAGE_COUNT` except for the `07` display).

## Testing

- [ ] Add screenshot-based visual regression for the eight scenes at fixed
      times, using the headless preview.
- [ ] Add a browser test that asserts keyboard, wheel, touch, rail and CTA
      navigation (the local Puppeteer script is currently ad hoc).
- [ ] Test Safari 26 WebGPU and Firefox WebGPU on real hardware. Local testing
      was Chrome/D3D12 only.
- [ ] Test a 3× DPR phone; the canvas DPR fit in `index.html` is designed for it
      but has only been exercised at 1× and 2×.

## Decisions

- **No WebGL2 fallback.** WebGPU is part of the brief. Unsupported browsers get
  the styled fallback page instead of a different renderer.
- **No external images.** All backgrounds are WGSL; screenshots in `docs/` are
  documentation artifacts, not runtime assets.
- **No subagents.** The build environment could not guarantee that subagents
  would run exactly the same model, so all code and docs were written serially
  by the primary agent to keep the art direction and module boundaries
  consistent.
- **Canvas DPR fit.** eframe sizes the backing store from a resize observer; in
  some high-DPI / emulated contexts the observer reports CSS pixels. `index.html`
  re-asserts `clientWidth × devicePixelRatio` after layout. This is a pragmatic
  fix, not a claim that eframe is wrong.
- **Static fallback.** The no-WebGPU page is plain HTML/CSS so it works before
  wasm, without a second framework.
- **Fonts.** Only the current copy is subsetted. Adding chapters requires
  regenerating the fonts; this is intentional to keep the wasm payload small.
