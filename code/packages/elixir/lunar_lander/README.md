# CodingAdventures.LunarLander

Pure, deterministic, platform-independent simulation core and ASCII telemetry for the classic discrete Lunar Module flight simulation in Elixir.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME02`).

---

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by Apollo 11, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), becoming an enduring computing classic.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system clock (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **Pure Functional State Transition:** State transitions are pure, referentially transparent mathematical functions without side effects or process dictionary mutation:
  ```elixir
  CodingAdventures.LunarLander.step_burn(state, cmd) :: {:ok, LanderState.t()} | {:error, error_reason()}
  ```
- **Bit-Exact Cross-Language Determinism:** All state transitions use exclusively bounded integer arithmetic (`div` truncating toward zero, Euclidean division with non-negative remainder, 128-bit bitwise square root). Produces bit-exact identical trajectories across all programming languages in the repository.
- **Touchdown Contact Solver & Fuel Refund:** When the projected altitude reaches or penetrates the lunar surface ($Y_{t+\Delta t} \le 0$), solves for exact contact time $\delta t \in [1, \Delta t]$ and terminal impact velocity, automatically refunding unburned propellant for any unused interval fraction.
- **Declarative 76x5 2D Telemetry & ASCII HUD:** Exposes `to_paint_scene/2` generating a structured 76x5 `PaintScene` and `render_telemetry_ascii/2` executing character-grid rasterization (overlaying block characters over background bar gauges, indexed by Unicode character cells).
- **26-CWE Matrix Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

### BEAM-Specific Security Hardening

1. **CWE-400 (Atom Table Exhaustion):** Zero calls to `String.to_atom/1` or dynamic atom creation. All user inputs, string reports, and JSON decodings match strictly on existing static atoms or explicit string patterns; `:json.decode/1` is used without dynamic atoms.
2. **CWE-843 (Type Confusion):** Guarded with `is_integer/1` across all numerical fields, strictly rejecting floats (e.g. `50.0`), booleans (`true`/`false`), `nil`, binaries, and atoms.
3. **CWE-190 (Integer Overflow):** Enforces strict coordinate boundaries (`altitude_mft <= 200_000_000`, `|velocity_mft_s| <= 10_000_000`, `time_seconds <= 86_400`, `burn_rate_lbs_s <= 200`, `duration_seconds in 1..10`).
4. **CWE-369 (Division by Zero):** Rejects corrupted dry mass (`dry_mass_lbs != 16_500`), and `isqrt_u128` uses an entirely division-free binary square root algorithm.
5. **CWE-94 / CWE-502 (Code / Binary Deserialization Injection):** Zero usage of `Code.eval_*`, `Code.string_to_quoted`, or `:erlang.binary_to_term`.

---

## Domain Types & Data Structures

### 1. `CodingAdventures.LunarLander.LanderState`

Immutable struct representing the complete flight simulation state:

| Field | Type | Description |
|---|---|---|
| `time_seconds` | `0..86_400` | Elapsed mission flight time in seconds |
| `altitude_mft` | `0..200_000_000` | Altitude above lunar surface in millifeet (1 ft = 1,000 mft) |
| `velocity_mft_s` | `-10_000_000..10_000_000` | Vertical velocity in millifeet/s (+ ascending, - descending) |
| `fuel_lbs` | `0..16_500` | Remaining usable propellant in pounds |
| `dry_mass_lbs` | `16_500` | Constant dry mass of the Apollo LM descent stage |
| `status` | `flight_status()` | `:in_flight` or `{:landed, outcome}` |
| `impact_velocity_mft_s` | `integer() \| nil` | Vertical touchdown impact velocity in millifeet/s (`nil` in flight) |

### 2. `CodingAdventures.LunarLander.BurnCommand`

Pilot burn order for a discrete simulation turn:

| Field | Type | Description |
|---|---|---|
| `burn_rate_lbs_s` | `0..200` | Engine propellant burn rate in pounds per second |
| `duration_seconds` | `1..10` | Duration of burn interval in integer seconds |

### 3. `LandingOutcome` & `FlightStatus`

- `:in_flight`
- `{:landed, :perfect}` — 0 to 5 ft/s (0 to 5,000 mft/s)
- `{:landed, :hard}` — 5.001 to 15 ft/s (5,001 to 15,000 mft/s)
- `{:landed, :crash}` — 15.001 to 30 ft/s (15,001 to 30,000 mft/s)
- `{:landed, :fatal}` — 30.001 to 60 ft/s (30,001 to 60,000 mft/s)
- `{:landed, :crater}` — > 60 ft/s (> 60,000 mft/s)

### 4. `LanderError`

Burn command refusal error reasons:
- `:flight_already_ended`
- `:excessive_burn_rate`
- `:invalid_duration`
- `:arithmetic_overflow`

---

## API Overview

```elixir
alias CodingAdventures.LunarLander
alias CodingAdventures.LunarLander.{BurnCommand, LanderState}

# Initialize a new flight at standard Apollo LM descent conditions
flight = LunarLander.new_flight()

# Render the 76x5 ASCII telemetry HUD
IO.puts(LunarLander.render_telemetry(flight))

# Validate a burn command without modifying state
cmd = %BurnCommand{burn_rate_lbs_s: 50, duration_seconds: 5}
:ok = LunarLander.validate_burn(flight, cmd)

# Advance simulation by one discrete turn
case LunarLander.step_burn(flight, cmd) do
  {:ok, next_state} ->
    IO.puts(LunarLander.render_telemetry_ascii(next_state, cmd.burn_rate_lbs_s))

  {:error, reason} ->
    IO.puts("Burn rejected: #{LunarLander.error_to_string(reason)}")
end
```

---

## Telemetry HUD Display

```text
┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐
│ TIME:      0 SEC    │ ALTITUDE: 120,000 FT [████████████████████] 100%   │
│ VELOCITY: -3000 FT/S│ FUEL:     16,500 LBS [████████████████████] 100%   │
│ BURN:        0 LB/S │ RATE:   DESCENT  ▼                                 │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Testing & Verification

```bash
# Verify formatting
mix format --check-formatted

# Compile test environment with warnings as errors
MIX_ENV=test mix compile --warnings-as-errors

# Run full test suite with code coverage
mix test --cover
```
