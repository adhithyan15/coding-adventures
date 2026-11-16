# GAME02 — Lunar Lander: Fixed-Point Discrete Flight Simulation & ASCII Telemetry

## Overview & Historical Context

The `lunar-lander` package provides a pure, deterministic, platform-independent simulation core for the classic discrete lunar landing flight simulation.

### Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by the Apollo 11 moon landing in July 1969, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage. Players entered burn rates per 10-second interval, attempting to gently touch down before exhausting fuel.
- **1970–1973 — DEC BASIC & "101 BASIC Computer Games":** Ported to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), spawning dozens of graphical and text-based lunar lander adaptations across every personal computing platform.

### The Problem: Cross-Platform Floating-Point Drift

Interactive flight simulations and orbital mechanics traditionally rely on floating-point integration. However, IEEE-754 floating-point arithmetic is inherently non-deterministic across heterogeneous computing platforms:

```text
Host CPU (x87 80-bit vs SSE2 64-bit vs ARM NEON FMA)
Compiler Optimizations (-O2, -O3, fused-multiply-add contraction)
Language Runtimes (JS 53-bit Number, Python arbitrary float, Java strictfp)
                               │
                               ▼
        Subtle divergence in rounding at turn 12
                               │
                               ▼
        Catastrophic desynchronization in multi-turn traces!
```

In a verified polyglot repository targeting 18 implementation lanes, bit-exact replayability is mandatory (`VER04i`, `VER27`). Floating-point differences would break cross-language test vectors, sabotage proofs, and Warrant conformance.

### The Solution: Integer-Scaled Fixed-Point Physics

`lunar-lander` eliminates IEEE-754 drift by conducting all orbital kinematics in **integer-scaled fixed-point units**:
- **Altitude ($Y$):** Scaled to **millifeet** ($1\text{ foot} = 1,000\text{ millifeet}$).
- **Velocity ($V$):** Scaled to **millifeet per second** ($1\text{ ft/s} = 1,000\text{ millifeet/s}$).
- **Acceleration ($A$):** Scaled to **millifeet per second squared** ($\text{millifeet/s}^2$).
- **Fuel ($F$) & Mass ($M$):** Scaled to whole **pounds** ($\text{lbs}$).
- **Burn Rate ($B$):** Scaled to **pounds per second** ($\text{lbs/s}$).

Using 64-bit integer arithmetic (`i64`/`u64`), every square root, acceleration, velocity increment, and altitude integration produces the **exact same integer values** on every CPU architecture and language runtime.

Furthermore, `lunar-lander` couples its pure simulation core to the repository's 2D paint engine, generating a declarative `PaintScene` (`P2D00`) rendered to an ASCII instrument strip via `paint-vm-ascii` (`P2D02`).

---

## Package Identity & Dependency Architecture

### Metadata & Capability Registry

| Field | Value |
|---|---|
| **Package Identity** | `lunar-lander` (canonical normalized name across all 18 languages) |
| **Parity Class** | `portable` |
| **Declared Capabilities** | `[]` (strictly zero ambient capabilities in `required_capabilities.json`) |
| **Direct Dependencies** | None in core simulation; `paint-vm-ascii` (`P2D02`) in presentation bridge |
| **Transitive Dependencies**| None |
| **Verification Level** | Tier 0 / Tier 2 Warrant Verified (`state=explicit`, `VER04m`) |

### Static Capability Cage Guarantees

In accordance with `VER08a`, `VER08b`, and `Spec 13`:
- **`CAP001` (Zero Ambient Authority):** No access to system clock (`time`), filesystem (`fs`), network (`net`), processes (`proc`), terminal I/O (`stdin`/`stdout`), or multithreading (`thread`).
- **`CAP002` (Zero Unsafe / FFI):** Rust implementation forbids unsafe code (`#![forbid(unsafe_code)]`). C/C++ implementations avoid dynamic allocation and raw pointer arithmetic.
- **`CAP003` (Fail Closed):** All math constructs must be statically verifiable by `ca-capability-analyzer`.

---

## Physics Model & Fixed-Point Mathematics

### Physical Constants & Apollo LM Parameters

| Parameter | Symbol | Engineering Value | Integer-Scaled Value |
|---|---|---|---|
| **Lunar Surface Gravity** | $g_{\text{lunar}}$ | $5.31\text{ ft/s}^2$ | $5,310\text{ millifeet/s}^2$ |
| **LM Dry Mass** | $M_{\text{dry}}$ | $16,500\text{ lbs}$ | $16,500\text{ lbs}$ |
| **Initial Fuel Mass** | $F_0$ | $16,500\text{ lbs}$ | $16,500\text{ lbs}$ |
| **Total Initial Mass** | $M_0$ | $33,000\text{ lbs}$ | $33,000\text{ lbs}$ |
| **Initial Altitude** | $Y_0$ | $120,000\text{ ft}$ | $120,000,000\text{ millifeet}$ |
| **Initial Velocity** | $V_0$ | $-3,000\text{ ft/s}$ | $-3,000,000\text{ millifeet/s}$ |
| **Exhaust Velocity** | $V_{\text{ex}}$ | $8,000\text{ ft/s}$ | $8,000,000\text{ millifeet/s}$ |
| **Maximum Burn Rate** | $B_{\text{max}}$ | $200\text{ lbs/s}$ | $200\text{ lbs/s}$ |
| **Standard Turn Step** | $\Delta t$ | $1\text{ to }10\text{ seconds}$ | Integer seconds |

---

### Step Integration Equations

Given current state $(T_t, Y_t, V_t, F_t)$ and burn command $(B, \Delta t)$:

#### 1. Fuel Burn Calculation
Player commands burn rate $B$ in pounds per second over duration $\Delta t$ seconds.
- Fuel requested: $\text{fuel\_req} = B \cdot \Delta t$
- Actual fuel burned:
  $$\Delta F = \min(F_t, \text{fuel\_req})$$
- Remaining fuel:
  $$F_{t+\Delta t} = F_t - \Delta F$$

If $F_t < \text{fuel\_req}$ and $B > 0$, fuel is exhausted after fractional time:
$$\tau_{\text{burn}} = \lfloor \frac{\Delta F}{B} \rfloor$$
For the remainder of the interval $(\Delta t - \tau_{\text{burn}})$, thrust is zero.

#### 2. Mass & Thrust Acceleration
To avoid complex logarithmic series while maintaining second-order accuracy, average mass across the burn interval is used:
$$\bar{M} = M_{\text{dry}} + F_t - \lfloor \frac{\Delta F}{2} \rfloor$$

Thrust force impulse over the interval:
$$\text{Impulse} = \Delta F \cdot V_{\text{ex}} \quad (\text{lbs} \cdot \text{millifeet/s})$$

Average upward thrust acceleration during the step:
$$a_{\text{thrust}} = \lfloor \frac{\text{Impulse}}{\Delta t \cdot \bar{M}} \rfloor \quad (\text{millifeet/s}^2)$$

Net downward/upward acceleration:
$$A_{\text{net}} = a_{\text{thrust}} - g_{\text{lunar}} \quad (\text{millifeet/s}^2)$$

#### 3. Kinematic State Update (Euler-Verlet)
Using standard second-order equations of motion:
$$V_{t+\Delta t} = V_t + A_{\text{net}} \cdot \Delta t$$
$$Y_{t+\Delta t} = Y_t + V_t \cdot \Delta t + \lfloor \frac{A_{\text{net}} \cdot (\Delta t)^2}{2} \rfloor$$
$$T_{t+\Delta t} = T_t + \Delta t$$

---

### Ground Touchdown & Interpolation

If $Y_{t+\Delta t} \le 0$, the lander struck the lunar surface during the turn. The simulation must compute the exact contact moment $\delta t \in (0, \Delta t]$ and terminal impact velocity $V_{\text{touchdown}}$ at $Y = 0$.

#### Solving for Contact Time ($\delta t$)
We solve the quadratic position equation for altitude $0$:
$$Y_t + V_t \cdot \delta t + \lfloor \frac{A_{\text{net}}}{2} \rfloor \cdot (\delta t)^2 = 0$$

Let $A_{\text{half}} = \lfloor \frac{A_{\text{net}}}{2} \rfloor$.
- **Case 1: $A_{\text{half}} == 0$ (Constant velocity descent):**
  $$\delta t = \lfloor \frac{-Y_t}{V_t} \rfloor$$
- **Case 2: $A_{\text{half}} \ne 0$:**
  Using the integer discriminant:
  $$\text{disc} = V_t^2 - 4 \cdot A_{\text{half}} \cdot Y_t$$
  Since the lander is descending into the surface, $\text{disc} \ge 0$. We compute $\text{isqrt}(\text{disc})$ (pure integer square root via binary search or Babylonian method):
  $$\delta t = \lfloor \frac{-V_t - \text{isqrt}(\text{disc})}{2 \cdot A_{\text{half}}} \rfloor$$

Clamp $\delta t$ to the range $[1, \Delta t]$.

#### Terminal Values at Touchdown
- **Impact Velocity:**
  $$V_{\text{touchdown}} = V_t + A_{\text{net}} \cdot \delta t$$
- **Final Altitude:** $Y_{\text{final}} = 0$
- **Final Time:** $T_{\text{final}} = T_t + \delta t$
- **Unused Fuel Refund:** If touchdown occurred before the full turn elapsed ($\delta t < \Delta t$), unburned fuel is refunded:
  $$F_{\text{refund}} = (\Delta t - \delta t) \cdot B$$
  $$F_{\text{final}} = F_{t+\Delta t} + F_{\text{refund}}$$

---

## Domain Types & State Machine

### 1. `LanderState` (Explicit State Record)

| Field | Type | Domain | Description |
|---|---|---|---|
| `time_seconds` | `u32` | `0..=86400` | Elapsed mission flight time in seconds. |
| `altitude_mft` | `i64` | `0..=200_000_000` | Altitude above lunar surface in millifeet ($1\text{ ft} = 1000\text{ mft}$). |
| `velocity_mft_s` | `i64` | `-10_000_000..=10_000_000` | Velocity in millifeet/sec (positive = ascending, negative = descending). |
| `fuel_lbs` | `u32` | `0..=16_500` | Remaining usable propellant in pounds. |
| `dry_mass_lbs` | `u32` | `16_500` | Constant dry mass of the Apollo LM descent stage. |
| `status` | `FlightStatus` | Sum type | Current flight status (`InFlight` or `Landed`). |
| `impact_velocity_mft_s` | `Option<i64>` | Signed integer | Touchdown velocity at surface impact ($Y=0$). `None` while in flight. |

### 2. `BurnCommand` (Pilot Orders Record)

| Field | Type | Domain | Description |
|---|---|---|---|
| `burn_rate_lbs_s` | `u32` | `0..=200` | Engine fuel burn rate in pounds per second ($0 \le B \le 200$). |
| `duration_seconds` | `u32` | `1..=10` | Duration of burn interval in seconds (default: 1 or 10). |

### 3. `FlightStatus` & `LandingOutcome`

```rust
pub enum FlightStatus {
    InFlight,
    Landed(LandingOutcome),
}

pub enum LandingOutcome {
    /// 0 to 5 ft/s (0 to 5,000 mft/s): Flawless landing.
    Perfect,
    /// 5.001 to 15 ft/s (5,001 to 15,000 mft/s): Minor gear stress.
    Hard,
    /// 15.001 to 30 ft/s (15,001 to 30,000 mft/s): Craft crippled, crew survives.
    Crash,
    /// 30.001 to 60 ft/s (30,001 to 60,000 mft/s): Total structural destruction.
    Fatal,
    /// > 60 ft/s (> 60,000 mft/s): Blasted a crater in the lunar surface.
    Crater,
}
```

#### Outcome Classification Thresholds

Impact velocity magnitude is measured at contact ($Y=0$):

$$\text{speed}_{\text{fps}} = \lfloor \frac{|V_{\text{touchdown}}|}{1000} \rfloor$$

| Speed ($\text{mft/s}$) | Speed ($\text{ft/s}$) | Outcome | Historical Debrief |
|---|---|---|---|
| $0 \le \|V\| \le 5,000$ | $0 \le v \le 5$ | `LandingOutcome::Perfect` | "The Eagle has landed! Magnificent landing!" |
| $5,001 \le \|V\| \le 15,000$ | $6 \le v \le 15$ | `LandingOutcome::Hard` | "Good landing, but landing gear damaged. Takeoff doubtful." |
| $15,001 \le \|V\| \le 30,000$ | $16 \le v \le 30$ | `LandingOutcome::Crash` | "Crash landing! Craft wrecked, crew injured, await rescue." |
| $30,001 \le \|V\| \le 60,000$ | $31 \le v \le 60$ | `LandingOutcome::Fatal` | "Fatal impact! Lunar module destroyed, no survivors." |
| $\|V\| > 60,000$ | $v > 60$ | `LandingOutcome::Crater` | "Catastrophic! You blasted a crater 30 feet deep into the lunar regolith!" |

### 4. `LanderError` (Closed Refusal Enum)

```rust
pub enum LanderError {
    /// Attempted to burn when craft has already touched down.
    FlightAlreadyEnded,
    /// Burn rate exceeds engine maximum (B > 200 lbs/s).
    ExcessiveBurnRate,
    /// Duration is 0 or exceeds turn limit (duration == 0 || duration > 10).
    InvalidDuration,
    /// Arithmetic overflow in intermediate physics coordinates.
    ArithmeticOverflow,
}
```

---

## ASCII Telemetry Strip & `paint-vm-ascii` (`P2D02`) Bridge

In addition to pure data transitions, `lunar-lander` integrates with `paint-vm-ascii` (`P2D02`) by compiling flight state into a declarative 2D scene (`PaintScene`).

### Instrument Layout

The telemetry strip renders a terminal dashboard:

```text
┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐
│ TIME:     42 SEC    │ ALTITUDE:  45,200 FT [████████████░░░░░░░░░░] 37%  │
│ VELOCITY: -120 FT/S │ FUEL:       6,400 LBS [████████░░░░░░░░░░░░] 38%  │
│ BURN:      150 LB/S │ RATE:     DESCENT  ▼                               │
└──────────────────────────────────────────────────────────────────────────┘
```

### Conversion to `PaintScene` (`P2D00`)

The presentation function `to_paint_scene(state: &LanderState) -> PaintScene` produces:
1. Outer border rectangle (`rect` instruction with box-drawing characters).
2. Altitude gauge: Bar filled with `█` characters scaled to $Y / Y_0$.
3. Fuel gauge: Bar filled with `█` characters scaled to $F / F_0$.
4. Velocity tape: Descent indicator displaying negative vertical velocity.
5. Numerical readouts: Rendered as `glyph_run` or `format_doc` text runs.

When executed through `paint-vm-ascii` (`P2D02`), this guarantees deterministic text output without requiring native terminal control sequences or ANSI escape codes.

---

## Warrant Formal Specification (`lunar-lander.warrant`)

The formal verification contract for `lunar-lander` is specified under `VER04j` and `VER04m` (`state=explicit` / `WX-S`):

```warrant
package lunar_lander
version 1

type LanderState = record {
    time_seconds: U32,
    altitude_mft: I64,
    velocity_mft_s: I64,
    fuel_lbs: U32,
    dry_mass_lbs: U32,
    status: FlightStatus,
    impact_velocity_mft_s: Option<I64>,
}

type BurnCommand = record {
    burn_rate_lbs_s: U32,
    duration_seconds: U32,
}

type FlightStatus = variant {
    InFlight,
    Landed(LandingOutcome),
}

type LandingOutcome = variant {
    Perfect,
    Hard,
    Crash,
    Fatal,
    Crater,
}

type LanderError = variant {
    FlightAlreadyEnded,
    ExcessiveBurnRate,
    InvalidDuration,
    ArithmeticOverflow,
}

state_inv(s: LanderState) -> Bool =
    s.altitude_mft >= 0 &&
    s.dry_mass_lbs == 16500 &&
    s.fuel_lbs <= 16500

// Fuel monotonicity & Conservation Claim
op step_burn(s: LanderState, cmd: BurnCommand) -> Result<LanderState, LanderError>
    state = explicit
    requires = state_inv(s)
    ensures = match result {
        Ok(s_next) =>
            state_inv(s_next) &&
            s_next.fuel_lbs <= s.fuel_lbs &&
            s_next.time_seconds > s.time_seconds,
        Err(_) => true,
    }
    steps <= 128
    heap <= 0
```

### Deterministic Microbenchmark & Zero Allocation Budget (`VER07`)

In compliance with `VER07`:
- **`max_alloc_bytes: Some(0)`:** The simulation step `step_burn` MUST perform **zero heap allocations** on the hot path. Rust packages install `CountingAllocator` in benchmark binaries to enforce this at CI time.
- **`max_steps: 128`:** The abstract step count for executing one burn step is constant-bounded, ensuring complete protection against algorithmic complexity exhaustion (CWE-400).

---

## 26-CWE Matrix Closure (`VER15`) & Per-Lane CVE Proof Gates

In accordance with `VER15` (`code/specs/data/weakness-registry-v1.json`), every implementation lane MUST discharge or exclude all 26 canonical CWE classes.

### CWE Matrix Accounting

| CWE | Name | Status | Method of Closure |
|---|---|---|---|
| **CWE-416** | Use After Free | `excluded` | Excluded by safe language memory models; zero raw pointers. |
| **CWE-787** | Out-of-bounds Write | `excluded` | No array indexing or raw memory mutation; pure value records. |
| **CWE-125** | Out-of-bounds Read | `excluded` | Pure record field access; bounds checked by compiler. |
| **CWE-415** | Double Free | `excluded` | Stack-allocated / garbage-collected value types. |
| **CWE-476** | NULL Pointer Dereference | `excluded` | Safe Option types / non-nullable references. |
| **CWE-190** | Integer Overflow | `discharged` | Checked 64-bit arithmetic on thrust, velocity, and altitude updates. |
| **CWE-78** | OS Command Injection | `excluded` | `capabilities: []` (no `proc` capability). |
| **CWE-89** | SQL Injection | `excluded` | No database capability. |
| **CWE-22** | Path Traversal | `excluded` | `capabilities: []` (no `fs` capability). |
| **CWE-79** | XSS | `excluded` | `capabilities: []` (no `ui` or HTML DOM rendering). |
| **CWE-94** | Code Injection / Eval | `excluded` | Zero `eval`/`exec`/reflection dispatch. |
| **CWE-116** | Improper Output Encoding | `discharged` | Telemetry text formatter operates on bounded integers and fixed box glyphs. |
| **CWE-117** | Log Injection | `excluded` | Zero ambient stdout/logging within pure simulation core. |
| **CWE-400** | Resource Exhaustion (DoS)| `discharged` | Constant step bound (`steps <= 128`), zero dynamic allocation (`heap <= 0`). |
| **CWE-409** | Decompression Bomb | `excluded` | No compression or streaming payloads. |
| **CWE-674** | Uncontrolled Recursion | `discharged` | Pure iterative quadratic solver; zero recursive calls. |
| **CWE-1333**| ReDoS | `excluded` | Zero regular expression usage. |
| **CWE-407** | Algorithmic Complexity | `discharged` | O(1) step computation. |
| **CWE-208** | Observable Timing | `excluded` | Non-cryptographic flight simulation; no secret data. |
| **CWE-330** | Insufficiently Random | `excluded` | Fully deterministic physical simulation; zero randomness. |
| **CWE-327** | Broken Cryptography | `excluded` | No cryptographic operations performed. |
| **CWE-326** | Inadequate Encryption | `excluded` | No encryption operations performed. |
| **CWE-362** | Race Conditions | `excluded` | `capabilities: []` (no `thread` capability); immutable state. |
| **CWE-367** | TOCTOU | `excluded` | Pure mathematical state function without ambient filesystem. |
| **CWE-502** | Deserialization of Untrusted Data | `discharged` | Typed parsing with explicit bounds; no dynamic object formatters. |
| **CWE-1321**| Prototype Pollution | `discharged` | Object prototype frozen / null-prototype in JavaScript/TypeScript. |

---

### Per-Language Common-CVE Proof Requirements (18 Warrant Lanes)

Every language port must implement dedicated regression test suites verifying immunization against its platform-specific hazards:

1. **Rust (`code/packages/rust/lunar-lander`):**
   - **CWE-190:** Intermediate physics terms (e.g. `impulse = fuel_burn * exhaust_vel`, `v_sq = v * v`) must be calculated using checked or 64-bit integer arithmetic. Tested with `cargo test --release` to ensure overflow checks are preserved.
   - **`CAP002`:** Must compile with `#![forbid(unsafe_code)]`.
   - **`VER08b`:** Passes seccomp/Landlock jail on Linux (`BUILD_linux` with `RCG_STRICT=1`).

2. **TypeScript / JavaScript (`code/packages/typescript/lunar-lander`):**
   - **CWE-843 & CWE-190:** Coordinates in millifeet exceed JavaScript's $2^{31}-1$ 32-bit bitwise limit. Millifeet altitude ($120,000,000$) must be handled with `BigInt` or `Number.isSafeInteger()`. Rejects `NaN`, `Infinity`, and non-integers.
   - **CWE-1321:** State records frozen with `Object.freeze()`.

3. **Python (`code/packages/python/lunar-lander`):**
   - **CWE-843:** Integer inputs guarded with `type(x) is int` (rejecting `bool`).
   - **CWE-400:** Input validation enforcing $0 \le B \le 200$ and $1 \le \Delta t \le 10$ before math, preventing arbitrary-precision integer DoS.

4. **Go (`code/packages/go/lunar-lander`):**
   - **CWE-190:** Checked multiplication before velocity updates to prevent silent wrapping.
   - **CWE-362:** Pure value receivers (`(s LanderState) StepBurn(...)`), ensuring race-free concurrency.

5. **C / C++ (`code/packages/c/lunar-lander`, `code/packages/cpp/lunar-lander`):**
   - **CWE-125 / CWE-787:** Zero dynamic heap allocation. Caller passes stack-allocated struct pointers.
   - **CWE-190 & CWE-476:** Explicit NULL checks; signed 64-bit integer types (`int64_t`) for kinematic equations; ASan/UBSan clean.

6. **Java / Kotlin (`code/packages/java/lunar-lander`, `code/packages/kotlin/lunar-lander`):**
   - **CWE-190:** Kinematic updates use `long` arithmetic with `Math.multiplyExact()`.
   - **CWE-502:** Pure Java `record` / Kotlin `data class` with zero `Serializable` implementation.

7. **C# / F# (.NET) (`code/packages/csharp/lunar-lander`, `code/packages/fsharp/lunar-lander`):**
   - **CWE-190:** Math enclosed in `checked { ... }` blocks.
   - **CWE-502:** Zero binary formatters; pure immutable records.

8. **Swift (`code/packages/swift/lunar-lander`):**
   - **CWE-190 & CWE-400:** Pre-check input bounds to prevent Swift runtime traps on arithmetic overflow, ensuring typed `Result.failure(LanderError)` is returned.

9. **Ruby / Perl / Lua (`code/packages/ruby/lunar-lander`, etc.):**
   - **CWE-843:** Reject implicit string coercion; enforce explicit 64-bit signed integer math.

10. **Elixir / Haskell / OCaml (`code/packages/elixir/lunar-lander`, etc.):**
    - **CWE-400:** Strict record fields (`!`) in Haskell; zero dynamic atom generation in Elixir.

---

## Polyglot Rollout Waves & Backlog Plan

The rollout of `lunar-lander` proceeds across 5 ordered waves:

```text
Wave 1 (PR 1): Specs & Architecture (This PR)
  └── GAME00 master roadmap + GAME01 Hamurabi spec + GAME02 Lunar Lander spec
Wave 2 (PR 2): Rust Reference Implementation + Warrant + Corpus
  └── rust/lunar-lander + lunar-lander.warrant + fixtures/lunar-lander-v1/cases.json
Wave 3 (PR 3): Tier 1 Implementation Lanes
  └── python/lunar-lander + typescript/lunar-lander + go/lunar-lander
Wave 4 (PR 4): Tier 2 Scripting & Functional Lanes
  └── ruby/lunar-lander + elixir/lunar-lander + lua/lunar-lander + perl/lunar-lander
Wave 5 (PR 5): Tier 3 Systems & Enterprise Lanes
  └── csharp, fsharp, java, kotlin, dart, swift, haskell, c, cpp, ocaml
```

### Shared Conformance Corpus (`lunar-lander-v1/cases.json`)

The shared test vectors under `code/specs/fixtures/lunar-lander-v1/cases.json` specify trajectories for:
1. **Freefall Descent ($B=0$):** Apollo LM under pure lunar gravity until cratering.
2. **Hover Equilibrium ($a_{\text{thrust}} \approx g_{\text{lunar}}$):** Maintaining constant altitude.
3. **Controlled Braking ($B=150\text{ lbs/s}$):** Deceleration to a `Perfect` landing ($v < 5\text{ ft/s}$).
4. **Fuel Exhaustion Mid-Flight:** Depleting fuel at $10,000\text{ ft}$, followed by ballistic impact.
5. **Boundary Inputs:** Hostile $B > 200$ and $\Delta t = 0$ rejections.

---

## Package Parity Matrix

| Lane | Directory | Package / Module Name | Capability Certificate |
|---|---|---|---|
| **Rust** | `code/packages/rust/lunar-lander` | `lunar_lander` | `capabilities: []` |
| **Python** | `code/packages/python/lunar-lander` | `lunar_lander` | `capabilities: []` |
| **TypeScript**| `code/packages/typescript/lunar-lander`| `@coding-adventures/lunar-lander` | `capabilities: []` |
| **Go** | `code/packages/go/lunar-lander` | `lunar_lander` | `capabilities: []` |
| **Ruby** | `code/packages/ruby/lunar-lander` | `CodingAdventures::LunarLander` | `capabilities: []` |
| **Elixir** | `code/packages/elixir/lunar-lander` | `CodingAdventures.LunarLander` | `capabilities: []` |
| **Lua** | `code/packages/lua/lunar-lander` | `coding_adventures.lunar_lander` | `capabilities: []` |
| **Perl** | `code/packages/perl/lunar-lander` | `CodingAdventures::LunarLander` | `capabilities: []` |
| **Java** | `code/packages/java/lunar-lander` | `com.codingadventures.lunar_lander` | `capabilities: []` |
| **Kotlin** | `code/packages/kotlin/lunar-lander` | `com.codingadventures.lunar_lander` | `capabilities: []` |
| **C#** | `code/packages/csharp/lunar-lander` | `CodingAdventures.LunarLander` | `capabilities: []` |
| **F#** | `code/packages/fsharp/lunar-lander` | `CodingAdventures.LunarLander` | `capabilities: []` |
| **Dart** | `code/packages/dart/lunar-lander` | `coding_adventures_lunar_lander` | `capabilities: []` |
| **Swift** | `code/packages/swift/lunar-lander` | `LunarLander` | `capabilities: []` |
| **Haskell** | `code/packages/haskell/lunar-lander`| `CodingAdventures.LunarLander` | `capabilities: []` |
| **C** | `code/packages/c/lunar-lander` | `lunar_lander` | `capabilities: []` |
| **C++** | `code/packages/cpp/lunar-lander` | `lunar_lander` | `capabilities: []` |
| **OCaml** | `code/packages/ocaml/lunar-lander` | `LunarLander` | `capabilities: []` |
