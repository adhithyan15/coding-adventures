# hamurabi (Go)

Pure, deterministic, platform-independent simulation core for the seminal 1968 mainframe resource allocation game.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME01`).

---

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. It introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER15`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`os.OpenFile`), network (`net`), process (`os/exec`), environment variable (`os.Getenv`), system time (`time.Now`), terminal I/O (`os.Stdin`), or system random (`crypto/rand`) access.
- **Pure State Transition:** State transitions follow a pure mathematical function:
  ```go
  StepYear(state HamurabiState, order TurnOrder) (HamurabiState, error)
  ```
- **Bit-Exact Cross-Language Determinism:** All state transitions use exclusively bounded integer types (`uint8`, `uint32`, `uint64`, `int32`). By avoiding IEEE-754 floating point arithmetic, transitions produce bit-exact identical trajectories across all 18 programming languages in the repository.
- **Explicit PRNG State:** All pseudo-randomness enters deterministically through explicit PCG32 state (`RngState: uint64`, `RngInc: uint64`) stored directly inside [`HamurabiState`].
- **Zero Allocation Budget (`VER07`):** The simulation step `StepYear` and order validation `ValidateOrder` operate purely on value structs and perform **zero heap allocations** on the simulation hot path (`testing.AllocsPerRun == 0`).
- **Complete Concurrency Safety:** Value semantics ensure that concurrent goroutines calling `StepYear` on shared states run with zero shared mutable state and zero data races.
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes (e.g. CWE-190 overflow prevention via checked 64-bit bounds and saturating additions, CWE-400/835 infinite loop prevention by rejecting even `RngInc`, and `-math.MinInt32` signed wrap prevention).

---

## Domain Types & API

### 1. `HamurabiState`

Immutable value record representing the city-state of Lagash:

```go
type HamurabiState struct {
    Year               uint8      `json:"year"`                 // 1..=10 (playable), 11 (terminal summary)
    Population         uint32     `json:"population"`           // City-state population (initial: 100)
    Bushels            uint32     `json:"bushels"`              // Grain in storage (initial: 2800)
    Acres              uint32     `json:"acres"`                // Farmland owned (initial: 1000)
    LandPrice          uint32     `json:"land_price"`           // Bushels per acre (17..=26)
    HarvestPerAcre     uint32     `json:"harvest_per_acre"`     // Yield from previous harvest (1..=5)
    RatsEaten          uint32     `json:"rats_eaten"`           // Bushels destroyed by rats (initial: 200)
    StarvedLastYear    uint32     `json:"starved_last_year"`    // Citizens starved in previous year
    StarvedTotal       uint32     `json:"starved_total"`        // Cumulative citizens starved across reign
    PercentStarvedSum  uint32     `json:"percent_starved_sum"`  // Sum of starvation percentages
    ImmigrantsLastYear uint32     `json:"immigrants_last_year"` // New citizens arriving in previous year
    PlagueStruck       bool       `json:"plague_struck"`        // Whether plague halved population
    Status             GameStatus `json:"status"`               // Current simulation lifecycle status
    RngState           uint64     `json:"rng_state"`            // Internal 64-bit PCG32 state
    RngInc             uint64     `json:"rng_inc"`              // Internal 64-bit odd PCG32 increment
}
```

### 2. `TurnOrder`

Player orders for a single year:

```go
type TurnOrder struct {
    AcresDelta    int32  `json:"acres_delta"`     // Acres to buy (>0) or sell (<0)
    BushelsToFeed uint32 `json:"bushels_to_feed"` // Bushels allocated to feed citizens (20/person)
    AcresToPlant  uint32 `json:"acres_to_plant"`  // Acres to plant (1 person / 10 acres, 1 bushel / 2 acres)
}
```

### 3. `GameStatus` & `Rating`

```go
type Rating int
const (
    RatingCharlemagne Rating = iota // Outstanding: avg starved <= 3% && acres/person >= 10
    RatingAcre                      // Reasonable: avg starved <= 10% && acres/person >= 9
    RatingNero                      // Poor: avg starved <= 33% && acres/person >= 7
    RatingDisaster                  // Atrocious: avg starved > 33% || acres/person < 7
)

type GameStatusKind int
const (
    StatusPlaying GameStatusKind = iota
    StatusWon
    StatusImpeached
    StatusGameOverExtinction
)
```

### 4. `HamurabiError`

Closed error set:

- `ErrGameAlreadyEnded`
- `ErrInvalidOrderBounds`
- `ErrInsufficientBushelsForLand`
- `ErrInsufficientAcresToSell`
- `ErrInsufficientBushelsToFeed`
- `ErrInsufficientAcresToPlant`
- `ErrInsufficientPopulationToPlant`
- `ErrInsufficientGrainForSeed`

---

## Verification & Build

```bash
go vet ./...
go test ./... -v -cover
go test -bench=. -benchmem
```
