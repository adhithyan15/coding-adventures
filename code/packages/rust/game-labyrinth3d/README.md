# game-labyrinth3d

**GAME34**: Pure Tier 1 deterministic, zero-allocation (steady-state) 2.5D/3D first-person dungeon crawler simulation core with dual software renderers outputting into caller-owned `PixelContainer` (`IMG00`).

## Architecture

* **Zero ambient authority (`"capabilities": []`)**: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, zero filesystem/network/clock/env/process authority, and zero `std` `libm` calls (all trigonometric and square-root operations use `trig` `PHY00`).
* **Deterministic state machine (`step_in_place` / `step`)**:
  * Fixed-timestep stepping via `game-clock` (`GameClockConfig` / `GameClockState`).
  * Explicit deterministic randomness inside `LabyrinthState` via `LabyrinthRng` (`rng::Pcg32`).
  * Bounded tile grid (`MAX_GRID_DIM = 64`), walls, start/exit tiles, locked doors (`DoorState`), collectible keys/health/treasure (`PickupItem`), and patrolling/LOS-chasing entities (`EntityState`).
  * Circle-vs-grid sliding collision (preventing wall clipping and corner snagging) plus win/loss state transitions (`GameStatus::Playing`, `Won`, `Lost`).
* **Dual software renderers (`render_frame_into`)**:
  * `RenderMode::Raycast25D` via `raycast-dda` (`GAME31`).
  * `RenderMode::Software3D` via `raster3d-soft` (`GAME33`) + `math3d` (`GAME30`) using caller-owned Z-buffers and pre-allocated `LabyrinthRenderScratch`.
* **Canonical digests**: `frame_digest_fnv1a` and `state_digest_fnv1a` (canonicalizing `-0.0` to `+0.0`).

## Verification

```bash
cargo fmt -p game-labyrinth3d -- --check
cargo clippy -p game-labyrinth3d --all-targets -- -D warnings
cargo test -p game-labyrinth3d -- --nocapture
cargo test -p game-labyrinth3d --release -- --nocapture
cargo test -p coding_adventures_warrant_grammar --test v0_compat
```
