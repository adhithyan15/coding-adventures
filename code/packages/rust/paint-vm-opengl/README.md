# paint-vm-opengl

OpenGL backend profile and shared GPU-plan adapter for the Paint VM runtime.

## Status

This crate now consumes `paint-vm-gpu-core` plans for the Tier 1 solid-vector
slice: indexed meshes, scissor clips, group/layer transform lowering, opacity
folding, and degraded solid-gradient fallback. The runtime renderer still
returns `BackendUnavailable` until a real OpenGL context, framebuffer, shader,
and readback path lands.

The `descriptor()` intentionally remains a Tier 0 scaffold so automatic runtime
selection does not pick OpenGL before it can execute pixels. Use `profile()` and
`plan()` to validate the backend contract while the native execution path is
being implemented.

## Planned Execution Path

```text
PaintScene
  -> paint-vm-gpu-core::GpuPaintPlan
  -> GLSL vertex/fragment shaders
  -> offscreen framebuffer
  -> glReadPixels RGBA8 readback
  -> PixelContainer
```

## Next Steps

- Create platform-specific headless GL contexts.
- Upload shared meshes to VBO/IBO buffers and draw them with GLSL.
- Add texture upload/sampling for `PaintImage`.
- Add glyph atlas sampling once the shared text shaping path is ready.
