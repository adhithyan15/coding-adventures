# Changelog

## Unreleased

- Consume the shared isolated-layer command vocabulary and reject it with a
  stable offscreen-pass diagnostic instead of silently flattening layer
  opacity, filters, or blend modes.
- Declare isolated layer capabilities explicitly in the shared GPU profile.

## 0.1.0

- Added a Tier 1 WGPU renderer for solid `paint-vm-gpu-core` meshes.
- Added RGBA texture upload and nearest-neighbor sampling for
  `ImageSrc::Pixels`.
- Added linear gradient rendering through shared ramp textures and linear
  filtering.
- Added radial gradient rendering through shared 2D gradient textures and
  linear filtering.
- Added offscreen `Rgba8Unorm` render target creation and CPU readback.
- Added WGPU scissor-stack support for rectangular clips.
- Declared text, glyphs, and filters as unsupported so runtime selection
  remains honest.
