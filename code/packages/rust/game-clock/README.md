# game-clock

The Rust package's conformance tests consume [`GAME10`'s shared v1 vectors](../../../specs/fixtures/game-clock-v1/README.md) through the public API. The corpus covers 23 finite examples and is not an exhaustive proof of the clock contract.

Deterministic integer-nanosecond fixed-timestep accumulator and frame pacer (`GAME10`).

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME00` §3 Era 1).

---

## Architectural Purpose

In real-time arcade and physics simulations, variable frame deltas cause non-deterministic physics, collision tunneling, and floating-point drift. Following Glenn Fiedler's seminal "Fix Your Timestep!" architecture, `game-clock` cleanly decouples the wall-clock rendering frame rate from deterministic physics simulation ticks.

To achieve bit-exact reproducibility across CPU architectures and operating systems without drift over millions of frames, all calculations use **exact 64-bit unsigned integer nanoseconds (`u64`)**:
- $1\text{ second} = 1{,}000{,}000{,}000\text{ ns}$.
- Standard 60 Hz arcade loop: $16{,}666{,}667\text{ ns} \approx 16.666667\text{ ms}$.
- Standard 120 Hz high-refresh loop: $8{,}333{,}333\text{ ns} \approx 8.333333\text{ ms}$.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. Contains zero filesystem (`fs`), network (`net`), process (`proc`), environment (`env`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) handles.
- **`CAP002` (Safe Rust):** `#![forbid(unsafe_code)]`. Contains zero unsafe blocks and no FFI bindings.
- **Zero Runtime Dependencies:** Pure Rust standard library only (`[dependencies]` is empty).
- **Zero Heap Allocation (`VER07`):** All core structures (`GameClockConfig`, `GameClockState`, `ClockStepResult`, `ClockError`) are compact, stack-only `Copy` types. Zero heap allocations on all paths (`max_alloc_bytes: Some(0)`).
- **Exact Nanosecond Conservation Law:**
  $$\text{elapsed\_sim\_ns} + \text{accumulator\_ns} + \text{total\_dropped\_ns} = \sum \text{raw\_frame\_delta\_ns}$$
  Every single nanosecond is strictly accounted for without loss or floating-point cancellation.
- **Spiral of Death Mitigation (`CWE-400`):**
  - Raw frame deltas exceeding `max_frame_delta_ns` (default $250\text{ ms}$) are capped, and the excess is recorded as dropped.
  - If a frame backlog accumulates more than `max_frame_steps` (default 8 steps), the step count is clamped to `max_frame_steps`, `spiral_of_death_clamped` is set to `true`, and the excess backlog is drained so the clock recovers immediately on the next frame.
- **Divide-by-Zero Immunity (`CWE-369`):** Rejects `fixed_dt_ns == 0` and `from_hz(0, ...)` with `Err(ClockError::ZeroFixedTimestep)`.
- **Integer Overflow Immunity (`CWE-190`):** All counters (`tick_count`, `frame_count`, `elapsed_sim_ns`, `elapsed_wall_ns`, `total_dropped_ns`) use checked arithmetic returning `Err(ClockError::TimestampOverflow)`.
- **Assurance status (`VER15`):** The checked-in `assurance_report.json` is report-only; it does not establish 26-CWE closure. Its seven claims have 1,000-case property entries without linked evidence, null exhaustive/fuzz/proof fields, and two unapproved exemptions. See `GAME-BACKLOG.md` items `GBL-003`, `GBL-004`, and `GBL-007` for the remaining verification work.
- **Constructor model execution (`GBL-003a`):** A Rust integration test checks the pure Warrant ground model against the exported `new_clock` API over 648 boundary configurations and seeded, stratified property inputs. This finite execution is not a universal proof, does not prove the Warrant model is equivalent for every `GameClockConfig`, and does not cover `validate_state` or `advance_frame`.
- **Validator model execution (`GBL-003b`, finite evidence):** `tests/warrant_validate_state.rs` parses and typechecks the exact Warrant claim, evaluates its checked-multiplication model, and compares complete results with public `validate_state` and an independent `u128` oracle. It includes 14 branch vectors, 35 threshold vectors, 2,048 seeded property inputs, and separate 1,024-iteration typed-state and 2,048-iteration malformed-transport VER06 campaigns. Each fuzz callback checks native Warrant, API, and oracle results; the targets report seed callbacks separately. These are finite native executions, not a universal proof or Rust-source equivalence claim. The canonical assurance report remains report-only with null proof/exhaustive slots: this package does not yet have the existing test/proof manifest inputs needed to rederive those report fields from this custom adapter harness.
- **Manifest integration follow-up (`VER-P046`):** The current generated binding schema has no host `unit` type, while `Unit` in this Warrant document aliases `Bool` and Rust `validate_state` returns `Result<(), ClockError>`. The custom test adapter normalizes `Ok(())` to `Ok(true)` for the evaluator; it does not create manifest-backed evidence, so the report above remains unchanged and report-only.

---

## Core API & Usage

```rust
use game_clock::{advance_frame, new_clock, GameClockConfig, GameClockState};

// 1. Initialize clock for a 60 Hz simulation
let config = GameClockConfig::standard_60hz();
let mut clock = new_clock(config).expect("valid clock config");

// 2. Advance frame in your main render loop with elapsed wall-clock nanoseconds
let raw_delta_ns = 16_666_667; // e.g. from high-precision monotonic timer
let step_result = clock.advance(raw_delta_ns).expect("advance succeeded");

// 3. Run the exact number of fixed simulation steps needed
for _ in 0..step_result.steps_to_run {
    // physics_world.step(config.fixed_dt_ns);
}

// 4. Interpolate rendering between previous and current physics state
let alpha = step_result.alpha_f64(); // in [0.0, 1.0)
// renderer.render_interpolated(prev_state, curr_state, alpha);
```

---

## Sub-Tick Interpolation Factors

- `alpha_milli: u32` — integer thousandths (`0..1000`) for integer-based fixed-point renderers.
- `alpha_q16: u32` — Q16.16 fixed-point fraction (`0..65536`) for retro/arcade fixed-point pipelines.
- `alpha_f64() -> f64` — standard floating-point alpha in $[0.0, 1.0)$ for modern GPU shaders.
