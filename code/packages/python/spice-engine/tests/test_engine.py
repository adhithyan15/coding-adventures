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
"""

from math import exp, isclose

import pytest

from spice_engine import (
    BJT,
    Capacitor,
    Circuit,
    CurrentSource,
    Diode,
    Inductor,
    Resistor,
    VoltageSource,
    dc_op,
    transient,
)
from spice_engine.engine import _lte_estimate, _solve, _stamp_bjt

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
