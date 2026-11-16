# lunar-lander

Pure, deterministic, platform-independent simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME02`).

---

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by the Apollo 11 moon landing in July 1969, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage. Players entered burn rates per 10-second interval, attempting to gently touch down before exhausting fuel.
- **1970–1973 — DEC BASIC & "101 BASIC Computer Games":** Ported to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), spawning dozens of graphical and text-based lunar lander adaptations across every personal computing platform.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **Pure State Transition:** State transitions follow a pure mathematical function:
  ```python
  step_burn(state: LanderState, cmd: BurnCommand) -> LanderState
  ```
- **Bit-Exact Cross-Language Determinism:** All orbital kinematics and state transitions use exclusively integer-scaled fixed-point coordinates with zero floating point operations in the simulation core:
  - Altitude ($Y$): Millifeet ($1\text{ ft} = 1,000\text{ mft}$).
  - Velocity ($V$): Millifeet per second ($1\text{ ft/s} = 1,000\text{ mft/s}$).
  - Acceleration ($A$): Millifeet per second squared ($\text{mft/s}^2$).
  - Fuel Mass ($F$): Pounds ($\text{lbs}$).
- **Zero Runtime Dependencies:** Fully standalone pure Python implementation using only the standard library.
- **Declarative 2D Telemetry (`P2D02`):** Presentation bridge translates flight state into a declarative `PaintScene` rendered through terminal ASCII HUD.
- **26-CWE Weakness Closure (`VER15`):** Complete closure of CWE-190 (Integer Overflow), CWE-369 (Divide by Zero), and CWE-843 (Type Confusion).

---

## Domain Types & API

### 1. `LanderState`

Immutable `@dataclass(frozen=True, slots=True)` value record capturing the complete flight state:

```python
@dataclass(frozen=True, slots=True)
class LanderState:
    time_seconds: int
    altitude_mft: int
    velocity_mft_s: int
    fuel_lbs: int
    dry_mass_lbs: int
    status: FlightStatus
    impact_velocity_mft_s: int | None = None
```

### 2. `BurnCommand`

Pilot burn command for a discrete turn:

```python
@dataclass(frozen=True, slots=True)
class BurnCommand:
    burn_rate_lbs_s: int
    duration_seconds: int
```

### 3. `FlightStatus` & `LandingOutcome`

```python
class LandingOutcome(StrEnum):
    Perfect = "Perfect"  # 0 to 5 ft/s
    Hard = "Hard"  # 5.001 to 15 ft/s
    Crash = "Crash"  # 15.001 to 30 ft/s
    Fatal = "Fatal"  # 30.001 to 60 ft/s
    Crater = "Crater"  # > 60 ft/s
```

### 4. `LanderError`

Hierarchy of simulation errors:

- `FlightAlreadyEnded`: Attempted burn after touchdown.
- `ExcessiveBurnRate`: Burn rate exceeds engine capacity ($B > 200\text{ lbs/s}$).
- `InvalidDuration`: Duration is 0 or exceeds turn limit ($1 \le \Delta t \le 10$).
- `ArithmeticOverflow`: Arithmetic overflow or coordinate boundary violation.

---

## Usage

```python
from lunar_lander import (
    BurnCommand,
    FlightStatus,
    new_flight,
    render_telemetry_ascii,
    step_burn,
    validate_burn,
)

state = new_flight()
print(render_telemetry_ascii(state, last_burn_rate_lbs_s=0))

# Execute a 50 lbs/s burn for 10 seconds
cmd = BurnCommand(burn_rate_lbs_s=50, duration_seconds=10)
validate_burn(state, cmd)
next_state = step_burn(state, cmd)
print(render_telemetry_ascii(next_state, last_burn_rate_lbs_s=50))
```
