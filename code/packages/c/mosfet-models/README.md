# mosfet-models (C)

**CCPP02 port campaign — bucket A (pure-ISO).** The classical SPICE **Level-1
(Shockley) MOSFET** I-V model — the square-law model you use for hand
calculations and canonical CMOS smoke tests (inverter, ring oscillator). The C
port of the Rust `mosfet-models` crate, a pure-ISO crate that needs no OS, so it
rides the `iso-harness` (links nothing, strict-conformance flags on).

Given a bias point (V_GS, V_DS, V_BS, T) it returns the drain current, the
small-signal conductances (gm / gds / gmb), the Meyer intrinsic + overlap
capacitances, and the operating region.

```c
MosLevel1Params p;
mos_level1_params_default(&p);              /* 130 nm NMOS at room temperature */

MosResult r = mosfet_dc(MOS_NMOS, &p, 1.8, 1.8, 0.0, 300.15);
/* r.region == MOS_REGION_SATURATION; r.id > 0; r.gm, r.gds, … populated */
```

| Region | Condition | Id |
|--------|-----------|----|
| Cutoff | V_OV ≤ 0 | 0 (or the subthreshold exp if enabled) |
| Triode | 0 < V_DS < V_OV | β(V_OV·V_DS − V_DS²/2)(1 + λV_DS) |
| Saturation | V_DS ≥ V_OV | (β/2)·V_OV²(1 + λV_DS) |

where β = KP·(W/L) and V_OV = V_GS − V_t.

| Function | Purpose |
|----------|---------|
| `mos_level1_params_default` | fill the default parameter set |
| `mos_evaluate_level1` | evaluate at a bias point (NMOS convention) |
| `mosfet_dc` | evaluate an NMOS or PMOS device (PMOS sign flips handled) |
| `mos_region_str` | region name string |

## Composes `c/device-physics` + `c/float-math`

The thermal voltage kT/q comes from [`device-physics`](../device-physics)
(`dp_thermal_voltage`), and the `sqrt`/`exp` from the from-scratch
[`float-math`](../float-math) (`fm_sqrt` / `fm_exp`). The model needs no libm —
`run.sh` compiles both packages' sources in and nothing is linked.

## Model notes

- **Body effect.** V_t = VT0 + γ(√(Φ − V_BS) − √Φ), valid for Φ ≥ V_BS; under
  strong forward body bias V_t is clamped to VT0. `gmb = −gm·dV_t/dV_BS`.
- **Subthreshold.** When enabled, below threshold Id follows
  β·n·V_T²·exp(V_OV/(n·V_T))·(1 − exp(−V_DS/V_T)), smoothly matching strong
  inversion at V_OV ≈ 0; otherwise the device is in hard cutoff (Id = 0).
- **Channel-length modulation** via λ; **Meyer** piecewise gate capacitances plus
  W/L-scaled overlap caps.
- **PMOS** (`mosfet_dc(MOS_PMOS, …)`) negates the input voltages before
  evaluation and negates the resulting Id, so the caller sees conventional PMOS
  polarity. (The Rust `Level1Model` — parameters + `dc` — is just
  `mos_evaluate_level1`.)

Everything is by value: no allocation, no OS, no libm.

## Build & test

```sh
cd code/packages/c/mosfet-models
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 45 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
mosfet-models/
├── include/mosfet_models/mosfet_models.h   # public API
├── src/mosfet_models.c                       # the model — one pure-ISO source
├── tests/mosfet_models_test.c                # the Rust tests (regions, gm/gds/gmb, PMOS, caps)
├── tools/run.sh  · run.ps1                     # build via iso-harness (+ device-physics, float-math)
├── BUILD  · BUILD_windows                      # deps: c/iso-harness c/device-physics c/float-math
└── .gitignore
```
