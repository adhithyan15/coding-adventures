# @coding-adventures/lunar-lander

Pure, deterministic simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation (GAME02).

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer, a high school student at Lexington High School in Massachusetts, on a DEC PDP-8 in FOCAL-69. Inspired by Apollo 11, Storer modelled the physics of the Apollo Lunar Module (LM) descent stage.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), becoming an enduring computing classic.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER15`, `VER27`):
- **Zero Ambient Authority:** `"capabilities": []` declared in `required_capabilities.json`. Zero filesystem, network, process, or system time access.
- **Pure Fixed-Point Kinematics:** Discrete physics executed using pure integer arithmetic (`BigInt`) in millifeet ($1\text{ ft} = 1{,}000\text{ mft}$) and seconds, with zero floating-point operations in the core simulation.
- **Euclidean Division Parity:** Exact `divEuclid(a, b)` implementation ensuring non-negative remainder $r \in [0, |b|)$, guaranteeing bit-for-bit parity with Rust's `div_euclid`.
- **Bitwise Integer Square Root:** Iterative 64-step bitwise binary square root (`isqrtU128`) operating on 128-bit unsigned integers without division operations (`CWE-369`).
- **Touchdown Contact Solver:** Exact quadratic touchdown contact-time solver and landing outcome classification (`Perfect`, `Hard`, `Crash`, `Fatal`, `Crater`) with automatic unburned propellant refund.
- **Declarative 2D Telemetry:** Exposes `to_paint_scene` generating a 76x5 intermediate representation `PaintScene` and `render_telemetry_ascii` producing an authentic teletype dashboard HUD.
- **Weakness Closure (`VER15`):** Complete closure against prototype pollution (`CWE-1321`), integer overflow (`CWE-190`), division by zero (`CWE-369`), and IEEE-754 precision loss (`CWE-681`).
- **Immutable Frozen Records (`VER04m`):** Every returned `LanderState` and `PaintScene` is frozen via `Object.freeze()`.

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
  new_flight,
  validate_burn,
  step_burn,
  render_telemetry_ascii,
  type BurnCommand,
} from "@coding-adventures/lunar-lander";

// Initialize Apollo Lunar Module descent at 120,000 ft
let state = new_flight();

// Command engine burn: 50 lbs/s for 5 seconds
const cmd: BurnCommand = {
  burn_rate_lbs_s: 50,
  duration_seconds: 5,
};

// Validate command bounds without advancing simulation
const val = validate_burn(state, cmd);
if (!val.ok) {
  console.error("Burn order refused:", val.error);
}

// Step flight simulation
const res = step_burn(state, cmd);
if (res.ok) {
  state = res.value;
  console.log(render_telemetry_ascii(state, cmd.burn_rate_lbs_s));
}
```
