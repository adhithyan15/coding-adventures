# Changelog

All notable changes to `game-pong` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- Parked `code/specs/game-pong.warrant` as `game-pong.warrant.draft`: it uses `F64` (not implemented in Warrant yet) and had a `labels` syntax error. Restore after VER04p floats (WX-FL1, backlog WX-FL2). No code change.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `game-pong` (`GAME13`), deterministic 2-player arcade Pong state machine.
- `PongConfig` & `PongState` with court dimensions, paddle bounds, ball position/velocity, scores, serve countdown, and rally counter.
- `step_pong` physics step using `collision2d` swept circle-AABB continuous collision detection (anti-tunneling).
- Integration with `arcade-sfx` presets (`PongPaddleHit`, `PongWallBounce`, `PongScore`).
- `step_pong_with_clock` fixed-timestep loop helper integrating `game-clock`.
- Zero-allocation sprite generation `build_pong_sprites` emitting court net dashes, paddles, ball, and 7-segment score digits.
- `lower_pong_to_paint_scene` lowering to universal `PaintScene` IR using `paint_batch_tiles`.
- Real-time audio rendering helper `render_pong_audio_into` streaming into `audio-mixer-realtime::PcmRingBuffer`.
- Comprehensive 26-CWE matrix closure (`assurance_report.json`), `assurance-profile.json`, and `effect_certificate.json`.
- Conformance test suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability conformance gate (`tests/runtime_capability.rs`).
