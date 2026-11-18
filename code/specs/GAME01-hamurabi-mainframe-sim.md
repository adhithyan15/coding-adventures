# GAME01 — Hamurabi: Mainframe Turn-Based Resource Allocation Simulation

## Overview & Historical Context

The `hamurabi` package provides a pure, deterministic, platform-independent simulation core for the seminal 1960s mainframe resource allocation game.

### Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. It introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

### The Architectural Thesis: Pure Deterministic Simulation Core

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`), the game engine is strictly factored into two distinct layers:

```text
┌──────────────────────────────────────────────────────────────────┐
│                   Presentation & I/O Shell                       │
│    (Terminal stdin/stdout, EventLoop, Web Canvas, PaintVM)       │
└─────────────────────────────────┬────────────────────────────────┘
                                  │ TurnOrder
                                  ▼
┌──────────────────────────────────────────────────────────────────┐
│             Pure Simulation Core: `hamurabi`                     │
│               - "capabilities": [] (Pure)                       │
│               - State machine: (State, Order) -> Result<State>   │
│               - Seeded PRNG: explicit rng::Pcg32 state           │
│               - 100% Deterministic & Cross-Language Identical    │
└──────────────────────────────────────────────────────────────────┘
```

1. **Pure Simulation Core (`hamurabi`):**
   - Zero ambient system authority: `"capabilities": []` declared in `required_capabilities.json`.
   - Pure state transition: `step_year(state: &HamurabiState, order: &TurnOrder) -> Result<HamurabiState, HamurabiError>`.
   - All pseudo-randomness enters deterministically through explicit `PCG32` state stored inside `HamurabiState`.
   - Uses bounded integer types (`u8`, `u32`, `u64`, `i32`). This is the intended deterministic contract for conforming ports; package presence alone does not establish bit-exact cross-language trajectories. The current dated lane inventory and evidence limits are recorded in `GAME-BACKLOG.md`.
   - Required verification targets include conservation invariants, step bounds, zero allocations (`VER07`), and per-lane disposition of all 26 CWE classes (`VER15`). Warrant (`code/specs/hamurabi.warrant`) must be checked under `VER04j` and `VER04m` (`state=explicit`) before proof claims are made. The checked-in Rust assurance report is `report-only`: claim evidence is empty, proof/exhaustive/fuzz entries are null, 24 weakness entries are excluded, and two exemptions are unapproved. Corpus conformance does not close those gaps.

2. **Thin Host Presentation Shell:**
   - Wires terminal stdin/stdout, command line arguments, or web interfaces to the simulation core.
   - Formats the game report using `format_report()` or terminal rendering through `paint-vm-ascii` (`P2D02`) and `format-doc`.

---

## Package Identity & Dependency Architecture

### Metadata & Capability Registry

| Field | Value |
|---|---|
| **Package Identity** | `hamurabi` (portable package name; per-language implementation and evidence are inventoried in `GAME-BACKLOG.md`) |
| **Parity Class** | `portable` |
| **Declared Capabilities** | `[]` (strictly zero ambient capabilities in `required_capabilities.json`) |
| **Direct Dependencies** | `rng` (`code/specs/rng.md` — specifically `rng::Pcg32`) |
| **Transitive Dependencies**| None |
| **Verification Level** | Required target: Tier 0 / Tier 2 Warrant verification (`state=explicit`, `VER04m`); the checked-in Rust assurance report is `report-only`, not a Warrant proof receipt |

### Static Capability Cage Guarantees

In accordance with `VER08a`, `VER08b`, and `Spec 13`:
- **`CAP001` (Zero Ambient Authority):** The package must not access filesystem (`fs`), network (`net`), processes (`proc`), environment variables (`env`), system time (`time`), terminal I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`).
- **`CAP002` (Zero Unsafe / FFI):** Rust implementation forbids unsafe code (`#![forbid(unsafe_code)]`). C/C++ implementations prohibit dynamic heap allocation (`malloc`/`free`) and raw pointer arithmetic outside bounded arrays.
- **`CAP003` (Fail Closed):** All macros and constructs must be statically analyzable by `ca-capability-analyzer`.
- **`VER29` (Contract Linking):** The package assumes deterministic PRNG behavior from its dependency via:
  ```warrant
  assume platform: contract.rng.pcg32_deterministic
  ```

---

## Domain Types & Data Structures

Every language implementation MUST expose the following canonical types, adhering to native casing conventions (`PascalCase` for types, `snake_case` or `camelCase` for fields and methods).

### 1. `HamurabiState` (Explicit State Record)

In mode `state=explicit` (`VER04m`), the complete mutable state of the simulation is reified into an immutable value record.

| Field | Type | Domain / Constraints | Description |
|---|---|---|---|
| `year` | `u8` | `1..=11` | Current year of reign. Years 1 to 10 are playable; 11 is terminal summary state. |
| `population` | `u32` | `0..=1_000_000` | Current population of the city-state (initial: `100`). |
| `bushels` | `u32` | `0..=100_000_000` | Bushels of grain in storage (initial: `2800`). |
| `acres` | `u32` | `1..=10_000_000` | Acres of farmland owned by the city (initial: `1000`). |
| `land_price` | `u32` | `17..=26` | Current market price of land in bushels per acre. |
| `harvest_per_acre` | `u32` | `1..=5` | Yield from the previous harvest in bushels per acre. |
| `rats_eaten` | `u32` | `0..=100_000_000` | Bushels consumed by rats during previous year. |
| `starved_last_year` | `u32` | `0..=1_000_000` | Citizens starved during previous year. |
| `starved_total` | `u32` | `0..=10_000_000` | Cumulative citizens starved across the reign. |
| `percent_starved_sum` | `u32` | `0..=1000` | Sum of yearly starvation percentages (used for 10-year average). |
| `immigrants_last_year`| `u32` | `0..=1_000_000` | New citizens arriving during previous year. |
| `plague_struck` | `bool` | `true \| false` | Whether a plague occurred in the previous year. |
| `status` | `GameStatus` | Sum type | Current game lifecycle status. |
| `rng_state` | `u64` | `0..=2^64 - 1` | Internal 64-bit state of the `PCG32` generator. |
| `rng_inc` | `u64` | Odd integer | Internal 64-bit sequence increment of `PCG32` (must have bit 0 set). |

### 2. `TurnOrder` (Player Orders Record)

| Field | Type | Domain / Constraints | Description |
|---|---|---|---|
| `acres_delta` | `i32` | `-(acres as i32)..=i32::MAX` | Acres to buy (positive) or sell (negative). `0` indicates no land trade. |
| `bushels_to_feed` | `u32` | `0..=u32::MAX` | Total bushels allocated to feed the population (20 bushels per person required). |
| `acres_to_plant` | `u32` | `0..=u32::MAX` | Acres of owned land to sow with grain (1 bushel seeds 2 acres; 1 person tends 10 acres). |

### 3. `GameStatus` (Simulation Lifecycle Status)

```rust
pub enum GameStatus {
    Playing,
    Won { rating: Rating },
    Impeached { starved_this_turn: u32, percent: u32 },
    GameOverExtinction,
}
```

#### Rating Classification (`Rating`)
Assigned when the player successfully reaches `year == 11`:
- **`Charlemagne` (Outstanding):** Average starvation $\le 3\%$ AND acres per person $\ge 10$. ("A fantastic performance! Charlemagne, Disraeli, and Jefferson combined could not have done better!")
- **`Acre` (Good):** Average starvation $\le 10\%$ AND acres per person $\ge 9$. ("Your performance could have been worse, but you left them reasonable acreage.")
- **`Nero` (Poor):** Average starvation $\le 33\%$ AND acres per person $\ge 7$. ("Your performance leaves much to be desired. Nero would have been proud of you.")
- **`Disaster` (Atrocious):** Average starvation $> 33\%$ or acres per person $< 7$. ("You did so poorly you have been declared an official national disaster!")

### 4. `HamurabiError` (Closed Refusal Enum)

When a `TurnOrder` is invalid or cannot be executed due to resource deficits, `step_year` and `validate_order` return an explicit typed error. **The simulation state is NEVER modified on error.**

```rust
pub enum HamurabiError {
    /// Attempted to execute an order when status is not Playing.
    GameAlreadyEnded,
    /// Arithmetic overflow or invalid bounds on input parameters.
    InvalidOrderBounds,
    /// Player ordered to buy land but lacks sufficient bushels:
    /// acres_delta > 0 && acres_delta * land_price > bushels.
    InsufficientBushelsForLand,
    /// Player ordered to sell more land than the city owns:
    /// acres_delta < 0 && (-acres_delta) > acres.
    InsufficientAcresToSell,
    /// Player ordered to feed more bushels than available after land trade:
    /// bushels_to_feed > bushels_after_trade.
    InsufficientBushelsToFeed,
    /// Player ordered to plant more acres than owned after land trade:
    /// acres_to_plant > acres_after_trade.
    InsufficientAcresToPlant,
    /// Player ordered to plant more acres than population can tend:
    /// acres_to_plant > population * 10.
    InsufficientPopulationToPlant,
    /// Player lacks grain to seed the fields (1 bushel seeds 2 acres):
    /// (acres_to_plant + 1) / 2 > bushels_after_feed.
    InsufficientGrainForSeed,
}
```

---

## State Transition Mathematics & PCG32 Draw Order

### Initial State (`new_game`)

Calling `new_game(seed: u64)` initializes a deterministic 10-year reign:
```text
year: 1
population: 100
bushels: 2800
acres: 1000
harvest_per_acre: 3
rats_eaten: 200
starved_last_year: 0
starved_total: 0
percent_starved_sum: 0
immigrants_last_year: 5
plague_struck: false
status: GameStatus::Playing
```

`PCG32` is seeded with `(seed, stream_id = 54)` using standard PCG initialization:
```rust
let mut pcg = Pcg32::new(seed, 54);
let land_price = pcg.next_int_in_range(17, 26) as u32;
```
The state stores `pcg.state` and `pcg.inc`.

---

### Step Year Algorithm (`step_year`)

The state transition function `step_year(state: &HamurabiState, order: &TurnOrder) -> Result<HamurabiState, HamurabiError>` executes in two strict, atomic phases:

#### Phase A: Validation & Constraint Checking

Validation MUST evaluate checks in this exact canonical order:

1. **Status Check:**
   If `state.status != GameStatus::Playing`, return `Err(HamurabiError::GameAlreadyEnded)`.
2. **Bounds & Overflow Guard:**
   Ensure inputs do not overflow 64-bit unsigned bounds:
   - If `order.acres_delta > 0`: `cost = (order.acres_delta as u64) * (state.land_price as u64)`. If `cost > state.bushels as u64`, return `Err(HamurabiError::InsufficientBushelsForLand)`.
   - If `order.acres_delta < 0`: `sell_count = (-order.acres_delta) as u64`. If `sell_count > state.acres as u64`, return `Err(HamurabiError::InsufficientAcresToSell)`.
3. **Land Trade Balances:**
   - `acres_after_trade = (state.acres as i64 + order.acres_delta as i64) as u32`
   - `bushels_after_trade = (state.bushels as i64 - (order.acres_delta as i64 * state.land_price as i64)) as u32`
4. **Food Allocation Check:**
   - If `order.bushels_to_feed > bushels_after_trade`, return `Err(HamurabiError::InsufficientBushelsToFeed)`.
   - `bushels_after_feed = bushels_after_trade - order.bushels_to_feed`
5. **Planting Land & Labor Checks:**
   - If `order.acres_to_plant > acres_after_trade`, return `Err(HamurabiError::InsufficientAcresToPlant)`.
   - Maximum plantable by labor: `max_labor_acres = (state.population as u64) * 10`. If `(order.acres_to_plant as u64) > max_labor_acres`, return `Err(HamurabiError::InsufficientPopulationToPlant)`.
6. **Planting Seed Check:**
   - Seed needed: Each bushel seeds 2 acres. To avoid fractional bushels, the integer formula is:
     $$\text{seed\_needed} = \lfloor \frac{\text{acres\_to\_plant} + 1}{2} \rfloor$$
     *(Equivalently: `acres_to_plant / 2` when acres is even; `(acres_to_plant / 2) + 1` when odd).*
   - If `seed_needed > bushels_after_feed`, return `Err(HamurabiError::InsufficientGrainForSeed)`.
   - `bushels_after_seed = bushels_after_feed - seed_needed`

---

#### Phase B: Deterministic Simulation Update & PCG32 Draw Sequence

To ensure identical execution traces across all 18 programming languages, the PRNG draw sequence per year is **strictly locked and non-reordering**.

Let `pcg` be initialized from `(state.rng_state, state.rng_inc)`.

```text
Step 1: Harvest Yield Draw
  Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre (u32)
  total_harvest = order.acres_to_plant * harvest_per_acre
  bushels_stored = bushels_after_seed + total_harvest

Step 2: Rat Infestation Draws
  Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll (u32)
  if rat_roll <= 40: (40% chance of infestation)
      Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor (u32)
      rats_eaten = bushels_stored / rat_factor
  else:
      rats_eaten = 0
      (Draw 3 is SKIPPED when rat_roll > 40)
  bushels_final = bushels_stored - rats_eaten

Step 3: Plague Draw
  Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll (u32)
  plague_struck = (plague_roll <= 15)  (15% chance of plague)

Step 4: Starvation & Impeachment Evaluation
  people_fed = order.bushels_to_feed / 20
  if people_fed >= state.population:
      starved = 0
      survivors = state.population
  else:
      starved = state.population - people_fed
      survivors = people_fed

  percent_starved = (starved * 100) / state.population
  new_starved_total = state.starved_total + starved
  new_percent_sum = state.percent_starved_sum + percent_starved

  if starved * 100 > state.population * 45:
      // Over 45% of the population starved in a single year -> Impeached!
      new_status = GameStatus::Impeached {
          starved_this_turn: starved,
          percent: percent_starved
      }
  else if survivors == 0:
      new_status = GameStatus::GameOverExtinction
  else:
      new_status = GameStatus::Playing

Step 5: Plague Resolution
  if plague_struck && new_status == GameStatus::Playing:
      survivors = survivors / 2

Step 6: Immigration Draw
  // Immigrant factor draw is UNCONDITIONAL to preserve stream alignment
  Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (u32)
  if starved == 0 && new_status == GameStatus::Playing:
      // Standard Sumerian formula:
      // immigrants = [factor * (20 * acres + bushels) / (100 * population)] + 1
      numerator = (imm_factor as u64) * (20 * (acres_after_trade as u64) + (bushels_final as u64))
      denominator = 100 * (state.population as u64)
      immigrants = ((numerator / denominator) + 1) as u32
  else:
      immigrants = 0

  final_population = survivors + immigrants

Step 7: Next Year Land Price Draw
  Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price (u32)

Step 8: Turn Advance & Rating
  next_year = state.year + 1
  if next_year == 11 && new_status == GameStatus::Playing:
      avg_starved = new_percent_sum / 10
      acres_per_person = acres_after_trade / final_population
      rating = if avg_starved <= 3 && acres_per_person >= 10 {
          Rating::Charlemagne
      } else if avg_starved <= 10 && acres_per_person >= 9 {
          Rating::Acre
      } else if avg_starved <= 33 && acres_per_person >= 7 {
          Rating::Nero
      } else {
          Rating::Disaster
      }
      new_status = GameStatus::Won { rating }
```

The resulting `HamurabiState` record is constructed with `pcg.state` and `pcg.inc`.

---

## Terminal Report Formatting (`format_report`)

The simulation core provides a pure string formatting function:
```rust
pub fn format_report(state: &HamurabiState) -> String
```

It formats the annual report in authentic 1970s teletype uppercase styling:

```text
HAMURABI: I BEG TO REPORT TO YOU,
IN YEAR 1, 0 PEOPLE STARVED, 5 CAME TO THE CITY.
POPULATION IS NOW 105.
THE CITY NOW OWNS 1000 ACRES.
YOU HARVESTED 3 BUSHELS PER ACRE.
RATS ATE 200 BUSHELS.
YOU NOW HAVE 2800 BUSHELS IN STORE.
LAND IS TRADING AT 24 BUSHELS PER ACRE.
```

If a plague occurred during the year:
```text
A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.
```

When impeached:
```text
YOU STARVED 48 PEOPLE IN ONE YEAR!!!
DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!
```

When completing Year 10 (Terminal Summary at Year 11):
```text
IN YOUR 10-YEAR TERM OF OFFICE, 0 PERCENT OF THE
POPULATION STARVED PER YEAR ON AVERAGE.
A TOTAL OF 0 PEOPLE DIED!!
YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH
9 ACRES PER PERSON.

YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.
```

---

## Warrant Formal Specification (`hamurabi.warrant`)

The formal verification contract for `hamurabi` is specified under `VER04j` and `VER04m` (`state=explicit` / `WX-S`):

```warrant
package hamurabi
version 1

assume platform: contract.rng.pcg32_deterministic

type HamurabiState = record {
    year: U8,
    population: U32,
    bushels: U32,
    acres: U32,
    land_price: U32,
    harvest_per_acre: U32,
    rats_eaten: U32,
    starved_last_year: U32,
    starved_total: U32,
    percent_starved_sum: U32,
    immigrants_last_year: U32,
    plague_struck: Bool,
    status: GameStatus,
    rng_state: U64,
    rng_inc: U64,
}

type TurnOrder = record {
    acres_delta: I32,
    bushels_to_feed: U32,
    acres_to_plant: U32,
}

type GameStatus = variant {
    Playing,
    Won(Rating),
    Impeached(U32, U32),
    GameOverExtinction,
}

type Rating = variant {
    Charlemagne,
    Acre,
    Nero,
    Disaster,
}

type HamurabiError = variant {
    GameAlreadyEnded,
    InvalidOrderBounds,
    InsufficientBushelsForLand,
    InsufficientAcresToSell,
    InsufficientBushelsToFeed,
    InsufficientAcresToPlant,
    InsufficientPopulationToPlant,
    InsufficientGrainForSeed,
}

// State Invariant
state_inv(s: HamurabiState) -> Bool =
    s.year >= 1 && s.year <= 11 &&
    s.land_price >= 17 && s.land_price <= 26 &&
    s.harvest_per_acre >= 1 && s.harvest_per_acre <= 5 &&
    (s.rng_inc & 1 == 1)

// Deterministic step claim
op step_year(s: HamurabiState, o: TurnOrder) -> Result<HamurabiState, HamurabiError>
    state = explicit
    requires = state_inv(s)
    ensures = match result {
        Ok(s_next) => state_inv(s_next) && s_next.year == s.year + 1,
        Err(_) => true,
    }
    steps <= 256
    heap <= 0
```

### Deterministic Microbenchmark & Zero Allocation Budget (`VER07`)

In compliance with `VER07`:
- **`max_alloc_bytes: Some(0)`:** The simulation step `step_year` operates purely on value records and MUST perform **zero heap allocations** on the hot path. Rust packages install `CountingAllocator` in benchmark binaries to enforce this at CI time.
- **`max_steps: 256`:** The abstract step count for executing one year is constant-bounded, ensuring complete protection against algorithmic complexity exhaustion (CWE-400).

---

## CWE Requirements and Evidence Limits (`VER15`)

In accordance with `VER15` (`code/specs/data/weakness-registry-v1.json`), every implementation lane MUST discharge or formally exclude all 26 canonical CWE classes. The table below records required dispositions and arguments to verify; it is not a receipt of completed proof. In particular, the checked-in Rust Hamurabi assurance report has verdict `report-only`, no evidence attached to its claims, null proof/exhaustive/fuzz entries, and two unapproved exemptions. Finite corpus checks do not upgrade these statuses.

### CWE Disposition Targets (Not Current Closure Evidence)

| CWE | Name | Status | Method of Closure |
|---|---|---|---|
| **CWE-416** | Use After Free | `excluded` | Excluded by safe language memory models; zero raw pointers. |
| **CWE-787** | Out-of-bounds Write | `excluded` | No array indexing or raw memory mutation; pure records. |
| **CWE-125** | Out-of-bounds Read | `excluded` | Pure record field access; bounds checked by compiler. |
| **CWE-415** | Double Free | `excluded` | Stack-allocated / garbage-collected value types. |
| **CWE-476** | NULL Pointer Dereference | `excluded` | Non-nullable references / safe Option types. |
| **CWE-190** | Integer Overflow | `discharged` | Checked 64-bit arithmetic on `acres_delta * land_price` and planting math. |
| **CWE-78** | OS Command Injection | `excluded` | `capabilities: []` (no `proc` capability). |
| **CWE-89** | SQL Injection | `excluded` | No database capability. |
| **CWE-22** | Path Traversal | `excluded` | `capabilities: []` (no `fs` capability). |
| **CWE-79** | XSS | `excluded` | `capabilities: []` (no `ui` or HTML rendering). |
| **CWE-94** | Code Injection / Eval | `excluded` | Zero `eval`/`exec`/reflection dispatch. |
| **CWE-116** | Improper Output Encoding | `discharged` | Fixed ASCII teletype report generator with bounded integer conversions. |
| **CWE-117** | Log Injection | `excluded` | Zero ambient stdout/logging within pure simulation core. |
| **CWE-400** | Resource Exhaustion (DoS)| `discharged` | Constant step bound (`steps <= 256`), zero dynamic allocation (`heap <= 0`). |
| **CWE-409** | Decompression Bomb | `excluded` | No compression or streaming payloads. |
| **CWE-674** | Uncontrolled Recursion | `discharged` | Completely iterative state machine; zero recursive calls. |
| **CWE-1333**| ReDoS | `excluded` | Zero regular expression usage. |
| **CWE-407** | Algorithmic Complexity | `discharged` | O(1) state transition time. |
| **CWE-208** | Observable Timing | `excluded` | Non-cryptographic game simulation; no secret-bearing assets. |
| **CWE-330** | Insufficiently Random | `excluded` | Explicit pseudorandom simulation engine using proven `PCG32`. |
| **CWE-327** | Broken Cryptography | `excluded` | No cryptographic operations performed. |
| **CWE-326** | Inadequate Encryption | `excluded` | No encryption operations performed. |
| **CWE-362** | Race Conditions | `excluded` | `capabilities: []` (no `thread` capability); immutable state. |
| **CWE-367** | TOCTOU | `excluded` | Pure mathematical state function without ambient filesystem. |
| **CWE-502** | Deserialization of Untrusted Data | `discharged` | Typed parsing with explicit bounds; no dynamic object formatters. |
| **CWE-1321**| Prototype Pollution | `discharged` | Object prototype frozen / null-prototype in JavaScript/TypeScript. |

---

### Per-Language Common-CVE Proof Requirements (18 Warrant Lanes)

Every language port must implement dedicated regression test suites verifying immunization against its platform-specific hazards:

1. **Rust (`code/packages/rust/hamurabi`):**
   - **CWE-190:** Math on `order.acres_delta * state.land_price` must use `checked_mul` / `i64` widening. Test must pass under `cargo test --release` to ensure overflow checks are not stripped in optimized release builds (`LG01` SEC-007).
   - **`CAP002`:** Must compile with `#![forbid(unsafe_code)]`.
   - **`VER08b`:** Passes seccomp/Landlock jail on Linux (`BUILD_linux` with `RCG_STRICT=1`).

2. **TypeScript / JavaScript (`code/packages/typescript/hamurabi`):**
   - **CWE-843 & CWE-190:** Prevent float coercion and safe integer loss ($> 2^{53}-1$). Inputs must be guarded with `Number.isSafeInteger()`. Reject `NaN`, `Infinity`, `-0`, and fractional floats.
   - **CWE-1321:** State objects must be frozen via `Object.freeze()` or constructed with `Object.create(null)` to eliminate prototype pollution risks.

3. **Python (`code/packages/python/hamurabi`):**
   - **CWE-843:** Python's `bool` is an `int` subclass (`isinstance(True, int) == True`). All integer inputs must be validated with `type(x) is int` (rejecting `bool`).
   - **CWE-400:** Guard against arbitrary-precision integer denial-of-service (4300-digit DoS). Enforce bounds `-2^31 <= acres_delta <= 2^31 - 1` before arithmetic.

4. **Go (`code/packages/go/hamurabi`):**
   - **CWE-190:** In Go, integer overflow wraps silently. Multiplication of `acres_delta` and `land_price` must be checked before evaluation.
   - **CWE-362:** State returns must be value structs (`HamurabiState`), never shared mutable pointers, preventing slice/array aliasing data races.

5. **C / C++ (`code/packages/c/hamurabi`, `code/packages/cpp/hamurabi`):**
   - **CWE-125 / CWE-787:** Zero dynamic heap allocation (`malloc`/`free`/`new`) inside `hamurabi_step_year()`. Caller provides destination struct pointer.
   - **CWE-190 & CWE-476:** Explicit checks for NULL state pointers; promotion of arithmetic to `int64_t` before bounds validation; clean build under `-Wall -Wextra -Werror` with ASan/UBSan targets.

6. **Java / Kotlin (`code/packages/java/hamurabi`, `code/packages/kotlin/hamurabi`):**
   - **CWE-190:** Absence of unsigned 32-bit types in Java requires arithmetic promotion to `long` or usage of `Math.multiplyExact()`.
   - **CWE-502:** Complete absence of Java `Serializable` / `ObjectInputStream`. State is represented as immutable Java `record` / Kotlin `data class`.

7. **C# / F# (.NET) (`code/packages/csharp/hamurabi`, `code/packages/fsharp/hamurabi`):**
   - **CWE-190:** .NET defaults to `unchecked` arithmetic. All calculation must be wrapped in `checked { ... }` blocks or use explicit guard clauses.
   - **CWE-502:** Zero reflection-based binary formatters; pure immutable C# `record`.

8. **Swift (`code/packages/swift/hamurabi`):**
   - **CWE-190 & CWE-400:** Swift runtime traps (crashes) by default on integer overflow. Hostile inputs must NOT cause process abortion; bounds must be validated before arithmetic using `addingReportingOverflow()` / `multipliedReportingOverflow()` to return `Result.failure(HamurabiError)`.

9. **Ruby / Perl / Lua (`code/packages/ruby/hamurabi`, etc.):**
   - **CWE-843:** Prevent implicit string-to-number coercions (e.g. `"100 acres" -> 100`). Reject non-integer types strictly. Enforce 64-bit unsigned bitwise masks.

10. **Elixir / Haskell / OCaml (`code/packages/elixir/hamurabi`, etc.):**
    - **CWE-400:** Never convert untrusted input strings to Elixir atoms (`String.to_atom` exhaustion). Enforce strict record fields (`!`) in Haskell to prevent lazy thunk accumulation across simulation turns.

---

## Polyglot Rollout Waves & Backlog Plan

The rollout of `hamurabi` across the coding-adventures monorepo proceeds in 5 ordered waves:

```text
Wave 1 (PR 1): Specs & Architecture (This PR)
  └── GAME00 master roadmap + GAME01 Hamurabi spec + GAME02 Lunar Lander spec
Wave 2 (PR 2): Rust Reference Implementation + Warrant + Corpus
  └── rust/hamurabi + hamurabi.warrant + fixtures/hamurabi-v1/cases.json
Wave 3 (PR 3): Tier 1 Implementation Lanes
  └── python/hamurabi + typescript/hamurabi + go/hamurabi
Wave 4 (PR 4): Tier 2 Scripting & Functional Lanes
  └── ruby/hamurabi + elixir/hamurabi + lua/hamurabi + perl/hamurabi
Wave 5 (PR 5): Tier 3 Systems & Enterprise Lanes
  └── csharp, fsharp, java, kotlin, dart, swift, haskell, c, cpp, ocaml
```

### Shared Conformance Corpus (`hamurabi-v1/cases.json`)

The conformance test vectors are committed to `code/specs/fixtures/hamurabi-v1/cases.json`. Each language runner executes the suite against its public API and verifies bit-exact equality of the resulting state and output string.

```json
{
  "version": 1,
  "suite": "hamurabi_deterministic_reign",
  "cases": [
    {
      "id": "year1_standard_peaceful_reign",
      "seed": 1337,
      "orders": [
        { "acres_delta": 0, "bushels_to_feed": 2000, "acres_to_plant": 800 }
      ],
      "expected_states": [
        {
          "year": 2,
          "population": 105,
          "bushels": 3200,
          "acres": 1000,
          "land_price": 22,
          "harvest_per_acre": 4,
          "rats_eaten": 0,
          "starved_last_year": 0,
          "plague_struck": false,
          "status": "Playing"
        }
      ]
    },
    {
      "id": "immediate_impeachment_starvation",
      "seed": 42,
      "orders": [
        { "acres_delta": 0, "bushels_to_feed": 0, "acres_to_plant": 0 }
      ],
      "expected_error_or_status": {
        "status": "Impeached",
        "starved_this_turn": 100,
        "percent": 100
      }
    }
  ]
}
```

---

## Package Parity Matrix

| Lane | Directory | Package / Module Name | Capability Certificate |
|---|---|---|---|
| **Rust** | `code/packages/rust/hamurabi` | `hamurabi` | `capabilities: []` |
| **Python** | `code/packages/python/hamurabi` | `hamurabi` | `capabilities: []` |
| **TypeScript**| `code/packages/typescript/hamurabi`| `@coding-adventures/hamurabi` | `capabilities: []` |
| **Go** | `code/packages/go/hamurabi` | `hamurabi` | `capabilities: []` |
| **Ruby** | `code/packages/ruby/hamurabi` | `CodingAdventures::Hamurabi` | `capabilities: []` |
| **Elixir** | `code/packages/elixir/hamurabi` | `CodingAdventures.Hamurabi` | `capabilities: []` |
| **Lua** | `code/packages/lua/hamurabi` | `coding_adventures.hamurabi` | `capabilities: []` |
| **Perl** | `code/packages/perl/hamurabi` | `CodingAdventures::Hamurabi` | `capabilities: []` |
| **Java** | `code/packages/java/hamurabi` | `com.codingadventures.hamurabi` | `capabilities: []` |
| **Kotlin** | `code/packages/kotlin/hamurabi` | `com.codingadventures.hamurabi` | `capabilities: []` |
| **C#** | `code/packages/csharp/hamurabi` | `CodingAdventures.Hamurabi` | `capabilities: []` |
| **F#** | `code/packages/fsharp/hamurabi` | `CodingAdventures.Hamurabi` | `capabilities: []` |
| **Dart** | `code/packages/dart/hamurabi` | `coding_adventures_hamurabi` | `capabilities: []` |
| **Swift** | `code/packages/swift/hamurabi` | `Hamurabi` | `capabilities: []` |
| **Haskell** | `code/packages/haskell/hamurabi`| `CodingAdventures.Hamurabi` | `capabilities: []` |
| **C** | `code/packages/c/hamurabi` | `hamurabi` | `capabilities: []` |
| **C++** | `code/packages/cpp/hamurabi` | `hamurabi` | `capabilities: []` |
| **OCaml** | `code/packages/ocaml/hamurabi` | `Hamurabi` | `capabilities: []` |
