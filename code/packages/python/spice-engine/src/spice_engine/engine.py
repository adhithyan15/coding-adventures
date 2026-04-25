"""SPICE engine: MNA matrix construction + DC + transient analysis.

Modified Nodal Analysis (MNA) treats node voltages and source-current
"branch unknowns" as one unified vector. For each element, we 'stamp' its
contribution onto the conductance matrix G and the right-hand-side b.

For DC: solve G x = b. For nonlinear elements (Diode, MOSFET), wrap
Newton-Raphson iterations with linearized Jacobians.

For transient: forward Euler with capacitor companion model
(C/h conductance + history source).
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
    points: list[TransientPoint]
    converged: bool


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
# Transient analysis
# ---------------------------------------------------------------------------


def transient(
    circuit: Circuit,
    *,
    t_stop: float,
    t_step: float,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> TransientResult:
    """Forward-Euler transient with capacitor companion models.

    For each timestep t_n -> t_n+1:
      - Replace each capacitor with conductance C/h in parallel with current
        source I_eq = (C/h) * V(t_n).
      - Solve DC at the new step.
      - Save state for next step.
    """
    if t_step <= 0 or t_stop <= 0:
        return TransientResult(points=[], converged=False)

    points: list[TransientPoint] = []

    # Initialize cap voltages from each cap's initial_voltage.
    cap_voltages: dict[str, float] = {
        el.name: el.initial_voltage
        for el in circuit.elements
        if isinstance(el, Capacitor)
    }

    # Compute t=0 node voltages: replace each cap with a VoltageSource at its
    # initial_voltage. Other nodes settle to be consistent with this.
    init_circuit = Circuit(elements=[
        e for e in circuit.elements if not isinstance(e, Capacitor)
    ])
    for el in circuit.elements:
        if isinstance(el, Capacitor):
            init_circuit.add(VoltageSource(
                name=f"_C_{el.name}_V0", n_plus=el.n_plus, n_minus=el.n_minus,
                voltage=el.initial_voltage,
            ))
    op = dc_op(init_circuit, max_iterations=max_iterations, tol=tol)
    if not op.converged:
        return TransientResult(points=[], converged=False)
    points.append(TransientPoint(time=0.0, node_voltages=dict(op.node_voltages)))

    t = t_step
    while t <= t_stop + 1e-12:
        # Build a "transient circuit": replace caps with their backward-Euler
        # companions. The companion is g = C/h (conductance between n+ and n-)
        # in parallel with a current source pumping g*V_C(t_n) FROM n- TO n+
        # (i.e., reversed sign w.r.t. cap.n_plus->n_minus convention).
        aug = Circuit(elements=[
            e for e in circuit.elements if not isinstance(e, Capacitor)
        ])
        for el in circuit.elements:
            if isinstance(el, Capacitor):
                g_eq = el.capacitance / t_step
                v_prev = cap_voltages.get(el.name, el.initial_voltage)
                I_eq = g_eq * v_prev
                aug.elements.append(Resistor(
                    name=f"_C_{el.name}_R", n_plus=el.n_plus, n_minus=el.n_minus,
                    resistance=1.0 / g_eq,
                ))
                # Current source: positive flows from n_minus to n_plus, so we
                # set source.n_plus = cap.n_minus and source.n_minus = cap.n_plus
                # with current = I_eq.
                aug.elements.append(CurrentSource(
                    name=f"_C_{el.name}_I", n_plus=el.n_minus, n_minus=el.n_plus,
                    current=I_eq,
                ))

        op = dc_op(aug, max_iterations=max_iterations, tol=tol)
        if not op.converged:
            return TransientResult(points=points, converged=False)
        points.append(TransientPoint(time=t, node_voltages=dict(op.node_voltages)))

        # Update cap voltages for next step
        for el in circuit.elements:
            if isinstance(el, Capacitor):
                v_plus = (
                    0.0 if _is_ground(el.n_plus) else op.node_voltages.get(el.n_plus, 0.0)
                )
                v_minus = (
                    0.0 if _is_ground(el.n_minus) else op.node_voltages.get(el.n_minus, 0.0)
                )
                cap_voltages[el.name] = v_plus - v_minus

        t += t_step

    return TransientResult(points=points, converged=True)
