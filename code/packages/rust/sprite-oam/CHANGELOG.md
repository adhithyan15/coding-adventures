# Changelog

All notable changes to `sprite-oam` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `GAME22: sprite-oam`.
- `OamTable` (128 slots) and `OamEntry` with GBA/NES style attributes: position, `SpriteShape` x `SpriteSize`, H/V flip, palette bank, priority 0..=3, affine slot, tile index, hidden / affine / double-size flags.
- 32 `AffineParams` matrix groups in 8.8 fixed point with `Affine2D` (`G2D01`) conversion and `texel_at` flip / affine sampling.
- Deterministic zero-allocation priority sort (`draw_order_into`), scanline sprite-limit evaluation (`scanline_sprites_into`) and whole-frame overflow statistics (`scanline_stats`).
- `MetaFrame`, `Animation` and `Animator` metasprite sequencer with one-shot, loop and ping-pong modes and deterministic, split-invariant ticks; `OamTable::stamp_frame` with H/V mirroring.
- Lowering to `paint-batch-tiles` (`P2D09`) `SpriteQuad`s (`lower_to_quads_into`, palette banks resolved via `palette-indexed` `GAME20`) and to `paint-instructions` (`P2D00`) scenes (`lower_to_paint_scene`).
- Security controls: CWE-20/125/190/369/400/787 guards, safe Rust only (`#![forbid(unsafe_code)]`), zero ambient capabilities (`CAP001`).
- Conformance suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `sprite-oam.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`.
