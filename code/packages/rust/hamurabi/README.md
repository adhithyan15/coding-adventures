# hamurabi

Pure, deterministic, platform-independent simulation core for the seminal 1968 mainframe resource allocation game.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME01`).

---

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. It introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Pure State Transition:** State transitions follow a pure mathematical function:
  ```rust
  step_year(state: &HamurabiState, order: &TurnOrder) -> Result<HamurabiState, HamurabiError>
  ```
- **Bit-Exact Cross-Language Determinism:** All state transitions use exclusively bounded integer types (`u8`, `u32`, `u64`, `i32`). By avoiding IEEE-754 floating point arithmetic, transitions produce bit-exact identical trajectories across all 18 programming languages in the repository.
- **Explicit PRNG State:** All pseudo-randomness enters deterministically through explicit PCG32 state (`rng_state: u64`, `rng_inc: u64`) stored inside [`HamurabiState`].
- **Zero Allocation Budget (`VER07`):** The simulation step `step_year` and order validation `validate_order` operate purely on value records and perform **zero heap allocations** on the simulation hot path (`max_alloc_bytes: Some(0)`).
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

---

## Domain Types & API

### 1. `HamurabiState`

Immutable value record representing the city-state of Lagash at a given turn:

```rust
pub struct HamurabiState {
    pub year: u8,                   // 1..=10 (playable), 11 (terminal summary)
    pub population: u32,             // City-state population (initial: 100)
    pub bushels: u32,                // Grain in storage (initial: 2800)
    pub acres: u32,                  // Farmland owned (initial: 1000)
    pub land_price: u32,             // Bushels per acre (17..=26)
    pub harvest_per_acre: u32,       // Yield from previous harvest (1..=5)
    pub rats_eaten: u32,             // Bushels destroyed by rats (initial: 200)
    pub starved_last_year: u32,      // Citizens starved in previous year
    pub starved_total: u32,          // Cumulative citizens starved across reign
    pub percent_starved_sum: u32,    // Cumulative starvation percentages
    pub immigrants_last_year: u32,   // New citizens arrived in previous year
    pub plague_struck: bool,         // Whether plague struck in previous year
    pub status: GameStatus,          // Current lifecycle status
    pub rng_state: u64,              // PCG32 PRNG state
    pub rng_inc: u64,                // PCG32 sequence increment (odd integer)
}
```

### 2. `TurnOrder`

Player orders for the upcoming year:

```rust
pub struct TurnOrder {
    pub acres_delta: i32,            // Acres to buy (>0) or sell (<0)
    pub bushels_to_feed: u32,        // Grain allocated for food (20 bu/person)
    pub acres_to_plant: u32,         // Acres to sow (1 bu seeds 2 acres; 1 person tends 10 acres)
}
```

### 3. `GameStatus` & `Rating`

```rust
pub enum GameStatus {
    Playing,
    Won { rating: Rating },
    Impeached { starved_this_turn: u32, percent: u32 },
    GameOverExtinction,
}

pub enum Rating {
    Charlemagne,  // Avg starvation <= 3% AND acres per person >= 10
    Acre,         // Avg starvation <= 10% AND acres per person >= 9
    Nero,         // Avg starvation <= 33% AND acres per person >= 7
    Disaster,     // Avg starvation > 33% OR acres per person < 7
}
```

### 4. `HamurabiError`

Closed enum returned upon order rejection:

- `GameAlreadyEnded`: Attempted order after game completion or impeachment.
- `InvalidOrderBounds`: Parameter bounds violated or arithmetic overflow.
- `InsufficientBushelsForLand`: Lack bushels to purchase specified acreage.
- `InsufficientAcresToSell`: Attempted to sell more land than owned.
- `InsufficientBushelsToFeed`: Bushels to feed exceed bushels remaining after land trade.
- `InsufficientAcresToPlant`: Acres to plant exceed owned acreage after land trade.
- `InsufficientPopulationToPlant`: Acres to plant exceed labor capacity (`population * 10`).
- `InsufficientGrainForSeed`: Grain remaining after feeding insufficient to seed fields (`ceil(acres / 2)`).

---

## Example Usage

```rust
use hamurabi::{new_game, step_year, format_report, TurnOrder, GameStatus};

fn main() {
    // Initialize a new deterministic reign seeded with seed 1337
    let mut state = new_game(1337);
    println!("{}", format_report(&state));

    // Place an order for Year 1
    let order = TurnOrder {
        acres_delta: 0,
        bushels_to_feed: 2000,
        acres_to_plant: 800,
    };

    match step_year(&state, &order) {
        Ok(next_state) => {
            state = next_state;
            println!("{}", format_report(&state));
        }
        Err(e) => eprintln!("Order rejected: {}", e),
    }
}
```

---

## Building and Testing

```bash
# Unit & integration tests in debug profile
cargo test

# Unit & integration tests in release profile (verifies CWE-190 overflow checks)
cargo test --release

# VER07 microbenchmark suite (verifies zero heap allocations on hot path)
cargo test --test microbenchmarks

# VER08b runtime capability gate
cargo test --test runtime_capability

# Static capability verification
cargo run -p coding_adventures_ca_capability_analyzer -- --dir . --check-module
cargo run -p coding_adventures_ca_capability_analyzer -- --dir . --verify-cert effect_certificate.json
```
