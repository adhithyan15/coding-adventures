# hamurabi (Python)

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
- **Pure State Transition:** State transitions follow a pure mathematical function:
  ```python
  step_year(state: HamurabiState, order: TurnOrder) -> HamurabiState
  ```
- **Bit-Exact Cross-Language Determinism:** All state transitions use exclusively bounded integer operations (`u8`, `u32`, `u64`, `i32`). By avoiding IEEE-754 floating point arithmetic, transitions produce bit-exact identical trajectories across all 18 programming languages in the repository.
- **Explicit PRNG State:** All pseudo-randomness enters deterministically through explicit PCG32 state (`rng_state: int`, `rng_inc: int`) stored inside `HamurabiState`.
- **Immutable Value Records (`VER04m`):** `HamurabiState` and `TurnOrder` are implemented with `@dataclass(frozen=True, slots=True)`. All mutations produce fresh distinct instances.
- **Strict CWE / CVE Closure:** Comprehensive hardening against CWE-190 (Integer Overflow), CWE-843 (Type Confusion, explicitly rejecting `bool` and float via `type(x) is int`), CWE-400 (Infinite Loop / LCG period collapse), and CWE-502 / CWE-94 (Zero deserialization, `eval`, or `exec`).

---

## Domain Types & API

### 1. `HamurabiState`

Immutable value record representing the city-state of Lagash at a given turn:

```python
@dataclass(frozen=True, slots=True)
class HamurabiState:
    year: int                 # 1..=10 (playable), 11 (terminal summary)
    population: int           # City-state population (initial: 100)
    bushels: int              # Grain in storage (initial: 2800)
    acres: int                # Farmland owned (initial: 1000)
    land_price: int           # Bushels per acre (17..=26)
    harvest_per_acre: int     # Yield from previous harvest (1..=5)
    rats_eaten: int           # Bushels destroyed by rats (initial: 200)
    starved_last_year: int    # Citizens starved in previous year
    starved_total: int        # Cumulative citizens starved across reign
    percent_starved_sum: int  # Cumulative starvation percentages
    immigrants_last_year: int # New citizens arrived in previous year
    plague_struck: bool       # Whether plague struck in previous year
    status: GameStatus        # Current lifecycle status (Playing, Won, Impeached, GameOverExtinction)
    rng_state: int            # PCG32 PRNG state (u64)
    rng_inc: int              # PCG32 sequence increment (odd u64)
```

### 2. `TurnOrder`

Player orders for the upcoming year:

```python
@dataclass(frozen=True, slots=True)
class TurnOrder:
    acres_delta: int          # Acres to buy (>0) or sell (<0)
    bushels_to_feed: int      # Grain allocated for food (20 bu/person)
    acres_to_plant: int       # Acres to sow (1 bu seeds 2 acres; 1 person tends 10 acres)
```

### 3. `GameStatus` & `Rating`

```python
class Rating(str, Enum):
    CHARLEMAGNE = "Charlemagne"  # Avg starvation <= 3% AND acres per person >= 10
    ACRE = "Acre"                # Avg starvation <= 10% AND acres per person >= 9
    NERO = "Nero"                # Avg starvation <= 33% AND acres per person >= 7
    DISASTER = "Disaster"        # Avg starvation > 33% OR acres per person < 7

GameStatus = Playing | Won | Impeached | GameOverExtinction
```

### 4. `HamurabiError`

Closed exception hierarchy returned upon order rejection:

- `GameAlreadyEnded`: Attempted order after game completion or impeachment.
- `InvalidOrderBounds`: Parameter bounds violated, invalid types, or arithmetic overflow.
- `InsufficientBushelsForLand`: Lack bushels to purchase specified acreage.
- `InsufficientAcresToSell`: Attempted to sell more land than owned.
- `InsufficientBushelsToFeed`: Bushels to feed exceed bushels remaining after land trade.
- `InsufficientAcresToPlant`: Acres to plant exceed owned acreage after land trade.
- `InsufficientPopulationToPlant`: Acres to plant exceed labor capacity (`population * 10`).
- `InsufficientGrainForSeed`: Insufficient grain to sow planted acreage (ceil(acres / 2)).

---

## Usage Example

```python
from hamurabi import new_game, step_year, TurnOrder, format_report

# Initialize Year 1 with deterministic seed
state = new_game(seed=42)
print(format_report(state))

# Submit orders: buy 10 acres, feed 2000 bushels, plant 800 acres
order = TurnOrder(acres_delta=10, bushels_to_feed=2000, acres_to_plant=800)
next_state = step_year(state, order)
print(format_report(next_state))
```

---

## Verification & Testing

Tests cover 100% of canonical specifications:
- Full `code/specs/fixtures/hamurabi-v1/cases.json` 10-year trajectories and refusal cases.
- Property-based tests verifying state conservation and invariants.
- Adversarial tests for CWE-190, CWE-843, CWE-400, CWE-502, and CWE-94.
- Static capability analyzer AST audit asserting zero ambient authority.
