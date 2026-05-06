"""Fixed-step Runge-Kutta 4 numeric ODE integrator.

Overview
--------
This module integrates a *system* of first-order ODEs::

    dy₁/dt = f₁(t, y₁, y₂, …, yₙ)
    dy₂/dt = f₂(t, y₁, y₂, …, yₙ)
    ⋮
    dyₙ/dt = fₙ(t, y₁, y₂, …, yₙ)

The right-hand side functions ``f₁ … fₙ`` are given as IR trees.  At
each RK4 stage the current time and state are bound into the VM
environment so that ``vm.eval(f_ir[i])`` returns a numeric result.

Why RK4?
--------
RK4 balances accuracy (4th-order local truncation error O(h⁵)) and
simplicity.  For SPICE-style transient analysis of nonlinear circuits
(diodes, MOSFETs) the device equations cannot be integrated in closed
form, so a numeric integrator is required.  RK4 is the standard choice
for educational simulators:

- The Gear-2 / trapezoidal methods used by production SPICE are more
  complex but give better stability for stiff systems.  RK4 is correct
  and educational.

RK4 algorithm (scalar, for clarity)
-------------------------------------
Given dy/dt = f(t, y) and stepsize h::

    k₁ = h · f(t,       y)
    k₂ = h · f(t + h/2, y + k₁/2)
    k₃ = h · f(t + h/2, y + k₂/2)
    k₄ = h · f(t + h,   y + k₃)

    y(t+h) = y + (k₁ + 2·k₂ + 2·k₃ + k₄) / 6

For a system of ``n`` equations, the same recurrence is applied
simultaneously to all components.

VM interaction
--------------
The IR trees reference free symbols for the state variables and (optionally)
the time variable.  At each sub-step we:

1. Save the current environment values for all state names and the time name.
2. Bind ``IRFloat(value)`` for each state variable and the time variable.
3. Call ``vm.eval(f_ir[i])`` for each component.  The VM evaluates the
   expression numerically because all symbols are now bound to floats.
4. Restore the saved environment entries.

Step 1/4 ensure that user-defined variables set outside the integrator
are not permanently overwritten.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import IRFloat, IRNode, IRSymbol

if TYPE_CHECKING:
    from symbolic_vm import VM


def rk4_solve(
    f_ir: list[IRNode],
    y0: list[float],
    t_span: tuple[float, float],
    dt: float,
    vm: "VM",
    *,
    state_names: list[str] | None = None,
    t_name: str = "t",
) -> list[tuple[float, list[float]]]:
    """Integrate a system of ODEs using the classic RK4 method.

    The system is::

        dy_i/dt = f_i(t, y_1, …, y_n)   for i = 0, …, n-1

    where each ``f_i`` is an IR expression that may reference the time
    variable (``t_name``) and any of the state variable names
    (``state_names``).

    Parameters
    ----------
    f_ir:
        List of ``n`` IR expressions — one per ODE component.  Each
        expression's free symbols must be a subset of
        ``{t_name} ∪ set(state_names)``.
    y0:
        Initial conditions as a list of ``n`` floats.  Must have the
        same length as ``f_ir``.
    t_span:
        Tuple ``(t_start, t_end)``.  Integration runs from ``t_start``
        to ``t_end`` inclusive (the last step may be shortened to land
        exactly on ``t_end``).
    dt:
        Fixed step size (positive float).  A smaller step gives higher
        accuracy at higher computational cost.
    vm:
        A live :class:`symbolic_vm.VM` instance.  The integrator
        temporarily binds state values into its environment.
    state_names:
        Names of the state variable symbols in the IR trees, in the
        same order as ``y0`` and ``f_ir``.  Defaults to
        ``["y0", "y1", …, "y{n-1}"]`` when ``None``.
    t_name:
        Name of the time symbol referenced in the IR trees.  Defaults
        to ``"t"``.

    Returns
    -------
    List of ``(t, state)`` tuples, one per recorded step.  The first
    entry is ``(t_start, y0)``; the last entry is approximately
    ``(t_end, y_final)``.

    Raises
    ------
    ValueError
        If ``f_ir``, ``y0``, and ``state_names`` have different lengths,
        or if ``dt <= 0``.

    Examples
    --------
    Scalar exponential decay  ``dy/dt = -2y``,  ``y(0) = 1``::

        from symbolic_ir import IRApply, IRInteger, IRSymbol, MUL
        from symbolic_vm import VM
        from macsyma_runtime import MacsymaBackend
        from cas_ode_numeric import rk4_solve
        import math

        vm = VM(MacsymaBackend())
        y = IRSymbol("y")
        f = IRApply(MUL, (IRInteger(-2), y))   # f(y) = -2*y
        traj = rk4_solve([f], [1.0], (0.0, 1.0), 0.001, vm, state_names=["y"])
        t_end, (y_end,) = traj[-1]
        assert abs(y_end - math.exp(-2.0)) < 1e-5

    Coupled oscillator (simple harmonic):  ``dy/dt = v``, ``dv/dt = -y``::

        from symbolic_ir import IRSymbol, IRApply, NEG
        y_sym = IRSymbol("y")
        v_sym = IRSymbol("v")
        traj = rk4_solve(
            [v_sym, IRApply(NEG, (y_sym,))],
            [1.0, 0.0],
            (0.0, 2 * math.pi),
            0.001,
            vm,
            state_names=["y", "v"],
        )
        t_end, (y_end, v_end) = traj[-1]
        # After one full period: y ≈ 1, v ≈ 0
        assert abs(y_end - 1.0) < 0.01
    """
    n = len(f_ir)
    if len(y0) != n:
        raise ValueError(
            f"f_ir has {n} components but y0 has {len(y0)} entries."
        )
    if dt <= 0:
        raise ValueError(f"dt must be positive, got {dt!r}.")

    if state_names is None:
        state_names = [f"y{i}" for i in range(n)]
    if len(state_names) != n:
        raise ValueError(
            f"state_names has {len(state_names)} entries but f_ir has {n}."
        )

    t_start, t_end = t_span
    t_sym = IRSymbol(t_name)
    state_syms = [IRSymbol(name) for name in state_names]

    # All names we will temporarily bind in the VM environment.
    # We save and restore so the caller's environment is unaffected.
    all_names: list[str] = [t_name] + state_names

    def _eval_rhs(t_val: float, y_vals: list[float]) -> list[float]:
        """Evaluate all f_i at the given (t, y) state.

        Temporarily binds ``t_name`` and each ``state_names[i]`` to
        ``IRFloat`` values via ``vm.backend.bind()``, evaluates all IR
        components, then restores the originals via ``vm.backend.unbind()``
        or ``vm.backend.bind()`` depending on whether a prior binding existed.

        Using the public ``bind``/``unbind``/``lookup`` API on the backend
        is the correct way to interact with the VM's environment from
        outside the VM; the ``_env`` attribute is considered private.
        """
        backend = vm.backend

        # Save existing bindings (may be None if not bound).
        saved = {name: backend.lookup(name) for name in all_names}

        # Bind current values.
        backend.bind(t_name, IRFloat(t_val))
        for sym, val in zip(state_names, y_vals, strict=True):
            backend.bind(sym, IRFloat(val))

        # Evaluate each component.
        result = []
        for fi in f_ir:
            out = vm.eval(fi)
            # The result should be a numeric IR node.  Extract float.
            result.append(_ir_to_float(out))

        # Restore saved bindings — unbind names that weren't bound before,
        # re-bind names that had a prior value.
        for name, old_val in saved.items():
            if old_val is None:
                backend.unbind(name)
            else:
                backend.bind(name, old_val)

        return result

    def _ir_to_float(node: IRNode) -> float:
        """Extract a Python float from a numeric IR node.

        Accepts :class:`IRFloat`, :class:`IRInteger`, and
        :class:`IRRational`.  Raises ``TypeError`` for symbolic nodes —
        this indicates that some variable was not properly bound.
        """
        from symbolic_ir import IRInteger, IRRational

        if isinstance(node, IRFloat):
            return node.value
        if isinstance(node, IRInteger):
            return float(node.value)
        if isinstance(node, IRRational):
            return node.numer / node.denom
        raise TypeError(
            f"RK4: expected numeric IR node from vm.eval, got {node!r}. "
            f"Check that all free symbols in f_ir are included in state_names."
        )

    # -------------------------------------------------------------------------
    # Main integration loop
    # -------------------------------------------------------------------------

    trajectory: list[tuple[float, list[float]]] = []
    t_cur = float(t_start)
    y_cur = list(y0)

    trajectory.append((t_cur, list(y_cur)))

    while t_cur < t_end - dt * 1e-10:
        # Clamp the last step to land exactly on t_end.
        h = min(dt, t_end - t_cur)

        # ---- Stage k1 --------------------------------------------------------
        k1 = _eval_rhs(t_cur, y_cur)

        # ---- Stage k2 --------------------------------------------------------
        y_mid1 = [y_cur[i] + 0.5 * h * k1[i] for i in range(n)]
        k2 = _eval_rhs(t_cur + 0.5 * h, y_mid1)

        # ---- Stage k3 --------------------------------------------------------
        y_mid2 = [y_cur[i] + 0.5 * h * k2[i] for i in range(n)]
        k3 = _eval_rhs(t_cur + 0.5 * h, y_mid2)

        # ---- Stage k4 --------------------------------------------------------
        y_end_stage = [y_cur[i] + h * k3[i] for i in range(n)]
        k4 = _eval_rhs(t_cur + h, y_end_stage)

        # ---- Combine ---------------------------------------------------------
        # y_next = y + h/6 * (k1 + 2*k2 + 2*k3 + k4)
        y_next = [
            y_cur[i] + (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i])
            for i in range(n)
        ]

        t_cur = t_cur + h
        y_cur = y_next
        trajectory.append((t_cur, list(y_cur)))

    return trajectory
