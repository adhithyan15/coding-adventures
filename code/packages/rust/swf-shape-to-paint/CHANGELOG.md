# Changelog

All notable changes to `swf-shape-to-paint` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Enforced cumulative `MAX_STYLES` check (`existing_count.saturating_add(count) <= MAX_STYLES`) across repeated `state_new_styles` records in `read_style_count`, and used `checked_add` for style offset indexing (CWE-400).
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering cumulative style-change bombs, `i32::MIN`/`MAX` twips overflow, truncated/corrupt `DefineShape` 1–4 payloads, and a 1,000-case fuzz campaign.

## [0.1.0] - 2026-11-16

### Added
- Initial implementation of `swf-shape-to-paint` (`SWF02`).
- Safe binary parser for SWF `DefineShape`, `DefineShape2`, `DefineShape3`, and `DefineShape4` records.
- Support for `StyleChangeRecord`, `StraightEdgeRecord`, `CurvedEdgeRecord`, and `EndShapeRecord`.
- Solid color fills, linear/radial/focal gradient fills, bitmap fills, and line styles (`LINESTYLE` and `LINESTYLE2`).
- Coordinate conversion from TWIPS (1/20 px) to floating point pixel units.
- Dual-fill edge loop reconstruction (`FillStyle0` / `FillStyle1`) resolving closed directed contours and island cutouts with `FillRule::EvenOdd` and `FillRule::NonZero`.
- Lowering to `paint-instructions` (`P2D00`) using `bezier2d` (`G2D02`) for quadratic Bézier curves.
- Zero-ambient-authority capability cage (`CAP001`) and zero-unsafe guarantee (`CAP002`).
- 26-CWE weakness matrix coverage (`VER15`), conformance test suite, microbenchmarks with `CountingAllocator`, and runtime capability test.
