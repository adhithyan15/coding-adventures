# paint-vm-vulkan

Vulkan backend profile and shared GPU-plan adapter for the Paint VM runtime.

## Status

This crate now consumes `paint-vm-gpu-core` plans for the Tier 1 solid-vector
slice: indexed meshes, scissor clips, group/layer transform lowering, opacity
folding, and degraded solid-gradient fallback. The runtime renderer still
returns `BackendUnavailable` until a Vulkan instance, device, render pass,
pipeline, and readback path lands.

The `descriptor()` intentionally remains a Tier 0 scaffold so automatic runtime
selection does not pick Vulkan before it can execute pixels. Use `profile()` and
`plan()` to validate the backend contract while the native execution path is
being implemented.

## Planned Execution Path

```text
PaintScene
  -> paint-vm-gpu-core::GpuPaintPlan
  -> SPIR-V graphics pipeline
  -> offscreen color attachment
  -> transfer to host-visible buffer
  -> PixelContainer
```

## Next Steps

- Create instance/device/queue selection with headless surfaces avoided.
- Upload shared meshes to vertex/index buffers.
- Add render-pass and pipeline creation for solid-color meshes.
- Add sampled image textures and glyph atlas textures.
