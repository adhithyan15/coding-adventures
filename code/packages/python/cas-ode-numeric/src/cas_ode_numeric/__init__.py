"""cas-ode-numeric — Runge-Kutta 4 numeric ODE integrator.

This package provides a single public entry point:

- :func:`rk4_solve` — numerically integrate a system of first-order ODEs
  whose right-hand sides are expressed as IR trees evaluated through the
  symbolic VM.

Quick start::

    from symbolic_ir import IRSymbol, IRApply, MUL, ADD, NEG
    from symbolic_vm import VM
    from macsyma_runtime import MacsymaBackend
    from cas_ode_numeric import rk4_solve

    # Build f(y, t) IR for the scalar ODE  dy/dt = -2·y
    y = IRSymbol("y")
    t = IRSymbol("t")
    f_ir = IRApply(MUL, (IRInteger(-2), y))   # f(y, t) = -2*y

    vm = VM(MacsymaBackend())
    trajectory = rk4_solve(
        f_ir=[f_ir],      # list of RHS IR expressions (one per state variable)
        y0=[1.0],          # initial condition
        t_span=(0.0, 1.0), # time interval
        dt=0.01,           # step size
        vm=vm,
        state_names=["y"],  # variable names bound in the VM during evaluation
    )
    # trajectory is a list of (t, [y_values]) tuples.
    t_final, y_final = trajectory[-1]   # ≈ (1.0, [exp(-2)])

Architecture
------------
RK4 is a classic fixed-step explicit Runge-Kutta method::

    k1 = f(t,     y)
    k2 = f(t+h/2, y + h*k1/2)
    k3 = f(t+h/2, y + h*k2/2)
    k4 = f(t+h,   y + h*k3)
    y_next = y + h/6 * (k1 + 2*k2 + 2*k3 + k4)

For each stage the RHS is evaluated by temporarily binding the current
state values into the VM's environment, calling ``vm.eval(f_ir[i])`` for
each component, then restoring the original bindings.

The symbolic VM evaluates the IR tree numerically when all free symbols
are bound to ``IRFloat`` values.  The SPICE transient solver uses this to
integrate nonlinear circuits where the RHS cannot be solved in closed form.

See :func:`rk4_solve` for full API documentation.
"""

from __future__ import annotations

from cas_ode_numeric.rk4 import rk4_solve

__all__ = ["rk4_solve"]
