# Changelog — @coding-adventures/paint-vm-canvas

## Unreleased

### Added

- `PaintText` handler — dispatches the new `PaintText` instruction from P2D00 using `ctx.fillText`. Parses `canvas:<family>@<size>[:<weight>[:<style>]]` font_refs into CSS font shorthand, validates and sanitizes the family string, clamps out-of-range weights to 400, and throws `UnsupportedFontBindingError` for any non-`canvas:` scheme (enforcing the font-binding invariant from TXT00/TXT03d).
- Honor `PaintText.text_align` by setting `ctx.textAlign` to the matching value before dispatching `fillText`. Default remains `"start"` (Canvas default) when the field is omitted.
- `UnsupportedFontBindingError` export — thrown when `PaintText.font_ref` uses a scheme that this backend cannot consume (e.g. `coretext:`, `directwrite:`).
- Handler count increased from 10 to 11 (new `text` kind registered alongside `glyph_run`).

## [0.1.0] — 2026-04-03

### Added

- `createCanvasVM()` — factory that returns a fully-configured
  `PaintVM<CanvasRenderingContext2D>` with handlers for all 10 instruction
  kinds: `rect`, `ellipse`, `path`, `line`, `group`, `layer`, `clip`,
  `gradient`, `image`, `glyph_run`.
- `resolveFill(fill, ctx)` — resolves `"url(#id)"` references to
  `CanvasGradient` objects stored in the gradient registry, or returns the
  raw CSS color string unchanged.
- **Gradient registry** — a `WeakMap<CanvasRenderingContext2D, Map<string,
  CanvasGradient>>` that maps gradient ids to live `CanvasGradient` objects.
  Cleared on each `execute()` call so gradients stay in sync with the scene.
- **Rounded rect support** — `PaintRect.corner_radius` maps to
  `ctx.roundRect()` when available, with an `arcTo`-based polyfill for older
  environments.
- **Filter support** — `PaintLayer.filters` are converted to a CSS filter
  string (`ctx.filter`): `blur`, `drop_shadow`, `brightness`, `contrast`,
  `saturate`, `hue_rotate`, `invert`, `opacity`. `color_matrix` is skipped
  (no CSS equivalent).
- **Blend mode support** — `PaintLayer.blend_mode` maps to
  `ctx.globalCompositeOperation` (underscore → hyphen conversion).
- **PixelContainer image support** — `PaintImage` with an 8-bit RGBA
  `PixelContainer` src renders via `ctx.putImageData`. URI string sources
  render a placeholder rect (async image loading not supported in
  synchronous `execute()`).
- **export() via OffscreenCanvas** — `vm.export(scene, opts?)` renders to an
  internal `OffscreenCanvas`, reads back pixels via `getImageData`, and
  returns a `PixelContainer`. Throws `ExportNotSupportedError` in environments
  without `OffscreenCanvas`.
- 55 unit tests; 95.68% line coverage (the uncovered 4.32% is the
  `OffscreenCanvas` success path, which requires a real browser or
  `node-canvas` and cannot be exercised in a jsdom test environment).
