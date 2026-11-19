# Changelog

All notable changes to `game-clock` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added
- A durable public-API consumer for all 23 `game-clock-v1` shared-corpus cases, including exact output/error comparison, failed mutable-advance state preservation, and conformance negative controls.
- A Rust integration harness that checks the unguarded constructor Warrant model against `new_clock` over the full 648-case boundary matrix and seeded, stratified full-width property cases, with same-path fault controls.
- Pending in this draft: an exact ordered `validate_state` Warrant model and claim, plus a native parser/typechecker/evaluator harness comparing results with the public API and independent `u128` oracle across finite branch, threshold, property, and fuzz cases. This does not establish universal or Rust-source equivalence. Finalize this entry with the implementation before merge.

### Fixed
- `code/specs/game-clock.warrant` was invalid Warrant (`->` implication in a claim); rewritten as `if .. then .. else true`. Document only.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `game-clock` (`GAME10`), pure deterministic integer-nanosecond fixed-timestep accumulator.
- `GameClockConfig` with `standard_60hz()`, `standard_120hz()`, and `from_hz(hz, max_steps)` constructors.
- `GameClockState` and `ClockStepResult` stack-only `Copy` structures with zero heap allocation.
- `advance_frame` function with Glenn Fiedler "Fix Your Timestep!" accumulator logic and Spiral of Death guard (CWE-400).
- Sub-tick interpolation factors: `alpha_milli` (`0..1000`), `alpha_q16` (`0..65536`), and `alpha_f64()` (`[0.0, 1.0)`).
- Exact nanosecond conservation law tracking with `total_dropped_ns` and `spiral_clamped_frames`.
- Comprehensive 26-CWE matrix closure (`assurance_report.json`), `assurance-profile.json`, and `effect_certificate.json` (12 proved pure entry points).
- Conformance test suite (`tests/conformance.rs`) covering steady 60/120 Hz pace, 10,000-frame random jitter conservation law, CWE-400, CWE-369, CWE-190, and invariant corruption checks.
- Zero-allocation microbenchmark harness (`tests/microbenchmarks.rs`) with `CountingAllocator` and sabotage detection (`VER07`).
- Runtime capability gate (`tests/runtime_capability.rs`) validating `"capabilities": []` (`VER08b`).
