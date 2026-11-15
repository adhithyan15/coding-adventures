# @coding-adventures/hamurabi

Pure, deterministic simulation core for the seminal 1968 Hamurabi mainframe resource allocation game.

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. Introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):
- **Zero Ambient Authority:** `"capabilities": []` declared in `required_capabilities.json`. Zero filesystem, network, process, or time access.
- **Pure State Transition:** `step_year(state, order) -> Result<HamurabiState, HamurabiError>`. State transitions are side-effect free.
- **Explicit Deterministic PRNG:** Internal PCG32 XSH RR 64/32 generator seeded with stream ID 54 (`inc = 109n`). 64-bit state arithmetic is executed via native `BigInt` (`BigInt.asUintN(64, ...)`), strictly eliminating JavaScript IEEE-754 precision loss beyond $2^{53}-1$.
- **Pure Integer Arithmetic:** Uses strictly bounded integer arithmetic without floating-point math, producing bit-exact identical trajectories across all 18 implementation languages.
- **Weakness Closure (`VER15`):** Complete closure against prototype pollution (CWE-1321), integer overflow (CWE-190), type confusion (CWE-843), infinite loops from collapsed PRNG period (CWE-400 / CWE-835), and unsafe float conversions (CWE-681).
- **Immutable Value Records:** Every returned `HamurabiState` is deeply immutable and frozen via `Object.freeze()`.

## Installation & Build

```bash
# Build
npm ci --silent
npm run build

# Test
npm test
```

## Usage

```typescript
import {
  new_game,
  validate_order,
  step_year,
  format_report,
  HamurabiError,
  type TurnOrder,
} from "@coding-adventures/hamurabi";

// Initialize Year 1 with seed
const state = new_game(42n);

// Plan Year 1 orders
const order: TurnOrder = {
  acres_delta: 0,          // No land bought or sold
  bushels_to_feed: 2000,   // 20 bushels per person (100 population)
  acres_to_plant: 1000,    // Sow 1000 acres (requires 500 bushels seed)
};

// Validate orders without advancing state
const val = validate_order(state, order);
if (!val.ok) {
  console.error("Order refused:", val.error);
}

// Advance simulation by 1 year
const res = step_year(state, order);
if (res.ok) {
  const nextState = res.value;
  console.log(format_report(nextState));
}
```

## API Reference

### `new_game(seed: bigint | number): HamurabiState`
Initializes a new 10-year reign with baseline parameters (100 people, 2800 bushels, 1000 acres, initial land price in [17, 26]).

### `validate_order(state: HamurabiState, order: TurnOrder): Result<void, HamurabiError>`
Validates player orders against current state constraints and canonical resource bounds without modifying state.

### `step_year(state: HamurabiState, order: TurnOrder): Result<HamurabiState, HamurabiError>`
Advances the simulation by one year following the locked PRNG draw sequence (harvest yield, rat infestation, plague, starvation, immigration, next land price).

### `format_report(state: HamurabiState): string`
Formats an annual teletype status report or terminal 10-year performance evaluation in authentic uppercase teletype typography.
