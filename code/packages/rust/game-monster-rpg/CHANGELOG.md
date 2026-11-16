# Changelog

All notable changes to `game-monster-rpg` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-16

### Added
- Initial implementation of `game-monster-rpg` (`GAME25`), the complete playable GBA/Pokémon-style RPG vertical slice.
- Multi-map overworld integrating `tilemap2d` and `rpg-grid-world` with 5 distinct maps (Town, Route 1, Tall Grass Meadow, Healing Center, and Gym).
- Overworld features: warp pads with map transitions, NPC dialogue trees, Pokémon Center full-party healing, and tall grass step encounters.
- Battle engine integration with `rpg-battle-engine`: wild and trainer battles, moves with STAB and 15-type chart effectiveness, stat stages, capture calculations, fleeing, switching, and XP/level progression.
- Palette fade transitions using `palette-indexed` (`fade_palette_into`) between overworld and battle scenes.
- Zero-allocation rendering lowering OAM sprites, tilemaps, HP bar HUDs, and dialogue boxes to `PaintScene` and `PixelContainer`.
- Deterministic real-time chiptune audio cues (Overworld BGM, Battle BGM, bump, select, hit, level up, faint, flee) mixed into `audio-mixer-realtime::PcmRingBuffer` via `arcade-sfx`.
- Fixed-timestep clock loop integration with `game-clock` (`step_rpg_with_clock`).
- Zero ambient capabilities (`"capabilities": []`) with `#![forbid(unsafe_code)]`.
- Full 26-CWE matrix closure in `assurance_report.json`, `assurance-profile.json`, and formal verification warrant in `code/specs/game-monster-rpg.warrant`.
- Comprehensive conformance tests (`tests/conformance.rs`), zero-allocation microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability conformance (`tests/runtime_capability.rs`).
