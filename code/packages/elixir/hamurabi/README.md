# CodingAdventures.Hamurabi

Pure, deterministic, platform-independent simulation core for the seminal 1968 mainframe resource allocation game in Elixir.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME01`).

---

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. It introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system clock (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **Pure Functional State Transition:** State transitions are pure, referentially transparent mathematical functions without side effects or process dictionary mutation:
  ```elixir
  CodingAdventures.Hamurabi.step_year(state, order) :: {:ok, State.t()} | {:error, error_reason()}
  ```
- **Bit-Exact Cross-Language Determinism:** All state transitions use exclusively bounded integer types (`u8`, `u32`, `u64`, `i32`). By avoiding IEEE-754 floating point arithmetic, transitions produce bit-exact identical trajectories across all 18 programming languages in the repository.
- **Explicit PRNG State:** All pseudo-randomness enters deterministically through explicit PCG32 state (`rng_state`, `rng_inc`) stored inside [`CodingAdventures.Hamurabi.State`].
- **26-CWE Matrix Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

### BEAM-Specific Security Hardening

1. **CWE-400 (Atom Table Exhaustion):** Zero calls to `String.to_atom/1` or dynamic atom creation. All user inputs, string reports, and JSON decodings match strictly on existing static atoms or explicit string patterns.
2. **CWE-843 (Type Confusion):** Guarded with `is_integer/1` across all functions, strictly rejecting floats (e.g. `1.5`), booleans (`true`/`false`), `nil`, binaries, and atoms.
3. **CWE-190 (Integer Overflow):** Explicit 32-bit and 64-bit bounds checking. Calculations promotional to 64-bit and capped with saturating arithmetic where specified.
4. **CWE-400 / CWE-835 (Infinite Loop / LCG Period Collapse):** Validates that `Bitwise.band(state.rng_inc, 1) == 1` (odd increment), refusing even increments with `{:error, :invalid_order_bounds}`.
5. **CWE-94 / CWE-502 (Code / Binary Deserialization Injection):** Zero usage of `Code.eval_*`, `Code.string_to_quoted`, or `:erlang.binary_to_term`.

---

## Domain Types & Data Structures

### 1. `CodingAdventures.Hamurabi.State`

Immutable struct representing the state of Lagash:

| Field | Type | Description |
|---|---|---|
| `year` | `1..11` | Current reign year (1..10 playable, 11 terminal summary) |
| `population` | `non_neg_integer()` | Population of the city-state (initial: 100) |
| `bushels` | `non_neg_integer()` | Bushels of grain in storage (initial: 2800) |
| `acres` | `non_neg_integer()` | Acres of farmland owned (initial: 1000) |
| `land_price` | `17..26` | Current market price of land in bushels per acre |
| `harvest_per_acre` | `1..5` | Yield from previous harvest in bushels per acre |
| `rats_eaten` | `non_neg_integer()` | Bushels consumed by rats in previous year |
| `starved_last_year` | `non_neg_integer()` | Citizens starved in previous year |
| `starved_total` | `non_neg_integer()` | Cumulative citizens starved across reign |
| `percent_starved_sum` | `non_neg_integer()` | Cumulative sum of yearly starvation percentages |
| `immigrants_last_year`| `non_neg_integer()` | New citizens arrived in previous year |
| `plague_struck` | `boolean()` | Whether plague struck in previous year |
| `status` | `status()` | Lifecycle status (`:playing`, `{:won, rating}`, `{:impeached, starved, percent}`, `:game_over_extinction`) |
| `rng_state` | `non_neg_integer()` | 64-bit unsigned PCG32 state |
| `rng_inc` | `non_neg_integer()` | 64-bit unsigned PCG32 increment (odd integer) |

### 2. `CodingAdventures.Hamurabi.TurnOrder`

Player orders for a year:

| Field | Type | Description |
|---|---|---|
| `acres_delta` | `integer()` | Acres to buy (positive) or sell (negative) |
| `bushels_to_feed` | `non_neg_integer()` | Bushels allocated for food (20 bu/person) |
| `acres_to_plant` | `non_neg_integer()` | Acres to sow (1 bu seeds 2 acres; 1 person tends 10 acres) |

### 3. `GameStatus` & `Rating`

- `:playing`
- `{:won, :charlemagne | :acre | :nero | :disaster}`
- `{:impeached, starved_this_turn, percent}`
- `:game_over_extinction`

### 4. `HamurabiError`

Order refusal error atoms:
- `:game_already_ended`
- `:invalid_order_bounds`
- `:insufficient_bushels_for_land`
- `:insufficient_acres_to_sell`
- `:insufficient_bushels_to_feed`
- `:insufficient_acres_to_plant`
- `:insufficient_population_to_plant`
- `:insufficient_grain_for_seed`

---

## API Overview

```elixir
alias CodingAdventures.Hamurabi
alias CodingAdventures.Hamurabi.{State, TurnOrder}

# Initialize a new game with a 64-bit seed
state = Hamurabi.new_game(1337)

# Format the 1970s teletype status report
IO.puts(Hamurabi.format_report(state))

# Validate orders without modifying state
order = %TurnOrder{acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800}
:ok = Hamurabi.validate_order(state, order)

# Advance simulation by one year
case Hamurabi.step_year(state, order) do
  {:ok, next_state} ->
    IO.puts(Hamurabi.format_report(next_state))

  {:error, reason} ->
    IO.puts("Order rejected: #{Hamurabi.error_to_string(reason)}")
end
```

---

## Testing

```bash
# Run all test suites
mix test

# Run tests with code coverage
mix test --cover
```
