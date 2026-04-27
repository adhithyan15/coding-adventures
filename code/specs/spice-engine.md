# SPICE Engine

## Overview

A SPICE-compatible analog circuit simulator built around **Modified Nodal Analysis (MNA)** with transient, DC, and AC analyses. Consumes SPICE3-format netlists; produces voltage/current waveforms over time, DC operating points, or frequency responses. Used by `standard-cell-library.md` to characterize each Sky130 cell across PVT corners. The numerical heart of the analog stack.

What MNA does: at every node in a circuit, Kirchhoff's Current Law says ∑I = 0. Linear elements (R, C, L) and controlled sources contribute linear equations; nonlinear elements (diodes, MOSFETs) contribute equations that must be linearized via Newton-Raphson. The matrix that ties all this together is **G** (conductance) for DC, **G + sC** for AC (s = jω), and a discretized integration for transient.

This spec defines:
1. The MNA formulation, including how to "stamp" each element kind into the matrix.
2. The Newton-Raphson loop for handling nonlinearities (MOSFETs, diodes).
3. Time integration for transient analysis (backward Euler, trapezoidal, Gear).
4. The SPICE3 netlist parser (the input format).
5. Convergence aids (Gmin stepping, source stepping, pseudo-transient continuation).
6. The output format (CSV, VCD, raw SPICE waveform).

The engine integrates with `mosfet-models.md` for transistor I-V; with `device-physics.md` for parameter derivations; and with `standard-cell-library.md` as the characterization driver.

## Layer Position

```
device-physics.md
       │
       ▼
mosfet-models.md  ──► (used as black-box element types)
       │
       ▼
spice-engine.md ◀── THIS SPEC
       │
       ▼
standard-cell-library.md
       │ (cell characterization runs SPICE thousands of times)
       ▼
tech-mapping.md, ASIC backend, ...
```

## Concepts

### Modified Nodal Analysis (MNA)

The circuit's state is the vector of **node voltages** plus extra unknowns for **branches that don't have a voltage-defined current relationship** (current sources, voltage sources, inductors). The MNA matrix ties these together:

```
G × x = b
```

For an N-node circuit (excluding ground node 0) with M extra branch unknowns:
- `x` is (N + M)-dimensional: top N entries are node voltages, bottom M entries are branch currents.
- `G` is (N+M) × (N+M).
- `b` is (N+M)-dimensional: top N entries are net injected currents, bottom M entries are voltage source values.

**Stamping** is the act of contributing element terms to G and b. Each element kind has a stamping rule:

| Element | Stamp into G | Stamp into b |
|---|---|---|
| Resistor R between i,j | `G[i,i]+=1/R; G[j,j]+=1/R; G[i,j]-=1/R; G[j,i]-=1/R` | (none) |
| Independent current source between i,j (current I from i to j) | (none) | `b[i] -= I; b[j] += I` |
| Independent voltage source V between i,j (extra unknown k) | `G[i,k]+=1; G[j,k]-=1; G[k,i]+=1; G[k,j]-=1` | `b[k] = V` |
| Capacitor C between i,j (transient with timestep h, BE) | `G[i,i]+=C/h; G[j,j]+=C/h; G[i,j]-=C/h; G[j,i]-=C/h` | `b[i] += (C/h)×V_old(i,j); b[j] -= (C/h)×V_old(i,j)` |
| Inductor L between i,j (extra unknown k for branch current) | `G[i,k]+=1; G[j,k]-=1; G[k,i]+=1; G[k,j]-=1; G[k,k]-=L/h` | `b[k] -= (L/h)×I_old(k)` |
| MOSFET (after linearization) | gm, gds, gmb stamps | `b -= linearized residual` |
| Diode (after linearization) | conductance stamp | residual |
| VCCS (gm, controlled by V_ck) | `G[i,ck]+=gm; G[j,ck]-=gm` etc. | (none) |

(Capacitor stamps shown for backward Euler; trapezoidal has different coefficients.)

For DC: capacitors are open (C/h → 0), inductors are short (L/h → ∞ via numerical handling). For AC: capacitors stamp `jωC`, inductors stamp `1/(jωL)`. For transient: time-discretized as above.

### The Newton-Raphson loop

For nonlinear elements (every MOSFET, every diode), the MNA equation is:

```
F(x) = 0
```

Newton-Raphson iterates:
```
J × Δx = -F(x_k)
x_{k+1} = x_k + Δx
```

where `J = ∂F/∂x` is the Jacobian. For MNA, the Jacobian *is* the G matrix when MOSFET/diode small-signal parameters (`gm`, `gds`, etc.) are stamped. So each iteration:

1. Compute current operating point's `I_d`, `gm`, `gds`, `gmb`, capacitances for every transistor (call `mosfet-models.dc(...)`).
2. Stamp linear + linearized nonlinear contributions into G.
3. Compute residual `F(x_k)`.
4. Solve `J × Δx = -F(x_k)` (sparse LU or KLU).
5. Update `x_k+1 = x_k + α × Δx` (with optional damping `α ≤ 1`).
6. Check convergence: `||Δx|| < tol_x` and `||F(x)|| < tol_F`.

If diverging: cut step size, restart from a better initial guess, or use convergence aids.

### Time integration (transient)

For transient: at each timestep `t_n → t_n+1 = t_n + h`, solve a Newton-Raphson problem for the next state. The choice of integration formula determines the discretization:

- **Backward Euler (BE)**: `(x_n+1 - x_n)/h = f(x_n+1)` — implicit, A-stable, O(h¹). Robust but inaccurate.
- **Trapezoidal (TR)**: `(x_n+1 - x_n)/h = (f(x_n+1) + f(x_n))/2` — implicit, A-stable, O(h²). Most common in SPICE3.
- **Gear-2 (BDF2)**: implicit, stiff-stable, O(h²). Used when TR oscillates ("trap ringing").

Adaptive timestep based on **local truncation error (LTE)**:
- Compute LTE estimate at each step.
- If LTE > tol: reduce step.
- If LTE << tol: increase step.

### Convergence aids

When Newton diverges:

- **Gmin stepping**: add a tiny conductance to ground at every node. Solve. Reduce Gmin in steps. Eventually `Gmin = 0`.
- **Source stepping**: turn down all source voltages to zero (where the answer is trivially zero), solve, ramp them up. The first solution is a great initial guess for the next.
- **Pseudo-transient continuation**: solve a transient starting from zero state until it settles to DC steady state.

Most SPICE engines try them in sequence: regular DC → Gmin → source step → pseudo-transient.

### AC analysis

Linearize about the DC operating point. Build complex-valued G(ω) = G + jωC. Solve `G(ω) × X = B` for each frequency. Output magnitude/phase of node voltages or currents.

Frequency sweep is logarithmic (decades) by default; linear available.

## SPICE3 Netlist Format

```
* 4-bit adder NAND2 gate characterization
.title NAND2 cell test
.include sky130_fd_pr/models/nfet_01v8.spice
.include sky130_fd_pr/models/pfet_01v8.spice

V_DD vdd 0 1.8
V_A  a 0 PWL(0 0 1n 0 1.001n 1.8)   * step input
V_B  b 0 1.8

* PMOS pair (parallel, sources on VDD)
M1 y a vdd vdd pfet_01v8 W=2u L=130n
M2 y b vdd vdd pfet_01v8 W=2u L=130n

* NMOS stack (series, top at y, bottom at gnd)
M3 y a n1 0 nfet_01v8 W=1u L=130n
M4 n1 b 0 0 nfet_01v8 W=1u L=130n

* Output load
C_load y 0 5f

.tran 10p 5n
.print TRAN V(y) V(a)
.end
```

Element prefixes:
- `R` resistor
- `C` capacitor
- `L` inductor
- `V` voltage source
- `I` current source
- `D` diode
- `M` MOSFET (4-terminal: drain gate source body)
- `Q` BJT
- `E`/`F`/`G`/`H` controlled sources
- `X` subcircuit instance

Directives:
- `.tran` transient analysis
- `.dc` DC sweep
- `.ac` AC analysis
- `.op` operating-point only
- `.include` include another netlist
- `.subckt`/`.ends` subcircuit definition
- `.model` model card
- `.param` parameter
- `.print`, `.plot` output
- `.options` configuration

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum
from typing import Callable


@dataclass
class Element:
    name: str
    nodes: tuple[str, ...]   # node identifiers ('0' is ground)
    kind: str
    params: dict[str, float | str]


@dataclass
class Netlist:
    title: str
    elements: list[Element]
    models: dict[str, dict]     # model_name → params
    subcircuits: dict[str, "Subcircuit"]
    directives: list["Directive"]


@dataclass
class Subcircuit:
    name: str
    nodes: list[str]
    parameters: dict[str, float]
    elements: list[Element]


@dataclass
class Directive:
    kind: str   # 'tran', 'dc', 'ac', 'op', 'print', 'plot', 'options'
    args: dict[str, str | float]


class AnalysisType(Enum):
    OP   = "op"
    DC   = "dc"
    AC   = "ac"
    TRAN = "tran"


@dataclass
class TranOptions:
    tstep: float
    tstop: float
    method: str = "trap"        # 'be' | 'trap' | 'gear2'
    reltol: float = 1e-3
    abstol: float = 1e-12
    chgtol: float = 1e-14
    vntol: float = 1e-6
    itl4: int = 10              # max Newton iters per step


@dataclass
class DCOptions:
    src: str
    start: float
    stop: float
    step: float


@dataclass
class ACOptions:
    fstart: float
    fstop: float
    points_per_decade: int


@dataclass
class Result:
    analysis: AnalysisType
    time_or_freq: list[float]
    node_voltages: dict[str, list[float]]
    branch_currents: dict[str, list[float]]


class SpiceEngine:
    def __init__(self, netlist: Netlist) -> None: ...
    
    def operating_point(self) -> Result:
        """DC operating point: solve nonlinear equations at t=0, no time evolution."""
        ...
    
    def dc_sweep(self, opts: DCOptions) -> Result: ...
    
    def transient(self, opts: TranOptions) -> Result: ...
    
    def ac(self, opts: ACOptions) -> Result: ...
    
    def measure(self, expr: str) -> float:
        """Compute a derived quantity from the last analysis (e.g., 'TPLH=time when V(y) crosses 0.9 - time when V(a) crosses 0.9')"""
        ...


# ─── Element stamping ─────────────────────────────────────────

@dataclass
class StampingContext:
    G: "SparseMatrix"
    b: list[float]
    x: list[float]              # current solution
    h: float                    # timestep (for transient)
    method: str                 # 'be' | 'trap' | 'gear2'
    state: dict                 # element-specific state (cap voltages, etc.)


def stamp_resistor(elem: Element, ctx: StampingContext) -> None: ...
def stamp_capacitor(elem: Element, ctx: StampingContext) -> None: ...
def stamp_inductor(elem: Element, ctx: StampingContext) -> None: ...
def stamp_vsource(elem: Element, ctx: StampingContext) -> None: ...
def stamp_isource(elem: Element, ctx: StampingContext) -> None: ...
def stamp_diode(elem: Element, ctx: StampingContext, model: object) -> None: ...
def stamp_mosfet(elem: Element, ctx: StampingContext, model: "MOSFET") -> None: ...
```

## Sparse Matrix Operations

For circuits beyond ~10 nodes, dense LU is wasteful. The engine uses **sparse storage** (CSC or CSR) and **sparse LU** factorization. KLU (the standard SPICE solver) is the gold target; for v1 we use `scipy.sparse.linalg.splu` or implement a teaching-grade Markowitz-ordered Crout sparse LU.

Refactorization is required at each Newton iteration (the matrix structure stays the same; values change). With KLU's symbolic+numeric split, repeated factorizations are cheap.

## Worked Example 1 — DC operating point of CMOS inverter

```
* CMOS inverter, input at VDD/2

V_DD  vdd 0  1.8
V_in  in  0  0.9

M_p y in vdd vdd pmos W=4u L=130n
M_n y in 0   0   nmos W=2u L=130n

.op
```

Engine flow:
1. Parse netlist; build Element list.
2. Call `engine.operating_point()`.
3. Build initial guess: `V(y) = 0.9` (mid-rail).
4. Newton-Raphson loop:
   - For each MOSFET: call `mosfet-models.dc(V_GS, V_DS, V_BS, T)` to get `Id` and small-signal params.
   - Stamp G, b.
   - Solve `G × Δx = -b`.
   - Update; check convergence.
5. Converge in ~5 iterations.
6. Return: `V(y) = 0.9 V` (matches symmetry), `I(VDD) ≈ 0` (static current is just leakage).

## Worked Example 2 — Transient: NAND2 cell propagation delay

```
* NAND2 cell with 5fF output load
* (netlist as above)
.tran 10p 5n
```

Engine flow:
1. DC operating point first (initial state).
2. For t = 10p, 20p, ..., 5000p:
   - Update voltage sources (a steps from 0 to 1.8 V at t=1ns).
   - Build MNA matrix with capacitor stamps (BE/TR coefficients).
   - Newton-Raphson loop to solve for x(t_n+1).
   - Estimate LTE; adapt step.
   - Save results.
3. Compute propagation delay TPLH = time(V(y) = 0.9, rising) - time(V(a) = 0.9, rising).
4. Output: `~80 ps` for a 5fF load on a typical Sky130 NAND2_X1.

## Worked Example 3 — AC: cell input capacitance

```
* Drive cell input through 1 GΩ test resistor; measure I and V at AC frequency.
V_in vin 0 AC 1
R_test vin in 1G
M_n y in 0 0 nmos ...
.ac dec 10 1k 1G
```

Engine flow: linearize about DC OP; for each ω, solve complex (G + jωC) X = B; report |I(R_test)| and phase. Input capacitance is `C_in ≈ Im(I/V) / ω`.

## Worked Example 4 — Mid-scale: 4-bit adder cell-level SPICE

A 4-bit adder mapped to ~25 standard cells = ~250 transistors. Net count ~80 (incl. internal stages). MNA matrix is ~330 × 330. Sparse, < 5% fill. Transient simulation of a step input takes ~0.1 sec on a modern laptop. (Larger designs scale roughly linearly with element count due to sparse linear algebra.)

## Time Integration Details

### Backward Euler (BE)
For C between nodes i,j with voltage V = x_i - x_j:
```
I_C(t_n+1) = C × (V(t_n+1) - V(t_n)) / h
```
Stamp:
```
G[i,i] += C/h;  G[j,j] += C/h;  G[i,j] -= C/h;  G[j,i] -= C/h
b[i]   += (C/h) × V(t_n);  b[j] -= (C/h) × V(t_n)
```

### Trapezoidal (TR)
```
I_C(t_n+1) + I_C(t_n) = 2C × (V(t_n+1) - V(t_n)) / h
```
Stamp:
```
G[i,i] += 2C/h;  ...  similarly
b[i]   += (2C/h) × V(t_n) + I_C(t_n);  ...
```

### Gear-2 (BDF2)
```
3V(t_n+1) - 4V(t_n) + V(t_n-1) = 2h × dV/dt(t_n+1)
```

TR is default; switch to Gear when "trap ringing" (oscillation in the integration error) is detected. Gear is more dissipative — kills oscillations.

### LTE-based step control

Estimate local truncation error per step:
```
LTE ≈ (h³/12) × d³x/dt³   (for trapezoidal)
```
Approximated via divided differences of recent solutions. If `LTE > reltol × |x| + abstol`: reduce step; if `LTE << tol`: grow step.

## Convergence Aids

```python
def solve_dc_with_aids(engine, max_attempts=4):
    try:
        return engine._newton_solve_dc(initial="zero")
    except ConvergenceError:
        pass
    
    # Gmin stepping
    for gmin in [1e-3, 1e-6, 1e-9, 1e-12]:
        try:
            engine.options.gmin = gmin
            return engine._newton_solve_dc(initial="last")
        except ConvergenceError:
            continue
    
    # Source stepping
    try:
        return engine._source_step_dc()
    except ConvergenceError:
        pass
    
    # Pseudo-transient
    return engine._pseudo_transient_to_dc()
```

## Edge Cases

| Scenario | Handling |
|---|---|
| Zero-resistance loop | Numerical singularity; stamp tiny minimum resistance (1 μΩ). |
| Voltage source short circuit | Detected at netlist parse; reject. |
| Disconnected circuit | Each connected component solved independently. |
| Floating node (no DC path to ground) | Add Gmin to ground at floating nodes. |
| Inductor in DC | Treated as short-circuit (zero V); MNA accommodates. |
| Capacitor in DC | Treated as open (zero I); MNA accommodates. |
| Newton non-convergence | Try aids in sequence; ultimately raise. |
| Transient timestep below `1e-18 s` | Stop with non-convergence error. |
| Hierarchical subcircuits with parameters | Flatten at parse; instantiate per call. |
| AC source in transient analysis | Treated as DC value at t=0. |
| Multiple `.tran` cards | Use the last one. |

## Test Strategy

### Unit (target 95%+)
- Stamp tests for each element kind (R, C, L, V, I, M, D).
- Newton convergence on a simple diode + resistor circuit.
- Time integration: capacitor charging through a resistor — exact analytical solution match within reltol.
- AC: RC low-pass — first-order roll-off matches.
- DC sweep: NMOS I-V curve matches mosfet-models direct call.

### Integration
- Operating point of CMOS inverter: V(out) ≈ V_DD/2 at V(in) = V_DD/2 (within 1%).
- Transient of NAND2 cell: propagation delay matches Sky130 reference within 10%.
- Standard-cell library characterization run completes for all ~30 teaching cells in < 1 min.

### Property
- Energy conservation: in transient simulation of a passive LC tank, energy oscillates with bounded loss matching damping resistance.
- Reciprocity: AC two-port should satisfy reciprocity for passive circuits.
- Time-reversibility: trapezoidal integration of a lossless circuit run forward then backward returns to the initial state (within numerical precision).

## Conformance Matrix

| Standard / format | Coverage |
|---|---|
| **SPICE3 netlist syntax** | Subset (R, C, L, V, I, M, D, X, .tran, .dc, .ac, .op, .include, .subckt, .model, .param) |
| **HSPICE extensions** | Out of scope |
| **Berkeley SPICE3 I/O** | Output as text; binary `raw` format optional |
| **PWL/PULSE/SIN/EXP source forms** | Full |
| **Sub-circuit hierarchical parameter passing** | Full |
| **Verilog-AMS analog blocks** | Out of scope |

## Open Questions

1. **Sparse solver: KLU vs scipy.sparse vs custom Markowitz-ordered LU?** Recommendation: scipy for v1; KLU bindings as future optimization.
2. **Multi-rate / waveform-relaxation for digital-analog mixed?** Recommendation: defer; full SPICE on small circuits is fine for v1.
3. **Distributed transient (per-process)?** Defer.
4. **Verilog-A support for compact custom models?** Future spec `verilog-a-parser.md`.
5. **GPU acceleration?** Defer.

## Future Work

- KLU bindings for production-quality sparse solver.
- Verilog-A model support.
- Mixed-signal coupling with `hardware-vm.md` for AMS simulation.
- Multi-corner parallel sweep (one process per PVT corner).
- Behavioral modeling sources (B-element).
- Noise analysis (.noise).
- S-parameter extraction.
- Periodic steady-state (PSS) analysis for RF.
