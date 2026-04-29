# paint-vm-mesa

Mesa backend profile and shared GPU-plan adapter for the Paint VM runtime.

## Status

Mesa is a driver stack rather than one drawing API. This crate models Mesa as a
first-class runtime profile for software and driver-backed execution paths such
as llvmpipe and lavapipe. It now consumes `paint-vm-gpu-core` plans for the Tier
1 solid-vector slice while keeping runtime rendering unavailable until routing
to an OpenGL or Vulkan Mesa profile exists.

The `descriptor()` intentionally remains a Tier 0 scaffold so automatic runtime
selection does not pick Mesa before it can execute pixels. Use `profile()` and
`plan()` to validate the shared contract.

## Planned Execution Path

```text
PaintScene
  -> paint-vm-gpu-core::GpuPaintPlan
  -> Mesa profile router (llvmpipe/lavapipe)
  -> OpenGL or Vulkan execution path
  -> PixelContainer
```

## Next Steps

- Detect available Mesa profiles on Linux/WSL.
- Route llvmpipe through the OpenGL backend or lavapipe through Vulkan.
- Keep deterministic software execution selectable when native GPU drivers are
  unavailable.
