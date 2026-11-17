# Changelog

All notable changes to the `sim-crt-vector-beam` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- Parked `code/specs/sim-crt-vector-beam.warrant` as `sim-crt-vector-beam.warrant.draft`: it uses `F64` (not implemented in Warrant yet) and `implies` (not a keyword). Restore after VER04p floats (WX-FL1, backlog WX-FL2). No code change.

## 0.1.0 — 2026-11-15

### Added
- Initial release of `sim-crt-vector-beam`: cycle-accurate dual CRT beam simulation covering raster deflection with phosphor persistence decay and oscilloscope/vector display stroke synthesis (GAME15b).
- Pure Rust implementation in `src/lib.rs` under `#![forbid(unsafe_code)]`.
- Core data structures:
  - `RasterTiming`: Configurable NTSC and arcade video raster timing (`visible_width`, `visible_height`, `h_front_porch`, `h_sync`, `h_back_porch`, `v_front_porch`, `v_sync`, `v_back_porch`), with preset constructors `standard_ntsc()` and `standard_arcade()`.
  - `PhosphorType`: Historical phosphor decay modeling (`P4White`, `P31Green`, `P1Green`, `Custom`) with Q16.16 decay factor and RGB color tint.
  - `BeamFlags`: Electron beam raster sweep flags (`in_hblank`, `in_vblank`, `in_hsync`, `in_vsync`, `frame_completed`, `line_completed`, `visible_pixel`).
  - `CrtRasterBeam`: Zero-allocation raster electron beam deflection stepper (`step_raster_beam`) with saturating phosphor excitation (CWE-190) and slice bounds checks (CWE-125).
  - `VectorStroke`: Vector stroke definition (`x0`, `y0`, `x1`, `y1`, `intensity`, `dwell_ticks`) supporting lines and bright-spot dwell points.
  - `VectorBeamPoint`: Vector deflection state point (`x`, `y`, `blanked`, `intensity`, `settled`).
  - `VectorBeamDisplay`: Oscilloscope / vector beam deflection simulation with bounded slewing rate, Z-axis blanking modulation, zero-allocation stroke rasterization (`rasterize_vector_stroke_into`), and `PaintScene` lowering.
- Verification and Assurance:
  - `VER07` zero-heap-allocation microbenchmarks (`tests/microbenchmarks.rs`) using `CountingAllocator` and sabotage detection.
  - `VER08b` runtime capability confinement tests (`tests/runtime_capability.rs`).
  - Zero ambient capability declaration in `required_capabilities.json` (`capabilities: []`) verified by `ca-capability-analyzer`.
  - Proved effect certificate (`effect_certificate.json`).
  - Full 26-CWE weakness closure matrix documented in `assurance_report.json` per `VER15`.
