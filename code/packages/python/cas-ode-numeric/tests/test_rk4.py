"""Tests for the RK4 numeric ODE integrator.

These tests verify the correctness of :func:`cas_ode_numeric.rk4_solve`
across several analytically known ODE solutions.

Test organisation:
1. Scalar exponential decay — known solution exp(-kt).
2. Simple harmonic oscillator (coupled 2D system).
3. Step size / accuracy regression.
4. Error handling (bad arguments).
5. VM environment isolation — the integrator must not permanently modify
   the caller's environment.
6. SPICE-style RLC transient (underdamped 2nd-order system).
"""

from __future__ import annotations

import math

import pytest

from symbolic_ir import (
    ADD,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRSymbol,
)
from symbolic_vm import VM

from macsyma_runtime import MacsymaBackend

from cas_ode_numeric import rk4_solve


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _vm() -> VM:
    return VM(MacsymaBackend())


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _sym(name: str) -> IRSymbol:
    return IRSymbol(name)


# ---------------------------------------------------------------------------
# 1 — Scalar exponential decay: dy/dt = -k*y,  y(0) = 1
# ---------------------------------------------------------------------------


def test_scalar_decay_solution() -> None:
    """dy/dt = -2*y, y(0) = 1  →  y(1) ≈ exp(-2) ≈ 0.1353."""
    vm = _vm()
    y = _sym("y")
    f = IRApply(MUL, (_int(-2), y))    # f(y) = -2*y

    traj = rk4_solve([f], [1.0], (0.0, 1.0), 0.001, vm, state_names=["y"])

    t_end, state = traj[-1]
    assert abs(t_end - 1.0) < 1e-10
    assert abs(state[0] - math.exp(-2.0)) < 1e-4, (
        f"Expected exp(-2)≈{math.exp(-2.0):.6f}, got {state[0]:.6f}"
    )


def test_scalar_decay_trajectory_length() -> None:
    """The trajectory must have approximately (t_end - t_start)/dt + 1 entries."""
    vm = _vm()
    y = _sym("y")
    f = IRApply(MUL, (_int(-1), y))

    traj = rk4_solve([f], [1.0], (0.0, 1.0), 0.1, vm, state_names=["y"])

    # 10 steps + initial point = 11 entries
    assert len(traj) == 11, f"Expected 11 entries, got {len(traj)}"


def test_scalar_decay_first_entry_is_initial_condition() -> None:
    """The first trajectory entry must be (t_start, y0)."""
    vm = _vm()
    y = _sym("y")
    f = IRApply(MUL, (_int(-1), y))

    traj = rk4_solve([f], [2.5], (0.0, 0.5), 0.1, vm, state_names=["y"])
    t0, state0 = traj[0]

    assert abs(t0 - 0.0) < 1e-12
    assert abs(state0[0] - 2.5) < 1e-12


def test_scalar_zero_rhs() -> None:
    """dy/dt = 0  →  y stays constant for all t."""
    vm = _vm()
    # f(y, t) = 0 (a constant zero node)
    f = _int(0)

    traj = rk4_solve([f], [3.7], (0.0, 2.0), 0.5, vm, state_names=["y"])

    for t, state in traj:
        assert abs(state[0] - 3.7) < 1e-12, f"y changed at t={t}: {state[0]}"


# ---------------------------------------------------------------------------
# 2 — Coupled simple harmonic oscillator: dy/dt = v, dv/dt = -y
# ---------------------------------------------------------------------------


def test_coupled_oscillator_one_period() -> None:
    """After one full period (2π), y ≈ 1 and v ≈ 0.

    Analytical solution starting from (y=1, v=0):
        y(t) = cos(t)
        v(t) = -sin(t)
    """
    vm = _vm()
    y = _sym("y")
    v = _sym("v")

    # dy/dt = v,   dv/dt = -y
    f_y = v                          # IRSymbol("v")
    f_v = IRApply(NEG, (y,))         # Neg(y)

    T = 2.0 * math.pi
    traj = rk4_solve(
        [f_y, f_v],
        [1.0, 0.0],      # y(0)=1, v(0)=0
        (0.0, T),
        dt=0.001,
        vm=vm,
        state_names=["y", "v"],
    )

    t_end, state = traj[-1]
    y_end, v_end = state

    assert abs(t_end - T) < 1e-8
    assert abs(y_end - 1.0) < 0.01, f"y after 2π: expected ≈1, got {y_end:.6f}"
    assert abs(v_end - 0.0) < 0.01, f"v after 2π: expected ≈0, got {v_end:.6f}"


def test_coupled_oscillator_quarter_period() -> None:
    """After π/2, y ≈ 0 and v ≈ -1 (cos(π/2) = 0, -sin(π/2) = -1)."""
    vm = _vm()
    y = _sym("y")
    v = _sym("v")

    f_y = v
    f_v = IRApply(NEG, (y,))

    traj = rk4_solve(
        [f_y, f_v],
        [1.0, 0.0],
        (0.0, math.pi / 2.0),
        dt=0.0001,
        vm=vm,
        state_names=["y", "v"],
    )

    t_end, state = traj[-1]
    y_end, v_end = state

    assert abs(y_end - 0.0) < 0.005, f"y at π/2: expected ≈0, got {y_end:.6f}"
    assert abs(v_end - (-1.0)) < 0.005, f"v at π/2: expected ≈-1, got {v_end:.6f}"


# ---------------------------------------------------------------------------
# 3 — Accuracy vs step size
# ---------------------------------------------------------------------------


def test_smaller_dt_gives_lower_error() -> None:
    """Halving dt should roughly reduce error by a factor of 16 (RK4 is O(h^4))."""
    vm = _vm()
    y = _sym("y")
    f = IRApply(MUL, (_int(-1), y))   # dy/dt = -y,  y_exact(1) = exp(-1)
    exact = math.exp(-1.0)

    def _err(dt: float) -> float:
        traj = rk4_solve([f], [1.0], (0.0, 1.0), dt, vm, state_names=["y"])
        return abs(traj[-1][1][0] - exact)

    err_coarse = _err(0.1)
    err_fine = _err(0.05)

    # Coarse step should be noticeably worse than fine step (order-4 method).
    assert err_coarse > err_fine, (
        f"Expected err_coarse({err_coarse:.2e}) > err_fine({err_fine:.2e})"
    )


# ---------------------------------------------------------------------------
# 4 — Error handling
# ---------------------------------------------------------------------------


def test_negative_dt_raises_value_error() -> None:
    vm = _vm()
    with pytest.raises(ValueError, match="dt must be positive"):
        rk4_solve([_int(0)], [1.0], (0.0, 1.0), -0.1, vm, state_names=["y"])


def test_zero_dt_raises_value_error() -> None:
    vm = _vm()
    with pytest.raises(ValueError, match="dt must be positive"):
        rk4_solve([_int(0)], [1.0], (0.0, 1.0), 0.0, vm, state_names=["y"])


def test_mismatched_y0_f_ir_raises_value_error() -> None:
    vm = _vm()
    with pytest.raises(ValueError, match="y0 has 2 entries"):
        rk4_solve([_int(0)], [1.0, 2.0], (0.0, 1.0), 0.1, vm)


def test_mismatched_state_names_raises_value_error() -> None:
    vm = _vm()
    with pytest.raises(ValueError, match="state_names has"):
        rk4_solve(
            [_int(0)], [1.0], (0.0, 1.0), 0.1, vm, state_names=["a", "b"]
        )


# ---------------------------------------------------------------------------
# 5 — VM environment isolation
# ---------------------------------------------------------------------------


def test_vm_env_is_restored_after_integration() -> None:
    """The integrator must not permanently modify the caller's VM environment.

    We pre-bind 'y' to a sentinel value, run the integrator (which
    temporarily rebinds 'y'), then verify the sentinel is restored.
    """
    vm = _vm()
    y = _sym("y")
    f = IRApply(MUL, (_int(-1), y))

    sentinel = IRFloat(999.0)
    vm.backend.bind("y", sentinel)   # pre-bind before integration

    rk4_solve([f], [1.0], (0.0, 0.1), 0.05, vm, state_names=["y"])

    restored = vm.backend.lookup("y")
    assert restored is sentinel, (
        f"VM backend 'y' was not restored: got {restored!r}"
    )


def test_vm_env_key_absent_is_restored_as_absent() -> None:
    """If 'y' was NOT pre-bound, it must be absent again after integration."""
    vm = _vm()
    y = _sym("y")
    f = IRApply(MUL, (_int(-1), y))

    # Make sure 'y' is not in env by unbinding any prior value.
    vm.backend.unbind("y")

    rk4_solve([f], [1.0], (0.0, 0.1), 0.05, vm, state_names=["y"])

    after = vm.backend.lookup("y")
    assert after is None, f"Expected 'y' unbound after integration, found {after!r}"


# ---------------------------------------------------------------------------
# 6 — SPICE-style RLC transient (underdamped 2nd-order)
# ---------------------------------------------------------------------------


def test_rlc_underdamped_transient() -> None:
    """Underdamped RLC series circuit transient.

    For an RLC series circuit with unit step input V(t) = 1:

        L·i' + R·i + q/C = V     (KVL)

    Split into a 2-state system where state = [q, i] (charge and current,
    where i = dq/dt):

        dq/dt = i
        di/dt = (V - R·i - q/C) / L

    With L=1, R=0.5, C=1, V=1:

        dq/dt = i
        di/dt = 1 - 0.5·i - q

    This is underdamped (discriminant of characteristic eq < 0).

    We just verify the simulation runs and that q rises from 0 toward the
    steady-state q_ss = C·V = 1 without blowing up.
    """
    vm = _vm()

    q = _sym("q")
    i = _sym("i")

    # dq/dt = i
    f_q = i

    # di/dt = 1 - 0.5*i - q  =  1 - (1/2)*i - q
    half_i = IRApply(MUL, (IRFloat(0.5), i))
    di_dt = IRApply(SUB, (IRApply(SUB, (_int(1), half_i)), q))

    traj = rk4_solve(
        [f_q, di_dt],
        [0.0, 0.0],         # q(0) = 0, i(0) = 0
        (0.0, 20.0),        # run for 20 time units (several damped oscillations)
        dt=0.01,
        vm=vm,
        state_names=["q", "i"],
    )

    # The charge must approach q_ss = 1 without diverging.
    q_values = [state[0] for _, state in traj]
    assert max(q_values) < 3.0, f"q blew up: max={max(q_values):.2f}"
    assert min(q_values) > -1.0, f"q went too negative: min={min(q_values):.2f}"

    # Final charge should be within 5% of steady state (q_ss = 1).
    q_final = traj[-1][1][0]
    assert abs(q_final - 1.0) < 0.05, (
        f"Final charge expected ≈1.0, got {q_final:.4f}"
    )


def test_rk4_uses_ir_float_nodes_for_binding() -> None:
    """RHS IR that references an IRFloat already in the tree evaluates correctly.

    Verify that the integrator correctly handles f(y, t) that depends on
    the time variable 't'.
    """
    vm = _vm()
    t = _sym("t")
    y = _sym("y")

    # dy/dt = t  →  y(t) = t²/2 + y(0) = t²/2 (with y(0)=0)
    f = t

    traj = rk4_solve(
        [f],
        [0.0],
        (0.0, 2.0),
        dt=0.001,
        vm=vm,
        state_names=["y"],
        t_name="t",
    )

    t_end, state = traj[-1]
    # y(2) = 2²/2 = 2.0
    assert abs(state[0] - 2.0) < 0.01, (
        f"dy/dt=t → y(2)=2, got {state[0]:.6f}"
    )
