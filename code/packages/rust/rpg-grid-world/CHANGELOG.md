# Changelog

All notable changes to `rpg-grid-world` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- `code/specs/rpg-grid-world.warrant` was invalid Warrant (hyphenated document name); renamed to `rpg_grid_world`. Document only.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `GAME23: rpg-grid-world`.
- `GridActor` movement FSM (`Idle`, `Walking`, `Turning`, `LedgeHop`, `Surfing`, `Frozen`) with sub-tile pixel interpolation, facing, speed / turn-delay controls, deferred freeze and ledge-hop arcs, colliding against `tilemap2d` solid / one-way / water / hazard / trigger attributes.
- `GridWorld` binding of a tile layer with warps, NPCs and encounter tables (all validated and bounded).
- Deterministic step encounters (`EncounterTable`, `EncounterState`, seeded `GridRng`) with min / max step windows, Q8 rates and weighted species / level rolls.
- `WarpPad` / `resolve_warp` door and pad resolution (`OnEnter`, `OnBump`).
- NPC queries: `find_npc_at`, `npc_blocks_tile`, `interact_target`, `npc_sees_actor`, `face_npc_toward`.
- `DialogueBox` paged typewriter state machine with choices, jump targets and cancel, lowering to `paint-batch-tiles` cells (`lower_dialogue_cells_into`) and `paint-instructions` scenes (`lower_dialogue_to_paint_scene`).
- Conformance, zero-allocation microbenchmark and runtime capability suites; effect certificate, assurance report and warrant.
