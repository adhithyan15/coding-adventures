# Changelog

All notable changes to `game-breakout` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `game-breakout` (`GAME14`), deterministic arcade Breakout state machine.
- `BreakoutConfig` & `BreakoutState` with arena dimensions, paddle bounds, ball position/velocity, fixed-size brick grid (`[BrickSlot; 112]`), score, combo counter, lives, and level progression.
- `step_breakout` physics step using `collision2d` swept circle-AABB continuous collision detection (anti-tunneling).
- Integration with `arcade-sfx` presets (`PongPaddleHit`, `PongWallBounce`, `BreakoutBrickHit`, `BreakoutLoseLife`).
- `step_breakout_with_clock` fixed-timestep loop helper integrating `game-clock`.
- Zero-allocation sprite generation `build_breakout_sprites` emitting paddle, ball, active bricks with row colors, score, and lives HUD.
- `lower_breakout_to_paint_scene` lowering to universal `PaintScene` IR using `paint_batch_tiles`.
- Real-time audio rendering helper `render_breakout_audio_into` streaming into `audio-mixer-realtime::PcmRingBuffer`.
- Comprehensive 26-CWE matrix closure (`assurance_report.json`), `assurance-profile.json`, and `effect_certificate.json`.
- Conformance test suite (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability conformance gate (`tests/runtime_capability.rs`).
