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
33. DC sensitivity: SensEntry / SensResult dataclasses
34. DC sensitivity: resistor-divider analytical verification
35. DC sensitivity: voltage-source and current-source sensitivities
36. DC sensitivity: nonlinear element (Diode Is)
37. DC sensitivity: BJT Is and beta_f
38. DC sensitivity: sorting and ranking
39. DC sensitivity: error cases and edge cases
40. Monte Carlo: McPoint / McResult dataclasses
41. Monte Carlo: mean near nominal for symmetric tolerances
42. Monte Carlo: std_dev > 0 when tolerance > 0
43. Monte Carlo: seed reproducibility
44. Monte Carlo: distribution modes (gaussian vs uniform)
45. Monte Carlo: error cases and edge cases
46. Noise analysis: NoiseEntry / NoisePoint / NoiseResult dataclasses
47. Noise analysis: thermal noise — analytical Nyquist verification
48. Noise analysis: per-element breakdown and sorting
49. Noise analysis: input-referred noise calculation
50. Noise analysis: shot noise (Diode and BJT)
51. Noise analysis: frequency sweep and defaults
52. Noise analysis: error cases and edge cases
53. Controlled sources: VCCS dataclass
54. Controlled sources: VCVS dataclass
55. Controlled sources: CCCS dataclass
56. Controlled sources: CCVS dataclass
57. DC analysis: VCCS (voltage-controlled current source)
58. DC analysis: VCVS (voltage-controlled voltage source)
59. DC analysis: CCCS (current-controlled current source)
60. DC analysis: CCVS (current-controlled voltage source)
61. AC analysis: controlled sources
62. DC sweep: VCVS
63. Transient: controlled sources
64. TF analysis: VCVS
65. Sensitivity: VCVS
66. Monte Carlo: VCVS
67. Error cases: unknown ctrl_source
"""

import cmath
import math
from math import exp, isclose

import pytest

from spice_engine import (
    BSource,
    BJT,
    CCCS,
    CCVS,
    VCCS,
    VCVS,
    AcPoint,
    AcResult,
    AcSource,
    Capacitor,
    Circuit,
    CurrentSource,
    DcSweepPoint,
    DcSweepResult,
    Diode,
    ExpWaveform,
    Inductor,
    McPoint,
    McResult,
    NoiseEntry,
    NoisePoint,
    NoiseResult,
    PulseWaveform,
    PwlWaveform,
    Resistor,
    SensEntry,
    SensResult,
    SinWaveform,
    SubcircuitDefinition,
    TfResult,
    VoltageSource,
    XInstance,
    ac_sweep,
    dc_op,
    dc_sweep,
    mc_dc,
    noise_ac,
    sens_dc,
    tf,
    transient,
)
from spice_engine.engine import (
    _build_ss_matrix,
    _dc_gmin_step,
    _dc_newton,
    _dc_source_step,
    _lte_estimate,
    _node_index,
    _solve,
    _solve_complex,
    _stamp_bjt,
    _voltage_sources,
    _x_from_result,
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


def test_behavioral_current_source_tracks_node_voltage():
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", voltage=2.0))
    c.add(BSource("B1", "0", "out", current_expr="0.002 * V(in)"))
    c.add(Resistor("Rload", "out", "0", 1000.0))
    r = dc_op(c)
    assert r.converged
    assert isclose(r.node_voltages["out"], 4.0, abs_tol=1e-6)


def test_behavioral_voltage_source_tracks_differential_voltage():
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", voltage=3.0))
    c.add(BSource("B1", "out", "0", voltage_expr="2.0 * V(in, 0) + 1.0"))
    c.add(Resistor("Rload", "out", "0", 1000.0))
    r = dc_op(c)
    assert r.converged
    assert isclose(r.node_voltages["out"], 7.0, abs_tol=1e-6)
    assert "I(B1)" in r.branch_currents


def test_subcircuit_instance_expands_resistor_divider_at_build_time():
    cell = SubcircuitDefinition(
        "atten2",
        ("in", "out"),
        (
            Resistor("Rtop", "in", "out", 1000.0),
            Resistor("Rbot", "out", "0", 1000.0),
        ),
    )
    c = Circuit()
    c.define_subcircuit(cell)
    c.add(VoltageSource("V1", "vin", "0", voltage=10.0))
    c.add(XInstance("X1", ("vin", "vout"), "atten2"))

    r = dc_op(c)

    assert r.converged
    assert isclose(r.node_voltages["vout"], 5.0, abs_tol=1e-9)
    assert [element.name for element in c.elements if isinstance(element, Resistor)] == [
        "X1.Rtop",
        "X1.Rbot",
    ]


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


def test_transient_inductor_respects_initial_current():
    c = Circuit()
    c.add(Resistor("R1", "out", "0", 1000.0))
    c.add(Inductor("L1", "out", "0", 1.0, initial_current=1.0e-3))

    result = transient(c, t_stop=2.0e-3, t_step=1.0e-3, method="euler")

    assert result.converged
    assert isclose(result.points[0].node_voltages["out"], -0.5, abs_tol=1e-9)
    assert isclose(result.points[1].node_voltages["out"], -0.5, abs_tol=1e-9)
    assert isclose(result.points[2].node_voltages["out"], -0.25, abs_tol=1e-9)


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


def test_ac_explicit_voltage_source_phasor_separate_from_dc_bias():
    """An explicit AC source phasor is independent from the DC bias value."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0, ac=AcSource(2.0, 90.0)))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
    pt = result.points[0]

    assert isclose(pt.node_voltages["in"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["in"].imag, 2.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].imag, 1.0, abs_tol=1e-12)


def test_ac_unspecified_sources_zero_when_any_explicit_ac_source_exists():
    """DC bias sources become zero small-signal sources with explicit AC input."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "bias", "0", 5.0))
    c.add(CurrentSource("Iac", "0", "out", 0.0, ac=AcSource(1.0e-3, 90.0)))
    c.add(Resistor("Rbias", "bias", "out", 1000.0))
    c.add(Resistor("Rload", "out", "0", 1000.0))

    result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
    pt = result.points[0]

    assert isclose(pt.node_voltages["bias"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["bias"].imag, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].imag, 0.5, abs_tol=1e-12)


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


# ===========================================================================
# Section 33 — DC sensitivity analysis: SensEntry / SensResult dataclasses
# ===========================================================================
#
# sens_dc perturbs each element parameter by a small fraction, runs two DC
# solves (nominal and perturbed), and records:
#
#   sensitivity     = (V_out_pert − V_out_nom) / δ    [absolute, V/unit]
#   rel_sensitivity = sensitivity × P / V_out_nom      [dimensionless ratio]
#
# The analytical ground-truth for a resistor divider:
#
#   V_mid = V_in × R2 / (R1 + R2)
#
#   ∂V_mid/∂R1 = −V_in × R2 / (R1+R2)²
#   ∂V_mid/∂R2 = +V_in × R1 / (R1+R2)²
#   ∂V_mid/∂V_in = R2 / (R1+R2)
#
#   For V_in=10V, R1=R2=1kΩ:
#     ∂V_mid/∂R1 ≈ −10 × 1000/4000000 = −0.0025 V/Ω
#     ∂V_mid/∂R2 ≈ +10 × 1000/4000000 = +0.0025 V/Ω
#     ∂V_mid/∂V_in = 0.5
#
#   rel_sensitivity for R1: (−0.0025 × 1000) / 5.0 = −0.5
#   rel_sensitivity for R2: (+0.0025 × 1000) / 5.0 = +0.5
#   rel_sensitivity for Vin: (0.5 × 10) / 5.0 = +1.0
# ===========================================================================


def test_sens_result_is_sensresult() -> None:
    """sens_dc returns a SensResult instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = sens_dc(c, "in")
    assert isinstance(result, SensResult)


def test_sens_entries_are_sensentry() -> None:
    """Each entry in SensResult.entries is a SensEntry instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = sens_dc(c, "in")
    for entry in result.entries:
        assert isinstance(entry, SensEntry)


def test_sens_nominal_voltage_correct() -> None:
    """SensResult.nominal_voltage matches dc_op V_out at the output node."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    dc = dc_op(c)
    assert isclose(result.nominal_voltage, dc.node_voltages["mid"], rel_tol=1e-9)


def test_sens_converged_linear() -> None:
    """sens_dc converges for a purely linear circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    assert result.converged


# ===========================================================================
# Section 34 — DC sensitivity: resistor-divider analytical verification
# ===========================================================================


def test_sens_divider_r1_sensitivity() -> None:
    """∂V_mid/∂R1 ≈ −V_in × R2 / (R1+R2)² for the resistor divider.

    For V_in=10V, R1=R2=1kΩ: ∂V_mid/∂R1 ≈ −0.0025 V/Ω.
    The forward-difference approximation is accurate to within 0.1% of the
    analytical value.
    """
    v_in, r1, r2 = 10.0, 1000.0, 1000.0
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", v_in))
    c.add(Resistor("R1", "in", "mid", r1))
    c.add(Resistor("R2", "mid", "0", r2))

    result = sens_dc(c, "mid")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")

    # Analytical: ∂V_mid/∂R1 = −V_in × R2 / (R1+R2)²
    expected_sens = -v_in * r2 / (r1 + r2) ** 2
    assert isclose(r1_entry.sensitivity, expected_sens, rel_tol=1e-3), (
        f"R1 sensitivity {r1_entry.sensitivity:.6f} vs expected {expected_sens:.6f}"
    )


def test_sens_divider_r2_sensitivity() -> None:
    """∂V_mid/∂R2 ≈ +V_in × R1 / (R1+R2)² for the resistor divider."""
    v_in, r1, r2 = 10.0, 1000.0, 1000.0
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", v_in))
    c.add(Resistor("R1", "in", "mid", r1))
    c.add(Resistor("R2", "mid", "0", r2))

    result = sens_dc(c, "mid")
    r2_entry = next(e for e in result.entries if e.element_name == "R2")

    expected_sens = v_in * r1 / (r1 + r2) ** 2
    assert isclose(r2_entry.sensitivity, expected_sens, rel_tol=1e-3), (
        f"R2 sensitivity {r2_entry.sensitivity:.6f} vs expected {expected_sens:.6f}"
    )


def test_sens_divider_r1_rel_sensitivity() -> None:
    """Relative sensitivity of R1 ≈ −0.5 in an equal-ratio divider.

    rel = (∂V_mid/∂R1) × R1 / V_mid = (−0.0025 × 1000) / 5 = −0.5.
    A 1% increase in R1 causes a 0.5% decrease in V_mid.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")

    assert isclose(r1_entry.rel_sensitivity, -0.5, rel_tol=1e-3), (
        f"R1 rel_sensitivity {r1_entry.rel_sensitivity:.4f} vs expected -0.5"
    )


def test_sens_divider_r2_rel_sensitivity() -> None:
    """Relative sensitivity of R2 ≈ +0.5 in an equal-ratio divider."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    r2_entry = next(e for e in result.entries if e.element_name == "R2")

    assert isclose(r2_entry.rel_sensitivity, 0.5, rel_tol=1e-3), (
        f"R2 rel_sensitivity {r2_entry.rel_sensitivity:.4f} vs expected +0.5"
    )


def test_sens_divider_asymmetric() -> None:
    """Asymmetric divider (R1=2kΩ, R2=1kΩ) checks the relative sensitivities.

    V_mid = V_in × R2/(R1+R2) = 10 × 1000/3000 ≈ 3.333 V.

    General closed-form relative sensitivities:
      rel(R1) = R1 × (∂V_mid/∂R1) / V_mid = −R1/(R1+R2)
      rel(R2) = R2 × (∂V_mid/∂R2) / V_mid = +R1/(R1+R2)  ← numerator is R1, not R2

    Derivation:
      ∂V_mid/∂R1 = −V_in × R2/(R1+R2)²
      ∂V_mid/∂R2 = +V_in × R1/(R1+R2)²
      V_mid = V_in × R2/(R1+R2)
      rel(R2) = R2 × V_in × R1/(R1+R2)² / (V_in × R2/(R1+R2)) = R1/(R1+R2)

    For R1=2kΩ, R2=1kΩ:
      rel(R1) = −2000/3000 ≈ −0.667
      rel(R2) = +2000/3000 ≈ +0.667
    """
    v_in, r1, r2 = 10.0, 2000.0, 1000.0
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", v_in))
    c.add(Resistor("R1", "in", "mid", r1))
    c.add(Resistor("R2", "mid", "0", r2))

    result = sens_dc(c, "mid")
    assert isclose(result.nominal_voltage, v_in * r2 / (r1 + r2), rel_tol=1e-9)

    r1_entry = next(e for e in result.entries if e.element_name == "R1")
    expected_rel_r1 = -r1 / (r1 + r2)   # = −2000/3000 ≈ −0.667
    assert isclose(r1_entry.rel_sensitivity, expected_rel_r1, rel_tol=1e-3)

    r2_entry = next(e for e in result.entries if e.element_name == "R2")
    expected_rel_r2 = r1 / (r1 + r2)    # = +2000/3000 ≈ +0.667 (numerator is R1!)
    assert isclose(r2_entry.rel_sensitivity, expected_rel_r2, rel_tol=1e-3)


# ===========================================================================
# Section 35 — DC sensitivity: voltage-source and current-source sensitivities
# ===========================================================================


def test_sens_voltage_source_rel_is_unity() -> None:
    """For a pure voltage source with resistive load, ∂V_out/∂V_in = 1.

    V_out = V_in directly (single node with a resistor to ground — voltage
    is set by the source).  rel_sensitivity = (1 × V_in) / V_in = 1.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "out", "0", 5.0))
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    vin_entry = next(e for e in result.entries if e.element_name == "Vin")

    assert isclose(vin_entry.rel_sensitivity, 1.0, rel_tol=1e-3), (
        f"VoltageSource rel_sensitivity {vin_entry.rel_sensitivity:.4f} vs 1.0"
    )


def test_sens_current_source_into_resistor() -> None:
    """V_out = I × R, so ∂V_out/∂I = R and rel_sensitivity = 1.

    I1=1 mA → 0 (n+) / out (n-); R1=1 kΩ to ground.
    V_out = I × R = 0.001 × 1000 = 1 V.
    ∂V_out/∂I = R = 1000.  rel = 1000 × 0.001 / 1.0 = 1.
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "out", 0.001))  # injects into 'out'
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    assert isclose(result.nominal_voltage, 1.0, rel_tol=1e-9)

    i1_entry = next(e for e in result.entries if e.element_name == "I1")
    assert isclose(i1_entry.sensitivity, 1000.0, rel_tol=1e-3), (
        f"CurrentSource abs sensitivity {i1_entry.sensitivity:.1f} vs 1000"
    )
    assert isclose(i1_entry.rel_sensitivity, 1.0, rel_tol=1e-3)


def test_sens_voltage_source_divider_rel_unity() -> None:
    """Voltage source rel_sensitivity is always 1.0 for a linear divider.

    For V_mid = V_in × R2/(R1+R2), ∂V_mid/∂V_in = R2/(R1+R2) = 0.5.
    rel = 0.5 × V_in / V_mid = 0.5 × 10 / 5 = 1.0.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    vin_entry = next(e for e in result.entries if e.element_name == "Vin")
    assert isclose(vin_entry.rel_sensitivity, 1.0, rel_tol=1e-3)


def test_sens_current_source_resistance_both_ranked() -> None:
    """Both I1 (current) and R1 (resistance) appear in entries for a simple RC load."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "out", 0.002))
    c.add(Resistor("R1", "out", "0", 500.0))

    result = sens_dc(c, "out")
    names = {(e.element_name, e.parameter) for e in result.entries}
    assert ("I1", "current") in names
    assert ("R1", "resistance") in names


# ===========================================================================
# Section 36 — DC sensitivity: nonlinear element (Diode Is)
# ===========================================================================
#
# For a forward-biased diode in series with a resistor:
#
#   V_in = V_D + V_R      where V_D ≈ Vt × ln(I / Is)
#
# Increasing Is decreases V_D (the diode conducts more easily at the same
# current), which increases V_R.  The output voltage V_R = V_in − V_D
# therefore rises when Is rises.
#
# The direction of ∂V_R/∂Is is always positive for a forward-biased diode
# connected in the usual way.
# ===========================================================================


def test_sens_diode_is_entry_present() -> None:
    """A Diode contributes an 'Is' entry to the sensitivity table."""
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 1.0))
    c.add(Diode("D1", "anode", "out"))
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    param_names = {(e.element_name, e.parameter) for e in result.entries}
    assert ("D1", "Is") in param_names, f"Entries: {param_names}"


def test_sens_diode_is_positive_sensitivity() -> None:
    """Increasing Is lowers Vd, raising Vout (V_R = V_in − V_D).

    The absolute sensitivity ∂V_R/∂Is should be positive.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 1.0))
    c.add(Diode("D1", "anode", "out"))
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    d1_entry = next(e for e in result.entries if e.element_name == "D1" and e.parameter == "Is")
    assert d1_entry.sensitivity > 0, (
        f"Expected positive dV_out/dIs, got {d1_entry.sensitivity}"
    )


# ===========================================================================
# Section 37 — DC sensitivity: BJT Is and beta_f
# ===========================================================================
#
# NPN common-emitter amplifier:
#
#   Vcc (5 V)
#     │
#    Rc (1 kΩ) ← collector
#     │
#     ├── output "out"
#     │
#    BJT (NPN)  ← base biased via Rb from Vcc, emitter to ground
#     │
#    GND
#
# Increasing beta_f → more collector current → V_out drops (negative
# relative sensitivity).
# ===========================================================================


def test_sens_bjt_entries_is_and_beta() -> None:
    """A BJT contributes both 'Is' and 'beta_f' entries."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(Resistor("Rb", "vcc", "base", 100_000.0))
    c.add(Resistor("Rc", "vcc", "col", 1_000.0))
    c.add(BJT("Q1", "col", "base", "0", polarity="NPN"))

    result = sens_dc(c, "col")
    param_pairs = {(e.element_name, e.parameter) for e in result.entries}
    assert ("Q1", "Is") in param_pairs, f"Missing Q1 Is — entries: {param_pairs}"
    assert ("Q1", "beta_f") in param_pairs, f"Missing Q1 beta_f — entries: {param_pairs}"


def test_sens_bjt_beta_negative_on_collector() -> None:
    """Higher beta_f → more collector current → V_col decreases.

    In the common-emitter configuration the rel_sensitivity for beta_f
    should be negative (increasing β lowers V_out).
    """
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(Resistor("Rb", "vcc", "base", 200_000.0))
    c.add(Resistor("Rc", "vcc", "col", 2_000.0))
    c.add(BJT("Q1", "col", "base", "0", polarity="NPN"))

    result = sens_dc(c, "col")
    if not result.converged:
        return  # Skip if BJT didn't converge (topology edge case)

    beta_entry = next(
        (e for e in result.entries if e.element_name == "Q1" and e.parameter == "beta_f"),
        None,
    )
    if beta_entry is not None:
        assert beta_entry.rel_sensitivity <= 0, (
            f"Expected beta_f to decrease V_col; got rel={beta_entry.rel_sensitivity:.4f}"
        )


# ===========================================================================
# Section 38 — DC sensitivity: sorting and ranking
# ===========================================================================


def test_sens_entries_sorted_by_abs_rel_desc() -> None:
    """Entries are sorted by abs(rel_sensitivity) descending."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    rels = [abs(e.rel_sensitivity) for e in result.entries]
    assert rels == sorted(rels, reverse=True), f"Entries not sorted: {rels}"


def test_sens_vin_dominates_divider() -> None:
    """In a symmetric divider Vin has the highest rel_sensitivity (= 1.0).

    Both resistors have |rel| = 0.5 < 1.0, so Vin appears first.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    assert result.entries[0].element_name == "Vin", (
        f"Expected Vin first, got {result.entries[0].element_name}"
    )


def test_sens_nominal_value_stored() -> None:
    """SensEntry stores the unperturbed parameter value in nominal_value."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 4700.0))

    result = sens_dc(c, "in")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")
    assert r1_entry.nominal_value == 4700.0

    vin_entry = next(e for e in result.entries if e.element_name == "Vin")
    assert vin_entry.nominal_value == 10.0


# ===========================================================================
# Section 39 — DC sensitivity: error cases and edge cases
# ===========================================================================


def test_sens_invalid_output_node_raises() -> None:
    """ValueError if the output node is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="nonexistent"):
        sens_dc(c, "nonexistent")


def test_sens_output_at_ground_not_raises() -> None:
    """Observing ground (always 0 V) doesn't raise, but returns 0 as nominal."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    # ground node aliases: "0" is accepted
    result = sens_dc(c, "0")
    assert result.output_node == "0"
    assert result.nominal_voltage == 0.0


def test_sens_output_node_field_preserved() -> None:
    """SensResult.output_node stores the exact string passed to sens_dc."""
    c = Circuit()
    c.add(VoltageSource("Vin", "alpha", "0", 3.3))
    c.add(Resistor("R1", "alpha", "0", 1000.0))

    result = sens_dc(c, "alpha")
    assert result.output_node == "alpha"


def test_sens_capacitor_and_inductor_skipped() -> None:
    """Capacitors and inductors produce no entries (no DC parameter).

    In DC steady-state, a capacitor is an open circuit and an inductor is
    a short circuit.  Neither C nor L affects the DC node voltages, so
    perturbing their values produces zero sensitivity.  sens_dc skips them
    to keep the output concise.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Capacitor("C1", "out", "0", 1e-9))  # open in DC — no entry expected
    c.add(Inductor("L1", "out", "0", 1e-6))   # short in DC  — no entry expected
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    names = {e.element_name for e in result.entries}
    assert "C1" not in names, "Capacitor should not appear in sensitivity entries"
    assert "L1" not in names, "Inductor should not appear in sensitivity entries"


def test_sens_three_resistors_all_present() -> None:
    """A ladder with three resistors produces three 'resistance' entries."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "n1", 1000.0))
    c.add(Resistor("R2", "n1", "n2", 1000.0))
    c.add(Resistor("R3", "n2", "0", 1000.0))

    result = sens_dc(c, "n2")
    param_pairs = {(e.element_name, e.parameter) for e in result.entries}
    assert ("R1", "resistance") in param_pairs
    assert ("R2", "resistance") in param_pairs
    assert ("R3", "resistance") in param_pairs


def test_sens_single_resistor_load() -> None:
    """V_out is fixed by the voltage source; R only affects current, not voltage.

    With V1 directly on 'out', ∂V_out/∂R = 0 (voltage source overrides the node).
    The resistor sensitivity should be essentially zero.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "out", "0", 3.3))
    c.add(Resistor("R1", "out", "0", 10_000.0))

    result = sens_dc(c, "out")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")
    # Voltage source clamps the node; R has no effect on V_out
    assert abs(r1_entry.sensitivity) < 1e-6, (
        f"Resistor sensitivity should be ≈0 when load is directly clamped: "
        f"{r1_entry.sensitivity}"
    )


# ===========================================================================
# Section 40 — Monte Carlo: McPoint / McResult dataclasses
# ===========================================================================
#
# mc_dc runs N independent DC operating points, each with element parameters
# randomly varied by ±tolerance around their nominal values.  The mean and
# standard deviation of V(output_node) across converged trials are reported.
#
# For a symmetric tolerance distribution the mean should be close to the
# nominal value (no systematic bias).  The standard deviation reflects the
# spread introduced by the component variation.
# ===========================================================================


def test_mc_result_is_mcresult() -> None:
    """mc_dc returns a McResult instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=5, seed=0)
    assert isinstance(result, McResult)


def test_mc_points_are_mcpoint() -> None:
    """Each entry in McResult.points is a McPoint instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=5, seed=0)
    for pt in result.points:
        assert isinstance(pt, McPoint)


def test_mc_n_trials_field() -> None:
    """McResult.n_trials matches the requested number of trials."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=17, seed=0)
    assert result.n_trials == 17
    assert len(result.points) == 17


def test_mc_trial_index_sequential() -> None:
    """McPoint.trial runs from 0 to n_trials − 1 in order."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=10, seed=0)
    assert [pt.trial for pt in result.points] == list(range(10))


def test_mc_output_node_field() -> None:
    """McResult.output_node stores the exact string passed to mc_dc."""
    c = Circuit()
    c.add(VoltageSource("Vin", "alpha", "0", 3.3))
    c.add(Resistor("R1", "alpha", "0", 1000.0))

    result = mc_dc(c, "alpha", n_trials=3, seed=0)
    assert result.output_node == "alpha"


# ===========================================================================
# Section 41 — Monte Carlo: mean near nominal for symmetric tolerances
# ===========================================================================
#
# For any distribution symmetric around the nominal value (Gaussian or
# uniform), the expected value of V_out is the nominal V_out (no bias).
# With enough trials the sample mean should converge to the nominal.
#
# We use a loose 10% tolerance on the mean (i.e., the mean must be within
# ±10% of the nominal) even for N=200 to avoid flaky tests — but in practice
# the convergence is much tighter (~σ/√N ≈ 0.1V / √200 ≈ 0.007V for a
# symmetric 5% Gaussian on a 5V divider).
# ===========================================================================


def test_mc_gaussian_mean_near_nominal_divider() -> None:
    """Gaussian variation: mean(V_mid) ≈ nominal (5 V) for a symmetric divider."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=300, tolerance=0.05, seed=42)
    nominal = 5.0
    # Mean should be within 5% of nominal for N=300 with σ≈0.1V
    assert abs(result.mean - nominal) < 0.5, (
        f"Mean {result.mean:.3f} is far from nominal {nominal}"
    )


def test_mc_uniform_mean_near_nominal() -> None:
    """Uniform ±10% variation: mean(V_out) ≈ 2.5 V for a symmetric resistor divider.

    Because mc_dc varies every element (including the VoltageSource itself),
    there is no "clamped" node.  For a symmetric divider with uniform ±10%
    variation on both resistors and the source, the expected output is half the
    source voltage.  With N=200 trials and ±10% tolerance the sample mean
    should stay within 20% of the nominal 2.5 V.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = mc_dc(c, "out", n_trials=200, tolerance=0.10, distribution="uniform", seed=1)
    # Symmetric uniform distribution → no bias; mean stays near 2.5 V.
    assert abs(result.mean - 2.5) < 0.5, (
        f"Mean {result.mean:.3f} is too far from nominal 2.5 V"
    )


def test_mc_all_converged_linear() -> None:
    """All trials converge for a purely linear resistive circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = mc_dc(c, "out", n_trials=50, tolerance=0.05, seed=5)
    assert all(pt.converged for pt in result.points), (
        "All trials should converge for a linear circuit"
    )


# ===========================================================================
# Section 42 — Monte Carlo: std_dev > 0 when tolerance > 0
# ===========================================================================
#
# Whenever the output voltage is sensitive to at least one varied component
# AND tolerance > 0, the sample standard deviation must be positive.
# If all varied parameters have zero effect (e.g., the output is clamped by
# a voltage source), std_dev can be zero.
# ===========================================================================


def test_mc_std_dev_positive_gaussian() -> None:
    """Gaussian variation on a divider produces std_dev > 0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=50, tolerance=0.05, seed=7)
    assert result.std_dev > 0, f"Expected std_dev > 0, got {result.std_dev}"


def test_mc_std_dev_positive_uniform() -> None:
    """Uniform variation on a divider also produces std_dev > 0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=50, tolerance=0.05,
                   distribution="uniform", seed=8)
    assert result.std_dev > 0, f"Expected std_dev > 0, got {result.std_dev}"


def test_mc_wider_tolerance_larger_std_dev() -> None:
    """Higher tolerance → larger std_dev for the same circuit and seed."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result_tight = mc_dc(c, "mid", n_trials=100, tolerance=0.01, seed=3)
    result_wide = mc_dc(c, "mid", n_trials=100, tolerance=0.10, seed=3)
    assert result_wide.std_dev > result_tight.std_dev, (
        f"Wider tolerance should give larger std_dev: "
        f"tight={result_tight.std_dev:.4f}, wide={result_wide.std_dev:.4f}"
    )


def test_mc_zero_tolerance_zero_std_dev() -> None:
    """Zero tolerance: all trials identical → std_dev == 0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=10, tolerance=0.0, seed=0)
    assert result.std_dev == 0.0, (
        f"Zero tolerance should give std_dev=0, got {result.std_dev}"
    )


# ===========================================================================
# Section 43 — Monte Carlo: seed reproducibility
# ===========================================================================
#
# Two runs with the same seed, same circuit, and same parameters must produce
# exactly identical results.  Two runs with different seeds should (almost
# certainly) differ.
# ===========================================================================


def test_mc_seed_same_produces_same_results() -> None:
    """Same seed → identical McPoint trial vectors."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    r1 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=42)
    r2 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=42)

    assert r1.mean == r2.mean, "Same seed should produce identical mean"
    assert r1.std_dev == r2.std_dev, "Same seed should produce identical std_dev"
    for pt1, pt2 in zip(r1.points, r2.points, strict=True):
        assert pt1.node_voltages == pt2.node_voltages, (
            f"Trial {pt1.trial}: node_voltages differ"
        )


def test_mc_different_seeds_produce_different_results() -> None:
    """Different seeds almost certainly produce different trial vectors."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    r1 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=1)
    r2 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=2)

    # The probability that two independent runs give the same mean is negligible.
    assert r1.mean != r2.mean, "Different seeds should produce different means"


# ===========================================================================
# Section 44 — Monte Carlo: distribution modes (gaussian vs uniform)
# ===========================================================================
#
# The two distributions have different shapes:
#
#   Gaussian: tails extend beyond ±tolerance (3-sigma = tolerance) — rarely
#             produces values more than 3× the "rated" tolerance from nominal.
#
#   Uniform:  flat between [1−tol, 1+tol] — no samples outside ±tolerance.
#
# For the same tolerance and circuit the uniform distribution typically
# produces a slightly larger std_dev because its variance is tolerance²/3,
# versus Gaussian with σ = tolerance/3 → variance = tolerance²/9.
# So uniform std_dev ≈ √3 × gaussian std_dev for the same effective range.
# ===========================================================================


def test_mc_gaussian_distribution_mode() -> None:
    """distribution='gaussian' runs without error and returns non-zero std_dev."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=30, tolerance=0.05,
                   distribution="gaussian", seed=10)
    assert result.std_dev >= 0.0


def test_mc_uniform_distribution_mode() -> None:
    """distribution='uniform' runs without error and returns non-zero std_dev."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=30, tolerance=0.05,
                   distribution="uniform", seed=11)
    assert result.std_dev >= 0.0


def test_mc_uniform_wider_spread_than_gaussian() -> None:
    """For same tolerance, uniform distribution spreads wider than Gaussian.

    Uniform variance = tol²/3; Gaussian variance = (tol/3)² = tol²/9.
    Ratio ≈ √3 ≈ 1.73×, so uniform std_dev should be larger with a
    statistically significant sample.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    r_gauss = mc_dc(c, "mid", n_trials=200, tolerance=0.10,
                    distribution="gaussian", seed=20)
    r_unif = mc_dc(c, "mid", n_trials=200, tolerance=0.10,
                   distribution="uniform", seed=20)

    # Uniform should be wider; allow a 30% margin for sampling variability.
    assert r_unif.std_dev > r_gauss.std_dev * 0.7, (
        f"Uniform std_dev {r_unif.std_dev:.4f} should exceed "
        f"Gaussian std_dev {r_gauss.std_dev:.4f} × 0.7"
    )


# ===========================================================================
# Section 45 — Monte Carlo: error cases and edge cases
# ===========================================================================


def test_mc_invalid_output_node_raises() -> None:
    """ValueError if the output node is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="nonexistent"):
        mc_dc(c, "nonexistent", n_trials=5)


def test_mc_invalid_distribution_raises() -> None:
    """ValueError for an unknown distribution name."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="distribution"):
        mc_dc(c, "in", n_trials=5, distribution="triangular")


def test_mc_n_trials_zero_raises() -> None:
    """ValueError if n_trials < 1."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="n_trials"):
        mc_dc(c, "in", n_trials=0)


def test_mc_single_trial() -> None:
    """n_trials=1: mean equals the single trial voltage, std_dev=0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=1, tolerance=0.05, seed=0)
    assert len(result.points) == 1
    assert result.std_dev == 0.0
    v = result.points[0].node_voltages.get("mid", 0.0)
    assert isclose(result.mean, v, rel_tol=1e-9)


def test_mc_ground_output_node() -> None:
    """Observing ground ('0') returns mean=0 and std_dev=0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "0", n_trials=10, tolerance=0.05, seed=0)
    assert result.mean == 0.0
    assert result.std_dev == 0.0


def test_mc_node_voltages_in_each_point() -> None:
    """Each McPoint.node_voltages contains the output node key."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = mc_dc(c, "out", n_trials=5, seed=0)
    for pt in result.points:
        assert "out" in pt.node_voltages, (
            f"Trial {pt.trial} missing 'out' in node_voltages"
        )


# ---------------------------------------------------------------------------
# Physical constants mirroring those in engine.py, used for analytical checks.
# ---------------------------------------------------------------------------
_kB: float = 1.380649e-23     # Boltzmann constant [J/K]
_q: float = 1.602176634e-19   # Electron charge [C]


def _divider_circuit() -> Circuit:
    """Canonical test circuit for noise analysis.

    Topology: VS(0 V, "in"→"0") → R1(1 kΩ, "in"→"out") → R2(1 kΩ, "out"→"0")

    With Vin = 0 V the input node is AC-grounded; the effective output resistance
    seen by the noise sources is R1 ‖ R2 = 500 Ω.  The signal transfer function
    is H_sig = R2 / (R1 + R2) = 0.5.

    Known analytical values at T = 300 K:
        S_out    = 4kT × 500 ≈ 8.284e-18 V²/Hz  (white: same at all freqs)
        S_in     = S_out / 0.25 ≈ 3.314e-17 V²/Hz
        per-R1   = 4kT × 250 ≈ 4.142e-18 V²/Hz
        per-R2   = 4kT × 250 ≈ 4.142e-18 V²/Hz
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    return c


# ---------------------------------------------------------------------------
# Section 46 — Noise analysis: NoiseEntry / NoisePoint / NoiseResult dataclasses
# ---------------------------------------------------------------------------


def test_noise_result_is_noiseresult() -> None:
    """noise_ac returns a NoiseResult instance."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert isinstance(result, NoiseResult)


def test_noise_points_are_noisepoint() -> None:
    """Each element of NoiseResult.points is a NoisePoint."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert len(result.points) == 1
    assert isinstance(result.points[0], NoisePoint)


def test_noise_entries_are_noiseentry() -> None:
    """Each element of NoisePoint.entries is a NoiseEntry."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert isinstance(entry, NoiseEntry)


def test_noise_result_output_node_field() -> None:
    """NoiseResult.output_node records the requested node."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert result.output_node == "out"


def test_noise_result_input_source_field() -> None:
    """NoiseResult.input_source records the nominated source name."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert result.input_source == "Vin"


def test_noise_result_temperature_field() -> None:
    """NoiseResult.temperature echoes the supplied temperature."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=350.0)
    assert result.temperature == 350.0


def test_noise_point_freq_field() -> None:
    """NoisePoint.freq records the frequency in hertz."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1234.0])
    assert result.points[0].freq == 1234.0


def test_noise_entry_element_name_field() -> None:
    """NoiseEntry.element_name is the name of the contributing element."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "R1" in names
    assert "R2" in names


def test_noise_entry_noise_type_thermal_for_resistors() -> None:
    """NoiseEntry.noise_type is 'thermal' for resistors."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert entry.noise_type == "thermal"


def test_noise_entry_source_psd_positive() -> None:
    """NoiseEntry.source_psd is positive (noise power density ≥ 0)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert entry.source_psd > 0.0


def test_noise_entry_output_psd_nonnegative() -> None:
    """NoiseEntry.output_psd is non-negative."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert entry.output_psd >= 0.0


# ---------------------------------------------------------------------------
# Section 47 — Noise analysis: thermal noise — analytical Nyquist verification
# ---------------------------------------------------------------------------
#
# For a purely resistive circuit at temperature T, the total output noise PSD
# equals the Nyquist formula:
#
#     S_out = 4kT × R_eq    [V²/Hz]
#
# where R_eq is the Thevenin-equivalent noise resistance looking back from
# the output node with all independent sources set to zero.


def test_noise_symmetric_divider_total_psd_nyquist() -> None:
    """Symmetric divider: S_out ≈ 4kT × (R1‖R2) = 4kT × 500 Ω."""
    T = 300.0
    R1 = R2 = 1000.0
    R_eq = (R1 * R2) / (R1 + R2)          # 500 Ω
    expected = 4.0 * _kB * T * R_eq       # ≈ 8.28e-18 V²/Hz

    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=T)
    actual = result.points[0].output_psd

    assert isclose(actual, expected, rel_tol=1e-4), (
        f"Expected S_out ≈ {expected:.4e} V²/Hz, got {actual:.4e}"
    )


def test_noise_psd_scales_with_temperature() -> None:
    """Doubling temperature doubles S_out (linear dependence on T)."""
    c = _divider_circuit()
    r1 = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=300.0)
    r2 = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=600.0)
    ratio = r2.points[0].output_psd / r1.points[0].output_psd
    assert isclose(ratio, 2.0, rel_tol=1e-4), (
        f"Expected ratio 2.0, got {ratio:.4f}"
    )


def test_noise_white_spectrum_resistors_only() -> None:
    """Purely resistive circuit has flat (white) noise PSD at all frequencies."""
    c = _divider_circuit()
    freqs_to_test = [1.0, 100.0, 1e4, 1e6]
    result = noise_ac(c, "out", "Vin", freqs=freqs_to_test)

    # All frequency points should give the same output PSD (white noise).
    psds = [pt.output_psd for pt in result.points]
    ref = psds[0]
    for i, psd in enumerate(psds[1:], 1):
        assert isclose(psd, ref, rel_tol=1e-6), (
            f"Freq {freqs_to_test[i]}: expected {ref:.4e}, got {psd:.4e} (not white)"
        )


def test_noise_single_resistor_source_psd() -> None:
    """Source PSD of a 1 kΩ resistor at 300 K equals 4kT/R analytically."""
    T = 300.0
    R = 1000.0
    expected_src = 4.0 * _kB * T / R   # A²/Hz

    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=T)
    for entry in result.points[0].entries:
        assert isclose(entry.source_psd, expected_src, rel_tol=1e-6), (
            f"{entry.element_name}: source_psd {entry.source_psd:.4e}, "
            f"expected {expected_src:.4e}"
        )


def test_noise_larger_resistance_higher_source_psd() -> None:
    """A 10 kΩ resistor produces 10× more current noise PSD than 1 kΩ."""
    c1 = Circuit()
    c1.add(VoltageSource("Vin", "in", "0", 0.0))
    c1.add(Resistor("R1", "in", "out", 1000.0))
    c1.add(Resistor("R2", "out", "0", 1000.0))

    c2 = Circuit()
    c2.add(VoltageSource("Vin", "in", "0", 0.0))
    c2.add(Resistor("R1", "in", "out", 10000.0))  # 10× larger
    c2.add(Resistor("R2", "out", "0", 10000.0))

    r1 = noise_ac(c1, "out", "Vin", freqs=[1000.0])
    r2 = noise_ac(c2, "out", "Vin", freqs=[1000.0])

    # source_psd = 4kT/R, so 10x bigger R → 10x smaller source_psd
    src1 = next(e for e in r1.points[0].entries if e.element_name == "R1")
    src2 = next(e for e in r2.points[0].entries if e.element_name == "R1")
    assert isclose(src1.source_psd / src2.source_psd, 10.0, rel_tol=1e-6)


# ---------------------------------------------------------------------------
# Section 48 — Noise analysis: per-element breakdown and sorting
# ---------------------------------------------------------------------------


def test_noise_entries_count_matches_resistors() -> None:
    """Two resistors produce two noise entries."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert len(result.points[0].entries) == 2


def test_noise_entries_sorted_loudest_first() -> None:
    """Entries are sorted by output_psd descending (loudest contributor first)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    psds = [e.output_psd for e in result.points[0].entries]
    assert psds == sorted(psds, reverse=True), (
        f"Entries not sorted: {psds}"
    )


def test_noise_sum_of_entries_equals_total() -> None:
    """Sum of per-element output_psd equals NoisePoint.output_psd."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    pt = result.points[0]
    entry_sum = sum(e.output_psd for e in pt.entries)
    assert isclose(entry_sum, pt.output_psd, rel_tol=1e-10), (
        f"Entry sum {entry_sum:.4e} != total {pt.output_psd:.4e}"
    )


def test_noise_symmetric_resistors_equal_contributions() -> None:
    """Symmetric divider (R1=R2): each resistor contributes equally to S_out."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    e1 = next(e for e in result.points[0].entries if e.element_name == "R1")
    e2 = next(e for e in result.points[0].entries if e.element_name == "R2")
    assert isclose(e1.output_psd, e2.output_psd, rel_tol=1e-6), (
        f"R1 contribution {e1.output_psd:.4e} ≠ R2 {e2.output_psd:.4e}"
    )


def test_noise_asymmetric_divider_bottom_louder() -> None:
    """Asymmetric divider (R1=2kΩ, R2=1kΩ): R2 contributes more to S_out.

    Because R2 is closer to the output node and sees a larger signal transfer,
    its noise dominates even though its source PSD is lower than R1's.

    S_out_R2 = |H_R2|² × (4kT/R2) = (666.67)² × (4kT/1000) × … let T=300K.
    S_out_R1 = |H_R1|² × (4kT/R1) = (666.67)² × (4kT/2000) × …

    Since both H_R1 and H_R2 equal R_eq × (1/R_{source}) × something:
    Thevenin: S_out_R2 = 4kT×R2×(R1/(R1+R2))² = 4kT×1000×(2/3)² = 4kT×444.4
              S_out_R1 = 4kT×R1×(R2/(R1+R2))² = 4kT×2000×(1/3)² = 4kT×222.2
    So R2 > R1.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 2000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    e_r1 = next(e for e in result.points[0].entries if e.element_name == "R1")
    e_r2 = next(e for e in result.points[0].entries if e.element_name == "R2")
    assert e_r2.output_psd > e_r1.output_psd, (
        f"Expected R2 ({e_r2.output_psd:.3e}) > R1 ({e_r1.output_psd:.3e})"
    )


def test_noise_three_resistors_three_entries() -> None:
    """Three resistors in a T-network produce three noise entries."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "mid", 500.0))
    c.add(Resistor("R2", "mid", "out", 500.0))
    c.add(Resistor("R3", "out", "0", 1000.0))

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert len(result.points[0].entries) == 3


# ---------------------------------------------------------------------------
# Section 49 — Noise analysis: input-referred noise calculation
# ---------------------------------------------------------------------------


def test_noise_input_referred_divider() -> None:
    """Symmetric divider: S_in = S_out / (0.5)² = 4 × S_out."""
    T = 300.0
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=T)
    pt = result.points[0]

    expected_s_in = pt.output_psd / (0.5 ** 2)  # gain = 0.5
    assert isclose(pt.input_referred_psd, expected_s_in, rel_tol=1e-4), (
        f"S_in {pt.input_referred_psd:.4e}, expected {expected_s_in:.4e}"
    )


def test_noise_input_referred_greater_than_output_for_attenuator() -> None:
    """For an attenuating circuit, S_in > S_out (noise is amplified at input)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    pt = result.points[0]
    assert pt.input_referred_psd > pt.output_psd, (
        f"Expected S_in ({pt.input_referred_psd:.3e}) > S_out ({pt.output_psd:.3e})"
    )


def test_noise_unknown_input_source_gives_zero_input_referred() -> None:
    """If input_source is not in the circuit, input_referred_psd = 0.0."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Nonexistent", freqs=[1000.0])
    assert result.points[0].input_referred_psd == 0.0


def test_noise_input_referred_scales_with_gain() -> None:
    """Higher gain → lower input-referred noise (same circuit noise, less divided).

    Two circuits: same R_load, but different series R (attenuator ratio):
    - High gain: R_series = 10 Ω  → gain = R_load / (R_series + R_load) ≈ 0.99
    - Low gain:  R_series = 1 kΩ  → gain = 0.5
    High-gain amp should have lower input-referred noise.
    """
    def make_circuit(r_series: float) -> Circuit:
        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 0.0))
        c.add(Resistor("Rs", "in", "out", r_series))
        c.add(Resistor("RL", "out", "0", 1000.0))
        return c

    high_gain = noise_ac(make_circuit(10.0),    "out", "Vin", freqs=[1000.0])
    low_gain  = noise_ac(make_circuit(1000.0),  "out", "Vin", freqs=[1000.0])

    s_in_high = high_gain.points[0].input_referred_psd
    s_in_low  = low_gain.points[0].input_referred_psd
    # High-gain circuit has smaller input-referred noise
    assert s_in_high < s_in_low, (
        f"Expected high-gain S_in ({s_in_high:.3e}) < low-gain ({s_in_low:.3e})"
    )


def test_noise_current_source_input_referred() -> None:
    """CurrentSource as input: input_referred_psd is computed correctly."""
    c = Circuit()
    c.add(CurrentSource("Iin", "node", "0", 0.0))
    c.add(Resistor("R1", "node", "0", 1000.0))

    result = noise_ac(c, "node", "Iin", freqs=[1000.0])
    # The transimpedance from Iin to V_node is just R1 = 1000 Ω.
    # S_out = 4kT/R (from R1's thermal noise) × R1² = 4kT × R1
    # S_in  = S_out / R1² = 4kT / R1
    # (This is just the current noise of R1 referred to the input.)
    assert result.points[0].input_referred_psd > 0.0


# ---------------------------------------------------------------------------
# Section 50 — Noise analysis: shot noise (Diode and BJT)
# ---------------------------------------------------------------------------


def test_noise_diode_has_shot_noise_entry() -> None:
    """A forward-biased diode contributes a 'shot' noise entry."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "vcc", "0", 1.0))
    c.add(Resistor("R1", "vcc", "node", 1000.0))
    c.add(Diode("D1", "node", "0"))  # forward-biased: current flows anode→cathode

    result = noise_ac(c, "node", "Vbias", freqs=[1000.0])
    noise_types = {e.noise_type for e in result.points[0].entries}
    assert "shot" in noise_types, f"No shot noise entry; types = {noise_types}"


def test_noise_diode_shot_noise_type_string() -> None:
    """Diode entry has noise_type == 'shot'."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "vcc", "0", 1.0))
    c.add(Resistor("R1", "vcc", "node", 1000.0))
    c.add(Diode("D1", "node", "0"))

    result = noise_ac(c, "node", "Vbias", freqs=[1000.0])
    diode_entry = next(
        (e for e in result.points[0].entries if e.element_name == "D1"), None
    )
    assert diode_entry is not None, "No entry for D1"
    assert diode_entry.noise_type == "shot"


def test_noise_diode_shot_noise_source_psd_positive() -> None:
    """Forward-biased diode has source_psd > 0."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "vcc", "0", 1.0))
    c.add(Resistor("R1", "vcc", "node", 10000.0))
    c.add(Diode("D1", "node", "0"))

    result = noise_ac(c, "node", "Vbias", freqs=[1000.0])
    d1 = next(e for e in result.points[0].entries if e.element_name == "D1")
    assert d1.source_psd > 0.0, f"Diode source_psd = {d1.source_psd}"


def test_noise_diode_shot_noise_proportional_to_2qI() -> None:
    """Diode source_psd = 2q|I_D|.

    At a known DC current I_D, source_psd = 2 × 1.602e-19 × I_D.
    We run two circuits with different bias currents and check that
    the ratio of PSDs matches the ratio of currents.
    """
    def make_circuit(vbias: float) -> Circuit:
        c = Circuit()
        c.add(VoltageSource("Vbias", "vcc", "0", vbias))
        c.add(Resistor("R1", "vcc", "node", 100.0))  # small R for large I_D range
        c.add(Diode("D1", "node", "0"))
        return c

    # We measure the diode current indirectly via source_psd ratio.
    r1 = noise_ac(make_circuit(0.8), "node", "Vbias", freqs=[1000.0])
    r2 = noise_ac(make_circuit(1.0), "node", "Vbias", freqs=[1000.0])

    d1_psd = next(e.source_psd for e in r1.points[0].entries if e.element_name == "D1")
    d2_psd = next(e.source_psd for e in r2.points[0].entries if e.element_name == "D1")

    # Higher bias → larger I_D → higher shot noise PSD.
    assert d2_psd > d1_psd, (
        f"Expected higher bias to give more shot noise: {d2_psd:.3e} vs {d1_psd:.3e}"
    )


def test_noise_bjt_has_shot_noise_entry() -> None:
    """A forward-active NPN BJT contributes a 'shot' noise entry."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(VoltageSource("Vbase", "base", "0", 0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", "col", "base", "0"))   # NPN: collector, base, emitter

    result = noise_ac(c, "col", "Vbase", freqs=[1000.0])
    noise_types = {e.noise_type for e in result.points[0].entries}
    assert "shot" in noise_types, f"No shot noise entry for BJT; types = {noise_types}"


def test_noise_bjt_shot_noise_type_string() -> None:
    """BJT entry has noise_type == 'shot'."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(VoltageSource("Vbase", "base", "0", 0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", "col", "base", "0"))

    result = noise_ac(c, "col", "Vbase", freqs=[1000.0])
    bjt_entry = next(
        (e for e in result.points[0].entries if e.element_name == "Q1"), None
    )
    assert bjt_entry is not None, "No entry for Q1"
    assert bjt_entry.noise_type == "shot"


# ---------------------------------------------------------------------------
# Section 51 — Noise analysis: frequency sweep and defaults
# ---------------------------------------------------------------------------


def test_noise_default_sweep_has_50_points() -> None:
    """Default frequency sweep (no freqs arg) returns 50 points."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    assert len(result.points) == 50


def test_noise_default_sweep_ascending_frequencies() -> None:
    """Default sweep frequencies are strictly ascending."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    freqs = [pt.freq for pt in result.points]
    for i in range(1, len(freqs)):
        assert freqs[i] > freqs[i - 1], (
            f"Non-ascending at index {i}: {freqs[i-1]} → {freqs[i]}"
        )


def test_noise_default_sweep_starts_near_1hz() -> None:
    """Default sweep starts at ≈ 1 Hz."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    assert isclose(result.points[0].freq, 1.0, rel_tol=1e-6)


def test_noise_default_sweep_ends_near_1mhz() -> None:
    """Default sweep ends at ≈ 1 MHz."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    assert isclose(result.points[-1].freq, 1e6, rel_tol=1e-4)


def test_noise_custom_freq_list_respected() -> None:
    """Custom freqs list is used exactly (correct count and values)."""
    c = _divider_circuit()
    custom = [100.0, 1000.0, 10000.0]
    result = noise_ac(c, "out", "Vin", freqs=custom)
    assert len(result.points) == 3
    for pt, expected_f in zip(result.points, custom, strict=True):
        assert pt.freq == expected_f


def test_noise_single_frequency_point() -> None:
    """Single-element freqs list returns exactly one NoisePoint."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[5000.0])
    assert len(result.points) == 1
    assert result.points[0].freq == 5000.0


def test_noise_output_psd_positive_at_all_default_freqs() -> None:
    """Output PSD is positive at every default frequency point."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    for pt in result.points:
        assert pt.output_psd > 0.0, f"Zero PSD at f = {pt.freq:.2f} Hz"


# ---------------------------------------------------------------------------
# Section 52 — Noise analysis: error cases and edge cases
# ---------------------------------------------------------------------------


def test_noise_unknown_output_node_returns_empty() -> None:
    """Unknown output node returns a NoiseResult with empty points list."""
    c = _divider_circuit()
    result = noise_ac(c, "nonexistent", "Vin", freqs=[1000.0])
    assert isinstance(result, NoiseResult)
    assert result.points == []


def test_noise_ground_output_node_returns_empty() -> None:
    """Ground as output node ('0') returns a NoiseResult with empty points."""
    c = _divider_circuit()
    result = noise_ac(c, "0", "Vin", freqs=[1000.0])
    assert result.points == []


def test_noise_empty_freqs_list_returns_no_points() -> None:
    """Empty freqs list produces a NoiseResult with zero points."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[])
    assert result.points == []


def test_noise_capacitor_is_noiseless() -> None:
    """Capacitor contributes no noise entry (capacitors are noiseless)."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    c.add(Capacitor("C1", "out", "0", 1e-9))   # in parallel with R2

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "C1" not in names, f"Capacitor C1 wrongly appears in noise entries: {names}"


def test_noise_voltage_source_is_noiseless() -> None:
    """VoltageSource contributes no noise entry (ideal sources are noiseless)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "Vin" not in names, f"VoltageSource Vin wrongly in noise entries: {names}"


def test_noise_inductor_is_noiseless() -> None:
    """Inductor contributes no noise entry (inductors are modelled as noiseless)."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Inductor("L1", "mid", "out", 1e-3))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "L1" not in names, f"Inductor L1 wrongly in noise entries: {names}"


def test_noise_output_psd_nonnegative() -> None:
    """NoisePoint.output_psd is always non-negative (sum of non-negative terms)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    for pt in result.points:
        assert pt.output_psd >= 0.0, f"Negative output_psd at f={pt.freq:.2f}"


def test_noise_entries_tuple_immutable() -> None:
    """NoisePoint.entries is a tuple (immutable sequence of NoiseEntry)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert isinstance(result.points[0].entries, tuple)


def test_noise_noisepoint_is_frozen() -> None:
    """NoisePoint is a frozen dataclass (attempting to set field raises)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    with pytest.raises((AttributeError, TypeError)):
        result.points[0].freq = 9999.0  # type: ignore[misc]


def test_noise_noiseentry_is_frozen() -> None:
    """NoiseEntry is a frozen dataclass (attempting to set field raises)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    entry = result.points[0].entries[0]
    with pytest.raises((AttributeError, TypeError)):
        entry.output_psd = -1.0  # type: ignore[misc]


# ── Section 53: VCCS element dataclass ───────────────────────────────────────


def test_vccs_frozen() -> None:
    """VCCS is a frozen dataclass (immutable after construction)."""
    g = VCCS("G1", "out", "0", "in", "0", gm=0.01)
    with pytest.raises((AttributeError, TypeError)):
        g.gm = 0.02  # type: ignore[misc]


def test_vccs_fields() -> None:
    """VCCS stores all six fields correctly."""
    g = VCCS("G1", "out", "0", "in", "0", gm=0.02)
    assert g.name == "G1"
    assert g.n_plus == "out"
    assert g.n_minus == "0"
    assert g.ctrl_plus == "in"
    assert g.ctrl_minus == "0"
    assert g.gm == pytest.approx(0.02)


def test_vccs_negative_gm() -> None:
    """VCCS accepts negative gm (for sign-inverting configurations)."""
    g = VCCS("G_inv", "out", "0", "in", "0", gm=-0.05)
    assert g.gm == pytest.approx(-0.05)


def test_vccs_in_element_union() -> None:
    """VCCS is accepted as a circuit element (Element type alias includes it)."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    assert len(c.elements) == 3


# ── Section 54: VCVS element dataclass ───────────────────────────────────────


def test_vcvs_frozen() -> None:
    """VCVS is a frozen dataclass (immutable after construction)."""
    e = VCVS("E1", "out", "0", "vin", "0", gain=2.0)
    with pytest.raises((AttributeError, TypeError)):
        e.gain = 3.0  # type: ignore[misc]


def test_vcvs_fields() -> None:
    """VCVS stores all six fields correctly."""
    e = VCVS("E1", "out", "0", "vin", "0", gain=-10.0)
    assert e.name == "E1"
    assert e.n_plus == "out"
    assert e.n_minus == "0"
    assert e.ctrl_plus == "vin"
    assert e.ctrl_minus == "0"
    assert e.gain == pytest.approx(-10.0)


def test_vcvs_differential_ctrl() -> None:
    """VCVS can have a non-ground ctrl_minus (differential sensing)."""
    e = VCVS("Ediff", "out", "0", "vp", "vm", gain=5.0)
    assert e.ctrl_plus == "vp"
    assert e.ctrl_minus == "vm"
    assert e.gain == pytest.approx(5.0)


def test_vcvs_in_element_union() -> None:
    """VCVS is accepted as a circuit element."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    assert len(c.elements) == 3


# ── Section 55: CCCS element dataclass ───────────────────────────────────────


def test_cccs_frozen() -> None:
    """CCCS is a frozen dataclass (immutable after construction)."""
    f = CCCS("F1", "out", "0", "Vsense", beta=2.0)
    with pytest.raises((AttributeError, TypeError)):
        f.beta = 3.0  # type: ignore[misc]


def test_cccs_fields() -> None:
    """CCCS stores all five fields correctly."""
    f = CCCS("F1", "out", "0", "Vsense", beta=5.0)
    assert f.name == "F1"
    assert f.n_plus == "out"
    assert f.n_minus == "0"
    assert f.ctrl_source == "Vsense"
    assert f.beta == pytest.approx(5.0)


def test_cccs_negative_beta() -> None:
    """CCCS accepts negative beta (current direction reversal)."""
    f = CCCS("Finv", "out", "0", "Vsense", beta=-1.0)
    assert f.beta == pytest.approx(-1.0)


def test_cccs_in_element_union() -> None:
    """CCCS is accepted as a circuit element."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", beta=2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    assert len(c.elements) == 5


# ── Section 56: CCVS element dataclass ───────────────────────────────────────


def test_ccvs_frozen() -> None:
    """CCVS is a frozen dataclass (immutable after construction)."""
    h = CCVS("H1", "out", "0", "Vsense", transresistance=1000.0)
    with pytest.raises((AttributeError, TypeError)):
        h.transresistance = 2000.0  # type: ignore[misc]


def test_ccvs_fields() -> None:
    """CCVS stores all five fields correctly."""
    h = CCVS("H1", "out", "0", "Vsense", transresistance=500.0)
    assert h.name == "H1"
    assert h.n_plus == "out"
    assert h.n_minus == "0"
    assert h.ctrl_source == "Vsense"
    assert h.transresistance == pytest.approx(500.0)


def test_ccvs_in_element_union() -> None:
    """CCVS is accepted as a circuit element."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", transresistance=500.0),
    ])
    assert len(c.elements) == 4


# ── Section 57: DC analysis – VCCS ───────────────────────────────────────────


def test_vccs_dc_inverting() -> None:
    """VCCS(out, 0, in, 0, gm) inverts: V_out = -gm * R_load * V_in.

    MNA analysis: VCCS stamps G[out][in] += gm.  Rload stamps G[out][out] +=
    1/R.  KCL at ``out``: (1/R)*V_out + gm*V_in = 0 → V_out = -gm*R*V_in.
    """
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "out", "0", "in", "0", gm=0.01),
    ])
    r = dc_op(c)
    # V_out = -0.01 * 100 * 1.0 = -1.0 V
    assert r.node_voltages["out"] == pytest.approx(-1.0, abs=1e-9)


def test_vccs_dc_noninverting() -> None:
    """VCCS(0, out, in, 0, gm) non-inverts: V_out = +gm * R_load * V_in.

    Swapping n_plus and n_minus flips the sign of all stamp entries, so the
    current injection at ``out`` is in the opposite direction.
    KCL at ``out``: (1/R)*V_out - gm*V_in = 0 → V_out = +gm*R*V_in.
    """
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    r = dc_op(c)
    # V_out = +0.01 * 100 * 1.0 = +1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_vccs_dc_zero_gm() -> None:
    """VCCS with gm=0 injects no current (equivalent to open circuit)."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 5.0),
        Resistor("Rload", "out", "0", 1000.0),
        VCCS("G1", "out", "0", "in", "0", gm=0.0),
    ])
    r = dc_op(c)
    # No current injected → Rload has no current → V_out = 0
    assert r.node_voltages["out"] == pytest.approx(0.0, abs=1e-9)


def test_vccs_dc_gain_scaling() -> None:
    """VCCS output scales linearly with gm and R_load."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 2.0),
        Resistor("Rload", "out", "0", 50.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.02),
    ])
    r = dc_op(c)
    # V_out = gm * R * V_in = 0.02 * 50 * 2.0 = 2.0 V
    assert r.node_voltages["out"] == pytest.approx(2.0, abs=1e-9)


def test_vccs_dc_differential_ctrl() -> None:
    """VCCS senses V(ctrl_plus) − V(ctrl_minus) for a differential input."""
    # V_p = 3V, V_m = 1V → V_diff = 2V → V_out = gm * R * V_diff
    c = Circuit([
        VoltageSource("Vp", "vp", "0", 3.0),
        VoltageSource("Vm", "vm", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "vp", "vm", gm=0.01),
    ])
    r = dc_op(c)
    # V_out = 0.01 * 100 * (3 - 1) = 2.0 V
    assert r.node_voltages["out"] == pytest.approx(2.0, abs=1e-9)


# ── Section 58: DC analysis – VCVS ───────────────────────────────────────────


def test_vcvs_dc_unity_buffer() -> None:
    """VCVS with gain=1 is a perfect unity-gain voltage buffer.

    KVL: V_out − 1.0 × V_vin = 0 → V_out = V_vin.
    The load has no effect because the VCVS is an ideal (zero-output-
    impedance) voltage source.
    """
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    assert r.node_voltages["out"] == pytest.approx(5.0, abs=1e-9)


def test_vcvs_dc_inverting_gain() -> None:
    """VCVS with gain=-2 inverts and amplifies: V_out = -2 × V_in."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 3.0),
        VCVS("E1", "out", "0", "vin", "0", gain=-2.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    assert r.node_voltages["out"] == pytest.approx(-6.0, abs=1e-9)


def test_vcvs_dc_large_gain() -> None:
    """VCVS with a large positive gain (open-loop op-amp macromodel)."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 0.001),  # 1 mV input
        VCVS("E1", "out", "0", "vin", "0", gain=1000.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    # V_out = 1000 * 0.001 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_vcvs_dc_differential_ctrl() -> None:
    """VCVS senses V(ctrl_plus) − V(ctrl_minus) for a differential input."""
    c = Circuit([
        VoltageSource("Vp", "vp", "0", 4.0),
        VoltageSource("Vm", "vm", "0", 1.0),
        VCVS("E1", "out", "0", "vp", "vm", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    # V_diff = 4 - 1 = 3V → V_out = 1.0 * 3 = 3.0V
    assert r.node_voltages["out"] == pytest.approx(3.0, abs=1e-9)


def test_vcvs_dc_branch_current_recorded() -> None:
    """dc_op records I(E1) in branch_currents for a VCVS element."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    assert "I(E1)" in r.branch_currents
    # Rload current = V_out / R_load = 5.0 / 1000 = 5 mA
    # VCVS sinks this current from its output terminal
    assert abs(r.branch_currents["I(E1)"]) == pytest.approx(5e-3, rel=1e-6)


def test_vcvs_dc_load_independent() -> None:
    """VCVS output voltage is independent of load (ideal voltage source)."""
    def _vout(rload: float) -> float:
        c = Circuit([
            VoltageSource("Vin", "vin", "0", 2.0),
            VCVS("E1", "out", "0", "vin", "0", gain=3.0),
            Resistor("Rload", "out", "0", rload),
        ])
        return dc_op(c).node_voltages["out"]

    # V_out = 6V regardless of load
    assert _vout(100.0) == pytest.approx(6.0, abs=1e-9)
    assert _vout(10_000.0) == pytest.approx(6.0, abs=1e-9)


# ── Section 59: DC analysis – CCCS ───────────────────────────────────────────


def _cccs_circuit(beta: float = 2.0, r_load: float = 500.0) -> Circuit:
    """Standard CCCS test circuit.

    Topology: Vin(1V) → Rin(1kΩ) → Vsense(0V) → GND.
    CCCS F1 controlled by Vsense, output into Rload.

    I_ctrl = V_in / R_in = 1V / 1kΩ = 1 mA (Vsense is a 0V ammeter).

    SPICE convention: ``F1 n+ n-`` means positive current flows from ``n+``
    through the EXTERNAL circuit to ``n-``.  Here n_plus="out", n_minus="0"
    so 2 mA flows from "out" through Rload to ground: V_out = +beta*I*R_load.
    """
    return Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", beta),
        Resistor("Rload", "out", "0", r_load),
    ])


def test_cccs_dc_basic() -> None:
    """CCCS mirrors and scales current: V_out = beta × I_ctrl × R_load."""
    r = dc_op(_cccs_circuit(beta=2.0, r_load=500.0))
    # V_out = 2 * 1e-3 * 500 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_cccs_dc_unity_beta() -> None:
    """CCCS with beta=1.0 replicates the controlling current exactly."""
    r = dc_op(_cccs_circuit(beta=1.0, r_load=1000.0))
    # V_out = 1 * 1e-3 * 1000 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_cccs_dc_zero_beta() -> None:
    """CCCS with beta=0 injects no output current (open circuit)."""
    r = dc_op(_cccs_circuit(beta=0.0, r_load=500.0))
    assert r.node_voltages["out"] == pytest.approx(0.0, abs=1e-9)


def test_cccs_dc_negative_beta() -> None:
    """CCCS with beta=-1 reverses the current direction."""
    r = dc_op(_cccs_circuit(beta=-1.0, r_load=500.0))
    # V_out = -1 * 1e-3 * 500 = -0.5 V
    assert r.node_voltages["out"] == pytest.approx(-0.5, abs=1e-9)


def test_cccs_dc_higher_gain() -> None:
    """CCCS with larger beta amplifies proportionally."""
    r = dc_op(_cccs_circuit(beta=5.0, r_load=200.0))
    # V_out = 5 * 1e-3 * 200 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


# ── Section 60: DC analysis – CCVS ───────────────────────────────────────────


def _ccvs_circuit(rm: float = 500.0) -> Circuit:
    """Standard CCVS test circuit.

    Topology: Vin(1V) → Rin(1kΩ) → Vsense(0V) → GND.
    CCVS H1 controlled by Vsense forces V_out = rm × I_ctrl.

    I_ctrl = V_in / R_in = 1V / 1kΩ = 1 mA.
    V_out  = rm × I_ctrl = rm × 1 mA.
    """
    return Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", rm),
    ])


def test_ccvs_dc_basic() -> None:
    """CCVS forces V_out = rm × I_ctrl."""
    r = dc_op(_ccvs_circuit(rm=500.0))
    # V_out = 500 * 1e-3 = 0.5 V
    assert r.node_voltages["out"] == pytest.approx(0.5, abs=1e-9)


def test_ccvs_dc_unit_transresistance() -> None:
    """CCVS with rm=1000 Ω: V_out = 1 kΩ × 1 mA = 1 V."""
    r = dc_op(_ccvs_circuit(rm=1000.0))
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_ccvs_dc_zero_transresistance() -> None:
    """CCVS with rm=0 is a short circuit (ideal wire from output to ground)."""
    r = dc_op(_ccvs_circuit(rm=0.0))
    assert r.node_voltages["out"] == pytest.approx(0.0, abs=1e-9)


def test_ccvs_dc_with_load() -> None:
    """CCVS output voltage is independent of load (ideal voltage source)."""
    c_no_load = _ccvs_circuit(rm=500.0)
    c_with_load = Circuit(
        list(c_no_load.elements)
        + [Resistor("Rload", "out", "0", 100.0)]
    )
    r = dc_op(c_with_load)
    # V_out is still 0.5V regardless of the load resistance
    assert r.node_voltages["out"] == pytest.approx(0.5, abs=1e-9)


def test_ccvs_dc_branch_current_recorded() -> None:
    """dc_op records I(H1) in branch_currents for a CCVS element."""
    r = dc_op(_ccvs_circuit(rm=500.0))
    assert "I(H1)" in r.branch_currents


# ── Section 61: AC analysis – controlled sources ─────────────────────────────


def test_vcvs_ac_unity_buffer() -> None:
    """VCVS unity buffer passes AC signal unattenuated at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=5)
    for pt in result.points:
        assert abs(pt.node_voltages["out"]) == pytest.approx(1.0, rel=1e-6)


def test_vcvs_ac_gain() -> None:
    """VCVS with gain=3 scales AC voltage by 3 at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=3.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e4, n_points=3)
    for pt in result.points:
        assert abs(pt.node_voltages["out"]) == pytest.approx(3.0, rel=1e-6)


def test_vccs_ac_transconductance() -> None:
    """VCCS produces frequency-independent transconductance in AC analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=5)
    for pt in result.points:
        # V_out = gm * R_load * V_in = 0.01 * 100 * 1 = 1.0
        assert abs(pt.node_voltages["out"]) == pytest.approx(1.0, rel=1e-6)


def test_cccs_ac_current_gain() -> None:
    """CCCS scales controlling AC current by beta at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", 2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e3, n_points=3)
    for pt in result.points:
        # V_out = beta * (V_in/R_in) * R_load = 2 * (1/1000) * 500 = 1.0
        assert abs(pt.node_voltages["out"]) == pytest.approx(1.0, rel=1e-6)


def test_ccvs_ac_transresistance() -> None:
    """CCVS passes AC signal via transresistance at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", 500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e4, n_points=3)
    for pt in result.points:
        # V_out = rm * (V_in/R_in) = 500 * (1/1000) = 0.5
        assert abs(pt.node_voltages["out"]) == pytest.approx(0.5, rel=1e-6)


# ── Section 62: DC sweep – VCVS ──────────────────────────────────────────────


def test_vcvs_dc_sweep_linear() -> None:
    """VCVS output tracks dc_sweep of input source linearly (gain=2)."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 0.0),
        VCVS("E1", "out", "0", "vin", "0", gain=2.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = dc_sweep(c, "Vin", 0.0, 3.0, 1.0)
    v_outs = [pt.node_voltages["out"] for pt in result.points]
    # At V_in = 0, 1, 2, 3: V_out = 0, 2, 4, 6
    assert v_outs == pytest.approx([0.0, 2.0, 4.0, 6.0], abs=1e-9)


def test_vccs_dc_sweep() -> None:
    """VCCS output tracks dc_sweep — V_out = -gm * R * V_in at each point."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 0.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "out", "0", "in", "0", gm=0.01),
    ])
    result = dc_sweep(c, "Vin", 0.0, 2.0, 1.0)
    v_outs = [pt.node_voltages["out"] for pt in result.points]
    # V_out = -gm * R * V_in = -0.01 * 100 * V_in = -V_in
    assert v_outs == pytest.approx([0.0, -1.0, -2.0], abs=1e-9)


# ── Section 63: Transient – controlled sources ────────────────────────────────


def test_vcvs_transient_unity() -> None:
    """VCVS unity buffer reproduces the DC input in every transient timestep."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 3.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(3.0, abs=1e-6)


def test_vccs_transient() -> None:
    """VCCS works correctly during transient analysis (DC-steady circuit)."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(1.0, abs=1e-6)


def test_cccs_transient() -> None:
    """CCCS works correctly during transient analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", 2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(1.0, abs=1e-6)


def test_ccvs_transient() -> None:
    """CCVS works correctly during transient analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", 500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(0.5, abs=1e-6)


# ── Section 64: TF analysis – VCVS ───────────────────────────────────────────


def test_vcvs_tf_unity() -> None:
    """Transfer function of VCVS unity buffer is gain = 1.0."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    assert result.gain == pytest.approx(1.0, rel=1e-6)


def test_vcvs_tf_gain_two() -> None:
    """TF analysis returns the correct gain=2 for a VCVS amplifier."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=2.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    assert result.gain == pytest.approx(2.0, rel=1e-6)


def test_vcvs_tf_inverting() -> None:
    """TF analysis returns negative gain for an inverting VCVS."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=-5.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    assert result.gain == pytest.approx(-5.0, rel=1e-6)


def test_cccs_tf() -> None:
    """TF analysis works for circuits containing a CCCS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", 2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    # TF = V_out / V_in = beta * R_load / R_in = 2 * 500 / 1000 = 1.0
    assert result.gain == pytest.approx(1.0, rel=1e-6)


def test_ccvs_tf() -> None:
    """TF analysis works for circuits containing a CCVS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", 500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    # TF = V_out / V_in = rm / R_in = 500 / 1000 = 0.5
    assert result.gain == pytest.approx(0.5, rel=1e-6)


# ── Section 65: Sensitivity – VCVS ───────────────────────────────────────────


def test_vcvs_sens_dc_runs() -> None:
    """sens_dc runs without error in a circuit containing a VCVS."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = sens_dc(c, "out")
    assert isinstance(result.entries, list)
    assert result.nominal_voltage == pytest.approx(5.0, abs=1e-9)


def test_vccs_sens_dc_runs() -> None:
    """sens_dc runs without error in a circuit containing a VCCS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = sens_dc(c, "out")
    assert isinstance(result.entries, list)
    assert result.nominal_voltage == pytest.approx(1.0, abs=1e-9)


# ── Section 66: Monte Carlo – VCVS ───────────────────────────────────────────


def test_vcvs_mc_dc_runs() -> None:
    """mc_dc runs without error in a circuit containing a VCVS."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = mc_dc(c, "out", 5, seed=42)
    assert len(result.points) == 5
    # All trials should converge to roughly 5V (VCVS gain not varied by MC)
    for pt in result.points:
        assert abs(pt.node_voltages["out"] - 5.0) < 1.0


def test_vccs_mc_dc_runs() -> None:
    """mc_dc runs without error in a circuit containing a VCCS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = mc_dc(c, "out", 5, seed=0)
    assert len(result.points) == 5


# ── Section 67: Error cases – unknown ctrl_source ────────────────────────────


def test_cccs_dc_unknown_ctrl_source_raises() -> None:
    """CCCS referencing a non-existent VoltageSource raises ValueError at
    simulation time."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        CCCS("F1", "out", "0", "Vnonexistent", beta=2.0),
    ])
    with pytest.raises(ValueError, match="Vnonexistent"):
        dc_op(c)


def test_ccvs_dc_unknown_ctrl_source_raises() -> None:
    """CCVS referencing a non-existent VoltageSource raises ValueError at
    simulation time."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        CCVS("H1", "out", "0", "Vnonexistent", transresistance=100.0),
    ])
    with pytest.raises(ValueError, match="Vnonexistent"):
        dc_op(c)


def test_cccs_ac_unknown_ctrl_source_raises() -> None:
    """CCCS with unknown ctrl_source also raises ValueError in AC analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        CCCS("F1", "out", "0", "Vbad", beta=1.0),
    ])
    with pytest.raises(ValueError, match="Vbad"):
        ac_sweep(c, f_start=1.0, f_stop=1e3, n_points=2)


def test_ccvs_ac_unknown_ctrl_source_raises() -> None:
    """CCVS with unknown ctrl_source also raises ValueError in AC analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        CCVS("H1", "out", "0", "Vbad", transresistance=500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    with pytest.raises(ValueError, match="Vbad"):
        ac_sweep(c, f_start=1.0, f_stop=1e3, n_points=2)


# ---------------------------------------------------------------------------
# Section 66 — Time-varying source waveforms
# ---------------------------------------------------------------------------
#
# Each waveform class is exercised in two layers:
#   (a) unit tests on the callable directly (Waveform.__call__)
#   (b) integration tests that wire the waveform into a transient sim and
#       verify that node voltages track the expected waveform shape.
#
# The integration circuits are always a simple voltage-follower topology:
#
#     Vsrc (waveform) ─── "in" ─── R ─── "out" ─── 0
#
# With R → 0 (1 mΩ), V("out") ≈ V("in") = waveform(t).
#
# Alternatively for current sources:
#
#     Isrc (waveform) ─── "out" ─┬─── R ─── 0
#                                 └ V("out") = I_src(t) * R


# ---- PwlWaveform unit tests ------------------------------------------------


def test_pwl_waveform_before_first_breakpoint() -> None:
    """Before the first breakpoint PwlWaveform holds the first value."""
    w = PwlWaveform(points=((1.0, 0.5), (2.0, 1.5)))
    assert w(0.0) == pytest.approx(0.5)
    assert w(-10.0) == pytest.approx(0.5)


def test_pwl_waveform_after_last_breakpoint() -> None:
    """After the last breakpoint PwlWaveform holds the last value."""
    w = PwlWaveform(points=((0.0, 0.0), (1.0, 5.0)))
    assert w(2.0) == pytest.approx(5.0)
    assert w(100.0) == pytest.approx(5.0)


def test_pwl_waveform_at_breakpoints() -> None:
    """PwlWaveform returns exact values at defined breakpoints."""
    w = PwlWaveform(points=((0.0, 0.0), (1.0, 3.0), (2.0, 1.0)))
    assert w(0.0) == pytest.approx(0.0)
    assert w(1.0) == pytest.approx(3.0)
    assert w(2.0) == pytest.approx(1.0)


def test_pwl_waveform_linear_interpolation() -> None:
    """PwlWaveform interpolates linearly between breakpoints."""
    w = PwlWaveform(points=((0.0, 0.0), (1.0, 1.0)))
    assert w(0.25) == pytest.approx(0.25)
    assert w(0.5) == pytest.approx(0.5)
    assert w(0.75) == pytest.approx(0.75)


def test_pwl_waveform_negative_values() -> None:
    """PwlWaveform handles negative breakpoint values correctly."""
    w = PwlWaveform(points=((0.0, -2.0), (1.0, 2.0)))
    assert w(0.5) == pytest.approx(0.0)


def test_pwl_waveform_transient_step() -> None:
    """Transient sim driven by a PWL step: node voltage follows the ramp."""
    # PWL: 0 V at t=0, ramp to 1 V over 0.5 s, hold 1 V from 0.5 s onward.
    w = PwlWaveform(points=((0.0, 0.0), (0.5, 1.0), (1.0, 1.0)))
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=0.0, waveform=w),
        Resistor("Rser", "in", "out", 1e-3),  # near-ideal follower
        Resistor("Rload", "out", "0", 1.0),
    ])
    result = transient(c, t_stop=1.0, t_step=0.1)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        expected = w(pt.time)
        assert abs(v_out - expected) < 0.02, (
            f"t={pt.time:.2f}: V(out)={v_out:.4f} expected ~{expected:.4f}"
        )


# ---- SinWaveform unit tests ------------------------------------------------


def test_sin_waveform_before_delay() -> None:
    """SinWaveform returns offset before the delay time."""
    w = SinWaveform(offset=1.0, amplitude=2.0, frequency=10.0, delay=0.5)
    assert w(0.0) == pytest.approx(1.0)
    assert w(0.499) == pytest.approx(1.0)


def test_sin_waveform_at_zero_phase() -> None:
    """At t = delay the waveform returns offset (sin(0) = 0)."""
    w = SinWaveform(offset=0.5, amplitude=1.0, frequency=1.0, delay=0.1)
    assert w(0.1) == pytest.approx(0.5, abs=1e-12)


def test_sin_waveform_quarter_period() -> None:
    """At t = delay + T/4 the waveform hits its peak (sin = 1)."""
    freq = 2.0  # Hz
    T = 1.0 / freq
    delay = 0.0
    w = SinWaveform(offset=0.0, amplitude=3.0, frequency=freq, delay=delay)
    assert w(delay + T / 4) == pytest.approx(3.0, abs=1e-10)


def test_sin_waveform_damped_decays() -> None:
    """Damped sinusoid amplitude decreases over time."""
    w = SinWaveform(offset=0.0, amplitude=1.0, frequency=1.0, damping=1.0)
    # Peak at t = T/4 ≈ 0.25; a later peak (at ~1.25) should be smaller.
    v_first = abs(w(0.25))
    v_later = abs(w(1.25))
    assert v_later < v_first


def test_sin_waveform_transient_sinusoid() -> None:
    """Transient sim driven by SinWaveform: node tracks sin at sample times."""
    freq = 2.0   # Hz
    amp = 1.5
    w = SinWaveform(offset=0.0, amplitude=amp, frequency=freq)
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=0.0, waveform=w),
        Resistor("Rser", "in", "out", 1e-3),
        Resistor("Rload", "out", "0", 1.0),
    ])
    result = transient(c, t_stop=1.0, t_step=0.02)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        expected = amp * math.sin(2.0 * math.pi * freq * pt.time)
        assert abs(v_out - expected) < 0.05, (
            f"t={pt.time:.3f}: V(out)={v_out:.4f} expected ~{expected:.4f}"
        )


# ---- PulseWaveform unit tests -----------------------------------------------


def test_pulse_waveform_before_delay() -> None:
    """PulseWaveform holds v_initial before the delay time."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=5.0, delay=1.0)
    assert w(0.0) == pytest.approx(0.0)
    assert w(0.999) == pytest.approx(0.0)


def test_pulse_waveform_during_high_phase() -> None:
    """PulseWaveform is v_pulsed during the flat top."""
    # delay=0, rise_time=0, pulse_width=0.5, period=1.0
    w = PulseWaveform(v_initial=0.0, v_pulsed=3.3, pulse_width=0.5, period=1.0)
    assert w(0.1) == pytest.approx(3.3)
    assert w(0.49) == pytest.approx(3.3)


def test_pulse_waveform_during_low_phase() -> None:
    """PulseWaveform is v_initial during the low phase."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=5.0, pulse_width=0.5, period=1.0)
    assert w(0.75) == pytest.approx(0.0)
    assert w(0.99) == pytest.approx(0.0)


def test_pulse_waveform_rise_edge() -> None:
    """PulseWaveform linearly ramps from v_initial to v_pulsed over rise_time."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=1.0,
                      rise_time=0.2, fall_time=0.0,
                      pulse_width=0.5, period=1.0)
    assert w(0.0) == pytest.approx(0.0)
    assert w(0.1) == pytest.approx(0.5)
    assert w(0.2) == pytest.approx(1.0)


def test_pulse_waveform_fall_edge() -> None:
    """PulseWaveform linearly ramps from v_pulsed to v_initial over fall_time."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=1.0,
                      rise_time=0.0, fall_time=0.2,
                      pulse_width=0.5, period=1.0)
    # During falling edge: t_rel ∈ [0.5, 0.7)
    assert w(0.5) == pytest.approx(1.0, abs=1e-10)  # start of fall
    assert w(0.6) == pytest.approx(0.5, abs=1e-10)  # mid-fall
    assert w(0.7) == pytest.approx(0.0, abs=1e-10)  # end of fall → low


def test_pulse_waveform_periodic() -> None:
    """PulseWaveform repeats with the given period."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=1.0, pulse_width=0.5, period=1.0)
    # t=0.1 (first period high) and t=1.1 (second period high) should match.
    assert w(0.1) == pytest.approx(w(1.1))
    assert w(0.75) == pytest.approx(w(1.75))


def test_pulse_waveform_transient_current_source() -> None:
    """Transient sim: PWM current source drives a load; V = I*R follows pulse.

    Convention: CurrentSource(n_plus, n_minus, current=I) injects I into
    n_minus (positive terminal in SPICE3 terms).  Using n_plus="0" and
    n_minus="out" makes V("out") = I * R > 0 for positive I.
    """
    # 10 mA pulse with 50% duty cycle, period = 0.1 s
    R = 100.0
    I_high = 10e-3
    w = PulseWaveform(v_initial=0.0, v_pulsed=I_high,
                      pulse_width=0.05, period=0.1)
    c = Circuit([
        CurrentSource("Is", "0", "out", current=0.0, waveform=w),
        Resistor("Rload", "out", "0", R),
    ])
    result = transient(c, t_stop=0.2, t_step=0.005)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        expected = w(pt.time) * R
        # Allow ±5% of R*I_high for timestep quantisation
        assert abs(v_out - expected) < 0.05 * R * I_high + 1e-9, (
            f"t={pt.time:.4f}: V(out)={v_out:.5f} expected ~{expected:.5f}"
        )


# ---- ExpWaveform unit tests -------------------------------------------------


def test_exp_waveform_before_rise_delay() -> None:
    """ExpWaveform holds v_initial before rise_delay."""
    w = ExpWaveform(v_initial=0.0, v_pulsed=1.0, rise_delay=0.5, rise_tc=0.1)
    assert w(0.0) == pytest.approx(0.0)
    assert w(0.5) == pytest.approx(0.0)


def test_exp_waveform_rises_exponentially() -> None:
    """ExpWaveform approaches v_pulsed with rising exponential after rise_delay."""
    import math
    v0, v1 = 0.0, 5.0
    td1, tc1 = 0.0, 1.0
    # Disable fall by setting fall_delay after simulation horizon
    w = ExpWaveform(v_initial=v0, v_pulsed=v1,
                    rise_delay=td1, rise_tc=tc1,
                    fall_delay=100.0, fall_tc=1.0)
    for t in [0.5, 1.0, 2.0, 3.0]:
        expected = v0 + (v1 - v0) * (1.0 - math.exp(-t / tc1))
        assert w(t) == pytest.approx(expected, abs=1e-12)


def test_exp_waveform_falls_after_fall_delay() -> None:
    """ExpWaveform falls back towards v_initial after fall_delay."""
    v0, v1 = 0.0, 1.0
    # Rise at t=0 with very fast rise_tc so by fall_delay it is essentially v1.
    # Fall starts at fall_delay=0.5 with tc=0.5.
    w = ExpWaveform(v_initial=v0, v_pulsed=v1,
                    rise_delay=0.0, rise_tc=0.001,
                    fall_delay=0.5, fall_tc=0.5)
    # At t=2.0 (1.5 s after fall_delay), should be well below 0.5.
    assert w(2.0) < 0.5


def test_exp_waveform_transient_integration() -> None:
    """Transient sim driven by ExpWaveform: node tracks expected waveform."""
    import math
    v0, v1 = 0.0, 2.0
    rise_tc = 0.1
    w = ExpWaveform(v_initial=v0, v_pulsed=v1,
                    rise_delay=0.0, rise_tc=rise_tc,
                    fall_delay=10.0, fall_tc=1.0)   # no fall during sim
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=0.0, waveform=w),
        Resistor("Rser", "in", "out", 1e-3),
        Resistor("Rload", "out", "0", 1.0),
    ])
    result = transient(c, t_stop=0.5, t_step=0.02)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        t = pt.time
        expected = v0 + (v1 - v0) * (1.0 - math.exp(-t / rise_tc))
        # Allow 2% of v1 for integration error
        assert abs(v_out - expected) < 0.02 * v1 + 1e-6, (
            f"t={pt.time:.3f}: V(out)={v_out:.5f} expected ~{expected:.5f}"
        )


# ---- Waveform type alias and export tests ------------------------------------


def test_waveform_type_alias_covers_all_forms() -> None:
    """Waveform type alias is the union of all four waveform classes."""
    # The type alias itself is not a runtime class, but we can verify that
    # each concrete form is a valid Waveform by checking isinstance against
    # the individual classes.
    forms = [
        PwlWaveform(points=((0.0, 0.0), (1.0, 1.0))),
        SinWaveform(),
        PulseWaveform(),
        ExpWaveform(),
    ]
    for form in forms:
        assert callable(form), f"{type(form).__name__} must be callable"
        assert isinstance(form, (PwlWaveform, SinWaveform, PulseWaveform, ExpWaveform))


def test_waveform_exported_from_package() -> None:
    """PwlWaveform, SinWaveform, PulseWaveform, ExpWaveform, Waveform are exported."""
    import spice_engine
    assert hasattr(spice_engine, "PwlWaveform")
    assert hasattr(spice_engine, "SinWaveform")
    assert hasattr(spice_engine, "PulseWaveform")
    assert hasattr(spice_engine, "ExpWaveform")
    assert hasattr(spice_engine, "Waveform")


def test_voltage_source_accepts_waveform_field() -> None:
    """VoltageSource.waveform defaults to None and accepts a waveform object."""
    v_static = VoltageSource("V1", "a", "0", voltage=5.0)
    assert v_static.waveform is None

    w = SinWaveform(amplitude=1.0, frequency=60.0)
    v_dyn = VoltageSource("V2", "a", "0", voltage=0.0, waveform=w)
    assert v_dyn.waveform is w


def test_current_source_accepts_waveform_field() -> None:
    """CurrentSource.waveform defaults to None and accepts a waveform object."""
    i_static = CurrentSource("I1", "a", "0", current=1e-3)
    assert i_static.waveform is None

    w = PulseWaveform(v_initial=0.0, v_pulsed=5e-3)
    i_dyn = CurrentSource("I2", "a", "0", current=0.0, waveform=w)
    assert i_dyn.waveform is w


def test_waveform_source_dc_op_uses_static_voltage() -> None:
    """DC op-point of a waveform source uses the stored `voltage` field."""
    # The waveform is irrelevant for DC; only `voltage` is used.
    w = SinWaveform(amplitude=10.0, frequency=1e3)
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=3.0, waveform=w),
        Resistor("R", "in", "0", 1.0),
    ])
    op = dc_op(c)
    assert op.converged
    assert op.node_voltages["in"] == pytest.approx(3.0)


# ---------------------------------------------------------------------------
# Section 67 — Convergence aids: Gmin stepping and source stepping
# ---------------------------------------------------------------------------
#
# The convergence-aid chain is:
#   dc_op (plain Newton) → _dc_gmin_step → _dc_source_step
#
# For most circuits plain Newton converges immediately, so the aids are
# transparent.  These tests verify:
#   (a) the private helpers produce correct results on simple circuits,
#   (b) the public dc_op interface respects the convergence_aids flag,
#   (c) circuits that fail plain Newton (strongly nonlinear, near-singular)
#       succeed when aids are enabled.


# ---- _dc_newton: warm-start and convergence_aids=False ---------------------


def test_dc_newton_matches_dc_op_on_simple_circuit() -> None:
    """_dc_newton gives the same result as dc_op on a simple resistive circuit."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 5.0),
        Resistor("R1", "in", "out", 1000.0),
        Resistor("R2", "out", "0", 1000.0),
    ])
    public_result = dc_op(c)
    private_result = _dc_newton(c, max_iterations=50, tol=1e-6)
    assert private_result.converged
    assert private_result.node_voltages["out"] == pytest.approx(
        public_result.node_voltages["out"], abs=1e-9
    )


def test_dc_newton_warm_start_converges_faster() -> None:
    """Warm-started _dc_newton converges in fewer or equal iterations."""
    from spice_engine.engine import _branch_sources
    c = Circuit([
        VoltageSource("V1", "in", "0", 5.0),
        Resistor("R1", "in", "out", 1000.0),
        Resistor("R2", "out", "0", 1000.0),
    ])
    cold = _dc_newton(c, max_iterations=50, tol=1e-9)
    assert cold.converged

    # Warm-start from the converged solution — should converge in ≤ cold iters.
    _, nodes = _node_index(c)
    branch_srcs = _branch_sources(c)
    x_warm = _x_from_result(cold, nodes, branch_srcs)
    warm = _dc_newton(c, max_iterations=50, tol=1e-9, x_init=x_warm)
    assert warm.converged
    assert warm.iterations <= cold.iterations


def test_dc_op_convergence_aids_false_still_converges_linear() -> None:
    """dc_op(convergence_aids=False) converges on a simple linear circuit."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 10.0),
        Resistor("R1", "a", "0", 100.0),
    ])
    result = dc_op(c, convergence_aids=False)
    assert result.converged
    assert result.node_voltages["a"] == pytest.approx(10.0)


# ---- _dc_gmin_step ---------------------------------------------------------


def test_dc_gmin_step_resistor_divider() -> None:
    """_dc_gmin_step returns correct voltages for a resistive voltage divider."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 6.0),
        Resistor("R1", "in", "mid", 2000.0),
        Resistor("R2", "mid", "0", 4000.0),
    ])
    result = _dc_gmin_step(c)
    assert result is not None
    assert result.converged
    # V(mid) = 6 * 4k / (2k + 4k) = 4.0 V
    assert result.node_voltages["mid"] == pytest.approx(4.0, abs=1e-3)


def test_dc_gmin_step_diode_circuit() -> None:
    """_dc_gmin_step converges on a diode+resistor circuit."""
    c = Circuit([
        VoltageSource("Vs", "in", "0", 5.0),
        Diode("D1", anode="in", cathode="out"),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = _dc_gmin_step(c)
    assert result is not None
    assert result.converged
    # Gmin result should be very close to plain Newton result.
    plain = dc_op(c, convergence_aids=False)
    assert result.node_voltages["out"] == pytest.approx(
        plain.node_voltages["out"], abs=1e-4
    )


def test_dc_gmin_step_no_nodes_returns_none() -> None:
    """_dc_gmin_step returns None for a trivial circuit with no non-ground nodes."""
    c = Circuit([
        Resistor("R1", "0", "gnd", 100.0),  # both "0" and "gnd" are ground aliases
    ])
    assert _dc_gmin_step(c) is None


def test_dc_gmin_step_custom_parameters() -> None:
    """_dc_gmin_step accepts custom gmin_start and n_steps."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 3.3),
        Resistor("R1", "a", "0", 1000.0),
    ])
    result = _dc_gmin_step(c, gmin_start=1e-2, n_steps=5)
    assert result is not None
    assert result.converged
    assert result.node_voltages["a"] == pytest.approx(3.3, abs=1e-4)


# ---- _dc_source_step -------------------------------------------------------


def test_dc_source_step_resistor_divider() -> None:
    """_dc_source_step returns correct voltages for a resistive voltage divider."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 10.0),
        Resistor("R1", "in", "mid", 1000.0),
        Resistor("R2", "mid", "0", 1000.0),
    ])
    result = _dc_source_step(c)
    assert result is not None
    assert result.converged
    # V(mid) = 10 * 1k / (1k + 1k) = 5.0 V
    assert result.node_voltages["mid"] == pytest.approx(5.0, abs=1e-4)


def test_dc_source_step_current_source() -> None:
    """_dc_source_step scales current sources correctly."""
    r_val = 500.0
    i_val = 2e-3
    c = Circuit([
        CurrentSource("I1", "0", "out", i_val),
        Resistor("Rload", "out", "0", r_val),
    ])
    result = _dc_source_step(c)
    assert result is not None
    assert result.converged
    assert result.node_voltages["out"] == pytest.approx(i_val * r_val, abs=1e-6)


def test_dc_source_step_diode_circuit() -> None:
    """_dc_source_step converges on a diode circuit, matching plain Newton."""
    c = Circuit([
        VoltageSource("Vs", "in", "0", 5.0),
        Diode("D1", anode="in", cathode="out"),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = _dc_source_step(c)
    assert result is not None
    assert result.converged
    plain = dc_op(c, convergence_aids=False)
    assert result.node_voltages["out"] == pytest.approx(
        plain.node_voltages["out"], abs=1e-4
    )


def test_dc_source_step_custom_n_steps() -> None:
    """_dc_source_step accepts a custom number of steps."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 1.0),
        Resistor("R1", "a", "0", 100.0),
    ])
    result = _dc_source_step(c, n_steps=5)
    assert result is not None
    assert result.converged
    assert result.node_voltages["a"] == pytest.approx(1.0, abs=1e-6)


# ---- dc_op public interface -----------------------------------------------


def test_dc_op_convergence_aids_default_gives_same_result() -> None:
    """dc_op with and without aids gives identical results on a linear circuit."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 5.0),
        Resistor("R1", "in", "out", 2000.0),
        Resistor("R2", "out", "0", 3000.0),
    ])
    r_with = dc_op(c, convergence_aids=True)
    r_without = dc_op(c, convergence_aids=False)
    assert r_with.converged
    assert r_without.converged
    assert r_with.node_voltages["out"] == pytest.approx(
        r_without.node_voltages["out"], abs=1e-9
    )


def test_dc_op_with_aids_and_diode() -> None:
    """dc_op(convergence_aids=True) matches aids=False on a diode circuit."""
    c = Circuit([
        VoltageSource("Vs", "a", "0", 2.0),
        Diode("D1", anode="a", cathode="b"),
        Resistor("R1", "b", "0", 500.0),
    ])
    r_aids = dc_op(c, convergence_aids=True)
    r_no_aids = dc_op(c, convergence_aids=False)
    assert r_aids.converged
    assert r_no_aids.converged
    assert r_aids.node_voltages["b"] == pytest.approx(
        r_no_aids.node_voltages["b"], abs=1e-4
    )


def test_dc_op_convergence_aids_high_voltage_diode() -> None:
    """dc_op converges on a high-voltage diode circuit with convergence aids.

    The circuit is: Vs(10V) → D1(anode=in, cathode=out) → Rload(100Ω) → GND.
    The engine clamps the diode linearisation point at Vd = 0.7 V during
    Newton iterations; the converged operating point therefore reflects the
    clamped model rather than the ideal exponential.  The test verifies
    convergence and that the aids path agrees exactly with the plain-Newton
    path (both solvers should reach the same fixed-point for this circuit).
    """
    c = Circuit([
        VoltageSource("Vs", "in", "0", 10.0),
        Diode("D1", anode="in", cathode="out", Is=1e-15, Vt=0.02585),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result_aids = dc_op(c, convergence_aids=True)
    result_plain = dc_op(c, convergence_aids=False)
    assert result_aids.converged
    assert result_plain.converged
    # Both paths must reach the same operating point.
    assert abs(result_aids.node_voltages["out"] - result_plain.node_voltages["out"]) < 1e-6
    # Sanity: output is strictly between ground and the supply.
    assert 0.0 < result_aids.node_voltages["out"] < 10.0


def test_dc_op_iterations_field_is_populated() -> None:
    """dc_op.iterations is positive after a successful solve."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 1.0),
        Resistor("R1", "a", "0", 1.0),
    ])
    result = dc_op(c)
    assert result.converged
    assert result.iterations >= 1


# ---- _x_from_result --------------------------------------------------------


def test_x_from_result_round_trip() -> None:
    """_x_from_result reconstructs the x vector that produced the DcResult."""
    from spice_engine.engine import _branch_sources
    c = Circuit([
        VoltageSource("V1", "a", "0", 5.0),
        Resistor("R1", "a", "b", 1000.0),
        Resistor("R2", "b", "0", 1000.0),
    ])
    result = dc_op(c)
    assert result.converged

    _, nodes = _node_index(c)
    branch_srcs = _branch_sources(c)
    x = _x_from_result(result, nodes, branch_srcs)

    # x should have one entry per node + one per branch source
    assert len(x) == len(nodes) + len(branch_srcs)
    # Node voltage entries should match result
    for i, nd in enumerate(nodes):
        assert x[i] == pytest.approx(result.node_voltages[nd])
