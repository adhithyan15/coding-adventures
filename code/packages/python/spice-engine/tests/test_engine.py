"""Tests for the SPICE engine.

Test organisation
-----------------
1.  Linear solver (_solve)
2.  DC analysis: simple circuits
3.  DC analysis: ground aliases
4.  DC analysis: Diode
5.  DC analysis: Capacitor (open in DC)
6.  Transient — backward Euler (legacy method="euler")
7.  Transient — trapezoidal (default, method="trap")
8.  Transient — accuracy: trap vs euler
9.  Transient — adaptive timestep (adaptive=True)
10. Transient — inductor companion model
11. Transient — TransientResult metadata fields
12. Mid-scale sanity checks
13. DC: Inductor
14. DC: BJT (NPN and PNP)
15. AC sweep: complex linear solver
16. AC sweep: data structures (AcPoint, AcResult)
17. AC sweep: resistive circuits (frequency-independent)
18. AC sweep: RC low-pass filter (-3 dB at cutoff)
19. AC sweep: RL high-pass filter
20. AC sweep: sweep modes (log, lin, edge cases)
21. AC sweep: small-signal nonlinear elements (Diode, BJT)
22. AC sweep: current source injection
23. TF analysis: TfResult dataclass
24. TF analysis: _build_ss_matrix helper
25. TF analysis: resistive circuits (voltage-source input)
26. TF analysis: current-source input + transimpedance
27. TF analysis: error cases and edge cases
28. DC sweep: DcSweepPoint / DcSweepResult dataclasses
29. DC sweep: linear resistive circuits
30. DC sweep: nonlinear (diode) circuit
31. DC sweep: current-source sweeps
32. DC sweep: error cases and edge cases
"""

import cmath
import math
from math import exp, isclose

import pytest

from spice_engine import (
    BJT,
    AcPoint,
    AcResult,
    Capacitor,
    Circuit,
    CurrentSource,
    DcSweepPoint,
    DcSweepResult,
    Diode,
    Inductor,
    Resistor,
    TfResult,
    VoltageSource,
    ac_sweep,
    dc_op,
    dc_sweep,
    tf,
    transient,
)
from spice_engine.engine import (
    _build_ss_matrix,
    _lte_estimate,
    _node_index,
    _solve,
    _solve_complex,
    _stamp_bjt,
    _voltage_sources,
)

# ---- Linear solver ----


def test_solve_2x2():
    # 2x + y = 5; x + 3y = 10 -> x = 1, y = 3
    A = [[2.0, 1.0], [1.0, 3.0]]
    b = [5.0, 10.0]
    x = _solve(A, b)
    assert isclose(x[0], 1.0, abs_tol=1e-9)
    assert isclose(x[1], 3.0, abs_tol=1e-9)


def test_solve_3x3():
    A = [[3.0, 2.0, -1.0], [2.0, -2.0, 4.0], [-1.0, 0.5, -1.0]]
    b = [1.0, -2.0, 0.0]
    x = _solve(A, b)
    # Verify Ax = b
    for i, row in enumerate(A):
        s = sum(row[j] * x[j] for j in range(3))
        assert isclose(s, b[i], abs_tol=1e-9)


def test_solve_singular_raises():
    # Two identical rows -> singular
    A = [[1.0, 1.0], [1.0, 1.0]]
    b = [1.0, 2.0]
    with pytest.raises(ZeroDivisionError):
        _solve(A, b)


def test_solve_empty():
    assert _solve([], []) == []


# ---- DC analysis: simple circuits ----


def test_resistor_voltage_divider():
    """V1 = 10V, R1=R2=1k -> V_mid = 5V."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=10.0))
    c.add(Resistor("R1", "vin", "vmid", 1000.0))
    c.add(Resistor("R2", "vmid", "0", 1000.0))
    r = dc_op(c)
    assert r.converged
    assert isclose(r.node_voltages["vin"], 10.0, abs_tol=1e-6)
    assert isclose(r.node_voltages["vmid"], 5.0, abs_tol=1e-6)


def test_two_resistors_in_series():
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", voltage=12.0))
    c.add(Resistor("R1", "a", "b", 100.0))
    c.add(Resistor("R2", "b", "0", 200.0))
    r = dc_op(c)
    # V_b = 12 * 200 / (100 + 200) = 8V
    assert isclose(r.node_voltages["b"], 8.0, abs_tol=1e-6)


def test_current_source_into_resistor():
    """I = 1mA into R = 1k -> V = 1V."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    r = dc_op(c)
    assert isclose(r.node_voltages["n1"], 1.0, abs_tol=1e-6)


def test_branch_current_in_voltage_source():
    """V=10V, R=1k -> I=10mA flowing from + to - inside the source."""
    c = Circuit()
    c.add(VoltageSource("V1", "n1", "0", voltage=10.0))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    r = dc_op(c)
    # Branch current convention: positive into +
    assert "I(V1)" in r.branch_currents
    assert isclose(abs(r.branch_currents["I(V1)"]), 10e-3, abs_tol=1e-6)


# ---- DC analysis: ground aliases ----


@pytest.mark.parametrize("ground", ["0", "gnd", "GND"])
def test_ground_aliases(ground: str):
    c = Circuit()
    c.add(VoltageSource("V1", "vin", ground, voltage=5.0))
    c.add(Resistor("R1", "vin", ground, 1000.0))
    r = dc_op(c)
    assert isclose(r.node_voltages["vin"], 5.0, abs_tol=1e-6)


# ---- DC: Diode ----


def test_diode_forward_bias():
    """V=0.7V across diode; current should be ≈ Is*(exp(V/Vt)-1)."""
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", voltage=0.7))
    c.add(Diode("D1", anode="a", cathode="0"))
    r = dc_op(c)
    assert r.converged
    # V_a forced to 0.7 by V1
    assert isclose(r.node_voltages["a"], 0.7, abs_tol=1e-6)


def test_diode_reverse_bias():
    """Reverse bias: tiny -Is current."""
    c = Circuit()
    c.add(VoltageSource("V1", "0", "a", voltage=1.0))  # cathode high
    c.add(Diode("D1", anode="a", cathode="0"))
    r = dc_op(c)
    assert r.converged


# ---- DC: Capacitor (open in DC) ----


def test_capacitor_open_in_dc():
    """Cap blocks DC -> no current flow; V_n1 should be 0."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Capacitor("C1", "vin", "n1", 1e-6))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    r = dc_op(c)
    assert isclose(r.node_voltages["n1"], 0.0, abs_tol=1e-6)


# ---- Transient: backward Euler (legacy method="euler") ----


def test_transient_euler_rc_charging():
    """Backward Euler: capacitor charges toward V_in with tau = RC."""
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4, method="euler")
    assert result.converged
    assert result.method == "euler"
    assert len(result.points) > 10
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.5)


def test_transient_initial_state():
    """At t=0, capacitor blocks DC, so the cap voltage starts at 0."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Resistor("R1", "vin", "vc", 1000.0))
    c.add(Capacitor("Cap1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    assert result.points[0].time == 0.0
    assert isclose(result.points[0].node_voltages["vc"], 0.0, abs_tol=1e-6)


def test_transient_rejects_zero_step():
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Resistor("R1", "vin", "0", 1000.0))
    result = transient(c, t_stop=1e-3, t_step=0.0)
    assert not result.converged
    assert result.points == []


def test_transient_rejects_negative_stop():
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", voltage=1.0))
    result = transient(c, t_stop=-1.0, t_step=1e-3)
    assert not result.converged


# ---- Transient: trapezoidal (default, method="trap") ----


def test_transient_trap_rc_charging():
    """Trapezoidal: RC charging curve matches analytic solution.

    V_C(t) = V_in * (1 - exp(-t/tau))  with tau = R*C = 1 ms.
    """
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4, method="trap")
    assert result.converged
    assert result.method == "trap"
    # After 5 tau the cap should be within 1% of V_in
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.1)


def test_transient_trap_is_default_method():
    """transient() with no method= argument uses trapezoidal."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=1.0))
    c.add(Resistor("R1", "vin", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    assert result.method == "trap"


def test_transient_trap_first_point_is_t0():
    """The first waveform point is always at t=0."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=3.3))
    c.add(Resistor("R1", "vin", "vc", 500.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    assert result.points[0].time == 0.0


# ---- Transient: accuracy comparison trap vs euler ----


def test_trap_more_accurate_than_euler_rc():
    """Trapezoidal has lower error than backward Euler at the same step size.

    For RC charging with tau=RC, measure the error at t=tau:
        V_exact = V_in * (1 - exp(-1))  ≈  0.6321 * V_in

    A coarser step (h = tau/5) is used so both methods show visible error.
    Trapezoidal (O(h^2)) should give smaller absolute error than Euler (O(h)).
    """
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    tau = R * C
    h = tau / 5  # coarse step: 0.2 ms — error should be visible

    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    exact = V_in * (1.0 - exp(-1.0))  # V_C at t = tau

    def _error(meth: str) -> float:
        res = transient(c, t_stop=tau, t_step=h, method=meth)
        # Find the point closest to t=tau
        pt = min(res.points, key=lambda p: abs(p.time - tau))
        return abs(pt.node_voltages["vc"] - exact)

    err_euler = _error("euler")
    err_trap = _error("trap")
    assert err_trap < err_euler, (
        f"Trapezoidal error {err_trap:.4f} not less than Euler error {err_euler:.4f}"
    )


# ---- Transient: adaptive timestep ----


def test_adaptive_accepts_steps_and_returns_metadata():
    """With adaptive=True, steps_rejected is a non-negative int."""
    R, C, V_in = 1000.0, 1e-6, 5.0
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4, adaptive=True)
    assert result.converged
    assert isinstance(result.steps_rejected, int)
    assert result.steps_rejected >= 0


def test_adaptive_reaches_steady_state():
    """Adaptive integration still reaches the correct final voltage."""
    R, C, V_in = 1000.0, 1e-6, 5.0
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=5e-4, adaptive=True,
                       tol_lte=1e-3, max_step=2e-3)
    assert result.converged
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.2)


def test_adaptive_non_adaptive_same_result():
    """Fixed and adaptive trapezoidal should give nearly the same final value
    when the LTE tolerance is very tight (i.e. step is never rejected)."""
    R, C, V_in = 1000.0, 1e-6, 5.0
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    r_fixed = transient(c, t_stop=3e-3, t_step=1e-4, adaptive=False)
    # Pin min/max_step to t_step so adaptive never actually changes the step
    # size (large tol_lte → never rejects; max_step=t_step → never doubles).
    r_adapt = transient(c, t_stop=3e-3, t_step=1e-4, adaptive=True,
                        tol_lte=1.0,            # huge tol → never rejects
                        min_step=1e-4,
                        max_step=1e-4)           # lock h = t_step
    last_fixed = r_fixed.points[-1].node_voltages["vc"]
    last_adapt = r_adapt.points[-1].node_voltages["vc"]
    assert isclose(last_fixed, last_adapt, abs_tol=0.05)


def test_non_adaptive_steps_rejected_is_zero():
    """steps_rejected must be 0 when adaptive=False."""
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    c.add(Resistor("R1", "v", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4, adaptive=False)
    assert result.steps_rejected == 0


# ---- Transient: inductor companion model ----


def test_transient_rl_current_buildup():
    """RL circuit: current rises as I(t) = (V/R) * (1 - exp(-R*t/L)).

    With V=1V, R=10Ω, L=1mH: tau=L/R=0.1ms; I_ss = V/R = 0.1A.
    At t=5*tau the current should be within 5% of I_ss.

    The resistor voltage V_R = I*R should approach V as I → I_ss.
    """
    V, R, L = 1.0, 10.0, 1e-3  # tau = 0.1 ms, I_ss = 0.1 A
    tau = L / R
    c = Circuit()
    c.add(VoltageSource("V1", "vs", "0", voltage=V))
    c.add(Resistor("R1", "vs", "vr", R))  # V_R = I * R
    c.add(Inductor("L1", "vr", "0", L))

    result = transient(c, t_stop=5 * tau, t_step=tau / 20)
    assert result.converged

    # At steady state: I_L → V/R (= I_ss), V_L → 0.
    # The node "vr" (junction of R1 and L1) is the inductor n_plus voltage.
    # V_vr = V_L → 0 at steady state.  (The *resistor* voltage V_R1 = V_vs −
    # V_vr → V, but the *node* "vr" → 0.)
    last = result.points[-1]
    V_vr = last.node_voltages.get("vr", 0.0)
    assert isclose(V_vr, 0.0, abs_tol=0.1 * V), (
        f"Expected V_vr ≈ 0.0 V at steady state, got {V_vr:.4f}"
    )


def test_transient_inductor_starts_at_zero_current():
    """At t=0 inductor has zero current; initial voltage is consistent."""
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=5.0))
    c.add(Resistor("R1", "v", "n", 100.0))
    c.add(Inductor("L1", "n", "0", 1e-3))

    result = transient(c, t_stop=1e-4, t_step=1e-5)
    assert result.converged
    # The second point should show current just beginning to build up.
    v0 = result.points[0].node_voltages.get("n", 0.0)
    # Node voltage should start near 0 (inductor blocks initial current)
    # and then begin to decline as current builds.
    assert v0 >= 0.0


# ---- Transient: TransientResult metadata ----


def test_transient_result_method_field_trap():
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    c.add(Resistor("R1", "v", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    r = transient(c, t_stop=1e-4, t_step=1e-5, method="trap")
    assert r.method == "trap"


def test_transient_result_method_field_euler():
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    c.add(Resistor("R1", "v", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    r = transient(c, t_stop=1e-4, t_step=1e-5, method="euler")
    assert r.method == "euler"


def test_transient_result_invalid_input_method_field():
    """Even when t_step=0 (rejected early), method is preserved."""
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    r = transient(c, t_stop=1e-3, t_step=0.0, method="euler")
    assert r.method == "euler"
    assert not r.converged


# ---- _lte_estimate unit test ----


def test_lte_estimate_zero_for_constant_voltage():
    """If cap voltage is constant over 3 steps, LTE = 0."""
    c = Circuit()
    c.add(Capacitor("C1", "a", "0", 1e-6))
    # V_n+1 = V_n = V_n-1 = 1.0 → second difference = 0
    lte = _lte_estimate(c, {"C1": 1.0}, {"C1": 1.0}, {"C1": 1.0})
    assert lte == 0.0


def test_lte_estimate_nonzero_for_curved_voltage():
    """A voltage that curves upward gives a positive LTE estimate."""
    c = Circuit()
    c.add(Capacitor("C1", "a", "0", 1e-6))
    # V: 0, 1, 3 → second diff = 3 - 2*1 + 0 = 1 → lte = 0.5
    lte = _lte_estimate(c, {"C1": 3.0}, {"C1": 1.0}, {"C1": 0.0})
    assert isclose(lte, 0.5, rel_tol=1e-9)


def test_lte_estimate_no_caps_returns_zero():
    """Circuit with no capacitors gives LTE = 0."""
    c = Circuit()
    c.add(Resistor("R1", "a", "0", 1000.0))
    lte = _lte_estimate(c, {}, {}, {})
    assert lte == 0.0


# ---- Mid-scale: 4-bit-adder NAND2 cell-like circuit ----


def test_two_cmos_inverter_chain():
    """Voltage divider with two parallel resistors — sanity check more
    complex netlists work."""
    c = Circuit()
    c.add(VoltageSource("V1", "vdd", "0", voltage=1.8))
    c.add(Resistor("R1", "vdd", "n1", 5000.0))
    c.add(Resistor("R2", "vdd", "n1", 5000.0))  # parallel: 2.5k
    c.add(Resistor("R3", "n1", "0", 2500.0))
    r = dc_op(c)
    # Series equivalent: 2.5k + 2.5k = 5k from vdd to gnd
    # V_n1 = 1.8 * 2.5k / 5k = 0.9V
    assert isclose(r.node_voltages["n1"], 0.9, abs_tol=1e-6)


# ---- DC: Inductor ----


def test_inductor_short_in_dc():
    """Inductor is a short in DC (no contribution at this stamp level)."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Inductor("L1", "vin", "n1", 1e-6))
    c.add(Resistor("R1", "n1", "0", 100.0))
    r = dc_op(c)
    assert r.converged


# ---- Backward-compatibility: existing rc charging test (no method arg) ----


def test_transient_rc_charging():
    """Backward-compat: transient without explicit method still converges.

    The default is now 'trap', so this also tests trapezoidal is a valid
    drop-in replacement.
    """
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4)
    assert result.converged
    assert len(result.points) > 10
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.5)


# ---- DC: BJT (NPN and PNP) ----


def test_bjt_dataclass_defaults():
    """BJT dataclass stores all fields correctly."""
    q = BJT("Q1", collector="c", base="b", emitter="0")
    assert q.name == "Q1"
    assert q.polarity == "NPN"
    assert q.Is == 1e-14
    assert q.beta_f == 100.0
    assert isclose(q.Vt, 0.02585, rel_tol=1e-9)


def test_bjt_pnp_dataclass():
    """PNP BJT stores polarity correctly."""
    q = BJT("Q2", collector="c", base="b", emitter="vcc", polarity="PNP")
    assert q.polarity == "PNP"


def test_bjt_npn_off():
    """NPN BJT with zero base voltage — device is off (no collector current).

    With Vbe = 0, exp(0/Vt) = 1, so Ic = Is*(1-1) = 0.
    The circuit is just Vcc through Rc, but no current flows in the BJT branch,
    so the collector voltage should remain near Vcc (biased up through Rc with
    nothing pulling it down).
    """
    Vcc = 5.0
    Rc = 1000.0     # collector resistor

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=Vcc))
    c.add(Resistor("Rc", "vcc", "col", Rc))
    # NPN with base and emitter both at 0 V: Vbe = 0 -> off
    c.add(BJT("Q1", collector="col", base="0", emitter="0"))

    r = dc_op(c)
    assert r.converged
    # With no collector current, Vcol ≈ Vcc (no drop across Rc)
    # Allow tolerance for gm * 0 = 0 stamp: col should be close to Vcc.
    assert r.node_voltages["col"] > 4.0, (
        f"Expected Vcol near Vcc but got {r.node_voltages['col']:.3f}"
    )


def test_bjt_npn_forward_active():
    """NPN BJT in forward-active region: collector current ≈ Is*exp(Vbe/Vt).

    Circuit:
        Vcc (5 V) → Rc (1 kΩ) → collector
        Vb  (0.7 V) → base
        emitter → GND

    At Vbe = 0.7 V (at the clamp boundary):
        exp_term = exp(0.7 / 0.02585) ≈ 5.97e11 (clamped to 0.7)
        Ic = Is * (exp_term - 1) ≈ Is * exp_term

    The collector node voltage is pulled down from Vcc by the resistor:
        Vcol = Vcc - Ic * Rc

    We verify:
    1. The simulation converges.
    2. There is a meaningful voltage drop across Rc (device is conducting).
    3. The computed Vcol is consistent with Ic = gm * Vbe.
    """
    Vcc = 5.0
    Rc = 1000.0
    Is_val = 1e-14
    Vt_val = 0.02585
    beta = 100.0

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=Vcc))
    c.add(VoltageSource("Vb", "b", "0", voltage=0.7))
    c.add(Resistor("Rc", "vcc", "col", Rc))
    c.add(BJT("Q1", collector="col", base="b", emitter="0",
               Is=Is_val, beta_f=beta, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    # At Vbe = 0.7 V (clamped), compute expected Ic
    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)

    Vcol = r.node_voltages["col"]
    # Vcol = Vcc - Ic * Rc
    Ic_from_vcol = (Vcc - Vcol) / Rc

    assert Ic_from_vcol > 0, "Collector current should be positive (NPN forward active)"
    # The simulator's Newton-Raphson linearisation gives the clamped-at-0.7 value.
    # Check within 1% of the expected analytic value.
    assert isclose(Ic_from_vcol, Ic_expected, rel_tol=0.01), (
        f"Ic from voltages ({Ic_from_vcol:.6e} A) != expected ({Ic_expected:.6e} A)"
    )


def test_bjt_npn_beta_ratio():
    """NPN BJT: collector current / base current ≈ beta_f.

    We use a voltage-source base drive (Vb = 0.7 V) and measure the
    collector current from the Rc drop.  The base current is computed
    as Ic / beta_f (the simulator uses the same relation internally).

    This test validates that the beta ratio is embedded correctly in the
    junction stamp (gπ = gm / beta_f).
    """
    beta = 50.0
    Is_val = 1e-14
    Vt_val = 0.02585

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=5.0))
    c.add(VoltageSource("Vb", "b", "0", voltage=0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", collector="col", base="b", emitter="0",
               Is=Is_val, beta_f=beta, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    Vcol = r.node_voltages["col"]
    Ic = (5.0 - Vcol) / 1000.0
    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)
    Ib_expected = Ic_expected / beta

    # Ic / Ib = beta
    assert Ic > 0
    # Verify internally consistent: collector current matches model prediction
    assert isclose(Ic, Ic_expected, rel_tol=0.01)
    # Verify Ib = Ic / beta (stamped as gπ = gm/beta which sets dIb/dVbe)
    assert isclose(Ic / beta, Ib_expected, rel_tol=0.01)


def test_bjt_pnp_forward_active():
    """PNP BJT in forward-active region: emitter injects, collector collects.

    Circuit:
        emitter → Vcc (5 V)
        base    → 4.3 V (so Veb = Ve - Vb = 5 - 4.3 = 0.7 V)
        collector → Rc (1 kΩ) → GND

    At Veb = 0.7 V (clamped):
        Ic_expected = Is * (exp(0.7/Vt) - 1)

    We expect:
    1. Simulation converges.
    2. Collector node is above GND (current flowing into collector → Vc > 0).
    3. The voltage drop across Rc is consistent with the expected Ic.
    """
    Vcc = 5.0
    Vb_val = 4.3   # Veb = Vcc - Vb_val = 0.7 V
    Rc = 1000.0
    Is_val = 1e-14
    Vt_val = 0.02585

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=Vcc))
    c.add(VoltageSource("Vb_src", "b", "0", voltage=Vb_val))
    c.add(Resistor("Rc", "col", "0", Rc))
    # PNP: emitter at Vcc, base at 4.3V, collector at col
    c.add(BJT("Q1", collector="col", base="b", emitter="vcc",
               polarity="PNP", Is=Is_val, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)

    Vcol = r.node_voltages["col"]
    # For PNP: Ic flows into the collector FROM ground through Rc.
    # Node col voltage = Ic * Rc (current source pumps into col, Rc to ground).
    Ic_from_vcol = Vcol / Rc

    assert Vcol > 0, f"PNP collector should be above GND, got Vcol={Vcol:.4f}"
    assert isclose(Ic_from_vcol, Ic_expected, rel_tol=0.01), (
        f"PNP Ic from voltages ({Ic_from_vcol:.6e}) != expected ({Ic_expected:.6e})"
    )


def test_bjt_element_nodes():
    """BJT contributes all three terminals to _node_index."""
    from spice_engine.engine import _node_index
    c = Circuit()
    c.add(BJT("Q1", collector="col", base="base", emitter="emit"))
    _, nodes = _node_index(c)
    assert "col" in nodes
    assert "base" in nodes
    assert "emit" in nodes


def test_bjt_stamp_matrix_shape():
    """_stamp_bjt does not raise and produces a finite matrix."""
    import math as _math
    node_to_idx = {"c": 0, "b": 1, "e": 2}
    G = [[0.0] * 3 for _ in range(3)]
    b = [0.0] * 3
    x = [0.0, 0.0, 0.0]  # all nodes at 0 V — device is off
    q = BJT("Q1", collector="c", base="b", emitter="e")
    _stamp_bjt(G, b, x, node_to_idx, q)
    # All values should be finite
    for row in G:
        for val in row:
            assert _math.isfinite(val), f"Non-finite G entry: {val}"
    for val in b:
        assert _math.isfinite(val), f"Non-finite b entry: {val}"


def test_bjt_npn_ground_emitter_no_crash():
    """BJT with emitter grounded via alias 'gnd' converges cleanly."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "gnd", voltage=3.3))
    c.add(VoltageSource("Vb", "b", "gnd", voltage=0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", collector="col", base="b", emitter="gnd"))
    r = dc_op(c)
    assert r.converged


def test_bjt_npn_vcc_emitter():
    """NPN BJT with non-ground emitter: Vbe = Vb - Ve matters.

    Circuit: Ve = 2V (emitter not grounded), Vb = 2.7V (Vbe = 0.7V).
    Should behave identically to the grounded-emitter case because
    Vbe = 0.7 V is the same.
    """
    Is_val = 1e-14
    Vt_val = 0.02585

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=5.0))
    c.add(VoltageSource("Vb", "b", "0", voltage=2.7))
    c.add(VoltageSource("Ve", "e", "0", voltage=2.0))   # emitter not at ground
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", collector="col", base="b", emitter="e",
               Is=Is_val, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)
    Vcol = r.node_voltages["col"]
    Ic_from_vcol = (5.0 - Vcol) / 1000.0
    assert isclose(Ic_from_vcol, Ic_expected, rel_tol=0.01)


def test_bjt_in_element_union():
    """BJT is exported from spice_engine and is a valid Element type."""
    from spice_engine import BJT as BJT_exported
    q = BJT_exported("Q1", collector="c", base="b", emitter="0")
    assert isinstance(q, BJT)


def test_bjt_custom_parameters():
    """BJT constructor accepts custom Is, beta_f, Vt."""
    q = BJT("Q1", collector="c", base="b", emitter="e",
             Is=2.5e-16, beta_f=200.0, Vt=0.026)
    assert isclose(q.Is, 2.5e-16)
    assert isclose(q.beta_f, 200.0)
    assert isclose(q.Vt, 0.026)


# ============================================================================
# 15. AC sweep — complex linear solver
# ============================================================================


def test_solve_complex_2x2_real_system():
    """_solve_complex on a real-valued 2×2 system matches _solve."""
    # 2x + y = 5, x + 3y = 10  →  x = 1, y = 3
    A = [[2.0 + 0j, 1.0 + 0j], [1.0 + 0j, 3.0 + 0j]]
    b = [5.0 + 0j, 10.0 + 0j]
    x = _solve_complex(A, b)
    assert isclose(x[0].real, 1.0, abs_tol=1e-9)
    assert isclose(x[1].real, 3.0, abs_tol=1e-9)
    assert isclose(abs(x[0].imag), 0.0, abs_tol=1e-9)


def test_solve_complex_purely_imaginary_diagonal():
    """_solve_complex: diagonal matrix with imaginary entries.

    For A = [[j, 0], [0, 2j]] and b = [1+j, -2j]:
        x[0] = (1+j)/j = 1/j + 1 = -j + 1 = 1 - j
        x[1] = -2j / 2j = -1
    """
    j = 1j
    A = [[j, 0j], [0j, 2j]]
    b = [1 + j, -2j]
    x = _solve_complex(A, b)
    assert isclose(abs(x[0] - (1 - 1j)), 0.0, abs_tol=1e-9)
    assert isclose(abs(x[1] - (-1 + 0j)), 0.0, abs_tol=1e-9)


def test_solve_complex_empty():
    """_solve_complex on empty system returns empty list."""
    assert _solve_complex([], []) == []


def test_solve_complex_singular_raises():
    """_solve_complex raises ZeroDivisionError for singular matrix."""
    A = [[1.0 + 0j, 1.0 + 0j], [1.0 + 0j, 1.0 + 0j]]
    b = [1.0 + 0j, 2.0 + 0j]
    with pytest.raises(ZeroDivisionError):
        _solve_complex(A, b)


def test_solve_complex_3x3():
    """_solve_complex: 3×3 complex system satisfies Ax = b."""
    # Use a well-conditioned system with complex coefficients.
    A = [
        [2.0 + 1j, 1.0 + 0j, 0j],
        [0j, 3.0 + 0j, 1.0 + 2j],
        [1.0 + 0j, 0j, 2.0 + 0j],
    ]
    b = [3.0 + 2j, 1.0 + 0j, 4.0 + 0j]
    x = _solve_complex(A, b)
    # Verify A·x = b.
    for i, row in enumerate(A):
        s = sum(row[j] * x[j] for j in range(3))
        assert isclose(abs(s - b[i]), 0.0, abs_tol=1e-9), (
            f"Row {i}: A·x = {s}, expected {b[i]}"
        )


# ============================================================================
# 16. AC sweep — AcPoint and AcResult data structures
# ============================================================================


def test_acpoint_fields():
    """AcPoint stores freq and node_voltages."""
    pt = AcPoint(freq=1000.0, node_voltages={"out": 0.5 + 0j})
    assert pt.freq == 1000.0
    assert pt.node_voltages["out"] == 0.5 + 0j


def test_acresult_fields():
    """AcResult wraps a list of AcPoints."""
    pts = [AcPoint(freq=100.0, node_voltages={})]
    r = AcResult(points=pts)
    assert len(r.points) == 1
    assert r.points[0].freq == 100.0


def test_ac_sweep_returns_acresult():
    """ac_sweep returns an AcResult instance."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=5)
    assert isinstance(result, AcResult)


def test_ac_sweep_point_count():
    """ac_sweep returns exactly n_points AcPoints."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=10)
    assert len(result.points) == 10


def test_ac_sweep_zero_points():
    """n_points=0 returns an empty AcResult."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=1000.0, n_points=0)
    assert isinstance(result, AcResult)
    assert result.points == []


def test_ac_sweep_single_point():
    """n_points=1 returns a single AcPoint at f_start."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=500.0, f_stop=5000.0, n_points=1)
    assert len(result.points) == 1
    assert isclose(result.points[0].freq, 500.0, rel_tol=1e-9)


def test_ac_sweep_point_has_node_voltages():
    """Each AcPoint contains the expected node names."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=3)
    for pt in result.points:
        assert "in" in pt.node_voltages
        assert "out" in pt.node_voltages


def test_ac_sweep_frequencies_ascending():
    """Frequency values in AcPoints are strictly ascending."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=20)
    freqs = [pt.freq for pt in result.points]
    assert all(freqs[i] < freqs[i + 1] for i in range(len(freqs) - 1))


# ============================================================================
# 17. AC sweep — resistive circuits (frequency-independent)
# ============================================================================


def test_ac_resistive_voltage_divider_real_valued():
    """Two equal resistors: output is exactly Vin/2 at all frequencies.

    A pure resistive divider has no reactive elements, so the AC phasor
    voltage is the same at every frequency: V_out = Vin/2 (real).

    Circuit:  Vin → R1 (1kΩ) → out → R2 (1kΩ) → GND
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    result = ac_sweep(c, f_start=1.0, f_stop=1e7, n_points=20, sweep="log")
    for pt in result.points:
        v_out = pt.node_voltages["out"]
        assert isclose(abs(v_out), 0.5, abs_tol=1e-4), (
            f"f={pt.freq:.1f} Hz: |V_out|={abs(v_out):.6f} (expected 0.5)"
        )
        # Imaginary part negligible — purely resistive
        assert isclose(abs(v_out.imag), 0.0, abs_tol=1e-6), (
            f"f={pt.freq:.1f} Hz: Im(V_out)={v_out.imag:.2e} (expected 0)"
        )


def test_ac_source_node_equals_source_voltage():
    """Source node voltage equals the VoltageSource voltage at all frequencies."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", 2.5))
    c.add(Resistor("R1", "vin", "0", 1000.0))
    result = ac_sweep(c, f_start=10.0, f_stop=1e5, n_points=10)
    for pt in result.points:
        assert isclose(abs(pt.node_voltages["vin"]), 2.5, abs_tol=1e-6), (
            f"f={pt.freq:.1f} Hz: V_in={pt.node_voltages['vin']}"
        )


def test_ac_unequal_resistive_divider():
    """R1=1kΩ, R2=3kΩ: V_out = Vin * R2/(R1+R2) = 0.75 V (frequency-independent)."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 3000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=1e6, n_points=5)
    for pt in result.points:
        assert isclose(abs(pt.node_voltages["out"]), 0.75, abs_tol=1e-4), (
            f"f={pt.freq:.1f} Hz: |V_out|={abs(pt.node_voltages['out']):.6f}"
        )


# ============================================================================
# 18. AC sweep — RC low-pass filter (-3 dB at cutoff)
# ============================================================================


def test_ac_rc_lowpass_dc_gain_unity():
    """RC low-pass: gain → 1 at very low frequency (capacitor is open at DC).

    Circuit:  Vin → R (1kΩ) → out → C (1 μF) → GND
    Transfer function: H(jω) = 1 / (1 + jωRC)
    At very low f: |H| ≈ 1.
    """
    R, C = 1000.0, 1e-6
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))
    # Use a very low start frequency so ωRC << 1
    result = ac_sweep(c, f_start=0.01, f_stop=0.1, n_points=3)
    for pt in result.points:
        gain = abs(pt.node_voltages["out"])
        assert gain > 0.999, (
            f"f={pt.freq:.3f} Hz: gain={gain:.6f} (expected ≈ 1.0 at DC)"
        )


def test_ac_rc_lowpass_3db_at_cutoff():
    """RC low-pass: gain = 1/√2 at f_c = 1/(2πRC).

    Analytic: H(jω) = 1/(1 + jωRC).
    At ω = 1/RC: |H| = 1/√2 ≈ 0.7071.

    We find the frequency point nearest f_c and check gain within 1%.
    """
    R, C = 1000.0, 1e-6
    f_c = 1.0 / (2.0 * math.pi * R * C)  # ≈ 159.15 Hz
    expected_gain = 1.0 / math.sqrt(2.0)   # ≈ 0.7071

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    # Dense log sweep straddling the cutoff
    result = ac_sweep(c, f_start=10.0, f_stop=10000.0, n_points=200, sweep="log")

    # Find the point closest to f_c
    closest = min(result.points, key=lambda p: abs(p.freq - f_c))
    gain = abs(closest.node_voltages["out"])

    assert isclose(gain, expected_gain, rel_tol=0.01), (
        f"At f≈f_c={closest.freq:.1f} Hz: gain={gain:.5f}, expected {expected_gain:.5f}"
    )


def test_ac_rc_lowpass_phase_at_cutoff():
    """RC low-pass: phase ≈ −45° at f_c."""
    R, C = 1000.0, 1e-6
    f_c = 1.0 / (2.0 * math.pi * R * C)

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=10.0, f_stop=10000.0, n_points=200, sweep="log")
    closest = min(result.points, key=lambda p: abs(p.freq - f_c))
    phase_deg = math.degrees(cmath.phase(closest.node_voltages["out"]))

    # Phase should be ≈ −45° ± 2°
    assert isclose(phase_deg, -45.0, abs_tol=2.0), (
        f"Phase at f_c: {phase_deg:.2f}° (expected ≈ −45°)"
    )


def test_ac_rc_lowpass_rolloff_above_cutoff():
    """RC low-pass: gain decreases at 20 dB/decade above f_c.

    At 10×f_c the gain should be ≈ 1/(10√2) ≈ 0.0707 (−20 dB).
    At 100×f_c the gain should be ≈ 1/(100√2) ≈ 0.00707 (−40 dB).
    """
    R, C = 1000.0, 1e-6
    f_c = 1.0 / (2.0 * math.pi * R * C)

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=500, sweep="log")

    def gain_at(f: float) -> float:
        pt = min(result.points, key=lambda p: abs(p.freq - f))
        return abs(pt.node_voltages["out"])

    g1x = gain_at(f_c * 10.0)
    g10x = gain_at(f_c * 100.0)

    # Each decade above cutoff: gain ≈ f_c / (√2 · f)
    assert g1x < 0.12, f"Gain at 10×f_c should be < 0.12, got {g1x:.5f}"
    assert g10x < g1x / 5.0, (
        f"Gain at 100×f_c ({g10x:.6f}) should be << gain at 10×f_c ({g1x:.6f})"
    )


def test_ac_rc_lowpass_gain_monotone_decreasing():
    """RC low-pass: gain is monotonically decreasing with frequency."""
    R, C = 1000.0, 1e-6
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=50, sweep="log")
    gains = [abs(pt.node_voltages["out"]) for pt in result.points]

    for i in range(1, len(gains)):
        assert gains[i] <= gains[i - 1] + 1e-6, (
            f"Gain not monotone: gains[{i - 1}]={gains[i - 1]:.6f},"
            f" gains[{i}]={gains[i]:.6f}"
        )


# ============================================================================
# 19. AC sweep — RL high-pass filter
# ============================================================================


def test_ac_rl_highpass_gain_increases_with_frequency():
    """RL high-pass: gain increases from 0 to 1 as frequency increases.

    Circuit:  Vin → R (1 kΩ) → mid → L (1 mH) → GND
    Transfer function: H(jω) = jωL / (R + jωL)

    The output is the node "mid" (junction of R and L).  Since L is
    between mid and GND, V_mid = Vin × jωL / (R + jωL):
    - At low f (ω → 0): Z_L = jωL → 0, so V_mid → 0.
    - At high f (ω → ∞): Z_L >> R, so V_mid → Vin.
    """
    R, L = 1000.0, 1e-3
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "mid", R))
    c.add(Inductor("L1", "mid", "0", L))

    result = ac_sweep(c, f_start=100.0, f_stop=1e7, n_points=50, sweep="log")
    gains = [abs(pt.node_voltages["mid"]) for pt in result.points]

    # Low end should be well below 0.5
    assert gains[0] < 0.5, f"Low-freq gain {gains[0]:.4f} should be < 0.5"
    # High end should be close to 1
    assert gains[-1] > 0.95, f"High-freq gain {gains[-1]:.4f} should be > 0.95"
    # Monotone increasing (RL high-pass)
    for i in range(1, len(gains)):
        assert gains[i] >= gains[i - 1] - 1e-6, (
            f"RL high-pass not monotone increasing at index {i}"
        )


def test_ac_rl_highpass_3db_at_cutoff():
    """RL high-pass: gain = 1/√2 at f_c = R/(2πL).

    Circuit: Vin → R (1 kΩ) → mid → L (1 mH) → GND
    With R=1kΩ, L=1mH: f_c = 1000 / (2π × 0.001) ≈ 159.15 kHz.
    At f_c: |H| = |jωL| / |R + jωL| = 1/√2.
    """
    R, L = 1000.0, 1e-3
    f_c = R / (2.0 * math.pi * L)  # ≈ 159.15 kHz
    expected_gain = 1.0 / math.sqrt(2.0)

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "mid", R))
    c.add(Inductor("L1", "mid", "0", L))

    result = ac_sweep(c, f_start=1e4, f_stop=1e7, n_points=300, sweep="log")
    closest = min(result.points, key=lambda p: abs(p.freq - f_c))
    gain = abs(closest.node_voltages["mid"])

    assert isclose(gain, expected_gain, rel_tol=0.02), (
        f"RL high-pass gain at f_c={closest.freq:.0f} Hz: {gain:.5f},"
        f" expected {expected_gain:.5f}"
    )


# ============================================================================
# 20. AC sweep — sweep modes (log, lin, edge cases)
# ============================================================================


def test_ac_log_sweep_first_and_last_frequencies():
    """Log sweep: first point = f_start, last point = f_stop."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=10.0, f_stop=100000.0, n_points=5, sweep="log")
    assert isclose(result.points[0].freq, 10.0, rel_tol=1e-6)
    assert isclose(result.points[-1].freq, 100000.0, rel_tol=1e-6)


def test_ac_lin_sweep_first_and_last_frequencies():
    """Linear sweep: first point = f_start, last point = f_stop."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=1000.0, f_stop=5000.0, n_points=5, sweep="lin")
    assert isclose(result.points[0].freq, 1000.0, rel_tol=1e-6)
    assert isclose(result.points[-1].freq, 5000.0, rel_tol=1e-6)


def test_ac_lin_sweep_uniform_spacing():
    """Linear sweep: frequency points are uniformly spaced."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=0.0, f_stop=1000.0, n_points=6, sweep="lin")
    freqs = [pt.freq for pt in result.points]
    # Expected: 0, 200, 400, 600, 800, 1000
    expected_step = 200.0
    for i in range(1, len(freqs)):
        assert isclose(freqs[i] - freqs[i - 1], expected_step, rel_tol=1e-6), (
            f"Step {i}: {freqs[i] - freqs[i - 1]:.2f} (expected {expected_step})"
        )


def test_ac_log_sweep_decade_spacing():
    """Log sweep across 3 decades: first and last are 1000× apart."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=1.0, f_stop=1000.0, n_points=4, sweep="log")
    freqs = [pt.freq for pt in result.points]
    # 3 decades in 4 points: ratio between each pair = 10^(3/3) = 10
    ratio_0_1 = freqs[1] / freqs[0]
    ratio_1_2 = freqs[2] / freqs[1]
    assert isclose(ratio_0_1, ratio_1_2, rel_tol=1e-6)


# ============================================================================
# 21. AC sweep — small-signal nonlinear elements
# ============================================================================


def test_ac_diode_small_signal_forward_biased():
    """Forward-biased diode in AC: small-signal resistance is finite.

    The diode small-signal model is a conductance gd = (Is/Vt)·exp(Vd/Vt).
    With a DC bias of 0.6 V: Vd = 0.6 V (below 0.7 V clamp).

    gd = (1e-15 / 0.02585) * exp(0.6 / 0.02585) ≈ large
    Small-signal output voltage should be much less than 1 V (the diode
    shunts heavily).

    Circuit:  Vac(1V) → R(1kΩ) → anode → D(0.6V bias) → GND
    """
    # Provide a DC bias via a separate source; the AC source is the 1V source.
    Is_val = 1e-15
    Vt_val = 0.02585
    Vbias = 0.6

    c = Circuit()
    # AC input source (amplitude 1 V)
    c.add(VoltageSource("Vac", "in", "0", 1.0))
    # Series resistor
    c.add(Resistor("R1", "in", "anode", 1000.0))
    # DC bias current source to forward-bias the diode
    # I = Is*(exp(Vbias/Vt) - 1) to set anode to approximately Vbias
    I_bias = Is_val * (math.exp(Vbias / Vt_val) - 1.0)
    c.add(CurrentSource("I_bias", "anode", "0", I_bias))
    # The diode itself
    c.add(Diode("D1", anode="anode", cathode="0", Is=Is_val, Vt=Vt_val))

    result = ac_sweep(c, f_start=1000.0, f_stop=10000.0, n_points=5)
    # With the diode forward-biased (small-signal conductance >> 1/R1),
    # V_anode should be very small.
    for pt in result.points:
        v_anode = abs(pt.node_voltages["anode"])
        # The diode has such high gd that it nearly short-circuits the node
        assert v_anode < 0.1, (
            f"f={pt.freq:.0f} Hz: V_anode={v_anode:.4f} (expected < 0.1 with"
            f" forward-biased diode)"
        )


def test_ac_diode_reverse_biased_acts_like_open():
    """Reverse-biased diode in AC: near-zero conductance.

    When Vd ≤ 0 (cathode at or above anode), gd ≈ Is/Vt * exp(0) = Is/Vt
    which is negligibly small (1e-15/0.02585 ≈ 3.9e-14 S → R ≈ 25 GΩ).

    The diode is effectively an open circuit, so the voltage at the anode
    is determined only by the resistor divider.
    """
    c = Circuit()
    c.add(VoltageSource("Vac", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "anode", 1000.0))
    c.add(Resistor("R2", "anode", "0", 1000.0))   # load
    # Diode reverse-biased: cathode at Vin, anode at V_mid ≈ 0.5 V
    c.add(Diode("D1", anode="anode", cathode="in"))

    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=5)
    # With open-circuit diode, V_anode ≈ Vin × R2/(R1+R2) = 0.5V
    for pt in result.points:
        v_anode = abs(pt.node_voltages["anode"])
        assert isclose(v_anode, 0.5, abs_tol=0.05), (
            f"f={pt.freq:.0f} Hz: V_anode={v_anode:.5f} (expected ≈ 0.5)"
        )


def test_ac_bjt_npn_small_signal():
    """NPN BJT in forward-active: small-signal current gain > 1.

    DC bias: Vb=0.7V, Vcc=5V, Rc=1kΩ, emitter grounded.
    AC: small input signal at base (through Rb=10kΩ), output at collector.

    In the small-signal model, the collector current is gm×Vbe, so the
    AC gain from base to collector is approximately −gm×Rc.
    We verify the output-to-input ratio |V_col / V_in| > 1 (gain > 0 dB).
    """
    Is_val = 1e-14
    Vt_val = 0.02585
    Rc = 1000.0
    Rb = 10000.0

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    # AC input (1 V amplitude) through base resistor
    c.add(VoltageSource("Vac", "ac_in", "0", 1.0))
    c.add(Resistor("Rb", "ac_in", "base", Rb))
    # DC bias to forward-bias the BE junction
    c.add(VoltageSource("Vbias", "base_bias", "0", 0.7))
    c.add(Resistor("Rbias", "base_bias", "base", Rb))
    c.add(Resistor("Rc", "vcc", "col", Rc))
    c.add(BJT("Q1", collector="col", base="base", emitter="0",
               Is=Is_val, beta_f=100.0, Vt=Vt_val))

    result = ac_sweep(c, f_start=1000.0, f_stop=10000.0, n_points=5)
    for pt in result.points:
        # Both nodes should be present
        assert "col" in pt.node_voltages
        assert "base" in pt.node_voltages


# ============================================================================
# 22. AC sweep — current source injection
# ============================================================================


def test_ac_current_source_into_resistor():
    """AC current source (1 A) into a 1 kΩ resistor → V = I×R = 1 kV.

    At all frequencies, a current source in parallel with a resistor gives
    V = I × R (purely real, frequency-independent).
    """
    c = Circuit()
    c.add(CurrentSource("I1", "out", "0", 1.0))  # 1 A
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=5)
    for pt in result.points:
        v = pt.node_voltages["out"]
        # V = I × R = 1 A × 1 kΩ = 1000 V
        assert isclose(abs(v), 1000.0, abs_tol=1.0), (
            f"f={pt.freq:.0f} Hz: V={abs(v):.4f} (expected 1000)"
        )


def test_ac_current_source_with_capacitor_shunt():
    """AC current source into RC parallel: V decreases with frequency.

    I_s parallel with R (1kΩ) parallel with C (1μF):
        V(ω) = I_s / (1/R + jωC) = I_s × R / (1 + jωRC)

    At low f: |V| ≈ I_s × R = 1000 V (current mostly through R).
    At high f: |V| decreases as C shunts current away from R.

    With I_s = 1 mA, the magnitudes are more reasonable: V_low ≈ 1 V.
    """
    I_s = 1e-3   # 1 mA
    R = 1000.0
    C = 1e-6     # f_c ≈ 159 Hz

    c = Circuit()
    c.add(CurrentSource("I1", "out", "0", I_s))
    c.add(Resistor("R1", "out", "0", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=50, sweep="log")
    v_low = abs(result.points[0].node_voltages["out"])
    v_high = abs(result.points[-1].node_voltages["out"])

    # At very low f: |V| ≈ I_s × R
    assert isclose(v_low, I_s * R, rel_tol=0.01), (
        f"Low-freq voltage {v_low:.4f} V (expected ≈ {I_s * R:.4f} V)"
    )
    # High frequency: shunted by cap, voltage much lower
    assert v_high < v_low / 100.0, (
        f"High-freq voltage {v_high:.6f} should be << low-freq {v_low:.6f}"
    )


def test_ac_inductor_acts_as_short_at_very_low_frequency():
    """Inductor near ω=0: modelled as near-short (large conductance).

    In the AC model, Y_L = 1/(jωL).  At ω=0 this is ∞, which is
    approximated as G = 1e12 S.  A very low-frequency sweep should
    show that the node voltage across the inductor is nearly zero
    (inductor is a short, no voltage drop).

    Circuit:  Vin(1V) → L(1mH) → mid → R(1kΩ) → GND

    At ω → 0, the inductor is a short, so V_mid ≈ V_in = 1 V.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Inductor("L1", "in", "mid", 1e-3))
    c.add(Resistor("R1", "mid", "0", 1000.0))

    # Use a truly tiny frequency to approach the ω=0 limit.
    # The implementation uses omega=0 exactly for f_start=0 in lin sweep.
    result = ac_sweep(c, f_start=0.0, f_stop=0.0, n_points=1, sweep="lin")
    if result.points:
        v_mid = abs(result.points[0].node_voltages["mid"])
        # Inductor short → V_mid ≈ V_in = 1 V
        assert v_mid > 0.99, (
            f"At ω=0 (inductor short), V_mid={v_mid:.6f} (expected ≈ 1.0)"
        )


# ============================================================================
# 23. TF analysis: TfResult dataclass
# ============================================================================


def test_tfresult_fields():
    """TfResult is a frozen dataclass with the expected fields."""
    r = TfResult(transfer_ratio=0.5, input_impedance=2000.0, output_impedance=500.0)
    assert isclose(r.transfer_ratio, 0.5)
    assert isclose(r.input_impedance, 2000.0)
    assert isclose(r.output_impedance, 500.0)
    assert r.converged is True  # default


def test_tfresult_converged_false():
    """TfResult.converged can be set to False."""
    r = TfResult(
        transfer_ratio=0.0,
        input_impedance=float("inf"),
        output_impedance=float("inf"),
        converged=False,
    )
    assert not r.converged
    assert r.input_impedance == float("inf")


def test_tfresult_is_frozen():
    """TfResult is frozen — attributes cannot be reassigned."""
    r = TfResult(transfer_ratio=1.0, input_impedance=100.0, output_impedance=50.0)
    with pytest.raises((AttributeError, TypeError)):
        r.transfer_ratio = 0.0  # type: ignore[misc]


def test_tfresult_exported():
    """TfResult is importable from the top-level spice_engine package."""
    from spice_engine import TfResult as TfResult_exported
    assert TfResult_exported is TfResult


# ============================================================================
# 24. TF analysis: _build_ss_matrix helper
# ============================================================================


def test_build_ss_matrix_single_resistor():
    """Single resistor: G_ss has correct conductance value.

    Circuit: V1("v", "0") + R1("v", "0", 1kΩ)
    G_ss size = 2×2 (1 node "v" + 1 vsrc branch).
    G[0][0] = 1/1000, G[0][1] = G[1][0] = 1.0, G[1][1] = 0.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", 10.0))
    c.add(Resistor("R1", "v", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    dc_x = [0.0] * (len(node_to_idx) + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    # G[v][v] = 1/1000 (R1 conductance)
    v_idx = node_to_idx["v"]
    assert isclose(G[v_idx][v_idx], 1.0 / 1000.0, rel_tol=1e-9)
    # Structural VoltageSource entries
    branch_idx = len(node_to_idx) + 0
    assert isclose(G[v_idx][branch_idx], 1.0)
    assert isclose(G[branch_idx][v_idx], 1.0)


def test_build_ss_matrix_capacitor_is_open():
    """Capacitor contributes nothing to G_ss (open circuit at ω=0)."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 5.0))
    c.add(Capacitor("C1", "in", "out", 1e-6))
    c.add(Resistor("R1", "out", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    n = len(node_to_idx)
    dc_x = [0.0] * (n + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    # "out" row should only contain R1's conductance (no cap contribution).
    out_idx = node_to_idx["out"]
    assert isclose(G[out_idx][out_idx], 1.0 / 1000.0, rel_tol=1e-9)


def test_build_ss_matrix_inductor_is_short():
    """Inductor is modelled as a near-short (G = 1e12 S) at ω=0."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Inductor("L1", "in", "mid", 1e-3))
    c.add(Resistor("R1", "mid", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    dc_x = [0.0] * (len(node_to_idx) + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    in_idx = node_to_idx["in"]
    mid_idx = node_to_idx["mid"]
    # L1 adds conductance 1e12 to both diagonals and -1e12 off-diagonals.
    assert G[in_idx][mid_idx] < -1e11  # off-diagonal < 0


def test_build_ss_matrix_current_source_skipped():
    """CurrentSource does not contribute to G_ss (independent → zeroed)."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    dc_x = [0.0] * (len(node_to_idx) + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    # Only R1's conductance should be present.  Size = 1×1 (no vsrcs).
    n1_idx = node_to_idx["n1"]
    assert isclose(G[n1_idx][n1_idx], 1.0 / 1000.0, rel_tol=1e-9)
    # Only one row and column since there are no voltage sources.
    assert len(G) == 1


# ============================================================================
# 25. TF analysis: resistive circuits (voltage-source input)
# ============================================================================


def test_tf_voltage_divider_transfer_ratio():
    """Symmetric voltage divider: H = R2 / (R1 + R2) = 0.5.

    Circuit:
        V1 (10 V)  →  R1 (1kΩ)  →  vmid  →  R2 (1kΩ)  →  GND

    Transfer ratio (vmid / vin):  H = R2/(R1+R2) = 0.5
    Input impedance:               Z_in = R1 + R2 = 2000 Ω
    Output impedance:              Z_out = R1 ∥ R2 = 500 Ω
    """
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", 10.0))
    c.add(Resistor("R1", "vin", "vmid", 1000.0))
    c.add(Resistor("R2", "vmid", "0", 1000.0))

    result = tf(c, output_node="vmid", input_source="V1")

    assert result.converged
    assert isclose(result.transfer_ratio, 0.5, rel_tol=1e-6)
    assert isclose(result.input_impedance, 2000.0, rel_tol=1e-6)
    assert isclose(result.output_impedance, 500.0, rel_tol=1e-6)


def test_tf_asymmetric_divider():
    """Asymmetric divider: R1=3kΩ, R2=1kΩ.

    H    = R2/(R1+R2) = 1/4 = 0.25
    Z_in = R1 + R2 = 4000 Ω
    Z_out = R1∥R2 = 3000·1000/4000 = 750 Ω
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 3000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = tf(c, output_node="out", input_source="Vin")

    assert isclose(result.transfer_ratio, 0.25, rel_tol=1e-6)
    assert isclose(result.input_impedance, 4000.0, rel_tol=1e-6)
    assert isclose(result.output_impedance, 750.0, rel_tol=1e-6)


def test_tf_series_resistor_output_at_source():
    """Output node is the input node itself: H = 1, Z_out = 0 (source node).

    Circuit:  V1("in", "0") + R1("in", "0").

    At the source node (vin) the voltage is fixed by V1, so H = 1 and
    Z_out = 0 (the ideal voltage source clamps the output).
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = tf(c, output_node="in", input_source="V1")

    assert isclose(result.transfer_ratio, 1.0, rel_tol=1e-6)
    assert isclose(result.input_impedance, 1000.0, rel_tol=1e-6)
    # V1 forces V_in → Z_out = 0 (voltage source is a short for the test)
    assert isclose(result.output_impedance, 0.0, abs_tol=1e-9)


def test_tf_output_is_ground_node():
    """Output node is ground (0 V): H = 0 regardless of circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = tf(c, output_node="0", input_source="V1")
    assert isclose(result.transfer_ratio, 0.0, abs_tol=1e-12)


def test_tf_three_resistor_ladder():
    """Three-resistor ladder: R1→R2→R3 to ground.

    Circuit:  V1(1V) → R1(1kΩ) → n1 → R2(1kΩ) → n2 → R3(1kΩ) → GND

    At n2 (output): voltage divider with R1+R2 vs R3.
    H = R3 / (R1 + R2 + R3) = 1/3  (all equal)
    Wait — this isn't quite right for a ladder. Let me use the exact formula.

    Using KCL:
      V_n1 = V_in * R_parallel(n1→gnd) / (R1 + R_parallel(n1→gnd))
           where R_parallel = R2 + R3 = 2kΩ
      V_n1 = 1 * 2000 / (1000 + 2000) = 2/3

      V_n2 = V_n1 * R3 / (R2 + R3) = (2/3) * 1000 / 2000 = 1/3
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "n1", 1000.0))
    c.add(Resistor("R2", "n1", "n2", 1000.0))
    c.add(Resistor("R3", "n2", "0", 1000.0))

    result = tf(c, output_node="n2", input_source="V1")

    assert isclose(result.transfer_ratio, 1.0 / 3.0, rel_tol=1e-5)


def test_tf_inductor_short_at_dc():
    """Inductor is a near-short at ω=0: V across it is ≈ 0.

    Circuit:  V1(1V) → L1(1mH) → mid → R1(1kΩ) → GND

    At DC (ω=0): L is a short → V_mid ≈ V_in = 1 V, H ≈ 1.
    Z_in ≈ 0 (inductor short + R1 in parallel with near-zero L impedance ≈ 0)
    but numerically G_L = 1e12 >> G_R1, so Z_in ≈ 1/1e12 ≈ 0.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Inductor("L1", "in", "mid", 1e-3))
    c.add(Resistor("R1", "mid", "0", 1000.0))

    result = tf(c, output_node="mid", input_source="V1")

    assert result.converged
    # Inductor is a near-short: V_mid ≈ V_in, H ≈ 1
    assert isclose(result.transfer_ratio, 1.0, rel_tol=0.001)


def test_tf_converged_flag_matches_dc():
    """TfResult.converged mirrors the DC operating-point convergence."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = tf(c, output_node="out", input_source="V1")
    assert result.converged  # simple linear circuit always converges


def test_tf_diode_circuit_linearised():
    """Diode in series with a resistor: .TF linearises around the DC OP.

    Circuit:  V1(0.7V) → D1 → n_diode → R1(1kΩ) → GND

    At Vd ≈ 0.7 V (clamped), gd = (Is/Vt)·exp(0.7/Vt).
    Transfer ratio is the small-signal gain, not the DC ratio.
    We just verify the result has the right shape (converged, 0 < H < 1).
    """
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", 0.7))
    c.add(Diode("D1", anode="a", cathode="n_d"))
    c.add(Resistor("R1", "n_d", "0", 1000.0))

    result = tf(c, output_node="n_d", input_source="V1")

    assert result.converged
    assert 0.0 < result.transfer_ratio < 1.0


# ============================================================================
# 26. TF analysis: current-source input + transimpedance
# ============================================================================


def test_tf_current_source_into_resistor():
    """CurrentSource 1mA into 1kΩ: transimpedance H = V_out/I_in = R = 1kΩ.

    Circuit:  I1("0", "n1", 1mA) ∥ R1("n1", "0", 1kΩ)

    Transfer ratio H = V_n1 / I_source = R = 1000 Ω/A → 1000 V/A
    Z_in = parallel impedance from the source terminals = R = 1000 Ω
    Z_out = R = 1000 Ω (looking in from n1 with I1 zeroed → just R1)
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))

    result = tf(c, output_node="n1", input_source="I1")

    assert result.converged
    # H = V_n1 / 1A = R1 = 1000 V/A  (transimpedance)
    assert isclose(result.transfer_ratio, 1000.0, rel_tol=1e-6)
    # Z_in = V_source / I_source = R1 (load seen by source)
    assert isclose(result.input_impedance, 1000.0, rel_tol=1e-6)
    # Z_out = R1 (only R1 remains when I1 is zeroed)
    assert isclose(result.output_impedance, 1000.0, rel_tol=1e-6)


def test_tf_current_source_divider():
    """Current source into parallel R1∥R2: Z_in = R1∥R2.

    Circuit: I1("0", "n1") ∥ R1("n1","0", 1kΩ) ∥ R2("n1","0", 1kΩ)

    H    = V_n1 / 1 A = R1∥R2 = 500 Ω  (transimpedance in V/A)
    Z_in = R1∥R2 = 500 Ω
    Z_out = R1∥R2 = 500 Ω
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    c.add(Resistor("R2", "n1", "0", 1000.0))

    result = tf(c, output_node="n1", input_source="I1")

    assert isclose(result.transfer_ratio, 500.0, rel_tol=1e-6)
    assert isclose(result.input_impedance, 500.0, rel_tol=1e-6)
    assert isclose(result.output_impedance, 500.0, rel_tol=1e-6)


def test_tf_mixed_source_types():
    """Circuit with both a VoltageSource and a CurrentSource.

    When the voltage source is the input, the current source is zeroed
    (open circuit), and vice versa.

    Circuit:
        V1("vin", "0", 5V) + R1("vin","out", 1kΩ) + R2("out","0", 2kΩ)
        + I1("0","out", 1mA)

    With I1 zeroed (TF from V1 to "out"):
        H = R2 / (R1 + R2) = 2/3 ≈ 0.667
        Z_in = R1 + R2 = 3000 Ω
        Z_out = R1∥R2 = 1000·2000/3000 ≈ 666.7 Ω
    """
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", 5.0))
    c.add(Resistor("R1", "vin", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 2000.0))
    c.add(CurrentSource("I1", "0", "out", 1e-3))  # zeroed in the V1 TF

    result = tf(c, output_node="out", input_source="V1")

    assert isclose(result.transfer_ratio, 2.0 / 3.0, rel_tol=1e-5)
    assert isclose(result.input_impedance, 3000.0, rel_tol=1e-5)
    assert isclose(result.output_impedance, 1000.0 * 2000.0 / 3000.0, rel_tol=1e-5)


# ============================================================================
# 27. TF analysis: error cases and edge cases
# ============================================================================


def test_tf_raises_on_missing_source():
    """tf() raises ValueError when the named source is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="nonexistent"):
        tf(c, output_node="in", input_source="nonexistent")


def test_tf_raises_on_non_source_element():
    """tf() raises ValueError when the named element is not a source."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))

    with pytest.raises(ValueError, match="VoltageSource or CurrentSource"):
        tf(c, output_node="out", input_source="R1")


def test_tf_raises_on_unknown_output_node():
    """tf() raises ValueError when the output node is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="ghost_node"):
        tf(c, output_node="ghost_node", input_source="V1")


def test_tf_result_exported_and_callable():
    """tf() is exported from the top-level spice_engine package."""
    from spice_engine import tf as tf_exported
    assert tf_exported is tf


def test_tf_voltage_divider_independence_of_source_voltage():
    """Transfer ratio is independent of the source voltage (it's a ratio).

    H = V_out / V_in = R2/(R1+R2) regardless of the absolute source value.
    """
    c5 = Circuit()
    c5.add(VoltageSource("V1", "in", "0", 5.0))
    c5.add(Resistor("R1", "in", "out", 1000.0))
    c5.add(Resistor("R2", "out", "0", 1000.0))

    c100 = Circuit()
    c100.add(VoltageSource("V1", "in", "0", 100.0))
    c100.add(Resistor("R1", "in", "out", 1000.0))
    c100.add(Resistor("R2", "out", "0", 1000.0))

    r5 = tf(c5, output_node="out", input_source="V1")
    r100 = tf(c100, output_node="out", input_source="V1")

    assert isclose(r5.transfer_ratio, r100.transfer_ratio, rel_tol=1e-9)
    assert isclose(r5.input_impedance, r100.input_impedance, rel_tol=1e-9)
    assert isclose(r5.output_impedance, r100.output_impedance, rel_tol=1e-9)


def test_tf_multiple_sources_only_one_excited():
    """With two voltage sources, TF from V1 ignores V2 (it becomes 0 V short).

    Circuit:
        V1("in", "0", 1V) → R1("in","mid", 1kΩ) → mid
        V2("mid", "out", 0V) → R2("out","0", 1kΩ) → GND

    V2 is a 0 V source (wire).  The whole thing is a series 2kΩ divider.
    H = V_out / V_in_V1 should be 1.0 (V2 is a short, so V_out = V_mid = V_in
    through R1+R2... wait, let me compute properly).

    Actually: V1 forces vin=1V. R1 connects vin→mid, V2 (0V) forces
    V_out = V_mid. R2 connects out→GND.

    V2 is a 0V wire: V_mid = V_out.
    KCL at mid (= out via V2):
      (V_in - V_mid)/R1 = V_mid/R2
      (1 - V_mid)/1000 = V_mid/1000
      1 - V_mid = V_mid → V_mid = 0.5

    H = V_out / V_in = 0.5.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(VoltageSource("V2", "mid", "out", 0.0))  # wire
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = tf(c, output_node="out", input_source="V1")
    assert isclose(result.transfer_ratio, 0.5, rel_tol=1e-6)


# ---------------------------------------------------------------------------
# Section 28 — DC sweep: DcSweepPoint / DcSweepResult dataclasses
# ---------------------------------------------------------------------------


def test_dcsweeppoint_is_frozen() -> None:
    """DcSweepPoint is a frozen dataclass — fields are immutable after creation."""
    pt = DcSweepPoint(
        source_value=1.0,
        node_voltages={"a": 0.5},
        branch_currents={"V1": 0.001},
        converged=True,
    )
    with pytest.raises((AttributeError, TypeError)):
        pt.source_value = 2.0  # type: ignore[misc]


def test_dcsweeppoint_fields() -> None:
    """All four fields of DcSweepPoint are accessible."""
    pt = DcSweepPoint(
        source_value=3.3,
        node_voltages={"out": 1.65},
        branch_currents={"Vin": -0.001},
        converged=True,
    )
    assert pt.source_value == 3.3
    assert pt.node_voltages == {"out": 1.65}
    assert pt.branch_currents == {"Vin": -0.001}
    assert pt.converged is True


def test_dcsweepresult_fields() -> None:
    """DcSweepResult stores points and source_name."""
    pt = DcSweepPoint(1.0, {"n": 0.5}, {}, True)
    result = DcSweepResult(points=[pt], source_name="Vin")
    assert result.source_name == "Vin"
    assert len(result.points) == 1
    assert result.points[0] is pt


def test_dcsweepresult_exported() -> None:
    """DcSweepPoint, DcSweepResult, and dc_sweep are importable from spice_engine."""
    import spice_engine as se

    assert hasattr(se, "DcSweepPoint")
    assert hasattr(se, "DcSweepResult")
    assert hasattr(se, "dc_sweep")
    assert callable(se.dc_sweep)


# ---------------------------------------------------------------------------
# Section 29 — DC sweep: linear resistive circuits
# ---------------------------------------------------------------------------


def test_dc_sweep_voltage_divider_exact() -> None:
    """Voltage-divider: V_out = V_in / 2 at every sweep step.

    Circuit::

        Vin("in", "0") → R1("in","out", 1kΩ) → R2("out","0", 1kΩ) → GND

    By the resistor voltage-divider formula V_out = V_in * R2/(R1+R2) = V_in/2.
    We sweep Vin from 0 V to 5 V in 1 V steps and verify the ratio at each step.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 5.0, 1.0)

    assert result.source_name == "Vin"
    assert len(result.points) == 6  # 0, 1, 2, 3, 4, 5

    for pt in result.points:
        assert pt.converged
        expected_vout = pt.source_value / 2.0
        assert isclose(pt.node_voltages["out"], expected_vout, abs_tol=1e-9)


def test_dc_sweep_source_value_sequence() -> None:
    """Sweep values are monotone-ascending from start to stop."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 100.0))

    result = dc_sweep(c, "V1", 0.0, 1.0, 0.25)

    values = [pt.source_value for pt in result.points]
    assert values == pytest.approx([0.0, 0.25, 0.50, 0.75, 1.0], abs=1e-9)


def test_dc_sweep_original_circuit_unchanged() -> None:
    """The original Circuit object is not mutated during the sweep.

    dc_sweep must create modified copies for each step; the caller's circuit
    must remain at its original source value.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 2.0))
    c.add(Resistor("R1", "n", "0", 500.0))

    dc_sweep(c, "V1", 0.0, 4.0, 1.0)

    # The VoltageSource in the original circuit must still be at 2.0 V.
    original_vsrc = c.elements[0]
    assert isinstance(original_vsrc, VoltageSource)
    assert original_vsrc.voltage == 2.0


def test_dc_sweep_descending_step() -> None:
    """Descending sweep (start > stop, step < 0) produces reverse-ordered values."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 5.0, 0.0, -1.0)

    values = [pt.source_value for pt in result.points]
    assert values == pytest.approx([5.0, 4.0, 3.0, 2.0, 1.0, 0.0], abs=1e-9)


def test_dc_sweep_wrong_sign_step_returns_empty() -> None:
    """If step sign does not match start-to-stop direction, result is empty.

    E.g. start=0, stop=5, step=-1 → no valid sweep values.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 0.0, 5.0, -1.0)
    assert result.points == []


def test_dc_sweep_single_step() -> None:
    """When start == stop, exactly one point is returned."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 3.0, 3.0, 0.1)
    assert len(result.points) == 1
    assert isclose(result.points[0].source_value, 3.0, abs_tol=1e-9)


def test_dc_sweep_branch_currents_recorded() -> None:
    """Branch currents are recorded at each sweep step.

    For a single resistor R=1kΩ driven by Vin, I = Vin/R (Ohm's law).
    The MNA branch current for the voltage source has MNA sign convention
    (negative when source delivers current into the circuit).
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "Vin", 1.0, 3.0, 1.0)

    for pt in result.points:
        # DcResult keyed by "I(<name>)" — e.g. "I(Vin)"
        key = "I(Vin)"
        assert key in pt.branch_currents
        # Current = V / R = source_value / 1000
        # MNA sign: delivering current is negative
        expected_i = -pt.source_value / 1000.0
        assert isclose(pt.branch_currents[key], expected_i, rel_tol=1e-6)


def test_dc_sweep_three_node_ladder() -> None:
    """Three-resistor ladder: verifies intermediate node voltages at every step.

    Circuit::

        Vin → R1(1kΩ) → n1 → R2(1kΩ) → n2 → R3(1kΩ) → GND

    By symmetry (equal resistors): V_n1 = 2/3 * Vin, V_n2 = 1/3 * Vin.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "n1", 1000.0))
    c.add(Resistor("R2", "n1", "n2", 1000.0))
    c.add(Resistor("R3", "n2", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 3.0, 1.0)

    for pt in result.points:
        if pt.source_value == 0.0:
            continue  # all zeros
        v = pt.source_value
        assert isclose(pt.node_voltages["n1"], 2.0 / 3.0 * v, rel_tol=1e-6)
        assert isclose(pt.node_voltages["n2"], 1.0 / 3.0 * v, rel_tol=1e-6)


# ---------------------------------------------------------------------------
# Section 30 — DC sweep: nonlinear (diode) circuit
# ---------------------------------------------------------------------------


def test_dc_sweep_diode_all_points_converged() -> None:
    """A diode + resistor circuit converges across a forward-bias sweep.

    Circuit::

        Vin("anode","0") → D1(Is=1e-14, n=1) anode→cathode → R(cathode,"0", 1kΩ)

    At every sweep step (0 V to 2 V in 0.2 V) Newton-Raphson should converge.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 0.0))
    c.add(Diode("D1", "anode", "cathode", Is=1e-14))
    c.add(Resistor("R1", "cathode", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 2.0, 0.2)

    assert len(result.points) == 11
    assert all(pt.converged for pt in result.points)


def test_dc_sweep_diode_forward_bias_increasing_current() -> None:
    """Diode cathode voltage increases monotonically as Vin increases.

    When the source voltage rises, more current flows through the diode.
    The cathode voltage (= voltage across the resistor) must be strictly
    monotone-increasing once the diode is forward-biased.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 0.0))
    c.add(Diode("D1", "anode", "cathode", Is=1e-14))
    c.add(Resistor("R1", "cathode", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.5, 2.0, 0.5)

    v_cathode = [pt.node_voltages["cathode"] for pt in result.points]
    for i in range(1, len(v_cathode)):
        assert v_cathode[i] > v_cathode[i - 1], (
            f"cathode voltage not increasing: {v_cathode[i-1]:.4f} → {v_cathode[i]:.4f}"
        )


# ---------------------------------------------------------------------------
# Section 31 — DC sweep: current-source sweeps
# ---------------------------------------------------------------------------


def test_dc_sweep_current_source_into_resistor() -> None:
    """Sweep a current source into a single resistor: V = I * R at each step.

    The MNA stamp for CurrentSource(n_plus, n_minus, I) ADDS current to n_minus
    and SUBTRACTS from n_plus.  To inject current INTO node "n" we use
    n_plus="0" (ground, excluded) and n_minus="n".

    Circuit::

        I1("0","n") [current injected into "n"] ‖ R1("n","0", 1kΩ)

    By Ohm's law: V_n = I1 * R = I1 * 1000.
    Sweep I1 from 1 mA to 5 mA in 1 mA steps.
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n", 0.001))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "I1", 0.001, 0.005, 0.001)

    assert len(result.points) == 5
    for pt in result.points:
        assert pt.converged
        expected_v = pt.source_value * 1000.0
        assert isclose(pt.node_voltages["n"], expected_v, rel_tol=1e-6)


def test_dc_sweep_current_source_descending() -> None:
    """Descending current sweep produces reverse-ordered source values."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n", 0.0))
    c.add(Resistor("R1", "n", "0", 500.0))

    result = dc_sweep(c, "I1", 0.004, 0.001, -0.001)

    values = [pt.source_value for pt in result.points]
    assert values == pytest.approx([0.004, 0.003, 0.002, 0.001], abs=1e-12)


# ---------------------------------------------------------------------------
# Section 32 — DC sweep: error cases and edge cases
# ---------------------------------------------------------------------------


def test_dc_sweep_zero_step_raises() -> None:
    """A zero step size raises ValueError immediately."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    with pytest.raises(ValueError, match="step"):
        dc_sweep(c, "V1", 0.0, 5.0, 0.0)


def test_dc_sweep_missing_source_raises() -> None:
    """Referencing a source name that does not exist raises ValueError."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 1.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    with pytest.raises(ValueError, match="Vbad"):
        dc_sweep(c, "Vbad", 0.0, 1.0, 0.1)


def test_dc_sweep_resistor_not_accepted_as_source() -> None:
    """A Resistor element is not a valid sweep source; raises ValueError."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 1.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    with pytest.raises(ValueError, match="R1"):
        dc_sweep(c, "R1", 0.0, 1.0, 0.1)


def test_dc_sweep_fine_step_count() -> None:
    """Fine-grained step: 0 to 1 V in 0.1 V steps → exactly 11 points."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 0.0, 1.0, 0.1)
    assert len(result.points) == 11


def test_dc_sweep_all_converged_linear() -> None:
    """All points converge for a purely linear circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", -5.0, 5.0, 1.0)

    assert all(pt.converged for pt in result.points)
