# Changelog

## [0.6.0] — 2026-05-08

### Added

- **`dc_sweep()` function** — DC parameter sweep analysis (the SPICE `.DC` command).

  Steps one independent source (`VoltageSource` or `CurrentSource`) through a
  user-specified range `[start, stop]` with increment `step` and records a full
  DC operating-point snapshot at each step.  This enables transfer-curve
  measurements, bias-point sensitivity analysis, and DC load-line characterisation.

  **Signature:**
  ```python
  dc_sweep(
      circuit: Circuit,
      source_name: str,
      start: float,
      stop: float,
      step: float,
      *,
      max_iterations: int = 50,
      tol: float = 1e-6,
  ) -> DcSweepResult
  ```

  **Algorithm:**
  1. Validate that `step != 0`; locate the named source element in the circuit.
  2. Build the sweep-value list using integer-counted steps to avoid floating-point
     drift (`start + i * step` for i = 0, 1, …); stop value is included within
     half-step tolerance.  Wrong-sign steps silently produce an empty result list.
  3. For each sweep value:
     a. Create a **new** source element with the swept value (frozen dataclasses
        cannot be mutated; the original circuit is never modified).
     b. Rebuild the circuit with the new element in place of the original.
     c. Call `dc_op` on the modified circuit.
     d. Append a `DcSweepPoint` capturing `source_value`, `node_voltages`,
        `branch_currents`, and `converged`.
  4. Return `DcSweepResult(points=[…], source_name=source_name)`.

  **Why integer-counted steps:**  Floating-point addition accumulates error.
  After 100 steps of 0.1 V, `0.1 * 100 == 10.0` exactly in IEEE-754, but
  `sum(0.1 for _ in range(100))` drifts to ~9.99999…  Integer multiplication is
  exact and avoids accumulating any ULP error.

  **Original circuit immutability:** All elements are `frozen=True` dataclasses.
  To "change" a value dc_sweep creates a new instance and rebuilds the element
  list; the caller's `Circuit` object remains unchanged.

- **`DcSweepPoint` dataclass** — frozen snapshot of one DC operating point during
  a parameter sweep.
  - `source_value: float` — swept source value at this step (V or A).
  - `node_voltages: dict[str, float]` — DC node voltages (V), ground excluded.
  - `branch_currents: dict[str, float]` — DC branch currents (A) for all
    voltage sources, keyed by source name.
  - `converged: bool` — `True` when Newton-Raphson converged.

- **`DcSweepResult` dataclass** — collected results from `dc_sweep()`.
  - `points: list[DcSweepPoint]` — one entry per evaluated step, in sweep order.
  - `source_name: str` — name of the swept source.

### Tests added (sections 28-32)

| Section | Coverage |
|---------|----------|
| 28 | `DcSweepPoint` / `DcSweepResult` dataclass fields, frozen semantics, public API export |
| 29 | Linear resistive circuits: voltage-divider ratio, step sequence, circuit immutability, descending sweep, wrong-sign empty result, single-step, branch current recording, three-node ladder |
| 30 | Nonlinear diode circuit: all-converged forward-bias sweep, monotone-increasing cathode voltage |
| 31 | Current-source sweeps: Ohm's law at each step, descending current sweep |
| 32 | Error cases: zero step, missing source name, resistor (not a source) |

### Changed

- `__version__` bumped from `0.5.0` → `0.6.0`.
- Module docstring updated to mention DC sweep.

---

## [0.5.0] — 2026-05-08

### Added

- **`tf()` function** — DC small-signal transfer function analysis (the SPICE `.TF` command).

  **What it computes:**
  Given a circuit, one driving independent source (`input_source`), and one
  output node (`output_node`), `.TF` returns three DC small-signal quantities:

  | Quantity | Symbol | Definition |
  |---|---|---|
  | Transfer ratio | H | V_output / V_input (VoltageSource) or V_output / I_input (CurrentSource, transimpedance in Ω) |
  | Input impedance | Z_in | Thevenin impedance looking into the input port (Ω) |
  | Output impedance | Z_out | Thevenin impedance looking back from the output node (Ω) |

  **Algorithm (four steps):**
  1. **DC operating point** — run `dc_op` to bias all nonlinear devices (Diode,
     MOSFET, BJT).  This gives the linearisation point for the small-signal matrix.
  2. **Small-signal conductance matrix G_ss** — build a real (ω = 0) MNA matrix
     via `_build_ss_matrix`.  Independent sources are zeroed (only structural
     KVL/KCL entries remain for `VoltageSource`; `CurrentSource` is skipped
     entirely).  Reactive elements: Capacitor → open; Inductor → near-short
     (G = 1e12 S).  Nonlinear devices → small-signal conductances at the DC OP.
  3. **Forward solve** — apply a unit excitation at the input source while all
     other sources are zeroed; solve `G_ss · x_fwd = b_fwd`:
     - `VoltageSource` input: `b_fwd[branch] = 1.0` (1 V excitation);
       `H = x_fwd[output_idx]`;  `Z_in = -1 / x_fwd[branch]` (branch current is
       negative when the source delivers current — MNA stamp convention).
     - `CurrentSource` input: `b_fwd[n_plus] -= 1`, `b_fwd[n_minus] += 1` (1 A);
       `H = x_fwd[output_idx]`;  `Z_in = V_n_minus − V_n_plus` (compliance voltage).
  4. **Output impedance solve** — same G_ss, inject 1 A at `output_node`:
     `b_test[output_idx] = 1.0`;  `Z_out = x_test[output_idx]` (V/A = Thevenin Ω).

  **Why branch current is negative for VoltageSource:**
  The MNA stamp `G[n_plus][branch] = 1` places `x[branch]` in the KCL row for
  n_plus with coefficient +1.  For a resistive load:
  `(1/R)·V_n_plus + x[branch] = 0` → `x[branch] = −I_delivered`.
  So `Z_in = V_in / I_delivered = 1 / (−x[branch])`.

- **`TfResult` dataclass** — frozen result type for `tf()`.
  - `transfer_ratio: float` — V_out/V_in or V_out/I_in (transimpedance).
  - `input_impedance: float` — Thevenin input impedance (Ω).
  - `output_impedance: float` — Thevenin output impedance (Ω).
  - `converged: bool` — mirrors the DC operating-point convergence flag.

- **`_build_ss_matrix` helper** — builds the real DC small-signal MNA matrix.
  Stamping rules per element type:

  | Element | Stamp |
  |---|---|
  | Resistor R | conductance G = 1/R |
  | Capacitor | open circuit (skipped) |
  | Inductor | near-short G = 1e12 S |
  | VoltageSource | KVL/KCL structural entries only (b not set) |
  | CurrentSource | skipped (independent source → zero) |
  | Diode | gd = (Is/Vt)·exp(Vd/Vt) at DC OP |
  | MOSFET | gds + gm VCCS at DC OP |
  | BJT | g_π + gm VCCS at DC OP |

### Changed

- `__version__` bumped from `0.4.0` to `0.5.0`.
- `pyproject.toml` description updated to include DC transfer function analysis.

### Tests

27 new tests across 5 sections (23–27):

- **Section 23** (4 tests) — `TfResult` dataclass: fields, frozen immutability,
  `converged=False`, package export.
- **Section 24** (4 tests) — `_build_ss_matrix` unit tests: single resistor,
  capacitor open, inductor near-short, current source skipped.
- **Section 25** (7 tests) — voltage-source input: symmetric voltage divider
  (H=0.5, Z_in=2kΩ, Z_out=500Ω), asymmetric divider, source-node output (H=1,
  Z_out=0), ground output (H=0), three-resistor ladder, inductor-short, diode
  linearisation.
- **Section 26** (3 tests) — current-source input: transimpedance into R,
  parallel R∥R, mixed source circuit (VoltageSource input with CurrentSource
  zeroed).
- **Section 27** (5 tests) — error cases: missing source name, non-source
  element, unknown output node, independence of source voltage, two-source circuit.

Total: 107 tests, 80.16% coverage, ruff clean.

---

## [0.4.0] — 2026-05-08

### Added

- **`ac_sweep` function** — Small-signal AC frequency sweep (the SPICE `.AC` analysis).

  **Algorithm:**
  1. Compute DC operating point via `dc_op` to obtain bias voltages for
     nonlinear device linearisation.
  2. Build a frequency grid (log-spaced or linear).
  3. For each frequency ω = 2πf: construct a complex MNA matrix G_c, stamp every
     element with its complex admittance or small-signal model, then solve
     `G_c · x_c = b_c` via complex Gaussian elimination.
  4. Return one `AcPoint` per frequency containing phasor node voltages.

  **Linear element AC admittances:**
  - Resistor: `Y = 1/R` (purely real, frequency-independent)
  - Capacitor: `Y_C = jωC` (open circuit at DC, purely imaginary)
  - Inductor: `Y_L = 1/(jωL)` (short circuit at DC → modelled as `G = 1e12 S`
    when `ω = 0` to keep the matrix non-singular)
  - VoltageSource: ideal AC source at its `voltage` amplitude; a 0 V source is a
    short circuit (correct for DC-bias sources in AC analysis)
  - CurrentSource: phasor current injection into the RHS vector

  **Nonlinear element small-signal models** (linearised at DC OP):
  - **Diode**: `gd = (Is/Vt) · exp(Vd/Vt)` — small-signal conductance between
    anode and cathode; no Norton offset (DC terms vanish in AC)
  - **MOSFET**: `gds` (output conductance, drain–source) + `gm` VCCS
    (gate–source controls drain–source current); same stamp pattern as the DC
    Newton stamp but in the complex domain
  - **BJT**: `g_π = gm/beta_f` (junction conductance, B–E for NPN, E–B for PNP)
    + `gm` VCCS (junction voltage controls collector current)

  **Robustness:** if the AC MNA matrix is singular at a particular frequency
  (e.g. a floating node), all node voltages for that frequency are set to zero
  and the sweep continues.

- **`AcPoint` dataclass** — Phasor voltages at one frequency.
  - Fields: `freq` (Hz), `node_voltages` (dict `str → complex`).
  - Use `abs(v)` for magnitude, `cmath.phase(v)` for phase in radians.

- **`AcResult` dataclass** — Frequency-sweep output.
  - Field: `points` (list of `AcPoint`, ascending by frequency, empty when
    `n_points < 1`).

- **`_solve_complex` helper** — Gaussian elimination with partial pivoting for
  complex-valued matrices.  Same algorithm as `_solve` but operates on
  `list[list[complex]]` and `list[complex]`; pivot selection uses `abs()` (complex
  modulus) to choose the largest-magnitude pivot.

- **`_stamp_g_c` helper** — Stamps a complex admittance between two nodes onto the
  complex MNA matrix.  Parallel to the real-valued `_stamp_g` used in DC analysis.

- **`_stamp_ac` helper** — Dispatches AC stamping for all supported element types.

### Changed

- `__init__.py` now exports `AcPoint`, `AcResult`, `ac_sweep`.
- Version bumped: `0.3.0` → `0.4.0`.
- Package description updated to mention AC analysis.

### Tests

- **Section 15 — Complex linear solver** (5 tests):
  - `test_solve_complex_2x2_real_system` — matches real solver output
  - `test_solve_complex_purely_imaginary_diagonal` — imaginary diagonal matrix
  - `test_solve_complex_empty` — empty system returns empty list
  - `test_solve_complex_singular_raises` — singular matrix raises `ZeroDivisionError`
  - `test_solve_complex_3x3` — verifies A·x = b for a 3×3 complex system

- **Section 16 — Data structures** (5 tests):
  - `test_acpoint_fields`, `test_acresult_fields` — field storage
  - `test_ac_sweep_returns_acresult` — return type check
  - `test_ac_sweep_point_count` — exactly n_points points returned
  - `test_ac_sweep_zero_points`, `test_ac_sweep_single_point` — edge cases
  - `test_ac_sweep_point_has_node_voltages` — node names present in each point
  - `test_ac_sweep_frequencies_ascending` — frequencies in order

- **Section 17 — Resistive circuits** (3 tests):
  - `test_ac_resistive_voltage_divider_real_valued` — gain=0.5, Im≈0
  - `test_ac_source_node_equals_source_voltage` — source node matches amplitude
  - `test_ac_unequal_resistive_divider` — R2/(R1+R2) gain at all frequencies

- **Section 18 — RC low-pass filter** (5 tests):
  - `test_ac_rc_lowpass_dc_gain_unity` — gain ≈ 1 at very low f
  - `test_ac_rc_lowpass_3db_at_cutoff` — |H| = 1/√2 at f_c = 1/(2πRC)
  - `test_ac_rc_lowpass_phase_at_cutoff` — phase ≈ −45° at f_c
  - `test_ac_rc_lowpass_rolloff_above_cutoff` — 20 dB/decade roll-off
  - `test_ac_rc_lowpass_gain_monotone_decreasing` — strict monotone decrease

- **Section 19 — RL high-pass filter** (2 tests):
  - `test_ac_rl_highpass_gain_increases_with_frequency` — monotone increasing
  - `test_ac_rl_highpass_3db_at_cutoff` — 1/√2 at f_c = R/(2πL)

- **Section 20 — Sweep modes** (4 tests):
  - Log/lin first and last frequency endpoints
  - Linear spacing uniformity
  - Log decade spacing ratio

- **Section 21 — Small-signal nonlinear elements** (3 tests):
  - `test_ac_diode_small_signal_forward_biased` — heavy shunting when forward-biased
  - `test_ac_diode_reverse_biased_acts_like_open` — voltage divider unchanged
  - `test_ac_bjt_npn_small_signal` — node presence and convergence

- **Section 22 — Current source injection** (3 tests):
  - `test_ac_current_source_into_resistor` — V = I×R at all frequencies
  - `test_ac_current_source_with_capacitor_shunt` — voltage decreases with frequency
  - `test_ac_inductor_acts_as_short_at_very_low_frequency` — near-unity gain at DC

---

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
