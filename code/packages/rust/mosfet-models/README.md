# mosfet-models

SPICE Level-1 (Shockley) MOSFET I-V model with full small-signal parameter extraction, body effect, subthreshold conduction, and PMOS sign conventions.

## What it does

This crate implements the classical square-law MOSFET model used in introductory SPICE circuit analysis:

- **`Level1Params`** — 18 parameters (VT0, KP, LAMBDA, GAMMA, PHI, W, L, capacitances, temperature, zero-default KF flicker noise, and unit-default AF exponent) with textbook defaults for a 130 nm NMOS.
- **`evaluate_level1`** — core I-V evaluation returning `MosResult` with `Id`, `gm`, `gds`, `gmb`, gate/body capacitances, and the operating `Region`.
- **`Region`** — `Cutoff`, `Subthreshold`, `Triode`, `Saturation`.
- **`MosfetType`** — `Nmos` / `Pmos`.
- **`Mosfet`** — high-level wrapper that applies PMOS sign conventions automatically.
- **`Level1Model`** — direct wrapper around `Level1Params` for compatibility with external model card parsers.

## Operating regions

| Region      | Condition         | Id                                         |
|-------------|-------------------|--------------------------------------------|
| Cutoff      | V_OV ≤ 0         | 0 (or subthreshold exp when enabled)       |
| Triode      | 0 < V_DS < V_OV   | β(V_OV V_DS − V_DS²/2)(1 + λV_DS)        |
| Saturation  | V_DS ≥ V_OV      | (β/2) V_OV² (1 + λV_DS)                   |

where β = KP × W/L and V_OV = V_GS − V_t.

## Usage

```rust
use mosfet_models::{Mosfet, MosfetType, Level1Params, evaluate_level1};

// Default 130 nm NMOS
let params = Level1Params::default();
let result = evaluate_level1(&params, 1.8, 1.8, 0.0, 300.15);
println!("Region: {}", result.region.as_str());  // "saturation"
println!("Id = {:.3e} A", result.id);
println!("gm = {:.3e} A/V", result.gm);

// High-level wrapper with PMOS sign convention
let pmos = Mosfet::new(MosfetType::Pmos, Level1Params::default());
let r = pmos.dc(-1.8, -1.8, 0.0, 300.15);
assert!(r.id < 0.0);  // conventional PMOS drain current
```

## How it fits in the stack

```
device-physics  ←  thermal_voltage() used in subthreshold model
      │
mosfet-models   ←  Level-1 I-V + PMOS wrapper (this crate)
      │
spice-engine    ←  MNA matrix stamping for MOSFET elements
```
