# lunar-lander (Perl)

Pure, deterministic, platform-independent simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME02`).

---

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by Apollo 11, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage. Players entered burn rates per 10-second interval, attempting to gently touch down before exhausting fuel.
- **1970–1973 — DEC BASIC & "101 BASIC Computer Games":** Ported to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), spawning dozens of graphical and text-based lunar lander adaptations across every personal computing platform.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **Pure State Transition:** State transitions follow a pure mathematical function:
  ```perl
  my $next_state = step_burn($state, $cmd);
  ```
- **Bit-Exact Cross-Language Determinism:** All kinematics and state transitions use exclusively integer-scaled fixed-point coordinates (`i64`/`u32`/`u128`) with zero floating-point operations in the simulation core:
  - Altitude ($Y$): Millifeet ($1\text{ ft} = 1,000\text{ mft}$).
  - Velocity ($V$): Millifeet per second ($1\text{ ft/s} = 1,000\text{ mft/s}$).
  - Acceleration ($A$): Millifeet per second squared ($\text{mft/s}^2$).
  - Fuel Mass ($F$): Pounds ($\text{lbs}$).
- **Exact Euclidean Integer Division (`div_euclid`):** Ensures a non-negative remainder $0 \le r < |b|$, matching the Rust standard library `div_euclid` and guarantees bit-exact landing time solutions.
- **Declarative 2D Telemetry:** Presentation bridge translates flight state into a declarative `PaintScene` and renders a 5x76 character-indexed Unicode telemetry dashboard (`to_paint_scene`, `render_telemetry_ascii`).
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes, including `CWE-843` (strict integer validation), `CWE-190` (overflow bounds), `CWE-369` (division by zero), and `CWE-681` (numeric truncation).

---

## Domain Types & API

### 1. `LanderState`

Hash reference capturing the complete flight state:
- `time_seconds`: Elapsed flight time in seconds (0..=86400).
- `altitude_mft`: Altitude above lunar surface in millifeet (0..=200,000,000).
- `velocity_mft_s`: Vertical velocity in millifeet/s (-10,000,000..=+10,000,000).
- `fuel_lbs`: Usable fuel remaining in pounds (0..=16,500).
- `dry_mass_lbs`: Apollo LM dry mass (16,500 lbs).
- `status`: Lifecycle status (`'InFlight'` or `'Landed(<Outcome>)'`).
- `impact_velocity_mft_s`: Terminal touchdown impact velocity, or `undef` while in flight.

### 2. `BurnCommand`

Pilot burn command for a discrete turn:
- `burn_rate_lbs_s`: Fuel consumption rate (0..=200 lbs/s).
- `duration_seconds`: Burn duration (1..=10 seconds).

### 3. Core Functions

- `new_flight()`: Initializes a new flight at 120,000 ft with 16,500 lbs fuel.
- `validate_burn($state, $cmd)`: Validates lifecycle, state invariants, and command bounds. Dies with `CodingAdventures::LunarLander::LanderError` on violation.
- `check_burn($state, $cmd)`: Non-throwing validation returning `($ok, $err)`.
- `step_burn($state, $cmd)`: Advances flight state through numerical integration, solving for ground touchdown when $Y \le 0$.
- `to_paint_scene($state, $last_burn_rate)`: Generates declarative 76x5 `PaintScene` IR.
- `render_telemetry_ascii($state, $last_burn_rate)`: Renders 5x76 character-grid Unicode HUD.
