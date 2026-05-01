# Changelog — @coding-adventures/paint-vm-svg

## Unreleased

### Added

- Added SVG `PaintText` handling so graph visualizers can render browser-shaped
  labels instead of manually positioned glyph runs.

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
