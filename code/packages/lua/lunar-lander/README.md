# coding-adventures-lunar-lander (Lua)

Pure, deterministic simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation (GAME02).

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by Apollo 11, Storer modelled the numerical physics of the Apollo Lunar Module (LM) descent stage.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), becoming an enduring computing classic.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER15`, `VER27`):
- **Zero Ambient Authority (`VER08a` / `CAP001`):** `"capabilities": []` declared in `required_capabilities.json`. Strictly zero filesystem, network, process, or environment access.
- **Pure Integer Fixed-Point Physics:** 64-bit integer arithmetic with zero floating-point operations in the simulation core.
- **Immutable Fresh Records (`VER04m`):** State transitions (`step_burn`) return fresh record tables without mutating inputs.
- **Exact Euclidean Division & Square Root:** Exact `div_euclid(a, b)` satisfying $0 \le r < |b|$ and bounded digit-by-digit `isqrt(n)`.
- **Declarative 2D Telemetry (`P2D02`):** Generates structured 76x5 `PaintScene` and renders bit-exact ASCII HUD with bar gauges.
- **Full 26-CWE Weakness Closure (`VER15`):** Rigorous checks against integer overflow (`CWE-190`), division by zero (`CWE-369`), and type confusion (`CWE-843` / `CWE-681`) ensuring every numeric field is a strict Lua 64-bit integer.

## API

```lua
local lander = require("coding_adventures.lunar_lander")

-- Initialize a standard mission
local state = lander.new_flight()

-- Render ASCII telemetry dashboard
print(lander.render_telemetry(state))

-- Command a 5-second burn at 100 lbs/s
local cmd = {
    burn_rate_lbs_s = 100,
    duration_seconds = 5,
}

local ok, err = lander.validate_burn(state, cmd)
if not ok then
    error("Invalid command: " .. err)
end

local next_state, err = lander.step_burn(state, cmd)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
