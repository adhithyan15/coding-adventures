# paint-vm-wgpu

Portable WGPU backend for the Paint VM runtime.

## Overview

`paint-vm-wgpu` renders `paint-instructions::PaintScene` values into an
offscreen `Rgba8Unorm` WGPU texture and reads the result back as a
`PixelContainer`. It is the first concrete renderer built on top of
`paint-vm-gpu-core`, so Vulkan, OpenGL, Mesa, and other GPU-flavoured backends
can share the same scene lowering and tessellation model.

This crate is intentionally Tier 1: it validates the backend plumbing, command
ordering, solid mesh rendering, rectangular clips, and readback path without
pretending that text, image textures, gradients, or filters are already exact.

## Where It Fits

```text
Producer (barcode, Mermaid, layout, HTML)
  -> PaintScene              (paint-instructions)
  -> GpuPaintPlan            (paint-vm-gpu-core)
  -> paint-vm-wgpu           (THIS CRATE)
  -> PixelContainer          (pixel-container)
  -> paint-codec-png / other image codec
```

## Supported Slice

| Paint area | Status |
|------------|--------|
| Rect, line, ellipse, path | Implemented through shared solid mesh lowering |
| Clip | Implemented through WGPU scissor stack |
| Group transform / opacity | Implemented by the shared GPU plan |
| Layer transform / opacity | Implemented by the shared GPU plan |
| Offscreen readback | Implemented with padded row-copy handling |
| Images | Implemented for `ImageSrc::Pixels` through RGBA texture upload/sampling |
| Gradients | Degraded by `paint-vm-gpu-core` to first-stop solid fills |
| Text / glyph runs | Not implemented until glyph atlas and shaping strategy lands |
| Filters / blend modes | Not implemented |

## Runtime Use

```rust
let backend = paint_vm_wgpu::renderer();
let pixels = backend.render(&scene)?;
```

Register `renderer()` with `paint-vm-runtime` when a portable GPU backend is
acceptable. Exact text or image rendering should still select Direct2D, GDI,
Cairo, Skia, or future GPU backends until WGPU grows those paths.

## Next Steps

- Add gradient uniforms or small ramp textures instead of first-stop fallback.
- Add glyph atlas planning once the shared text shaping and font metric pipeline
  is ready.
- Reuse the same WGPU plumbing as a reference implementation for native
  Vulkan/OpenGL/Mesa backends.
