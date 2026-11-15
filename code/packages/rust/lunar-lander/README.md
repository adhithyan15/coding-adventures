# lunar-lander

Pure, deterministic, platform-independent simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME02`).

---

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by the Apollo 11 moon landing in July 1969, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage. Players entered burn rates per 10-second interval, attempting to gently touch down before exhausting fuel.
- **1970–1973 — DEC BASIC & "101 BASIC Computer Games":** Ported to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), spawning dozens of graphical and text-based lunar lander adaptations across every personal computing platform.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Pure State Transition:** State transitions follow a pure mathematical function:
  ```rust
  step_burn(state: &LanderState, cmd: &BurnCommand) -> Result<LanderState, LanderError>
  ```
- **Bit-Exact Cross-Language Determinism:** All orbital kinematics and state transitions use exclusively integer-scaled fixed-point coordinates (`i64`/`u32`/`u128`) with zero `f32`/`f64` floating point operations in the simulation core:
  - Altitude ($Y$): Millifeet ($1\text{ ft} = 1,000\text{ mft}$).
  - Velocity ($V$): Millifeet per second ($1\text{ ft/s} = 1,000\text{ mft/s}$).
  - Acceleration ($A$): Millifeet per second squared ($\text{mft/s}^2$).
  - Fuel Mass ($F$): Pounds ($\text{lbs}$).
- **Zero Allocation Budget (`VER07`):** Core functions `new_flight`, `validate_burn`, `step_burn`, and `isqrt_u128` operate purely on value records and perform **zero heap allocations** on the simulation path (`max_alloc_bytes: Some(0)`).
- **Declarative 2D Telemetry (`P2D02`):** Presentation bridge translates flight state into a declarative `PaintScene` rendered through `paint-vm-ascii`.
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

---

## Domain Types & API

### 1. `LanderState`

Immutable value record capturing the complete flight state:

```rust
pub struct LanderState {
    pub time_seconds: u32,
    pub altitude_mft: i64,
    pub velocity_mft_s: i64,
    pub fuel_lbs: u32,
    pub dry_mass_lbs: u32,
    pub status: FlightStatus,
    pub impact_velocity_mft_s: Option<i64>,
}
```

### 2. `BurnCommand`

Pilot burn command for a discrete turn:

```rust
pub struct BurnCommand {
    pub burn_rate_lbs_s: u32,
    pub duration_seconds: u32,
}
```

### 3. `FlightStatus` & `LandingOutcome`

```rust
pub enum FlightStatus {
    InFlight,
    Landed(LandingOutcome),
}

pub enum LandingOutcome {
    Perfect,  // 0 to 5 ft/s
    Hard,     // 5.001 to 15 ft/s
    Crash,    // 15.001 to 30 ft/s
    Fatal,    // 30.001 to 60 ft/s
    Crater,   // > 60 ft/s
}
```

### 4. `LanderError`

Closed enum returned upon command rejection:

- `FlightAlreadyEnded`: Attempted burn after touchdown.
- `ExcessiveBurnRate`: Burn rate exceeds engine capacity ($B > 200\text{ lbs/s}$).
- `InvalidDuration`: Duration is 0 or exceeds turn limit ($1 \le \Delta t \le 10$).
- `ArithmeticOverflow`: Arithmetic overflow or coordinate boundary violation.
