# coding-adventures-hamurabi (Lua)

Pure, deterministic simulation core for the seminal 1968 mainframe resource
allocation game `hamurabi`.

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited
  female video game designer and writer) and programmed by William McKay for
  an IBM 7090 mainframe at BOCES in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8
  in FOCAL-69, shortening the historical Sumerian ruler name to fit PDP-8
  teletype 8-character symbol constraints.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H.
  Ahl and published in DEC's landmark book, becoming the archetypal resource
  management simulation game.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER15`):
- **Zero Ambient Authority:** `"capabilities": []` declared in `required_capabilities.json`. Zero filesystem, network, process, or environment access.
- **Pure State Transition:** `step_year(state, order) -> (HamurabiState, nil)` or `(nil, HamurabiError)`.
- **Explicit PCG32 Pseudorandomness:** All simulation draws enter deterministically through explicit PCG32 state (`stream_id = 54`, `inc = 109`).
- **Bit-Exact Cross-Language Determinism:** Pure integer arithmetic using 64-bit integer bit patterns (`math.type(x) == "integer"`) with zero floating-point operations.
- **Immutable Fresh Records (`VER04m`):** State transitions return a newly allocated table without modifying inputs.
- **Complete 26-CWE Weakness Closure:** Rigorous bounds checking against integer overflow (CWE-190), infinite loops (CWE-835 / CWE-400), and type confusion (CWE-843).

## API

```lua
local hamurabi = require("coding_adventures.hamurabi")

-- Start a new game with deterministic seed
local state = hamurabi.new_game(1337)

-- Inspect current status
print(hamurabi.format_report(state))

-- Validate order
local order = {
    acres_delta = 0,
    bushels_to_feed = 2000,
    acres_to_plant = 1000,
}
local ok, err = hamurabi.validate_order(state, order)
if not ok then
    error("Order rejected: " .. err)
end

-- Step simulation by 1 year
local next_state, err = hamurabi.step_year(state, order)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires `busted` installed via LuaRocks.
