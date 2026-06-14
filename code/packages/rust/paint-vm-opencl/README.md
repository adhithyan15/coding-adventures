# paint-vm-opencl

OpenCL compute backend profile and shared GPU-plan adapter for the Paint VM
runtime.

## Status

OpenCL is modeled as a compute raster path rather than a native vector API. This
crate now consumes `paint-vm-gpu-core` plans for the Tier 1 textured slice:
indexed meshes, scissor clips, pixel-image uploads, and generated linear/radial
gradient textures. Runtime rendering remains unavailable until an OpenCL
context, kernels, buffers, and readback path lands.

The `descriptor()` intentionally remains a Tier 0 scaffold so automatic runtime
selection does not pick OpenCL before it can execute pixels. Use `profile()` and
`plan()` to validate the backend contract while compute kernels are being built.

## Planned Execution Path

```text
PaintScene
  -> paint-vm-gpu-core::GpuPaintPlan
  -> OpenCL C raster kernels
  -> RGBA8 storage buffer
  -> PixelContainer
```

## Next Steps

- Build a kernel-side triangle coverage rasterizer for solid meshes.
- Add buffer upload/readback and device fallback selection.
- Execute texture sampling for `PaintImage` and generated gradients.
- Add glyph atlas buffers once the shared text shaping path is ready.
