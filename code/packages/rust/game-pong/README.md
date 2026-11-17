# game-pong

Deterministic 2-player Pong arcade state machine, swept continuous collision physics, PaintScene rendering, and sound effects (`GAME13`).

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME00` §3 Era 1).

---

## Architectural Purpose

`game-pong` is a reference implementation of the iconic 1972 arcade classic *Pong* (Allan Alcorn / Atari Inc.). It integrates core platform capabilities from:
- `game-clock` (`GAME10`): Integer-nanosecond fixed-timestep simulation loop and accumulator.
- `collision2d` (`GAME11`): Continuous swept circle-AABB queries preventing tunneling at high ball velocities.
- `paint-batch-tiles` (`P2D09`): Retained-mode sprite batch construction and lowering to universal `PaintScene` IR.
- `arcade-sfx` (`GAME12`): Deterministic fixed-point sound synthesis and arcade audio presets (`PongPaddleHit`, `PongWallBounce`, `PongScore`).
- `audio-mixer-realtime` (`MUS08`): Low-latency real-time PCM ring buffer streaming.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. Contains zero filesystem (`fs`), network (`net`), process (`proc`), environment (`env`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) handles.
- **`CAP002` (Safe Rust):** `#![forbid(unsafe_code)]`. Contains zero unsafe blocks and no FFI bindings.
- **Zero Heap Allocation Physics & Sprites (`VER07`):** Physics stepping (`step_pong`), sprite buffer emission (`build_pong_sprites`), and audio streaming (`render_pong_audio_into`) operate entirely on stack and pre-allocated slices without heap allocation.
- **Continuous Swept Collision (Anti-Tunneling):** Swept circle-AABB geometry detects paddle hits even when the ball's per-frame displacement exceeds paddle thickness.
- **Deterministic Deflection Physics:** Deflection angles vary continuously based on the relative contact point along the paddle height, with speed increments capping at `ball_max_speed`.
- **Deterministic Pseudo-Random Angle Serve:** Pure LFSR/linear congruential PRNG state ensures repeatable ball serves without ambient system randomness.
- **26-CWE Matrix Closure (`VER15`):** Complete formal verification and exclusion matrix documented in `assurance_report.json`.

---

## Core API & Usage

```rust
use game_pong::{
    default_config, new_game, step_pong, build_pong_sprites, lower_pong_to_paint_scene,
    PongConfig, PongInput, PongState, PaddleAction, SpriteItem,
};
use arcade_sfx::SfxBank;

let config = default_config();
let mut state = new_game(config, 0x1234_5678).expect("valid game");
let mut sfx_bank = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

let input = PongInput::player_vs_ai(PaddleAction::Up);
let events = step_pong(&mut state, &input, &mut sfx_bank).expect("simulation step");

// Render to sprites (zero-alloc)
let mut sprites = [SpriteItem::new(0, 0.0, 0.0, 0.0, 0.0); 64];
let count = build_pong_sprites(&state, &mut sprites);

// Or lower to universal PaintScene
let scene = lower_pong_to_paint_scene(&state).expect("scene generation");
```
