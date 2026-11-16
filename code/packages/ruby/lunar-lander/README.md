# Lunar Lander Simulation Core (Ruby)

`coding_adventures_lunar_lander` is a pure, deterministic, platform-independent simulation core and ASCII telemetry generator for the classic discrete lunar landing flight simulation (**GAME02**), implemented in pure Ruby.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection.

---

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by Apollo 11, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage. Players entered burn rates per 10-second interval, attempting to gently touch down before exhausting fuel.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), becoming an enduring computing classic across microcomputer platforms.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **Strict Zero Ambient Authority (`CAP001`):** `"capabilities": []` declared in `required_capabilities.json`. The package contains zero filesystem, network, subprocess, environment, or system-clock access.
- **Pure Functional State Transitions:** `step_burn(state, cmd) -> LanderState` advances flight kinematics without mutating the input state.
- **Fresh Immutable Value Records (`VER04m`):** All returned `LanderState`, `BurnCommand`, `FlightStatus`, and `PaintScene` objects are `.freeze`d to ensure immutable value semantics across callers and threads.
- **Bit-Exact Cross-Language Determinism:** All orbital kinematics and state transitions use exclusively integer-scaled fixed-point coordinates (`i64`/`u32`/`u128`) with zero floating-point operations in the simulation core:
  - Altitude ($Y$): Millifeet ($1\text{ ft} = 1,000\text{ mft}$).
  - Velocity ($V$): Millifeet per second ($1\text{ ft/s} = 1,000\text{ mft/s}$).
  - Acceleration ($A$): Millifeet per second squared ($\text{mft/s}^2$).
  - Fuel Mass ($F$): Pounds ($\text{lbs}$).
  - Burn Rate ($B$): Pounds per second ($\text{lbs/s}$).
- **Exact Division & Square Root Primitives:**
  - `trunc_div(a, b)`: Truncating integer division toward zero, matching C/Rust `/` semantics across positive and negative operands.
  - `div_euclid(a, b)`: Euclidean integer division producing a strictly non-negative remainder $r \in [0, |b|)$. Satisfies `div_euclid(-46070, -5310) == 9`.
  - `isqrt_u128(n)`: Iterative 64-step bitwise integer square root $\lfloor \sqrt{n} \rfloor$ with zero division operations (`CWE-369` immunization).
- **Declarative 2D Telemetry & ASCII HUD (`P2D02`):** Generates structured intermediate `PaintScene` (`76x5`) rendered to a 5-line by 76-character terminal instrument dashboard with proportional Unicode bar gauges (`█`/`░`) and flight vector indicators (`▼`/`▲`/`◆`/`★`).
- **Strict CWE/CVE Hardening (`VER15`):**
  - **CWE-843 (Type Confusion):** Strict `is_a?(Integer)` guards on all numeric inputs, rejecting floats, strings, booleans, and nil.
  - **CWE-190 (Integer Overflow):** State coordinate boundary enforcement ($Y \in [0, 200,000,000]$, $V \in [-10,000,000, 10,000,000]$, $T \le 86,400$).
  - **CWE-369 (Division by Zero):** Complete elimination of zero divisors in thrust calculations and touchdown solver.
  - **CWE-502 / CWE-94:** Zero `Marshal.load`, `YAML.load`, or `eval`.

---

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_lunar_lander"
```

Or install locally:

```bash
gem install coding_adventures_lunar_lander
```

---

## Quick Start

```ruby
require "coding_adventures_lunar_lander"

# 1. Initialize a new Apollo Lunar Module descent flight
state = CodingAdventures::LunarLander.new_flight
puts CodingAdventures::LunarLander.render_telemetry(state)

# 2. Issue pilot burn orders: 50 lbs/s for 10 seconds
cmd = CodingAdventures::LunarLander::BurnCommand.new(burn_rate_lbs_s: 50, duration_seconds: 10)

# 3. Validate and integrate kinematics
CodingAdventures::LunarLander.validate_burn(state, cmd)
next_state = CodingAdventures::LunarLander.step_burn(state, cmd)

# 4. Render updated ASCII telemetry dashboard
puts CodingAdventures::LunarLander.render_telemetry_ascii(next_state, 50)
```

---

## Physical Constants & Apollo LM Parameters

| Parameter | Symbol | Value | Unit |
|---|---|---|---|
| Lunar Surface Gravity | $g_{\text{lunar}}$ | $5,310$ | $\text{millifeet/s}^2$ ($5.31\text{ ft/s}^2$) |
| LM Dry Mass | $M_{\text{dry}}$ | $16,500$ | $\text{lbs}$ |
| Initial Propellant | $F_0$ | $16,500$ | $\text{lbs}$ |
| Total Initial Mass | $M_0$ | $33,000$ | $\text{lbs}$ |
| Initial Altitude | $Y_0$ | $120,000,000$ | $\text{millifeet}$ ($120,000\text{ ft}$) |
| Initial Velocity | $V_0$ | $-3,000,000$ | $\text{millifeet/s}$ ($-3,000\text{ ft/s}$) |
| Exhaust Velocity | $V_{\text{ex}}$ | $8,000,000$ | $\text{millifeet/s}$ ($8,000\text{ ft/s}$) |
| Max Burn Rate | $B_{\text{max}}$ | $200$ | $\text{lbs/s}$ |
| Turn Interval | $\Delta t$ | $1..=10$ | $\text{seconds}$ |

---

## Running Tests

```bash
bundle exec rake test
```
