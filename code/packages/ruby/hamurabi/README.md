# Hamurabi Simulation Core (Ruby)

`coding_adventures_hamurabi` is a pure, deterministic, platform-independent simulation core for the seminal 1968 mainframe resource allocation game **Hamurabi**, implemented in pure Ruby.

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. It introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):

- **Strict Zero Ambient Authority (`CAP001`):** `"capabilities": []` declared in `required_capabilities.json`. The core performs zero filesystem, network, subprocess, environment, or system-clock access.
- **Pure Functional State Transitions:** `step_year(state, order) -> HamurabiState` advances the simulation without mutating the input state.
- **Fresh Immutable Value Records (`VER04m`):** All returned `HamurabiState` objects are `.freeze`d to ensure immutable value semantics across callers and threads.
- **Explicit Pseudorandomness:** All simulation randomness enters deterministically through explicit PCG32 state stored directly inside `HamurabiState` (`HAMURABI_STREAM_ID = 54`, `rng_inc = 109`).
- **Exact Bounded Integer Arithmetic:** Uses strictly checked integer operations with explicit 32-bit and 64-bit bounds and masks, guaranteeing 100% bit-exact parity with Rust, Python, Go, TypeScript, and all 18 languages.
- **Strict CWE/CVE Hardening:**
  - **CWE-843 (Type Confusion):** Strict `is_a?(Integer)` guards on all numerical inputs, preventing implicit string coercion or float injection.
  - **CWE-190 (Integer Overflow):** Checked 64-bit widening and bounds checking on land trades and planting math.
  - **CWE-400 / CWE-835 (Resource Exhaustion / Infinite Loops):** Enforcement of odd `rng_inc` to prevent LCG period collapse; rejection sampling bounded with constant step limits.
  - **CWE-94 / CWE-502:** Zero `eval`, `Marshal`, or `YAML.load`.
  - **CWE-1333 (ReDoS):** Zero backtracking regular expressions.

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_hamurabi"
```

Or install locally:

```bash
gem install coding_adventures_hamurabi
```

## Quick Start

```ruby
require "coding_adventures_hamurabi"

# 1. Initialize a new 10-year reign with a 64-bit seed
state = CodingAdventures::Hamurabi.new_game(1337)
puts CodingAdventures::Hamurabi.format_report(state)

# 2. Issue a turn order
# - Buy 20 acres (positive) or sell (negative)
# - Feed 2000 bushels (20 bushels per person to prevent starvation)
# - Plant 800 acres (1 bushel seeds 2 acres; 1 citizen tends 10 acres)
order = CodingAdventures::Hamurabi::TurnOrder.new(
  acres_delta: 0,
  bushels_to_feed: 2000,
  acres_to_plant: 800
)

# 3. Advance to the next year
next_state = CodingAdventures::Hamurabi.step_year(state, order)
puts CodingAdventures::Hamurabi.format_report(next_state)
```

## State Machine & Draw Sequence

Each year executes in two strict, atomic phases:

### Phase A: Validation (Canonical Refusal Order)
1. **Status check:** Must be `Playing`. If already ended, raises `GameAlreadyEnded`.
2. **State invariant:** Rejects invalid `rng_inc` (must be odd), invalid year (1..10), invalid land price (17..26), or invalid harvest (1..5) with `InvalidOrderBounds`.
3. **Order types & bounds:** Rejects non-integers and numbers out of i32/u32 bounds with `InvalidOrderBounds`.
4. **Land trade availability:** Rejects unaffordable purchases with `InsufficientBushelsForLand` or selling more acres than owned with `InsufficientAcresToSell`.
5. **Trade revenue overflow:** Rejects trades exceeding u32 limits with `InvalidOrderBounds`.
6. **Food allocation:** Rejects feeding more grain than stored with `InsufficientBushelsToFeed`.
7. **Planting acreage & labor:** Rejects planting more than owned with `InsufficientAcresToPlant`, or more than population can tend (10 acres/person) with `InsufficientPopulationToPlant`.
8. **Seed grain:** Rejects planting without seed grain (ceil(acres / 2)) with `InsufficientGrainForSeed`.
9. **Harvest overflow guard:** Rejects orders where maximum possible harvest could overflow u32 with `InvalidOrderBounds`.

### Phase B: Deterministic Simulation Updates (PCG32)
1. **Harvest yield draw:** `pcg.next_int_in_range(1, 5)`
2. **Rat infestation roll:** `pcg.next_int_in_range(1, 100)`. If $\le 40\%$, draws factor in `1..5`.
3. **Plague roll:** `pcg.next_int_in_range(1, 100)`. If $\le 15\%$, plague strikes and halves surviving population.
4. **Starvation & impeachment:** If $> 45\%$ starve in one year, ruler is impeached. If 0 survive, city is extinct.
5. **Immigration draw:** `pcg.next_int_in_range(1, 5)`. Only arrives if no one starved and ruler is not impeached.
6. **Next year land price:** `pcg.next_int_in_range(17, 26)`.
7. **End of Year 10 rating:** Evaluates ruler performance (`Charlemagne`, `Acre`, `Nero`, `Disaster`).

## Running Tests

```bash
rake test
```

Or using `bundle`:

```bash
bundle exec rake test
```

## License

MIT
