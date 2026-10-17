# Design notes

> 把目光交给黑暗，让光自己说话。

Deep Space Atlas is an editorial object disguised as a web page. The target was
not “a site with space pictures”; it was the feeling of opening a quiet,
well-printed atlas late at night, with a live sky behind the type.

## The idea in one paragraph

A field guide has a voice, a hierarchy and a pace. It does not shout. It gives
each subject a full page, a measured title, a few precise figures and a lot of
darkness. The interface should disappear until you need it: a rail, a counter, a
single line of instruction, a small control for motion. Everything else is the
subject.

## References and anti-references

**Close to the mark**

- Printed astronomical atlases: generous margins, hairline rules, a small
  catalogue number at the edge, figures set like captions.
- Long-form science writing: a serif display face, a sans body, a narrow
  measure, short paragraphs.
- Planetarium and observatory imaging: black is a color, highlights are sparse,
  and the eye is allowed to adapt.

**Deliberately avoided**

- Neon gradients, glassmorphism cards, floating dashboard panels.
- Full-bleed stock photography of galaxies (the backgrounds are procedural).
- Startup landing-page tropes: giant centered hero, three feature cards,
  gradient buttons, “scroll to explore” with a bouncing chevron.
- Fake data, fake co-ordinates, fake “live” telemetry, sound effects.
- Dense navigation chrome on every page.

## The grid

Each chapter is a full viewport with two zones: a **reading column** and a
**scene field**.

- Desktop: reading column on the left, `clamp(w·0.335, 360, 500)` wide, margin
  `clamp(w·0.055, 48, 96)`. The scene is composed right of center (`0.65, 0.50`
  in normalized screen coordinates). The two zones may overlap; a scrim keeps
  the type legible.
- Mobile: scene above, reading panel below. The panel is anchored to the bottom
  with 52 px of breathing room; the top 38% of the viewport is the scene. A
  gradient-to-solid scrim begins at 16% height so the title has a quiet field.
- The top bar is a single line: wordmark, subtitle, a motion pill, a counter.
  The rail lives at the right edge on desktop and becomes a horizontal row of
  ticks under the top bar on mobile.
- A footer line (`01 / 07` and `PROCEDURAL · REAL-TIME`) is intentionally quiet;
  it is the colophon, not navigation.

## Typography

| Role | Family | Size (desktop / mobile) | Notes |
| --- | --- | --- | --- |
| Chapter title | Noto Serif SC Light | `clamp(w·0.052, 52, 82)` / `clamp(w·0.118, 36, 54)` | Four or five characters; large, airy |
| English subtitle | Noto Sans SC / Latin | 12.5 / 10.5 | Uppercase, 0.17 em tracking |
| Kicker | Sans | 10.5 / 9.5 | Uppercase, 0.20 em tracking, accent |
| Lead | Noto Serif SC Regular | 19.2 / 16.8 | One sentence, 1.58 line height |
| Body | Noto Sans SC Regular | 15.2 / 13.8 | 27 / 23.6 line height, 0.78 white |
| Stat value | Noto Serif SC Regular | 21 / 19.5 | Tabular figures where available |
| Stat label | Sans | 8.8 / 8.6 | Chinese label + English note |
| Wordmark | Sans | 10.2 | 0.31 em tracking, all caps |
| Rail label | Sans | 8.4 | Numerals only |

The serif carries the subject; the sans carries the apparatus. The English
voice is small and tracked out, never a second headline. Chinese punctuation is
allowed to breathe; line heights are generous rather than tight.

## Palette

```text
ink             #04060B
deep ink        #070B14
text            #E6EAF2
muted           #8A93A6
faint           #566072
hairline        rgba(255,255,255,0.10–0.14)

prologue        cool blue        #88A8FF
star            warm gold        #FFB874
system          ice blue         #94C2FF
nebula          violet / rose    #CE94DC
galaxy          blue-white       #A0B8FF
quasar          amber / white    #FFC67E
black hole      amber / cyan     #FFB06A
epilogue        moonlight        #CCD8F4
```

Accents are used for the kicker, the active rail tick, the progress dot, the
motion pill and the cover CTA border. They are not used for body text or large
fills. The palettes in `src/scenes/mod.rs` are **linear** RGB for the shader;
the UI accents in `src/content/mod.rs` are display sRGB.

## The scrim

The background is allowed to be bright, but the type is never sacrificed.

- Desktop: a horizontal gradient from `rgba(3,5,10,238)` at the left edge to
  transparent at `margin + content_w + 150`, plus a faint top wash for the
  wordmark.
- Mobile: a vertical gradient from 16% to 38% height, then a solid
  `rgba(3,5,10,238)` reader panel with a 1 px hairline at its top edge.
- A barely visible accent glow sits in the bottom-left corner; it is warmth, not
  a light source.
- The shader has its own vignette; the UI scrim is separate and is painted over
  the callback.

## Motion

Motion is a page turn, not a scroll.

- Page transition: 820 ms. Outgoing type leaves over the first 46%, incoming
  type arrives from 32%, both with small vertical offsets and a 1.2% scale
  settle. The scene cross-blends with smootherstep and darkens by up to 24% at
  the midpoint.
- Reduced motion: 220 ms, no parallax, no twinkle/drift, no pulsing hint.
- Text entrance on an idle page: staggered fade/slide over ~0.9 s. The title
  arrives first, the figures last.
- Hover: the cover CTA and the motion pill get a subtle fill and a pointer
  cursor; the rail ticks brighten. No scale bounce, no glow.
- The scroll hint is a hairline with a slow travelling dot, not a bouncing
  chevron.

## Scene direction, chapter by chapter

### 00 Prologue

A sparse star field, one dim dust cloud, a single warm hairline arc. The cover
must feel like the first page of a book, not a hero section. The title is the
subject; the sky is the paper.

### 01 Star

A ray-sphere photosphere with multi-scale convection cells, dark sunspots,
bright faculae, limb darkening, a thin chromosphere and a restrained corona.
Warm gold, not white. The star is large but the type column stays on black.

### 02 System

A quiet clockwork: a central star, three analytical orbit planes, anti-aliased
orbit lines with a wide dust halo, and three representative worlds (rocky,
ocean/ice, banded gas giant with a ring). The chapter text covers all eight
planets; the scene is a stylised excerpt. The camera drifts slowly; the orbits
are not a diagram, but they are legible.

### 03 Nebula

Domain-warped fbm with ridged filaments, dust absorption, a violet/teal
ionisation rim, and a small hot nursery. The cloud is allowed to fill the right
side of the frame; the reading column gets the darkest part of the field.

### 04 Galaxy

An inclined thin disk sampled as a plane, with logarithmic spiral arms, dust
lanes, pink knots, a warm bulge and resolved disk stars. The arms should read as
a spiral within one second, without labels or arrows.

### 05 Quasar

A brilliant point, a thin accretion disk with 3D noise streaks and Doppler
beaming, and a collimated jet with knots. It is the brightest page in the atlas;
the scrim has to work hardest here.

### 06 Black hole

A black shadow, a thin photon ring, a direct and a weakly lensed disk, infalling
streams, and a soft upper glow. The composition is deliberately iconic: sphere,
ring, disk, darkness. The type sits to the left of the shadow.

### 07 Epilogue

The quiet returns: a deep field, a violet haze and one last light. The copy ends
with “天空没有尽头”; the visual should leave room for that sentence.

## Microtype and details

- Chapter numbers are `00`–`07`, set in the rail and the bottom-left colophon.
- The top counter is `NN — 07` on desktop, `NN/07` on mobile.
- Stat notes are uppercase English, tracked out, and intentionally faint. They
  are the “catalogue” layer, not essential reading.
- Hairlines are 1 px, 10–14% white. They align the kicker rule, the stats rule
  and the reader panel.
- The favicon is a ring and a point of light; no logotype.
- No shadows on text; contrast comes from the scrim. No rounded cards. The CTA
  uses a 2 px radius, the motion pill a full pill; these are the only two
  rounded shapes.

## Responsive behaviour

- The layout re-flows at `w < 760` or a portrait tablet ratio. It is not a
  scaled-down desktop.
- The text block auto-fits by measuring galleys and trying scales from 1.0 to
  0.70, then a compact mode without the English subtitle / stat notes.
- Long figures switch the stats to a 2 × 2 grid so every value keeps the same
  optical size.
- The canvas backing store is kept at `clientWidth × devicePixelRatio`, while
  the scene render scale is reduced independently for performance.
- Touch targets: the rail ticks are 28 × 24 px; the mobile motion pill is 30 ×
  26 px; the CTA is 198 × 42 px. The swipe threshold is 46 px with a velocity
  allowance.

## Accessibility

- Keyboard: arrows, PageUp/PageDown, Space, Home/End; `M` toggles motion.
- Touch: vertical swipe.
- The canvas `aria-label` names the current chapter.
- Reduced motion follows the OS and can be overridden manually; the preference
  is persisted only for the session (not localStorage).
- Text contrast is maintained by the scrim, not by disabling the art.
- No flashing, no rapid hue cycling, no autoplay audio.

## What would break the design

- Adding a second column of cards, a sidebar, or a nested scroll area.
- Replacing the procedural scenes with a photo carousel.
- Using pure white for large headings over a bright scene.
- Adding a bouncing chevron, a progress bar with a percentage, or a “next”
  button on every page.
- Making the stats into a dashboard with charts.
- Animating every element on hover; the page should feel still until touched.
