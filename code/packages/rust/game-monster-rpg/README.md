# game-monster-rpg

Complete Playable GBA/Pokémon-Style RPG Vertical Slice (`GAME25`).

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME00` §3 Era 2).

---

## Architectural Purpose

`game-monster-rpg` is the culmination of the Era 2 2D tile and turn-based engine track. It synthesizes all Era 2 platform components into a unified, playable monster-collecting RPG vertical slice:
- `tilemap2d` (`GAME21`): Multi-layer scrolling background engine, tile-attribute collisions, and camera viewport culling.
- `sprite-oam` (`GAME22`): Hardware-accurate sprite attribute table (128 OAM entries) and metasprite animations.
- `rpg-grid-world` (`GAME23`): Grid-aligned actor movement FSM, multi-map warp triggers, NPC interactions, step encounters, and typewriter dialogue boxes.
- `rpg-battle-engine` (`GAME24`): Turn-based creature battle state machine, 15-elemental type chart, stat stages, damage formulas, capture, fleeing, and XP progression.
- `palette-indexed` (`GAME20`): Indexed color formats and smooth palette fade transitions (`fade_palette_into`).
- `arcade-sfx` (`GAME12`) & `audio-mixer-realtime` (`MUS08`): Deterministic chiptune sound effects, musical cues, and real-time PCM ring buffer mixing.
- `game-clock` (`GAME10`): Fixed-timestep accumulator simulation loop (`step_rpg_with_clock`).
- `paint-batch-tiles` (`P2D09`) & `paint-instructions` (`P2D00`): Lowering tilemap layers, dialogue cells, and OAM sprites to universal `PaintScene` IR and raster blits into `PixelContainer`.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. Contains zero filesystem (`fs`), network (`net`), process (`proc`), environment (`env`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) handles.
- **`CAP002` (Safe Rust):** `#![forbid(unsafe_code)]`. Contains zero unsafe blocks and no FFI bindings.
- **Zero Heap Allocation on Hot Paths (`VER07`):** Physics/world stepping (`step_rpg_frame`), battle turn execution, OAM sprite lowering, dialogue cell rendering, and audio mixing operate with zero heap allocation.
- **Deterministic Simulation:** All overworld step encounters, damage rolls, and battle AI decisions derive from seeded deterministic PRNG states (`GridRng`, `BattleRng`).
- **26-CWE Matrix Closure (`VER15`):** Complete formal verification and exclusion matrix documented in `assurance_report.json`.

---

## Core API & Usage

```rust
use game_monster_rpg::{
    default_rpg_config, init_rpg_game, step_rpg_frame, render_rpg_audio_into,
    lower_rpg_to_paint_scene, RpgInput, RpgState,
};
use arcade_sfx::SfxBank;
use audio_mixer_realtime::PcmRingBuffer;

let config = default_rpg_config();
let mut game = init_rpg_game(config, 0x1234_5678).expect("valid game");
let mut sfx_bank = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

let input = RpgInput::walk_down();
let events = step_rpg_frame(&mut game, &input, &mut sfx_bank).expect("simulation step");

// Render scene to universal PaintScene IR
let scene = lower_rpg_to_paint_scene(&game).expect("scene lowering");

// Render audio into ring buffer
let mut ring = PcmRingBuffer::<256>::empty();
let mut scratch = [0i16; 64];
let samples = render_rpg_audio_into(&mut sfx_bank, &mut ring, &mut scratch).expect("audio rendering");
```
