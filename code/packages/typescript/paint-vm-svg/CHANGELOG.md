# Changelog — @coding-adventures/paint-vm-svg

## [0.1.1] — 2026-05-04

### Added

- **`PaintText` handler** (`"text"` instruction) — closes the gap between the
  canvas and SVG backends. Emits a `<text>` element with individual SVG
  presentation attributes (`font-family`, `font-size`, `font-weight`,
  `font-style`, `fill`, `text-anchor`).
- **`parseSvgFontRef()`** — parses `"canvas:"` and `"svg:"` font_ref schemes
  into separate SVG attributes. Accepts the same grammar as the canvas backend
  (`canvas:<family>@<size>[:<weight>[:<style>]]`) so `layout-to-paint` in
  `"text"` emit mode works without modification against both backends.
- **`textAlignToSvgAnchor()`** — maps `PaintText.text_align` ("start" |
  "center" | "end") to SVG `text-anchor` ("start" | "middle" | "end").
  The Canvas uses "center" but SVG requires "middle" — this mapping is
  intentional to produce valid SVG.
- **Security hardening** — `parseSvgFontRef` strips non-allowlisted characters
  from `font-family` (allowlist: `[a-zA-Z0-9 ,\-_.]`), validates `font-weight`
  in [1, 1000], and allowlists `font-style` to "italic" / "oblique". Unknown
  scheme prefixes degrade gracefully to `sans-serif` rather than throwing.
- **XML escaping for text content** — `escText()` applied to `instr.text` so
  `&`, `<`, `>` in user-supplied strings produce valid XML (`&amp;`, `&lt;`,
  `&gt;`).
- 21 new tests covering: basic output, scheme variants ("canvas:" / "svg:"),
  font-weight/style attributes, `text_align` → `text-anchor` mapping, XML
  escaping, `RangeError` on non-finite `font_size`, security injection
  hardening, and a cowsay-style composed scene.

## [0.1.0] — 2026-04-03

### Added

- `createSvgVM()` — factory that returns a fully-configured
  `PaintVM<SvgContext>` with handlers for all 10 instruction kinds: `rect`,
  `ellipse`, `path`, `line`, `group`, `layer`, `clip`, `gradient`, `image`,
  `glyph_run`.
- `renderToSvgString(scene)` — convenience entry point that creates a VM,
  executes the scene, and assembles a complete `<svg>` string.
- `assembleSvg(scene, ctx)` — assembles the final SVG string from a
  pre-populated `SvgContext`. Useful for custom pre/post-processing.
- `makeSvgContext()` — creates a fresh `SvgContext` (defs/elements arrays,
  clip/filter counters).
- **Gradient support** — `PaintGradient` emits `<linearGradient>` or
  `<radialGradient>` into `<defs>`. Referenced via `fill="url(#id)"`.
- **Filter support** — `PaintLayer.filters` are compiled to a `<filter>`
  element in `<defs>` using SVG filter primitives: `feGaussianBlur`,
  `feDropShadow`, `feColorMatrix`, `feComponentTransfer`.
- **Blend mode support** — `PaintLayer.blend_mode` maps to
  `style="mix-blend-mode:..."`. Runtime-allowlisted against valid CSS
  blend mode values.
- **Clip path support** — `PaintClip` emits a `<clipPath>` in `<defs>` and
  wraps children in a `<g clip-path="url(#...)">`.
- **PixelContainer image fallback** — `PaintImage` with a `PixelContainer`
  src emits a `data:` placeholder (full encode requires a codec).
- **export() throws ExportNotSupportedError** — SVG is a vector format;
  it does not produce pixel data. The export path is explicitly not supported.
- **Comprehensive security hardening** (applied during code review):
  - `safeNum()` validates every numeric value before SVG attribute
    interpolation (prevents NaN/Infinity producing malformed SVG and
    non-numeric values causing attribute injection).
  - Runtime allowlists for `fill_rule`, `stroke_cap`, `stroke_join`, and
    `blend_mode` (prevents enumerated-string attribute injection via `as any`
    or deserialized payloads).
  - `glyph_id` range-validated to `[0, 0x10FFFF]`; out-of-range values
    replaced with U+FFFD (prevents XML character reference injection).
  - `sanitizeImageHref()` allowlists only `data:`, `https:`, and `http:`
    URI schemes (prevents javascript: XSS, file: LFI, and SSRF).
  - `BLEND_MODE_ALLOWLIST` prevents CSS injection via crafted blend mode
    strings in `style` attributes.
- 63 unit tests; 98.01% line coverage.
