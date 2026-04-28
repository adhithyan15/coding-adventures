# MOSFET Models

## Overview

`device-physics.md` derives transistor behavior from drift-diffusion + depletion approximation. This spec packages those derivations into **named SPICE-compatible models** at three levels of fidelity: Level-1 (Shockley square-law, ~10 parameters), EKV (smooth all-region, ~30 parameters), and a teaching subset of BSIM3v3 (industry-standard, ~80 of the full 200+ parameters). Every model exposes the same interface — `current(V_GS, V_DS, V_BS)` — so the SPICE engine consumes them interchangeably.

Three models, three audiences:
- **Level-1**: pedagogically clean. Used for hand-derivation, quick sanity checks, and textbook examples. The right choice for the 4-bit adder smoke-test.
- **EKV**: industrial-quality without parameter explosion. Smooth in moderate inversion. The right choice for analog cell characterization.
- **BSIM3v3 (subset)**: enables Sky130 cell characterization. The choice for `standard-cell-library.md` runs.

Models are pure (stateless except for parasitic capacitance state), Python data classes, type-checked under `mypy --strict`. Each model produces both the DC current `I_d` and the small-signal Jacobian `(g_m, g_ds, g_mb, capacitances)` needed by the SPICE engine's Newton-Raphson loop.

## Layer Position

```
device-physics.md
       │ (constants, helpers, threshold formula, square-law)
       ▼
mosfet-models.md  ◀── THIS SPEC
       │ (Level-1, EKV, BSIM3v3-subset; uniform interface)
       ▼
spice-engine.md   ──► standard-cell-library.md
                       (cell characterization runs SPICE
                        with these models on each cell)
```

## Concepts

### Model interface (the ABI)

Every model implements:

```python
def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float) -> "MosResult":
    """Return Id and small-signal parameters at the given operating point."""
```

Returns:

```python
@dataclass(frozen=True)
class MosResult:
    Id: float           # Drain current (A)
    gm: float           # ∂Id/∂Vgs at fixed Vds, Vbs
    gds: float          # ∂Id/∂Vds at fixed Vgs, Vbs
    gmb: float          # ∂Id/∂Vbs at fixed Vgs, Vds (body transconductance)
    Cgs: float          # Gate-source capacitance (F)
    Cgd: float          # Gate-drain capacitance (F)
    Cgb: float          # Gate-body capacitance (F)
    Cbs: float          # Body-source junction capacitance (F)
    Cbd: float          # Body-drain junction capacitance (F)
    region: str         # "cutoff", "subthreshold", "triode", "saturation"
```

The SPICE engine stamps `gm`, `gds`, `gmb` into the MNA Jacobian; capacitances are stamped into the dynamic G(t) matrix. The `region` field is diagnostic only.

### NMOS / PMOS unification

We define models for an NMOS-shaped equation; the PMOS variant is obtained by flipping signs on V_GS, V_DS, V_BS, and I_d before/after the call. A single `MOSFET(type=NMOS|PMOS, model=Level1|EKV|BSIM3, params)` wrapper handles the transformation.

### PVT corners

Each model accepts process and temperature variation through its parameters. A **corner** is a named parameter set:
- `TT` (typical-typical): nominal V_t, mobility.
- `SS` (slow-slow): high V_t, low mobility — worst-case timing.
- `FF` (fast-fast): low V_t, high mobility — worst-case leakage.
- `SF`, `FS`: NMOS slow + PMOS fast and vice versa.

Cell characterization (`standard-cell-library.md`) runs SPICE in each corner across multiple temperatures and supply voltages.

## Level-1 (Shockley)

The classical square law. Square in `(V_GS - V_t)`. Includes channel-length modulation, body effect, subthreshold (optional). About 10 parameters.

### Parameters

| Symbol | Name | Default (NMOS, 130 nm-style) | Units |
|---|---|---|---|
| `VT0` | Threshold at V_BS=0 | 0.42 | V |
| `KP` | Transconductance, μ_n × C_ox | 220e-6 | A/V² |
| `LAMBDA` | Channel-length modulation | 0.05 | 1/V |
| `GAMMA` | Body-effect coefficient | 0.27 | √V |
| `PHI` | Surface potential at threshold (2φ_F) | 0.84 | V |
| `W` | Channel width | 1e-6 | m |
| `L` | Channel length | 130e-9 | m |
| `IS` | Saturation current (body-source/drain diodes) | 1e-15 | A |
| `N_SUB` | Subthreshold slope factor | 1.4 | — |
| `T_NOM` | Nominal temperature | 300.15 | K |

### Equations

```
V_t = VT0 + GAMMA × (sqrt(PHI - V_BS) - sqrt(PHI))     # body-effect
V_OV = V_GS - V_t                                       # overdrive

if V_OV ≤ 0:                                            # cutoff
    I_d = subthreshold_current(...)                     # if subthreshold enabled, else 0

elif V_DS < V_OV:                                       # triode
    I_d = KP × (W/L) × ((V_OV × V_DS) - V_DS²/2) × (1 + LAMBDA × V_DS)

else:                                                   # saturation
    I_d = (KP/2) × (W/L) × V_OV² × (1 + LAMBDA × V_DS)
```

Subthreshold (optional, gated by `subthreshold_enable=True`):
```
I_d_sub = (KP/2) × (W/L) × (N × V_T)² × exp((V_GS - V_t)/(N × V_T)) × (1 - exp(-V_DS/V_T))
```

### Small-signal Jacobian

```
gm  = ∂I_d/∂V_GS = KP × (W/L) × V_OV × (1 + LAMBDA × V_DS)            # saturation
gds = ∂I_d/∂V_DS = (KP/2) × (W/L) × V_OV² × LAMBDA                    # saturation (slope)
gmb = ∂I_d/∂V_BS = -gm × GAMMA / (2 × sqrt(PHI - V_BS))                # body
```

In triode, expressions are slightly different — the model returns analytical derivatives consistent with the region.

### Capacitance model (Meyer)

```
Cgs = Cgd = (1/2) × W × L × C_ox    in triode
Cgs = (2/3) × W × L × C_ox          in saturation
Cgd = 0                             in saturation
Cgb = 0                             in inversion (ignored)
```

This is the simple Meyer model — adequate for digital cells, not for analog. EKV and BSIM use better models.

### Limitations
- Hard transition at threshold (no smooth moderate inversion).
- No velocity saturation — fails in deep submicron (L < 100 nm).
- No DIBL, no narrow-width effects, no temperature-aware mobility unless explicitly added.
- Square law overpredicts saturation current in modern technologies by 30-100%.

Use Level-1 for: pedagogy, hand calculations, the 4-bit adder demo. Don't use it to characterize a real Sky130 cell.

## EKV (Enz-Krummenacher-Vittoz)

A charge-based model that's smooth across all regions (weak/moderate/strong inversion). About 30 parameters. Originally for analog low-power design; suitable for cell characterization in older nodes.

### Key idea

Express I_d in terms of the **forward** and **reverse** normalized currents, `i_f` and `i_r`:

```
I_d = I_S × (i_f - i_r)
```

where `I_S = 2 × n × μ × C_ox × V_T² × (W/L)` is the **specific current** (a normalization).

The forward current depends on the source voltage; the reverse on the drain voltage:

```
i_f = ln²(1 + exp((V_P - V_S)/(2 × V_T)))     # source-side normalized current
i_r = ln²(1 + exp((V_P - V_D)/(2 × V_T)))     # drain-side normalized current
```

with the **pinch-off voltage**:
```
V_P = (V_G - V_T0) / n     # n is the slope factor; ~1.2-1.4
```

The `ln²(1 + exp(x))` interpolation is the magic: for `x >> 0`, it reduces to `(x/2)² = ((V_P - V_S)/(2 V_T))²` (strong inversion); for `x << 0`, it reduces to `exp(x)` (weak inversion / subthreshold). Smooth transition.

### Parameters (subset; EKV v2.6 has 30+)

| Symbol | Description |
|---|---|
| `VT0` | Threshold at zero bias |
| `KP` | μ × C_ox |
| `GAMMA` | Body effect |
| `PHI` | Surface potential at threshold |
| `THETA` | Mobility reduction (vertical field) |
| `KAPPA` | Channel-length modulation (smoother than LAMBDA) |
| `EKV_E0` | Crossover field for vertical mobility reduction |
| `LD` | Lateral diffusion (shortens effective L) |
| `XJ` | Junction depth |
| ... | (many more for high-frequency, noise, etc.) |

### Use case
EKV is the right choice for analog/mixed-signal cells (op-amps, comparators, ADCs) in 250-nm-and-older processes. For digital Sky130 characterization, BSIM3v3 is the standard.

## BSIM3v3 (Teaching Subset)

The de-facto industrial standard for 0.25 μm to 90 nm processes. Sky130 (130 nm) is BSIM3v3-territory. The full model has 200+ parameters. We implement a teaching subset (~80) sufficient to characterize Sky130 cells with reasonable accuracy.

### Model structure

BSIM3v3 expresses current as the same `I_d = I_S × (i_f - i_r)`-style framework as EKV but with much richer parameterization for short-channel effects. The major sub-models:

| Sub-model | What it captures | BSIM parameters involved |
|---|---|---|
| **V_t shift** | Reverse-short-channel effect (RSCE), narrow-width effect | DVT0, DVT1, DVT2, NLX, W0, K3, K3B, NSUB |
| **DIBL** (drain-induced barrier lowering) | V_t reduction at high V_DS | ETA0, ETAB, DSUB |
| **Mobility degradation** | Vertical field, lateral field, velocity saturation | UA, UB, UC, VSAT, A0, AGS, A1, A2, B0, B1 |
| **Channel-length modulation** | Small ∂I_d/∂V_DS in saturation | PCLM, PDIBLC1, PDIBLC2, PSCBE1, PSCBE2 |
| **Substrate current** | Impact ionization | ALPHA0, BETA0 |
| **Gate tunneling** | Direct-tunneling gate leakage | (BSIM4 territory; not in BSIM3v3) |
| **Capacitances (CV model)** | Bulk-charge linearization | CGSO, CGDO, CGBO, CJ, CJSW, MJ, MJSW, ... |
| **Temperature** | Mobility / V_t / IS scaling | KT1, KT2, KT1L, UTE, UA1, UB1, UC1, AT, ... |

We ship default parameter sets corresponding to a 130 nm Sky130 NMOS and PMOS (transcribed from the open Sky130 PDK files at `models/sky130_fd_pr/cells/nfet_01v8/`). These are documented in `sky130-pdk.md`.

### Numerical robustness

BSIM3v3 has notorious convergence pitfalls:
- Discontinuities at region boundaries (between subthreshold and strong inversion).
- Negative parameter values in pathological corners.
- ln(0) and division-by-zero in compact-model expressions.

Our implementation:
- Uses **smoothing functions** (sigmoid blends) at region transitions.
- Validates parameter sets at construction; rejects out-of-bounds values.
- Returns analytical derivatives consistent with the smoothed function.
- Provides `damping` and `Gmin` configuration for the SPICE Newton loop.

## Worked Example 1 — 4-bit Adder NAND2 cell with Level-1

A NAND2 cell from the 4-bit adder synthesizes to two stacked NMOS + two parallel PMOS:

```
            VDD
       ┌─────┴─────┐
       │           │
  ┌────┤PMOS1  PMOS2├────┐
  A    │           │    B
       └────┬──────┘
            │ (output Y)
       ┌────┤
  ┌────┤NMOS1│
  A    └──┬──┘
          │
       ┌──┤
  ┌────┤NMOS2│
  B    └──┬──┘
            │
           GND
```

With Level-1 default parameters (VT0=0.42, KP=220e-6, W=1u, L=130n):

```python
nmos = MOSFET(type="NMOS", model=Level1, params=Level1Params(VT0=0.42, KP=220e-6, W=1e-6, L=130e-9))

# Operating point: V_GS = 1.8 V, V_DS = 0.1 V, V_BS = 0 V
result = nmos.dc(V_GS=1.8, V_DS=0.1, V_BS=0.0, T=300.15)
# I_d ≈ 220e-6 × (1e-6 / 130e-9) × ((1.38 × 0.1) - 0.005) × 1.005 ≈ 232 µA (triode)
# region = "triode"
```

For the SPICE engine running cell characterization, this kind of call is invoked thousands of times per simulation step.

## Worked Example 2 — Cell characterization with BSIM3v3

`standard-cell-library.md` characterizes each Sky130 cell across PVT corners. For the `nand2_X1` cell:

1. Load BSIM3v3 NMOS/PMOS parameters from Sky130 PDK files (corner: TT, T=27°C, V_DD=1.8V).
2. Build the cell schematic in the SPICE engine using two NMOS + two PMOS.
3. Apply input transitions; measure output transitions.
4. Extract: input-to-output delay, transition time, input pin capacitance, leakage current.
5. Repeat for each `(slew_rate, output_load)` cell-characterization grid.
6. Output Liberty-format cell entry.

The model interface is the integration point: `bsim3v3_nmos.dc(V_GS, V_DS, V_BS, T)` returns whatever the MNA solver needs.

## Worked Example 3 — Smooth transition through threshold

A demonstration that EKV (and BSIM3v3) provide smooth I_d at threshold, where Level-1 has a kink:

```
V_GS sweep from 0 V to 1.8 V at V_DS = 1.8 V, V_BS = 0:

Level-1:    I_d = 0 for V_GS < 0.42 V; jumps to (1/2) × KP × (W/L) × V_OV² above
            ∂I_d/∂V_GS has a discontinuity at V_GS = V_t

EKV:        smooth ln²(1+exp(...)) blends weak and strong inversion
            ∂I_d/∂V_GS is continuous everywhere

BSIM3v3:    similarly smooth via smoothing functions
            Newton convergence is much faster
```

Use this as a debugging tool: if a SPICE simulation fails to converge with Level-1, switching to EKV often resolves it without changing the circuit.

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum
from typing import Protocol


class MosfetType(Enum):
    NMOS = "NMOS"
    PMOS = "PMOS"


@dataclass(frozen=True)
class MosResult:
    Id: float
    gm: float
    gds: float
    gmb: float
    Cgs: float
    Cgd: float
    Cgb: float
    Cbs: float
    Cbd: float
    region: str


class MosfetModel(Protocol):
    """Common interface for all MOSFET models."""
    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float) -> MosResult: ...


# ─── Level-1 ─────────────────────────────────────────────────────

@dataclass(frozen=True)
class Level1Params:
    VT0: float = 0.42
    KP: float = 220e-6        # NMOS default
    LAMBDA: float = 0.05
    GAMMA: float = 0.27
    PHI: float = 0.84
    W: float = 1e-6
    L: float = 130e-9
    IS: float = 1e-15
    N_SUB: float = 1.4         # subthreshold slope factor
    T_NOM: float = 300.15
    subthreshold_enable: bool = True


@dataclass(frozen=True)
class Level1Model:
    params: Level1Params

    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float) -> MosResult:
        ...   # implementation per equations above


# ─── EKV (subset) ────────────────────────────────────────────────

@dataclass(frozen=True)
class EKVParams:
    VT0: float
    KP: float
    GAMMA: float
    PHI: float
    THETA: float = 0.0
    KAPPA: float = 0.0
    LD: float = 0.0
    XJ: float = 0.15e-6
    N: float = 1.4               # slope factor
    W: float = 1e-6
    L: float = 1e-6
    T_NOM: float = 300.15


@dataclass(frozen=True)
class EKVModel:
    params: EKVParams

    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float) -> MosResult:
        ...


# ─── BSIM3v3 (teaching subset) ──────────────────────────────────

@dataclass(frozen=True)
class BSIM3v3Params:
    """A teaching subset of BSIM3v3 parameters. Full BSIM3 has 200+;
    this subset covers ~80 sufficient for Sky130-like processes."""
    # Level-1-equivalent core
    VT0: float; KP: float; GAMMA: float; PHI: float

    # Mobility
    UA: float; UB: float; UC: float; VSAT: float

    # Threshold-shift sub-model
    DVT0: float; DVT1: float; DVT2: float
    NLX: float; W0: float; K3: float; K3B: float
    NSUB: float

    # DIBL
    ETA0: float; ETAB: float; DSUB: float

    # Channel-length modulation
    PCLM: float; PDIBLC1: float; PDIBLC2: float

    # Substrate current
    ALPHA0: float; BETA0: float

    # Capacitances
    CGSO: float; CGDO: float; CGBO: float
    CJ: float; CJSW: float; MJ: float; MJSW: float

    # Temperature
    KT1: float; KT2: float; UTE: float; AT: float

    # Geometry
    W: float; L: float
    LD: float; XJ: float

    T_NOM: float = 300.15


@dataclass(frozen=True)
class BSIM3v3Model:
    params: BSIM3v3Params

    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float) -> MosResult:
        ...


# ─── Wrapper for type+model selection ───────────────────────────

@dataclass(frozen=True)
class MOSFET:
    type: MosfetType
    model: MosfetModel

    def dc(self, V_GS: float, V_DS: float, V_BS: float, T: float = 300.15) -> MosResult:
        # For PMOS, flip signs:
        if self.type == MosfetType.PMOS:
            r = self.model.dc(-V_GS, -V_DS, -V_BS, T)
            return MosResult(
                Id=-r.Id, gm=r.gm, gds=r.gds, gmb=r.gmb,
                Cgs=r.Cgs, Cgd=r.Cgd, Cgb=r.Cgb, Cbs=r.Cbs, Cbd=r.Cbd,
                region=r.region
            )
        return self.model.dc(V_GS, V_DS, V_BS, T)
```

## Edge Cases

| Scenario | Handling |
|---|---|
| V_GS exactly at V_t | Level-1: returns I_d=0 (cutoff). EKV/BSIM: returns small smooth value. |
| V_DS = 0 | Triode formula: I_d = 0. All models agree. |
| V_DS negative (drain below source) | Symmetry: swap drain/source role; model recomputes from the new low-side terminal. Wrapper must detect. |
| Body-source forward biased (V_BS > 0 for NMOS) | Body diode conducts; out of normal operation. Model returns standard equations; SPICE engine flags. |
| Very large V_GS (V_OV > V_DD) | Mobility-degradation models (`THETA × V_OV`, BSIM `UA`/`UB`) cap effective mobility; models stay finite. |
| L below model validity range (e.g., 50 nm with BSIM3v3) | Warn; results may be inaccurate. |
| T outside [200K, 400K] | Warn; thermal extrapolation models break down. |
| Negative model parameters | Reject at construction with descriptive error. |
| Zero-W or zero-L | Reject. |
| Numerical overflow at very high V_OV | Clamp internally; return saturated value with `region="saturation_clamped"`. |

## Test Strategy

### Unit (target 95%+)
- Level-1 cutoff: V_GS=0.3, V_t=0.42 → I_d=0 (or subthreshold value if enabled).
- Level-1 saturation: V_GS=1.8, V_DS=1.8 → I_d ≈ KP/2 × W/L × V_OV² × (1+λV_DS).
- Level-1 triode: V_GS=1.8, V_DS=0.1 → I_d ≈ KP × W/L × (V_OV × V_DS - V_DS²/2).
- Body effect: V_BS=2 raises V_t by exact GAMMA × (sqrt(PHI+2)-sqrt(PHI)).
- EKV smoothness: ∂I_d/∂V_GS is continuous through V_t (no discontinuity > eps).
- BSIM3v3 against Sky130 reference simulation (golden vectors): I-V curves match within 5%.

### Property
- Monotonicity: I_d non-decreasing in V_GS (above cutoff).
- Symmetry: NMOS and PMOS return equal-magnitude I_d for sign-reflected operating points.
- DC consistency: small `dV` perturbation around an operating point matches the returned `gm`/`gds`/`gmb` to first order.

### Integration
- Drive a CMOS inverter from `transistors` package built with each model; verify switching threshold, propagation delay, leakage.
- Characterize a NAND2 Sky130 cell with BSIM3v3; compare delay to published Sky130 reference data within 10%.
- SPICE engine convergence: a 100-transistor cell solves DC in <50 Newton iterations across all corners.

## Conformance Matrix

| Standard / model | Coverage |
|---|---|
| **SPICE3 Level-1** (Berkeley original) | Full |
| **SPICE3 Level-2 (Grove-Frohman)** | Out of scope (rarely used today) |
| **SPICE3 Level-3 (semi-empirical)** | Out of scope |
| **EKV v2.6** | Subset (smooth core; limited high-freq, noise) |
| **EKV v3** | Out of scope |
| **BSIM3v3.3** | Subset (~80 of 200+ parameters) |
| **BSIM4** (90 nm and below) | Out of scope; future spec |
| **BSIMSOI** (SOI processes) | Out of scope |
| **PSP** (compact model standard, 65 nm and below) | Out of scope; future |

## Open Questions

1. **Should we ship a Sky130-tuned Level-1 default for "rough" simulation?** Yes — pedagogical. Place defaults under `Level1Params.sky130_nmos_typical`.
2. **EKV for v1 or skip?** Skip for v1; only Level-1 + BSIM3v3 subset. EKV is intermediate-complexity but limited target audience.
3. **BSIM3v3 parameter import** — accept SPICE `.MODEL` cards directly? Yes; provide a parser.
4. **Smoothing functions** — sigmoid vs polynomial blends? Sigmoid is closed-form; polynomial gives exact Jacobian. Recommendation: sigmoid for ease.

## Future Work

- BSIM4 (sub-65 nm).
- BSIM-CMG (FinFETs, 22 nm and below).
- PSP (PSP103+ for 65 nm and below).
- Self-heating extension.
- Aging models (NBTI, PBTI, HCI).
- Stochastic / Monte Carlo variation models.
- Model-order reduction for fast simulation of repeated transistors in standard cells.
