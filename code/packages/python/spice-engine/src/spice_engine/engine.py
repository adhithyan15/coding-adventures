"""SPICE engine: MNA matrix construction + DC + transient analysis.

Modified Nodal Analysis (MNA) treats node voltages and source-current
"branch unknowns" as one unified vector. For each element, we 'stamp' its
contribution onto the conductance matrix G and the right-hand-side b.

For DC: solve G x = b. For nonlinear elements (Diode, MOSFET), wrap
Newton-Raphson iterations with linearized Jacobians.

For transient: two integration methods are supported:

1. **Backward Euler** (``method="euler"``):
   Simple first-order method.  For a capacitor::

       I_{n+1} = (C/h) * (V_{n+1} - V_n)
       Companion: G_eq = C/h, I_eq = G_eq * V_n  (injected into n+)

2. **Trapezoidal** (``method="trap"``, default):
   Second-order (O(h^2) global error) unconditionally stable method.
   For a capacitor::

       C * (V_{n+1} - V_n)/h = (I_{n+1} + I_n)/2
       Companion: G_eq = 2C/h, I_eq = G_eq * V_n + I_n  (injected into n+)
       Post-step update:  I_{n+1} = G_eq * (V_{n+1} - V_n) - I_n

   Inductors get the dual Norton model::

       Companion: G_eq = h/(2L), I_eq = I_n + G_eq * V_n  (parallel current)
       Post-step update:  I_{n+1} = G_eq * (V_{n+1} - V_n_... ) + I_eq

**Adaptive timestep** (``adaptive=True``):
   After each trapezoidal step the Local Truncation Error (LTE) is
   estimated from the second finite difference of each capacitor voltage::

       lte_C ≈ |V_{n+1} - 2*V_n + V_{n-1}| / 2

   If ``max(lte_C) > tol_lte`` the step is rejected and the stepsize is
   halved (down to ``min_step``).  If ``max(lte_C) < tol_lte/8`` the next
   stepsize is doubled (up to ``max_step``).  The adaptive controller is
   only active when enough history exists (≥ 2 prior cap-voltage samples).
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field

from spice_engine.elements import (
    Capacitor,
    CurrentSource,
    Diode,
    Element,
    Inductor,
    Mosfet,
    Resistor,
    VoltageSource,
)


@dataclass
class Circuit:
    elements: list[Element] = field(default_factory=list)

    def add(self, element: Element) -> None:
        self.elements.append(element)


@dataclass
class DcResult:
    """Operating-point voltages by node + extra branch currents."""

    node_voltages: dict[str, float]
    branch_currents: dict[str, float]
    iterations: int
    converged: bool


@dataclass
class TransientPoint:
    time: float
    node_voltages: dict[str, float]


@dataclass
class TransientResult:
    """Waveform output from :func:`transient`.

    Attributes
    ----------
    points:
        One :class:`TransientPoint` per accepted timestep.
    converged:
        ``False`` if any DC solve diverged (integration stopped early).
    method:
        Integration method that was used: ``"trap"`` or ``"euler"``.
    steps_rejected:
        Number of timesteps rejected by the LTE adaptive controller.
        Always 0 when ``adaptive=False``.
    """

    points: list[TransientPoint]
    converged: bool
    method: str = "trap"
    steps_rejected: int = 0


# ---------------------------------------------------------------------------
# MNA infrastructure
# ---------------------------------------------------------------------------


def _node_index(circuit: Circuit) -> tuple[dict[str, int], list[str]]:
    """Build a node->index map. Ground node ('0' or 'gnd') is excluded
    (it's the reference node, always at 0 V)."""
    nodes: list[str] = []
    seen: set[str] = set()
    for el in circuit.elements:
        for n in _element_nodes(el):
            if n in ("0", "gnd", "GND"):
                continue
            if n not in seen:
                seen.add(n)
                nodes.append(n)
    return ({n: i for i, n in enumerate(nodes)}, nodes)


def _element_nodes(el: Element) -> list[str]:
    """All nodes touched by an element."""
    if isinstance(el, (Resistor, Capacitor, Inductor, VoltageSource, CurrentSource)):
        return [el.n_plus, el.n_minus]
    if isinstance(el, Diode):
        return [el.anode, el.cathode]
    if isinstance(el, Mosfet):
        return [el.drain, el.gate, el.source, el.body]
    return []


def _voltage_sources(circuit: Circuit) -> list[VoltageSource]:
    return [el for el in circuit.elements if isinstance(el, VoltageSource)]


def _is_ground(name: str) -> bool:
    return name in ("0", "gnd", "GND")


# ---------------------------------------------------------------------------
# DC analysis
# ---------------------------------------------------------------------------


def dc_op(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> DcResult:
    """Solve DC operating point via Newton-Raphson on a linearized MNA."""
    node_to_idx, nodes = _node_index(circuit)
    vsrcs = _voltage_sources(circuit)
    n = len(nodes)
    m = len(vsrcs)
    size = n + m

    # Initial guess: all zeros
    x = [0.0] * size

    for it in range(max_iterations):
        # Stamp linearized contributions at the current x.
        G = [[0.0] * size for _ in range(size)]
        b = [0.0] * size

        for el in circuit.elements:
            _stamp_dc(el, G, b, x, node_to_idx, vsrcs)

        # Solve G x_new = b via Gaussian elimination.
        try:
            x_new = _solve(G, b)
        except ZeroDivisionError:
            return DcResult({n: x[i] for n, i in node_to_idx.items()},
                            {}, iterations=it, converged=False)

        # Check convergence
        max_delta = max(abs(a - b) for a, b in zip(x, x_new, strict=False)) if x else 0.0
        x = x_new
        if max_delta < tol:
            break

    node_v = {n: x[i] for n, i in node_to_idx.items()}
    branch_i = {f"I({vs.name})": x[n + i] for i, vs in enumerate(vsrcs)}
    return DcResult(node_v, branch_i, iterations=it + 1, converged=max_delta < tol)


def _stamp_dc(
    el: Element,
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    vsrcs: list[VoltageSource],
) -> None:
    """Stamp one element's MNA contribution at the current operating point."""
    if isinstance(el, Resistor):
        _stamp_g(G, node_to_idx, el.n_plus, el.n_minus, 1.0 / el.resistance)
    elif isinstance(el, VoltageSource):
        i = vsrcs.index(el)
        _stamp_vsrc(G, b, node_to_idx, el, len(node_to_idx) + i)
    elif isinstance(el, CurrentSource):
        if not _is_ground(el.n_plus):
            b[node_to_idx[el.n_plus]] -= el.current
        if not _is_ground(el.n_minus):
            b[node_to_idx[el.n_minus]] += el.current
    elif isinstance(el, Diode):
        _stamp_diode(G, b, x, node_to_idx, el)
    elif isinstance(el, Mosfet):
        _stamp_mosfet(G, b, x, node_to_idx, el)
    elif isinstance(el, Capacitor):
        # In DC, capacitors are open circuits — no conductance contribution
        pass
    elif isinstance(el, Inductor):
        # In DC, inductors are short circuits — model as a 0V source
        pass


def _stamp_g(
    G: list[list[float]],
    node_to_idx: dict[str, int],
    n_plus: str,
    n_minus: str,
    g: float,
) -> None:
    """Stamp a conductance g between two nodes (resistor, linearized device)."""
    if not _is_ground(n_plus):
        G[node_to_idx[n_plus]][node_to_idx[n_plus]] += g
    if not _is_ground(n_minus):
        G[node_to_idx[n_minus]][node_to_idx[n_minus]] += g
    if not _is_ground(n_plus) and not _is_ground(n_minus):
        G[node_to_idx[n_plus]][node_to_idx[n_minus]] -= g
        G[node_to_idx[n_minus]][node_to_idx[n_plus]] -= g


def _stamp_vsrc(
    G: list[list[float]],
    b: list[float],
    node_to_idx: dict[str, int],
    el: VoltageSource,
    branch_idx: int,
) -> None:
    if not _is_ground(el.n_plus):
        i = node_to_idx[el.n_plus]
        G[i][branch_idx] = 1.0
        G[branch_idx][i] = 1.0
    if not _is_ground(el.n_minus):
        j = node_to_idx[el.n_minus]
        G[j][branch_idx] = -1.0
        G[branch_idx][j] = -1.0
    b[branch_idx] = el.voltage


def _stamp_diode(
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    el: Diode,
) -> None:
    """Linearized diode: I = Is*(exp(Vd/Vt) - 1).

    Newton: I0 = Is*(exp(Vd0/Vt) - 1), gd = (Is/Vt)*exp(Vd0/Vt).
    Stamp gd as conductance + (gd*Vd0 - I0) as current source from cathode."""
    Va = 0.0 if _is_ground(el.anode) else x[node_to_idx[el.anode]]
    Vk = 0.0 if _is_ground(el.cathode) else x[node_to_idx[el.cathode]]
    Vd = Va - Vk
    # Clamp to avoid exp overflow
    Vd = min(Vd, 0.7)
    exp_term = math.exp(Vd / el.Vt)
    I0 = el.Is * (exp_term - 1.0)
    gd = (el.Is / el.Vt) * exp_term

    _stamp_g(G, node_to_idx, el.anode, el.cathode, gd)
    Ieq = I0 - gd * Vd
    if not _is_ground(el.anode):
        b[node_to_idx[el.anode]] -= Ieq
    if not _is_ground(el.cathode):
        b[node_to_idx[el.cathode]] += Ieq


def _stamp_mosfet(
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    el: Mosfet,
) -> None:
    """Linearized MOSFET via mosfet_models.MOSFET.dc()."""
    Vd = 0.0 if _is_ground(el.drain) else x[node_to_idx[el.drain]]
    Vg = 0.0 if _is_ground(el.gate) else x[node_to_idx[el.gate]]
    Vs = 0.0 if _is_ground(el.source) else x[node_to_idx[el.source]]
    Vb = 0.0 if _is_ground(el.body) else x[node_to_idx[el.body]]

    V_GS = Vg - Vs
    V_DS = Vd - Vs
    V_BS = Vb - Vs

    # Call the MOSFET model
    r = el.model.dc(V_GS, V_DS, V_BS)  # type: ignore[attr-defined]
    Id = r.Id
    gm = r.gm
    gds = r.gds

    # Stamp gds (drain-source conductance) + Id companion source.
    _stamp_g(G, node_to_idx, el.drain, el.source, gds)
    # Stamp gm (transconductance: drain-current per V_GS).
    if not _is_ground(el.drain):
        d = node_to_idx[el.drain]
        if not _is_ground(el.gate):
            G[d][node_to_idx[el.gate]] += gm
        if not _is_ground(el.source):
            G[d][node_to_idx[el.source]] -= gm
    if not _is_ground(el.source):
        s = node_to_idx[el.source]
        if not _is_ground(el.gate):
            G[s][node_to_idx[el.gate]] -= gm
        if not _is_ground(el.source):
            G[s][node_to_idx[el.source]] += gm
    # Companion current source for Id at this operating point
    Ieq = Id - gm * V_GS - gds * V_DS
    if not _is_ground(el.drain):
        b[node_to_idx[el.drain]] -= Ieq
    if not _is_ground(el.source):
        b[node_to_idx[el.source]] += Ieq


# ---------------------------------------------------------------------------
# Linear solver
# ---------------------------------------------------------------------------


def _solve(A: list[list[float]], b: list[float]) -> list[float]:
    """Gaussian elimination with partial pivoting. Returns x s.t. A x = b."""
    n = len(A)
    if n == 0:
        return []
    # Augmented matrix
    aug = [row[:] + [b[i]] for i, row in enumerate(A)]

    for i in range(n):
        # Partial pivot: find max abs element in column i below diagonal
        pivot = i
        for r in range(i + 1, n):
            if abs(aug[r][i]) > abs(aug[pivot][i]):
                pivot = r
        if abs(aug[pivot][i]) < 1e-15:
            raise ZeroDivisionError(f"singular matrix at row {i}")
        aug[i], aug[pivot] = aug[pivot], aug[i]

        # Eliminate column i below row i
        for r in range(i + 1, n):
            factor = aug[r][i] / aug[i][i]
            for c in range(i, n + 1):
                aug[r][c] -= factor * aug[i][c]

    # Back-substitution
    x = [0.0] * n
    for i in range(n - 1, -1, -1):
        s = aug[i][n]
        for c in range(i + 1, n):
            s -= aug[i][c] * x[c]
        x[i] = s / aug[i][i]
    return x


# ---------------------------------------------------------------------------
# Transient analysis — companion-model builders and helpers
# ---------------------------------------------------------------------------


def _node_voltage(name: str, node_voltages: dict[str, float]) -> float:
    """Return the solved node voltage, 0 V for any ground alias."""
    return 0.0 if _is_ground(name) else node_voltages.get(name, 0.0)


def _build_transient_companions(
    circuit: Circuit,
    h: float,
    method: str,
    cap_voltages: dict[str, float],
    cap_currents: dict[str, float],
    ind_currents: dict[str, float],
    ind_voltages: dict[str, float],
) -> Circuit:
    """Build the linearised companion circuit for one timestep.

    Replaces each capacitor and inductor with their Norton companion models.
    All other elements pass through unchanged.

    Capacitor companion (backward Euler, method="euler")
    ----------------------------------------------------
    Given: dV_C/dt ≈ (V_{n+1} - V_n) / h = I_{n+1} / C

    ::

        G_eq = C/h
        I_eq = G_eq * V_n         (injected into n+)

    Capacitor companion (trapezoidal, method="trap")
    ------------------------------------------------
    Given: C*(V_{n+1}-V_n)/h = (I_{n+1}+I_n)/2, so
    I_{n+1} = G_eq*(V_{n+1}-V_n) - I_n  with G_eq = 2C/h.

    ::

        G_eq = 2C/h
        I_eq = G_eq * V_n + I_n   (injected into n+)

    In both cases a resistor ``1/G_eq`` is stamped between n+ and n-, and a
    current source ``I_eq`` is inserted flowing from cap.n_minus to cap.n_plus
    (i.e. injecting current INTO the positive terminal).

    Inductor companion (trapezoidal, method="trap")
    -----------------------------------------------
    Dual of the capacitor: L*(I_{n+1}-I_n)/h = (V_{n+1}+V_n)/2.

    Norton equivalent with G_eq = h/(2L):
    I_{n+1} = G_eq*V_{n+1} + (I_n + G_eq*V_n)

    ::

        G_eq = h/(2L)
        I_eq = I_n + G_eq * V_n   (parallel current source from n+ to n-)

    Inductor companion (backward Euler, method="euler")
    ---------------------------------------------------
    ::

        G_eq = h/L
        I_eq = I_n                (parallel current source from n+ to n-)
    """
    aug = Circuit(elements=[
        e for e in circuit.elements
        if not isinstance(e, (Capacitor, Inductor))
    ])

    for el in circuit.elements:
        # ---- Capacitor companion ------------------------------------------
        if isinstance(el, Capacitor):
            v_prev = cap_voltages.get(el.name, el.initial_voltage)
            if method == "trap":
                g_eq = 2.0 * el.capacitance / h
                I_eq = g_eq * v_prev + cap_currents.get(el.name, 0.0)
            else:  # backward Euler
                g_eq = el.capacitance / h
                I_eq = g_eq * v_prev

            # Stamp resistor 1/g_eq between n+ and n-.
            aug.elements.append(Resistor(
                name=f"_C_{el.name}_R",
                n_plus=el.n_plus,
                n_minus=el.n_minus,
                resistance=1.0 / g_eq,
            ))
            # Inject history current I_eq INTO cap.n_plus.
            # CurrentSource(n_plus=A, n_minus=B) means current flows A→B,
            # so node B receives current.  Setting n_minus=cap.n_plus makes
            # I_eq enter the positive terminal.
            aug.elements.append(CurrentSource(
                name=f"_C_{el.name}_I",
                n_plus=el.n_minus,
                n_minus=el.n_plus,
                current=I_eq,
            ))

        # ---- Inductor companion ------------------------------------------
        elif isinstance(el, Inductor):
            I_prev = ind_currents.get(el.name, 0.0)
            if method == "trap":
                g_eq = h / (2.0 * el.inductance)
                V_prev = ind_voltages.get(el.name, 0.0)
                I_eq = I_prev + g_eq * V_prev
            else:  # backward Euler
                g_eq = h / el.inductance
                I_eq = I_prev

            # Stamp resistor 1/g_eq between n+ and n-.
            aug.elements.append(Resistor(
                name=f"_L_{el.name}_R",
                n_plus=el.n_plus,
                n_minus=el.n_minus,
                resistance=1.0 / g_eq,
            ))
            # Parallel Norton current source I_eq flowing from n+ to n-.
            aug.elements.append(CurrentSource(
                name=f"_L_{el.name}_I",
                n_plus=el.n_plus,
                n_minus=el.n_minus,
                current=I_eq,
            ))

    return aug


def _update_reactive_state(
    circuit: Circuit,
    h: float,
    method: str,
    op: DcResult,
    cap_voltages: dict[str, float],
    cap_currents: dict[str, float],
    ind_currents: dict[str, float],
    ind_voltages: dict[str, float],
) -> None:
    """Update capacitor and inductor history in-place after a successful step.

    Capacitor voltage update (both methods):
        V_{n+1} = V_{n+,new} - V_{n-,new}

    Capacitor current update (trapezoidal):
        I_{n+1} = G_eq * (V_{n+1} - V_n) - I_n   where G_eq = 2C/h

    Capacitor current update (backward Euler):
        I_{n+1} = G_eq * (V_{n+1} - V_n)          where G_eq = C/h

    Inductor current update (trapezoidal):
        I_{n+1} = G_eq * (V_{n+1,+} - V_{n+1,-}) + I_eq
        where I_eq = I_n + G_eq * V_n (same value used in the companion build)

    Inductor voltage update (trapezoidal):
        V_{n+1} = V_{n+1,+} - V_{n+1,-}   (for use in the next companion build)
    """
    for el in circuit.elements:
        if isinstance(el, Capacitor):
            v_plus = _node_voltage(el.n_plus, op.node_voltages)
            v_minus = _node_voltage(el.n_minus, op.node_voltages)
            v_new = v_plus - v_minus
            v_prev = cap_voltages.get(el.name, el.initial_voltage)

            if method == "trap":
                g_eq = 2.0 * el.capacitance / h
                I_prev = cap_currents.get(el.name, 0.0)
                cap_currents[el.name] = g_eq * (v_new - v_prev) - I_prev
            else:
                g_eq = el.capacitance / h
                cap_currents[el.name] = g_eq * (v_new - v_prev)

            cap_voltages[el.name] = v_new

        elif isinstance(el, Inductor):
            v_plus = _node_voltage(el.n_plus, op.node_voltages)
            v_minus = _node_voltage(el.n_minus, op.node_voltages)
            v_new = v_plus - v_minus
            I_prev = ind_currents.get(el.name, 0.0)
            V_prev = ind_voltages.get(el.name, 0.0)

            if method == "trap":
                g_eq = h / (2.0 * el.inductance)
                I_eq = I_prev + g_eq * V_prev
                ind_currents[el.name] = g_eq * v_new + I_eq
            else:
                g_eq = h / el.inductance
                ind_currents[el.name] = g_eq * v_new + I_prev

            ind_voltages[el.name] = v_new


def _lte_estimate(
    circuit: Circuit,
    cap_voltages_now: dict[str, float],
    cap_voltages_prev: dict[str, float],
    cap_voltages_prev2: dict[str, float],
) -> float:
    """Estimate the Local Truncation Error (LTE) after a trapezoidal step.

    The trapezoidal method has local truncation error O(h^3).  A practical
    per-step estimate is the magnitude of the second finite difference of
    each capacitor voltage, normalised by 2::

        lte_C ≈ |V_{n+1} - 2*V_n + V_{n-1}| / 2

    This is the leading-order coefficient of the h^2 Taylor remainder.
    Returns the maximum LTE across all capacitors (0.0 if none exist).

    Why capacitors?  In an RLC circuit the capacitor voltage is the primary
    state variable.  Its second difference captures the curvature of the
    waveform, which governs how much the linear interpolation in the
    trapezoidal quadrature deviates from the true curve.
    """
    max_lte = 0.0
    for el in circuit.elements:
        if isinstance(el, Capacitor):
            v1 = cap_voltages_now.get(el.name, 0.0)
            v0 = cap_voltages_prev.get(el.name, el.initial_voltage)
            vm1 = cap_voltages_prev2.get(el.name, el.initial_voltage)
            lte_c = abs(v1 - 2.0 * v0 + vm1) / 2.0
            if lte_c > max_lte:
                max_lte = lte_c
    return max_lte


# ---------------------------------------------------------------------------
# Transient analysis — public entry point
# ---------------------------------------------------------------------------


def transient(
    circuit: Circuit,
    *,
    t_stop: float,
    t_step: float,
    method: str = "trap",
    adaptive: bool = False,
    tol_lte: float = 1e-4,
    min_step: float | None = None,
    max_step: float | None = None,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> TransientResult:
    """Transient (time-domain) analysis with trapezoidal or backward-Euler integration.

    Replaces each reactive element (capacitor, inductor) with its Norton
    companion model at every timestep and solves the resulting DC problem
    via Newton-Raphson MNA.

    Parameters
    ----------
    circuit:
        The circuit to simulate.
    t_stop:
        End time (seconds).  Must be > 0.
    t_step:
        Initial (or fixed) timestep (seconds).  Must be > 0.
    method:
        ``"trap"`` (trapezoidal, default — 2nd-order accurate) or
        ``"euler"`` (backward Euler — 1st-order, unconditionally stable).
    adaptive:
        When ``True``, enable LTE-based adaptive timestepping.  Only
        meaningful with ``method="trap"``.
    tol_lte:
        LTE tolerance for the adaptive controller.  A step is rejected when
        the estimated LTE exceeds this threshold.
    min_step:
        Minimum allowed timestep (adaptive mode).  Defaults to
        ``t_step / 1000``.
    max_step:
        Maximum allowed timestep (adaptive mode).  Defaults to ``t_step * 10``.
    max_iterations:
        Maximum Newton-Raphson iterations per DC solve.
    tol:
        Convergence tolerance for DC solves.

    Returns
    -------
    TransientResult
        ``converged=True`` when all DC solves converged.  ``points`` contains
        one entry per accepted timestep (including t=0).
        ``steps_rejected`` is non-zero only when ``adaptive=True``.

    Notes
    -----
    Inductor handling: in v0.1.0 inductors were no-ops in transient.  This
    release models them with a proper Norton companion (G_eq = h/(2L) for
    trapezoidal, G_eq = h/L for backward Euler) so inductor currents now
    accumulate correctly across time.
    """
    if t_step <= 0 or t_stop <= 0:
        return TransientResult(points=[], converged=False, method=method)

    _min_step = min_step if min_step is not None else t_step / 1000.0
    _max_step = max_step if max_step is not None else t_step * 10.0

    # ---- t = 0: solve initial conditions -----------------------------------
    # Replace each capacitor with a voltage source at its initial voltage so
    # that the rest of the circuit settles consistently.
    init_circuit = Circuit(elements=[
        e for e in circuit.elements
        if not isinstance(e, (Capacitor, Inductor))
    ])
    for el in circuit.elements:
        if isinstance(el, Capacitor):
            init_circuit.add(VoltageSource(
                name=f"_C_{el.name}_V0",
                n_plus=el.n_plus,
                n_minus=el.n_minus,
                voltage=el.initial_voltage,
            ))
        # Inductors at t=0: use a backward-Euler companion resistor R = L/h so
        # the DC OP reflects near-zero inductor current (L blocks current) and
        # the full source voltage appears across the inductor.  A 0 V source
        # forces I_L(0) = V/R (steady-state value) which is physically wrong.
        elif isinstance(el, Inductor):
            r_init = el.inductance / t_step  # BE: G_eq = h/L
            init_circuit.add(Resistor(
                name=f"_L_{el.name}_R0",
                n_plus=el.n_plus,
                n_minus=el.n_minus,
                resistance=r_init,
            ))
    op = dc_op(init_circuit, max_iterations=max_iterations, tol=tol)
    if not op.converged:
        return TransientResult(points=[], converged=False, method=method)

    points: list[TransientPoint] = [
        TransientPoint(time=0.0, node_voltages=dict(op.node_voltages))
    ]

    # ---- Reactive element history ------------------------------------------
    cap_voltages: dict[str, float] = {
        el.name: el.initial_voltage
        for el in circuit.elements if isinstance(el, Capacitor)
    }
    cap_currents: dict[str, float] = {
        el.name: 0.0
        for el in circuit.elements if isinstance(el, Capacitor)
    }
    ind_currents: dict[str, float] = {
        el.name: 0.0
        for el in circuit.elements if isinstance(el, Inductor)
    }
    ind_voltages: dict[str, float] = {
        el.name: 0.0
        for el in circuit.elements if isinstance(el, Inductor)
    }

    # ---- Seed history from the t=0 DC solve ----------------------------------
    # Capacitor: the initial charging current I_C(0) is the branch current of
    # the substitute voltage source.  Without this seed, the trapezoidal method
    # starts with I_n = 0 which introduces a large O(h) error at the first step.
    for _el in circuit.elements:
        if isinstance(_el, Capacitor):
            _key = f"I(_C_{_el.name}_V0)"
            if _key in op.branch_currents:
                cap_currents[_el.name] = op.branch_currents[_key]

    # Inductor: seed V_L(0) from the node voltages of the BE-companion init
    # solve so that the first trapezoidal step has the correct history voltage.
    for _el in circuit.elements:
        if isinstance(_el, Inductor):
            _vp = _node_voltage(_el.n_plus, op.node_voltages)
            _vm = _node_voltage(_el.n_minus, op.node_voltages)
            ind_voltages[_el.name] = _vp - _vm

    # Two-step history for LTE estimation (adaptive mode)
    cap_voltages_prev: dict[str, float] = dict(cap_voltages)   # V_{n-1}
    steps_rejected = 0

    # ---- Main time loop ----------------------------------------------------
    t = t_step
    h = t_step  # current step size
    while t <= t_stop + 1e-12 * t_stop:
        # Clamp last step to land exactly on t_stop.
        h = min(h, t_stop - (t - h) + 1e-12 * t_stop)
        if h < _min_step:
            h = _min_step  # floor; stop shrinking

        aug = _build_transient_companions(
            circuit, h, method,
            cap_voltages, cap_currents,
            ind_currents, ind_voltages,
        )
        op = dc_op(aug, max_iterations=max_iterations, tol=tol)
        if not op.converged:
            return TransientResult(points=points, converged=False,
                                   method=method, steps_rejected=steps_rejected)

        # ---- LTE estimate and adaptive control ----------------------------
        if adaptive and method == "trap" and len(points) >= 2:
            # Compute cap voltages at the proposed new time point
            cap_voltages_new: dict[str, float] = {}
            for el in circuit.elements:
                if isinstance(el, Capacitor):
                    v_plus = _node_voltage(el.n_plus, op.node_voltages)
                    v_minus = _node_voltage(el.n_minus, op.node_voltages)
                    cap_voltages_new[el.name] = v_plus - v_minus

            lte = _lte_estimate(circuit, cap_voltages_new,
                                 cap_voltages, cap_voltages_prev)

            if lte > tol_lte and h > _min_step + 1e-20:
                # Reject: halve step size and retry (without advancing t).
                h = max(h / 2.0, _min_step)
                steps_rejected += 1
                continue

            # Accept step; consider doubling h for the next step.
            t_cur = t
            t_actual = t  # the time we are committing to
            _update_reactive_state(
                circuit, h, method, op,
                cap_voltages, cap_currents, ind_currents, ind_voltages,
            )
            cap_voltages_prev = dict(cap_voltages)
            points.append(TransientPoint(time=t_actual,
                                         node_voltages=dict(op.node_voltages)))

            if lte < tol_lte / 8.0:
                h = min(h * 2.0, _max_step)
        else:
            # Non-adaptive or backward Euler or not enough history yet.
            _update_reactive_state(
                circuit, h, method, op,
                cap_voltages, cap_currents, ind_currents, ind_voltages,
            )
            cap_voltages_prev = dict(cap_voltages)
            points.append(TransientPoint(time=t,
                                         node_voltages=dict(op.node_voltages)))

        t += h

    return TransientResult(points=points, converged=True,
                           method=method, steps_rejected=steps_rejected)
