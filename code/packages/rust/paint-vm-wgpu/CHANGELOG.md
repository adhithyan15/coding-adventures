# Changelog

## 0.1.0

- Added a Tier 1 WGPU renderer for solid `paint-vm-gpu-core` meshes.
- Added RGBA texture upload and nearest-neighbor sampling for
  `ImageSrc::Pixels`.
- Added offscreen `Rgba8Unorm` render target creation and CPU readback.
- Added WGPU scissor-stack support for rectangular clips.
- Declared text, glyphs, filters, and exact gradients as unsupported or degraded
  so runtime selection remains honest.
