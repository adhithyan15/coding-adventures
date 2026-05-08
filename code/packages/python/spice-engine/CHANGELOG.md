# Changelog

## [0.3.0] — 2026-05-08

### Added

- **`BJT` element** — Bipolar Junction Transistor dataclass (NPN and PNP).
  - Parameters: `name`, `collector`, `base`, `emitter`, `polarity` ("NPN"/"PNP"),
    `Is` (saturation current, default 10 fA), `beta_f` (current gain hFE,
    default 100), `Vt` (thermal voltage, default 25.85 mV).
  - Frozen dataclass with `slots=True` matching the style of all other elements.

- **`_stamp_bjt` stamping function** — Newton-linearised Ebers-Moll forward-active model.

  **Simplified Ebers-Moll model:**
  The collector current in the forward-active region is modelled as::

      Ic = Is * (exp(Vjunc / Vt) - 1)

  where Vjunc is the controlling junction voltage:
  - **NPN**: Vjunc = Vbe = Vb − Ve
  - **PNP**: Vjunc = Veb = Ve − Vb

  **Newton linearisation** at operating point Vjunc0 (clamped to 0.7 V)::

      exp_term = exp(Vjunc0 / Vt)
      Ic0      = Is * (exp_term - 1)      # collector current at OP
      gm       = (Is / Vt) * exp_term     # transconductance
      gπ       = gm / beta_f              # junction conductance
      Ib0      = Ic0 / beta_f             # base current at OP

  **Two MNA stamps:**
  1. **Junction stamp** (gπ): conductance between the controlling junction
     terminals (B–E for NPN, E–B for PNP) modelling the base-emitter diode.
     Norton companion: `Ieq_junc = Ib0 − gπ * Vjunc0`.
  2. **VCCS stamp** (gm): voltage-controlled current source; controlling nodes
     are the junction pair, output nodes are the collector-emitter pair.
     - NPN: `G[C][B] += gm`, `G[C][E] -= gm`, `G[E][B] -= gm`, `G[E][E] += gm`
     - PNP: roles of E and C are swapped (emitter injects, collector collects).
     Norton companion: `Ieq_coll = Ic0 − gm * Vjunc0`.

  All ground-node aliases (``"0"``, ``"gnd"``, ``"GND"``) are handled correctly;
  any terminal can be ground.

- **`_element_nodes` update** — `BJT` returns `[collector, base, emitter]`.
- **`_stamp_dc` update** — dispatches to `_stamp_bjt` for `BJT` instances.
- **`__init__.py` update** — `BJT` is exported from the top-level package.

### Changed

- `Element` union type now includes `BJT`.
- Version bumped: `0.2.0` → `0.3.0`

### Tests

- **12 new tests** in `tests/test_engine.py` under section 14 "DC: BJT":
  - `test_bjt_dataclass_defaults` — field values and defaults
  - `test_bjt_pnp_dataclass` — polarity field stored correctly
  - `test_bjt_npn_off` — Vbe = 0 → Ic ≈ 0, Vcol ≈ Vcc
  - `test_bjt_npn_forward_active` — Vbe = 0.7 V clamped; Ic matches analytic
  - `test_bjt_npn_beta_ratio` — Ic/Ib = beta_f held internally
  - `test_bjt_pnp_forward_active` — PNP with Veb = 0.7 V; Ic flows into collector
  - `test_bjt_element_nodes` — all three terminals in node index
  - `test_bjt_stamp_matrix_shape` — no NaN/Inf after stamping at Vbe = 0
  - `test_bjt_npn_ground_emitter_no_crash` — ground alias "gnd" on emitter
  - `test_bjt_npn_vcc_emitter` — non-ground emitter; Ic consistent with Vbe
  - `test_bjt_in_element_union` — BJT exported correctly
  - `test_bjt_custom_parameters` — Is/beta_f/Vt stored correctly

---

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
