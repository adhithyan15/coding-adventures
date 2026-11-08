## 0.2.1

- Implement the VER07 Level 5 microbenchmark harness and deterministic cost counters in `bench.rs`.
- Implement automatic batch calibration (`min_batch_duration` scaling) to eliminate timer overhead in microbenchmarks.
- Implement robust non-parametric statistics: median, Median Absolute Deviation (MAD), normalized MAD (nMAD), linear-interpolated percentiles (p25, p75, p95, p99), and deterministic SplitMix64 bootstrap confidence intervals.
- Implement abstract deterministic cost counters: `StepCounter` (abstract operation / WIR steps), `AllocCounter` (heap allocations and byte volumes), and `HardwarePerfCounter` (instructions retired and cycles via `perf_event_open` on Linux, falling back gracefully to `NotLive` on other platforms or unprivileged environments).
- Implement `Budget` enforcement distinguishing hard deterministic cost limits from soft advisory wall-clock targets to prevent noisy CI runner failures (VER00 §7.4.3).
- Format benchmark output cleanly into structured TAP 14 YAML diagnostics and JSON serialization.
- Achieves 93.9% line and 94.7% region coverage with zero external dependencies.
