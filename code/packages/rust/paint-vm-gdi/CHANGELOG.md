# Changelog

## Unreleased

### Fixed

- Cleared all 3 `cargo clippy --all-targets -- -D warnings` errors in this
  crate. Like its Direct2D sibling, `paint-vm-gdi` was linted by *no* CI leg:
  its default `BUILD` is a bare `echo SKIP` (the crate `compile_error!`s off
  Windows), and CI never ran the build tool on the Windows leg, so `BUILD_windows`
  never executed either. No rendering behaviour changed.
  - `unnecessary_cast` x2 — `BI_RGB.0 as u32` in `create_surface` and
    `render_unsafe`. `BI_RGB` is a `BI_COMPRESSION` newtype over `u32` and
    `BITMAPINFOHEADER::biCompression` is a `u32`, so the cast was a no-op.
  - `too_many_arguments` x1 — `render_offscreen_composited` takes 8 parameters
    (limit 7). Silenced with a scoped `#[allow]` plus a documented reason: its
    parameters come from three unrelated sources, and its two callers read the
    middle group off two *different* types (`PaintGroup` and `PaintLayer`), so
    there is no honest struct to bundle them into.

### Audited

- Checked for the `.min(255)`-after-`as u8` idiom that produced three of
  `paint-vm-direct2d`'s errors. **It does not occur here, and cannot**:
  this backend never un-premultiplies. It only ever multiplies *down*
  (`(channel * alpha + 127) / 255` in `u16`, max `(255*255+127)/255 = 255`),
  `finalize_surface_alpha` clamps colour to coverage with a live integer
  `.min(alpha)`, and the DIB readback is a plain BGRA->RGBA channel swap that
  forces alpha to 255. There is no division by alpha anywhere, so there is no
  quotient that could exceed 255 and no dead clamp hiding a colour bug.

## 0.1.0 — 2026-04-12

### Added

- Initial release
- `render(scene: &PaintScene) -> PixelContainer` — main API
- PaintRect support via GDI `FillRect`
- PaintLine support via GDI `CreatePen` + `MoveToEx`/`LineTo`
- PaintGroup support (recursive dispatch into children)
- PaintClip support via `SaveDC`/`IntersectClipRect`/`RestoreDC`
- PaintGradient support for linear and radial fills/strokes via masked pBGRA surfaces
- Hex colour parser (#rgb, #rrggbb, #rrggbbaa, "transparent")
- BGRA→RGBA pixel conversion from DIBSection memory
- Top-down DIBSection for correct coordinate system (no Y-flip)
- Barcode-pattern rendering test (alternating black/white bars)
- QR-like checkerboard rendering test
