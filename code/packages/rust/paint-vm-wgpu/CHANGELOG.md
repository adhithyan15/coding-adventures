# Changelog

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
