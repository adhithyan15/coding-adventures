# rpg-grid-world

Deterministic, zero-allocation overworld RPG core for Rust: tile-aligned grid movement FSM, ledge hops and surfing, step encounter counters, warp pads and doors, NPC / sign interaction queries and a paged typewriter dialogue box lowered into `paint-batch-tiles`.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME23`).

---

## Overview

Pokemon-style overworlds move on a tile grid: the player is always on a tile or interpolating between two, tall grass counts steps until a wild encounter, doors warp to other maps and NPCs talk when faced. `rpg-grid-world` models that on top of `tilemap2d` (`GAME21`):

1. **Movement FSM** (`GridActor`, `MoveState::{Idle, Walking, Turning, LedgeHop, Surfing, Frozen}`): sub-tile pixel interpolation (`pixel_position`), directional facing (`Facing`), configurable speed and turn delay, held input chains walks without an idle frame. Collision reads the `tilemap2d` attribute bits: `ATTR_SOLID` blocks, `ATTR_ONE_WAY` tiles are ledges hopped two tiles in one direction (`GridWorld::ledge_facing`), `ATTR_WATER` needs surfing (`can_surf`), `ATTR_HAZARD` and `ATTR_TRIGGER` are reported on arrival. The map edge always blocks, whatever the layer's wrap mode.
2. **Step encounters** (`EncounterTable`, `EncounterState`): every step on a tile whose id matches a table counts; after `min_steps` a per-step chance (`rate_q8`) rolls, at `max_steps` an encounter is guaranteed. Weighted entry and level selection use a seeded xorshift64* (`GridRng`), so a seed replays exactly.
3. **Warps** (`WarpPad`, `resolve_warp`): `WarpTrigger::OnEnter` pads and `WarpTrigger::OnBump(Facing)` doors resolve to a `WarpTarget` (map id, tile, facing).
4. **NPCs** (`Npc`, `interact_target`, `npc_sees_actor`, `face_npc_toward`): blocking, interaction reach over counters, trainer line of sight stopped by solid tiles, people turning to face the player.
5. **Dialogue** (`DialogueBox`): printable-ASCII pages with a Q8 typewriter reveal, skip-on-confirm, choices with optional per-choice jump targets, and cancel. `lower_dialogue_cells_into` writes frame, revealed glyphs, choices and cursor as `paint-batch-tiles` (`P2D09`) cells (one per grid position); `lower_dialogue_to_paint_scene` builds a `paint-instructions` (`P2D00`) scene.

```rust
use rpg_grid_world::*;

let cells = vec![TileCell::tile(1); 4 * 4];
let layer = TileLayer::attach(&cells, 4, 4, 16, 16).unwrap();
let world = GridWorld::bind(&layer, &[], &[], &[]).unwrap();
let mut enc = EncounterState::start(42);
let mut hero = GridActor::place(&layer, 1, 1, Facing::Right).unwrap();
hero.set_speed(4).unwrap();
let report = hero.advance(&world, Some(Facing::Right), &mut enc).unwrap();
assert!(report.started);
```

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** movement ticks, encounter steps, warp / NPC / sight queries, dialogue ticks and dialogue cell lowering never touch the heap (verified with the counting allocator in `tests/microbenchmarks.rs`). Only `lower_dialogue_to_paint_scene` builds an owned scene.
- **Strict Validation:** CWE-20 (table contents, ASCII-only text, ranges), CWE-125/787 (no unchecked indexing; the lowering validates buffer length before the first write), CWE-190 (tile arithmetic in widened `i64`, checked `u16` grid positions), CWE-369 (the PRNG reduces with multiply-shift, never division), CWE-400 (`MAX_WARPS = 256`, `MAX_NPCS = 256`, `MAX_ENCOUNTER_TABLES = 8`, `MAX_ENCOUNTER_ENTRIES = 32`, `MAX_INTERACT_RANGE = 4`, `MAX_SIGHT_RANGE = 16`, `MAX_PAGES = 64`, `MAX_LINES = 4`, `MAX_CHOICES = 4`, `MAX_LINE_LEN = 32`, `MAX_DIALOGUE_CELLS = 4096`).
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Dependencies

- `tilemap2d` (`GAME21`): tile layers and attribute bits.
- `paint-batch-tiles` (`P2D09`) and, through it, `paint-instructions` (`P2D00`): lowering targets.

## Testing

```bash
cargo test -p rpg-grid-world -- --nocapture
cargo test -p rpg-grid-world --release -- --nocapture
```

`tests/conformance.rs` covers the FSM, collision, warps, NPCs, encounters, dialogue and lowering; `tests/microbenchmarks.rs` is the zero-allocation TAP suite; `tests/runtime_capability.rs` is the VER08b runtime gate (live on Linux, `NOT-LIVE` elsewhere).
