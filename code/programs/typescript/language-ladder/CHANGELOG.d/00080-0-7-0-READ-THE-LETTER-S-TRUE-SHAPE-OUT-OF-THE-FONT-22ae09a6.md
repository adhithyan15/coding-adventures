## 0.7.0 — read the letter's TRUE shape out of the font

- **`src/truetype.ts`** — a zero-dependency TrueType reader: table directory,
  `cmap` (formats 4 and 12), `loca`, `glyf`, simple and composite glyphs, the
  delta-encoded coordinate flags, and the on-curve midpoint TrueType implies
  between consecutive off-curve points. Outlines come back in font units
  (y-up, baseline 0); the renderer applies one `scale(1,-1)`.
- **Why not hand-drawn SVG paths.** A subtly wrong ண looks fine to anyone who
  cannot already read Tamil — the entire audience — so the error would ship as
  the lesson. Extracting from the vendored font makes shapes correct by
  construction and keeps them identical to what the app renders text with.
- **Hostile input is bounded.** Every count and offset in a font file is
  attacker-controlled if this is ever pointed at an untrusted font, and it runs
  in the browser. `cmap` ranges clamp to U+10FFFF; a single decrementing budget
  bounds total mapping ITERATIONS across both cmap readers (capping the map's
  size alone is not enough — re-mapping groups and format 4's BMP-bounded keys
  both cost work without growing it); a component budget bounds composite
  FAN-OUT, which the depth cap does not (N components at each of 6 levels is
  N⁶ visits — minutes of frozen main thread from a 632-byte file);
  non-ascending contour end points and scaled components are refused rather
  than drawn wrong.
- **Tests rasterise the font** — flatten the quadratics, scan-convert with the
  non-zero winding rule — so shape assertions are checked against what the
  glyph actually looks like. **The raster window is derived from the glyphs'
  own bounding boxes and the rasteriser throws if a glyph would be clipped.**
  A second guard checks the metric's INPUT: the final-stroke measure reports
  its sample count, and the assertion requires samples before believing the
  answer. Without it the measure anchored on the top bar — which overhangs the
  final vertical — collected nothing, and `Math.max([]) - Math.min([])` is
  `-Infinity`, which satisfies any upper bound. It measured nothing and
  reported agreement.
  The window guard exists because a hard-coded window (x ≤ 1030, against ண's true
  extent of 1631) silently amputated 37% of the letter and produced a
  confident, wrong description of its final stroke. A clipped raster does not
  look like an error; it looks like a letter.

