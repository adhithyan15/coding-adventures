# Changelog — gpu-runtime

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `Runtime` — thread-safe GPU runtime holding the selected backend in a
  `Mutex<RuntimeInner>`.
- `Runtime::global()` — process-wide singleton via `OnceLock<Arc<Runtime>>`.
- `Runtime::detect()` — probes backends in priority order: Metal → CUDA → CPU.
- `Runtime::cpu_only()` — constructs a CPU-only runtime, bypassing detection.
  Useful in sandboxed test environments without GPU driver access.
- `Runtime::run_1d()` — dispatch a 1D operation over `count` output bytes
  (one thread per byte).
- `Runtime::run_pixels()` — dispatch a pixel-level operation over
  `pixel_count` RGBA pixels (one thread per pixel, output = `pixel_count * 4`
  bytes).  The correct primitive for image point operations.
- `Shaders` — per-backend shader bundle: MSL source, CUDA C source, and a
  Rust CPU fallback `fn`.
- `BackendKind` — enum: `Metal`, `Cuda`, `Cpu`.
- `GpuError` — typed error: `NoShaderForBackend`, `Metal(String)`,
  `Cuda(String)`, `Cpu(String)`.
- Feature flag `metal` (default: on): gates `metal-compute` dependency.
  Disable with `--no-default-features` to avoid loading Metal.framework at
  process startup (useful in sandboxed CI environments).
- GPU-specific tests (`detect_succeeds`, `global_is_singleton`) marked
  `#[ignore]`; run with `cargo test -- --ignored` on a real GPU machine.
