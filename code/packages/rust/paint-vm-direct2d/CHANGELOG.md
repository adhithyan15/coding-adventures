# Changelog

## Unreleased

### Fixed

- Cleared all 8 `cargo clippy --all-targets -- -D warnings` errors in this
  crate. They were invisible to this crate's own CI step but blocked every
  downstream crate: clippy lints path dependencies, so any workspace member
  depending on `paint-vm-direct2d` (e.g. `mosaic-compile`) could not get a
  clean `-D warnings` run. No rendering behaviour changed.
  - `field_reassign_with_default` ×2 — `D2D1_LAYER_PARAMETERS` in `with_layer`
    and `WindowAttributes` in `show_scene_in_window` are now built with a
    single struct initializer plus `..Default::default()`.
  - `redundant_guards` ×1 — `map_font_family`'s `other if other.is_empty()`
    arm is now a plain `""` pattern (the input is already `trim()`ed).
  - `missing_safety_doc` ×2 — added `# Safety` sections to the public unsafe
    `render_to_hwnd` and `show_scene_in_window`, documenting the HWND
    validity, COM apartment, and UI-thread requirements.
  - `unnecessary_min_or_max` ×3 — removed the unreachable `.min(255)` applied
    *after* the `as u8` cast in the premultiplied → straight alpha
    conversion.

### Changed

- Extracted the un-premultiplication arithmetic into a documented
  `unpremultiply_channel(premultiplied, alpha)` helper so the clamping
  contract is unit-testable without a live Direct2D device.

### Added

- Three regression tests for `unpremultiply_channel`, including an exhaustive
  check over all `(premultiplied, alpha)` pairs the conversion loop can see,
  proving the saturating `as u8` cast is equivalent to clamping before the
  cast. This is what makes removing the dead `.min(255)` provably behaviour
  preserving, and it guards the clamp against a future refactor.

## 0.1.0 — 2026-04-12

### Added

- Initial release
- `render(scene: &PaintScene) -> PixelContainer` — main API
- PaintRect support via Direct2D `FillRectangle`
- PaintLine support via Direct2D `DrawLine` with stroke width
- PaintGroup support (recursive dispatch into children)
- PaintClip support via `PushAxisAlignedClip`/`PopAxisAlignedClip`
- Hex colour parser (#rgb, #rrggbb, #rrggbbaa, "transparent")
- Offscreen rendering via WIC bitmap render target (no HWND needed)
- Premultiplied BGRA → straight RGBA pixel conversion
- COM initialization (CoInitializeEx) with automatic cleanup
- Barcode-pattern rendering test (alternating black/white bars)
- QR-like checkerboard rendering test
- Linear and radial `PaintGradient` rendering via native Direct2D gradient brushes
