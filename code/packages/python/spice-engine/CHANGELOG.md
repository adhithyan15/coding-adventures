# Changelog

## [0.2.0] — 2026-05-06

### Added

- **Trapezoidal integration method** (`method="trap"`, new default)
  - Capacitor companion: `G_eq = 2C/h`, `I_eq = G_eq·V_n + I_n` (Norton current
    injected into the positive plate).  Second-order accurate — `O(h²)` LTE vs
    `O(h)` for backward Euler.
  - Inductor companion: `G_eq = h/(2L)`, `I_eq = I_n + G_eq·V_n` (parallel Norton
    current flowing n+ → n−).  Correctly accumulates inductor flux across time.
  - Both capacitor and inductor companion histories are updated in
    `_update_reactive_state` after each accepted step.

- **Backward-Euler method** (`method="euler"`)
  - Capacitor companion: `G_eq = C/h`, `I_eq = G_eq·V_n`.
  - Inductor companion: `G_eq = h/L`, `I_eq = I_n`.
  - `method="euler"` is available as a fallback; `"trap"` is the default.

- **LTE-based adaptive timestepping** (`adaptive=True`)
  - Trapezoidal-specific LTE estimate: `lte ≈ |V_{n+1} − 2·V_n + V_{n-1}| / 2`
    (second finite difference of each capacitor voltage, normalised by 2).
  - Step accepted when `lte ≤ tol_lte`; rejected and halved when `lte > tol_lte`
    and `h > min_step`.
  - Step doubled (up to `max_step`) when `lte < tol_lte / 8`.
  - `TransientResult` now carries `method` (str) and `steps_rejected` (int).
  - `min_step` defaults to `t_step / 1000`; `max_step` defaults to `t_step × 10`.

- **Correct reactive-element initial conditions**
  - Capacitor: initial current `I_C(0)` is seeded from the branch current of the
    substitute voltage source in the t=0 DC solve.  This eliminates the large
    first-step error in trapezoidal that arises from assuming `I_0 = 0`.
  - Inductor: the t=0 DC solve now uses a backward-Euler companion resistor
    `R = L/h` instead of a 0 V voltage source.  A 0 V source forces the steady-
    state current at t=0; the companion resistor correctly models near-zero initial
    current with the full supply voltage appearing across the inductor.  The
    initial voltage `V_L(0)` is seeded from the DC OP into `ind_voltages` for the
    first trapezoidal step.

- **Helper functions** (`_build_transient_companions`, `_update_reactive_state`,
  `_lte_estimate`, `_node_voltage`)

- **`TransientResult` extended** — new fields `method: str = "trap"` and
  `steps_rejected: int = 0`.

### Changed

- `transient()` signature extended with `method`, `adaptive`, `tol_lte`,
  `min_step`, `max_step`.  Default `method="trap"` (was implicit forward Euler in
  0.1.0 — existing callers that did not pass `method` now use trapezoidal).
- Version bumped: `0.1.0` → `0.2.0`

### Tests

- **37 tests** (up from initial suite) covering:
  - Backward-Euler and trapezoidal RC charging accuracy
  - Trap accuracy vs Euler at same step size
  - Adaptive control: steps_rejected, steady-state convergence
  - Adaptive and fixed trapezoidal agree when h is locked
  - RL current build-up to steady state
  - Inductor initial condition (near-zero current at t=0)
  - LTE estimate for zero/curved signals and circuits without capacitors
  - TransientResult method + steps_rejected metadata fields
- Coverage: **88%**

---

## [0.1.0] — Unreleased

### Added
- Element classes: Resistor, Capacitor, Inductor, VoltageSource, CurrentSource, Diode (Shockley), Mosfet (mosfet-models-backed).
- `Circuit` container.
- MNA matrix construction with element-specific stamp functions.
- Gaussian elimination with partial pivoting (`_solve`).
- `dc_op(circuit, max_iterations=50, tol=1e-6)`: Newton-Raphson DC operating point. Returns DcResult with node_voltages + branch_currents + converged flag.
- `transient(circuit, t_stop, t_step)`: forward-Euler with capacitor companion model (g = C/h, I_eq = (C/h) × V(t_n)). Returns TransientResult with per-step TransientPoints.
- Diode linearization with V_d clamping to avoid exp overflow.
- MOSFET stamping uses mosfet_models.MOSFET.dc() for I_d, g_m, g_ds.
- Ground node aliases: '0', 'gnd', 'GND'.

### Out of scope (v0.2.0)
- AC analysis (.ac).
- Better integrators (backward Euler, trapezoidal, Gear-2).
- Adaptive timestep with LTE control.
- Convergence aids (Gmin stepping, source stepping, pseudo-transient).
- SPICE3 netlist parser.
- BJTs, JFETs, Verilog-A.
- Sparse matrix solver.
