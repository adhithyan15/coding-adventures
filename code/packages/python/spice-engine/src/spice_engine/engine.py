"""SPICE engine: MNA matrix construction + DC + transient + AC analysis.

Modified Nodal Analysis (MNA) treats node voltages and source-current
"branch unknowns" as one unified vector. For each element, we 'stamp' its
contribution onto the conductance matrix G and the right-hand-side b.

For DC: solve G x = b. For nonlinear elements (Diode, MOSFET, BJT), wrap
Newton-Raphson iterations with linearized Jacobians.

For AC: linearise each element around the DC operating point; replace
reactive elements with complex admittances (Y_C = jωC, Y_L = 1/jωL);
solve the resulting complex linear system at each frequency.  See
:func:`ac_sweep` and the Section 3 comment block below.

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

import cmath
import math
from dataclasses import dataclass, field

from spice_engine.elements import (
    BJT,
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


@dataclass
class AcPoint:
    """Phasor voltages at a single frequency point.

    Attributes
    ----------
    freq : float
        Frequency in hertz.
    node_voltages : dict[str, complex]
        Complex phasor voltage at each node.  Extract magnitude with
        ``abs(v)`` and phase (in radians) with ``cmath.phase(v)``.

    Examples
    --------
    Compute the dB gain at node "out" relative to a 1 V source::

        pt = ac_result.points[10]
        gain_db = 20 * math.log10(abs(pt.node_voltages["out"]))
        phase_deg = math.degrees(cmath.phase(pt.node_voltages["out"]))
    """

    freq: float
    node_voltages: dict[str, complex]


@dataclass
class AcResult:
    """Frequency-sweep results from :func:`ac_sweep`.

    Attributes
    ----------
    points : list[AcPoint]
        One :class:`AcPoint` per frequency, in ascending order.
        Empty when ``n_points < 1``.
    """

    points: list[AcPoint]


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
    if isinstance(el, BJT):
        return [el.collector, el.base, el.emitter]
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
    elif isinstance(el, BJT):
        _stamp_bjt(G, b, x, node_to_idx, el)
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


def _stamp_bjt(
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    el: BJT,
) -> None:
    """Linearized BJT using a simplified Ebers-Moll (forward-active) model.

    Simplified Ebers-Moll (forward-active only)
    -------------------------------------------
    The full Ebers-Moll model has both forward- and reverse-saturation currents.
    For the forward-active region (the dominant operating mode of a BJT amplifier
    or switch) the collector current is well approximated by::

        Ic = Is * (exp(Vjunc / Vt) - 1)

    where Vjunc is the controlling junction voltage (Vbe for NPN, Veb for PNP).
    The base current follows from the current gain: Ib = Ic / beta_f.

    Newton linearisation
    --------------------
    At operating point voltage Vjunc0 (clamped to 0.7 V to prevent exp overflow)::

        exp_term = exp(Vjunc0 / Vt)
        Ic0      = Is * (exp_term - 1)          # collector current at OP
        gm       = (Is / Vt) * exp_term          # transconductance dIc/dVjunc
        gπ       = gm / beta_f                   # junction conductance dIb/dVjunc
        Ib0      = Ic0 / beta_f                  # base current at OP

    The linearised device model has two stamping components:

    1. **Junction conductance gπ** (models the B-E diode resistance):
       Stamped as a conductance between the junction terminals:
       - NPN: between B and E  (controls base current)
       - PNP: between E and B  (same, but polarity-flipped circuit)

       Norton companion for the junction:
           Ieq_junc = Ib0 - gπ * Vjunc0

    2. **Voltage-controlled current source (VCCS) for gm** (transconductance):
       Ic = gm * Vjunc, controlled by the junction voltage.
       The VCCS has its *control* nodes on the junction pair and its
       *output* nodes on the collector-emitter pair.

       For NPN (Vjunc = Vb - Ve, current flows into C):
           G[C][B] += gm   (drain: collector, control+: base)
           G[C][E] -= gm   (drain: collector, control-: emitter)
           G[E][B] -= gm   (source: emitter, control+: base — KCL)
           G[E][E] += gm   (source: emitter, control-: emitter — KCL)
           b[C]    -= Ieq_c   (Norton offset, Ieq_c = Ic0 - gm*Vjunc0)
           b[E]    += Ieq_c

       For PNP (Vjunc = Ve - Vb, current flows out of C, i.e. leaves E):
           G[E][E] += gm   (drain-side: emitter plays C role for PNP)
           G[E][B] -= gm
           G[C][E] -= gm
           G[C][B] += gm
           b[E]    -= Ieq_c
           b[C]    += Ieq_c

    Why the sign inversion for PNP?  In a PNP the emitter is the injecting
    terminal (analogous to the NPN collector) and current flows from emitter
    to collector in the conventional direction.  Swapping C↔E and negating
    the control voltage (Ve - Vb vs Vb - Ve) yields the correct KCL stamps.
    """
    # --- Resolve node voltages at the current Newton iterate -----------------
    Vb = 0.0 if _is_ground(el.base) else x[node_to_idx[el.base]]
    Ve = 0.0 if _is_ground(el.emitter) else x[node_to_idx[el.emitter]]

    # --- Controlling junction voltage (clamped to avoid exp overflow) --------
    Vjunc = min(Vb - Ve, 0.7) if el.polarity == "NPN" else min(Ve - Vb, 0.7)

    exp_term = math.exp(Vjunc / el.Vt)
    Ic0 = el.Is * (exp_term - 1.0)
    gm = (el.Is / el.Vt) * exp_term
    g_pi = gm / el.beta_f
    Ib0 = Ic0 / el.beta_f

    Ieq_junc = Ib0 - g_pi * Vjunc      # junction Norton offset
    Ieq_coll = Ic0 - gm * Vjunc        # VCCS Norton offset

    if el.polarity == "NPN":
        # --- Junction stamp: gπ between B and E ------------------------------
        _stamp_g(G, node_to_idx, el.base, el.emitter, g_pi)
        if not _is_ground(el.base):
            b[node_to_idx[el.base]] -= Ieq_junc
        if not _is_ground(el.emitter):
            b[node_to_idx[el.emitter]] += Ieq_junc

        # --- VCCS stamp: gm * (Vb - Ve) drives Ic into C, out of E ----------
        if not _is_ground(el.collector):
            c_idx = node_to_idx[el.collector]
            if not _is_ground(el.base):
                G[c_idx][node_to_idx[el.base]] += gm
            if not _is_ground(el.emitter):
                G[c_idx][node_to_idx[el.emitter]] -= gm
        if not _is_ground(el.emitter):
            e_idx = node_to_idx[el.emitter]
            if not _is_ground(el.base):
                G[e_idx][node_to_idx[el.base]] -= gm
            if not _is_ground(el.emitter):
                G[e_idx][node_to_idx[el.emitter]] += gm
        # Norton companion for collector current
        if not _is_ground(el.collector):
            b[node_to_idx[el.collector]] -= Ieq_coll
        if not _is_ground(el.emitter):
            b[node_to_idx[el.emitter]] += Ieq_coll

    else:
        # PNP: Vjunc = Ve - Vb; emitter injects, collector collects.
        # --- Junction stamp: gπ between E and B ------------------------------
        _stamp_g(G, node_to_idx, el.emitter, el.base, g_pi)
        if not _is_ground(el.emitter):
            b[node_to_idx[el.emitter]] -= Ieq_junc
        if not _is_ground(el.base):
            b[node_to_idx[el.base]] += Ieq_junc

        # --- VCCS stamp: gm * (Ve - Vb) drives Ic out of E, into C ----------
        if not _is_ground(el.emitter):
            e_idx = node_to_idx[el.emitter]
            if not _is_ground(el.emitter):
                G[e_idx][node_to_idx[el.emitter]] += gm
            if not _is_ground(el.base):
                G[e_idx][node_to_idx[el.base]] -= gm
        if not _is_ground(el.collector):
            c_idx = node_to_idx[el.collector]
            if not _is_ground(el.emitter):
                G[c_idx][node_to_idx[el.emitter]] -= gm
            if not _is_ground(el.base):
                G[c_idx][node_to_idx[el.base]] += gm
        # Norton companion for collector current (enters C, leaves E)
        if not _is_ground(el.emitter):
            b[node_to_idx[el.emitter]] -= Ieq_coll
        if not _is_ground(el.collector):
            b[node_to_idx[el.collector]] += Ieq_coll


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


# ---------------------------------------------------------------------------
# Section 3 — AC small-signal analysis
# ---------------------------------------------------------------------------
#
# Background: what is AC analysis?
# ---------------------------------
# In a SPICE `.AC` sweep the simulator:
#
#  1. Finds the DC operating point (bias voltages) for all nonlinear devices.
#  2. Replaces each element with its small-signal equivalent:
#       - Resistor R → conductance G = 1/R  (real, frequency-independent)
#       - Capacitor C → admittance Y_C = jωC  (grows with frequency)
#       - Inductor L → admittance Y_L = 1/(jωL)  (shrinks with frequency)
#       - Diode, MOSFET, BJT → linearised transconductance / conductance at OP
#  3. Solves the resulting *complex* linear system G(ω)·x(ω) = b at each
#     frequency ω = 2πf, yielding complex phasor voltages.
#
# Reading the phasors
# -------------------
# Each node voltage v is a complex number.  Interpretation:
#
#   |v|          — peak amplitude relative to the input signal
#   arg(v) [rad] — phase shift between output and input
#   20 log₁₀|v| — gain in dB  (0 dB = unity gain)
#
# Bode plots are constructed by sweeping f on a log scale and plotting
# 20 log₁₀|v(f)| and arg(v(f)) per decade.
#
# Implementation
# --------------
# The DC Gaussian solver (_solve) is cloned for complex arithmetic
# (_solve_complex).  The DC conductance stamp (_stamp_g) is cloned for
# complex matrices (_stamp_g_c).  A new _stamp_ac dispatcher replaces the
# DC _stamp_dc, using complex admittances for reactive elements.
#
# Inductor at ω=0: Y = 1/(jωL) → ∞; we model it as a near-short (G=1e12 S)
# to keep the matrix non-singular.  Capacitors at ω=0 contribute Y=0 —
# correct (open circuit at DC).
# ---------------------------------------------------------------------------


def _solve_complex(A: list[list[complex]], b: list[complex]) -> list[complex]:
    """Gaussian elimination with partial pivoting for complex matrices.

    Identical algorithm to :func:`_solve` but operates on complex-valued
    entries.  Pivot selection uses ``abs()`` (modulus of the complex number)
    so the algorithm remains numerically stable.

    Raises ``ZeroDivisionError`` when a near-singular pivot (|pivot| < 1e-15)
    is encountered.

    Parameters
    ----------
    A : list[list[complex]]
        Square complex matrix.
    b : list[complex]
        Right-hand-side vector.

    Returns
    -------
    list[complex]
        Solution vector x such that A·x ≈ b.
    """
    n = len(A)
    if n == 0:
        return []
    aug = [row[:] + [b[i]] for i, row in enumerate(A)]

    for i in range(n):
        # Partial pivot: largest modulus below diagonal in column i.
        pivot = i
        for r in range(i + 1, n):
            if abs(aug[r][i]) > abs(aug[pivot][i]):
                pivot = r
        if abs(aug[pivot][i]) < 1e-15:
            raise ZeroDivisionError(f"singular matrix at row {i}")
        aug[i], aug[pivot] = aug[pivot], aug[i]

        for r in range(i + 1, n):
            factor = aug[r][i] / aug[i][i]
            for c in range(i, n + 1):
                aug[r][c] -= factor * aug[i][c]

    x: list[complex] = [0j] * n
    for i in range(n - 1, -1, -1):
        s = aug[i][n]
        for c in range(i + 1, n):
            s -= aug[i][c] * x[c]
        x[i] = s / aug[i][i]
    return x


def _stamp_g_c(
    G: list[list[complex]],
    node_to_idx: dict[str, int],
    n_plus: str,
    n_minus: str,
    g: complex,
) -> None:
    """Stamp a complex admittance between two nodes.

    Identical to :func:`_stamp_g` but for complex-valued conductance matrices.
    Used in AC analysis to stamp:

    - Resistor: ``g = 1/R`` (real)
    - Capacitor: ``g = jωC`` (imaginary at a given ω)
    - Inductor: ``g = 1/(jωL)`` (imaginary at a given ω; near-short at ω=0)
    - Linearised Diode: ``g = gd`` (real small-signal conductance)
    - Linearised MOSFET: ``g = gds`` (real; ``gm`` is stamped separately)
    - Linearised BJT: ``g = g_π`` (real junction conductance; ``gm`` separately)
    """
    if not _is_ground(n_plus):
        G[node_to_idx[n_plus]][node_to_idx[n_plus]] += g
    if not _is_ground(n_minus):
        G[node_to_idx[n_minus]][node_to_idx[n_minus]] += g
    if not _is_ground(n_plus) and not _is_ground(n_minus):
        G[node_to_idx[n_plus]][node_to_idx[n_minus]] -= g
        G[node_to_idx[n_minus]][node_to_idx[n_plus]] -= g


def _stamp_ac(
    el: Element,
    G: list[list[complex]],
    b: list[complex],
    omega: float,
    node_to_idx: dict[str, int],
    vsrcs: list[VoltageSource],
    dc_x: list[float],
) -> None:
    """Stamp one element's AC small-signal contribution at angular frequency ω.

    Linear elements (R, C, L, V, I) use their exact complex admittances.
    Nonlinear elements (Diode, MOSFET, BJT) are linearised at the DC operating
    point provided in ``dc_x``.

    VoltageSource AC handling
    -------------------------
    Each VoltageSource is treated as an ideal AC source with amplitude
    ``el.voltage`` volts (AC amplitude, typically 1 V for the input and 0 V
    for bias sources).  A 0 V AC source acts as a short circuit, which is
    correct for DC-bias voltage sources in an AC analysis.

    Parameters
    ----------
    el : Element
        Circuit element to stamp.
    G : list[list[complex]]
        Complex MNA matrix, modified in place.
    b : list[complex]
        Right-hand-side vector, modified in place.
    omega : float
        Angular frequency ω = 2πf (rad/s).
    node_to_idx : dict[str, int]
        Node-to-row-index map (ground excluded).
    vsrcs : list[VoltageSource]
        All voltage sources in the circuit (determines branch-variable index).
    dc_x : list[float]
        DC operating-point vector (node voltages then branch currents), indexed
        by ``node_to_idx``.  Used to compute small-signal parameters for
        nonlinear devices.
    """
    if isinstance(el, Resistor):
        # Purely real admittance: Y = 1/R
        _stamp_g_c(G, node_to_idx, el.n_plus, el.n_minus, (1.0 + 0j) / el.resistance)

    elif isinstance(el, Capacitor):
        # Admittance Y_C = jωC.  At ω = 0 this is 0 (open circuit) — correct.
        _stamp_g_c(G, node_to_idx, el.n_plus, el.n_minus, 1j * omega * el.capacitance)

    elif isinstance(el, Inductor):
        # Admittance Y_L = 1/(jωL).  At ω = 0, Y → ∞ (short circuit); model
        # as a very large conductance to keep the matrix non-singular.
        if omega == 0.0:
            y_l: complex = 1e12 + 0j
        else:
            y_l = 1.0 / (1j * omega * el.inductance)
        _stamp_g_c(G, node_to_idx, el.n_plus, el.n_minus, y_l)

    elif isinstance(el, VoltageSource):
        # Ideal voltage source stamp: adds branch current as an unknown.
        # Uses += so multiple elements don't overwrite each other's entries.
        i = vsrcs.index(el)
        branch = len(node_to_idx) + i
        if not _is_ground(el.n_plus):
            p = node_to_idx[el.n_plus]
            G[p][branch] += 1.0 + 0j
            G[branch][p] += 1.0 + 0j
        if not _is_ground(el.n_minus):
            q = node_to_idx[el.n_minus]
            G[q][branch] -= 1.0 + 0j
            G[branch][q] -= 1.0 + 0j
        b[branch] += el.voltage + 0j

    elif isinstance(el, CurrentSource):
        # AC current source: inject phasor current.
        if not _is_ground(el.n_plus):
            b[node_to_idx[el.n_plus]] -= el.current + 0j
        if not _is_ground(el.n_minus):
            b[node_to_idx[el.n_minus]] += el.current + 0j

    elif isinstance(el, Diode):
        # Small-signal model: linearised conductance gd = (Is/Vt)·exp(Vd/Vt).
        # The dynamic (differential) conductance is the derivative of
        # I = Is*(exp(Vd/Vt) − 1) with respect to Vd, evaluated at the OP.
        Va = 0.0 if _is_ground(el.anode) else dc_x[node_to_idx[el.anode]]
        Vk = 0.0 if _is_ground(el.cathode) else dc_x[node_to_idx[el.cathode]]
        Vd = min(Va - Vk, 0.7)
        gd = (el.Is / el.Vt) * math.exp(Vd / el.Vt)
        _stamp_g_c(G, node_to_idx, el.anode, el.cathode, gd + 0j)

    elif isinstance(el, Mosfet):
        # Small-signal model: gds (output conductance) + gm (transconductance).
        # The gm VCCS is stamped as off-diagonal conductance entries.
        Vd = 0.0 if _is_ground(el.drain) else dc_x[node_to_idx[el.drain]]
        Vg = 0.0 if _is_ground(el.gate) else dc_x[node_to_idx[el.gate]]
        Vs = 0.0 if _is_ground(el.source) else dc_x[node_to_idx[el.source]]
        Vb = 0.0 if _is_ground(el.body) else dc_x[node_to_idx[el.body]]
        r = el.model.dc(Vg - Vs, Vd - Vs, Vb - Vs)  # type: ignore[attr-defined]
        gm_m: float = r.gm
        gds_m: float = r.gds
        _stamp_g_c(G, node_to_idx, el.drain, el.source, gds_m + 0j)
        if not _is_ground(el.drain):
            d = node_to_idx[el.drain]
            if not _is_ground(el.gate):
                G[d][node_to_idx[el.gate]] += gm_m + 0j
            if not _is_ground(el.source):
                G[d][node_to_idx[el.source]] -= gm_m + 0j
        if not _is_ground(el.source):
            s = node_to_idx[el.source]
            if not _is_ground(el.gate):
                G[s][node_to_idx[el.gate]] -= gm_m + 0j
            if not _is_ground(el.source):
                G[s][node_to_idx[el.source]] += gm_m + 0j

    elif isinstance(el, BJT):
        # Small-signal model: g_π (junction conductance) + gm (transconductance
        # VCCS).  Mirror the DC _stamp_bjt stamps but in the complex domain and
        # without the Norton offsets (which are DC bias terms, zero in AC).
        Vb_dc = 0.0 if _is_ground(el.base) else dc_x[node_to_idx[el.base]]
        Ve_dc = 0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
        Vjunc = (
            min(Vb_dc - Ve_dc, 0.7) if el.polarity == "NPN"
            else min(Ve_dc - Vb_dc, 0.7)
        )
        exp_t = math.exp(Vjunc / el.Vt)
        gm_b: float = (el.Is / el.Vt) * exp_t
        g_pi: float = gm_b / el.beta_f

        if el.polarity == "NPN":
            _stamp_g_c(G, node_to_idx, el.base, el.emitter, g_pi + 0j)
            if not _is_ground(el.collector):
                c_i = node_to_idx[el.collector]
                if not _is_ground(el.base):
                    G[c_i][node_to_idx[el.base]] += gm_b + 0j
                if not _is_ground(el.emitter):
                    G[c_i][node_to_idx[el.emitter]] -= gm_b + 0j
            if not _is_ground(el.emitter):
                e_i = node_to_idx[el.emitter]
                if not _is_ground(el.base):
                    G[e_i][node_to_idx[el.base]] -= gm_b + 0j
                if not _is_ground(el.emitter):
                    G[e_i][node_to_idx[el.emitter]] += gm_b + 0j
        else:  # PNP
            _stamp_g_c(G, node_to_idx, el.emitter, el.base, g_pi + 0j)
            if not _is_ground(el.emitter):
                e_i = node_to_idx[el.emitter]
                if not _is_ground(el.emitter):
                    G[e_i][node_to_idx[el.emitter]] += gm_b + 0j
                if not _is_ground(el.base):
                    G[e_i][node_to_idx[el.base]] -= gm_b + 0j
            if not _is_ground(el.collector):
                c_i = node_to_idx[el.collector]
                if not _is_ground(el.emitter):
                    G[c_i][node_to_idx[el.emitter]] -= gm_b + 0j
                if not _is_ground(el.base):
                    G[c_i][node_to_idx[el.base]] += gm_b + 0j


def ac_sweep(
    circuit: Circuit,
    *,
    f_start: float,
    f_stop: float,
    n_points: int = 50,
    sweep: str = "log",
) -> AcResult:
    """Small-signal AC frequency sweep (the SPICE .AC analysis).

    Computes complex phasor node voltages at each frequency in the sweep
    range.  Linear elements are stamped with their exact complex admittances;
    nonlinear elements are linearised around the DC operating point.

    Algorithm
    ---------
    1. Compute the DC operating point via :func:`dc_op` to get bias voltages
       for nonlinear device linearisation.
    2. Build the frequency grid (log or linear spacing).
    3. For each frequency ω = 2πf:
       a. Build the complex MNA matrix G_c of size (n + m) × (n + m), where
          n = number of non-ground nodes, m = number of voltage sources.
       b. Stamp every element via :func:`_stamp_ac`.
       c. Solve G_c · x_c = b_c using complex Gaussian elimination.
       d. Record the complex phasor voltages as an :class:`AcPoint`.

    Parameters
    ----------
    circuit : Circuit
        The circuit to analyse.  All elements are accepted; unsupported types
        are silently ignored (future-proof for custom elements).
    f_start : float
        Start frequency in hertz.  Must be > 0 for a log sweep.
    f_stop : float
        Stop frequency in hertz.  Must be ≥ f_start.
    n_points : int
        Number of frequency points.  Default 50.  Returns an empty list when
        ``n_points < 1``.
    sweep : str
        ``"log"`` (default) — logarithmically spaced points per decade, like
        the standard SPICE ``.AC DEC`` sweep.
        ``"lin"`` — linearly spaced points between f_start and f_stop.

    Returns
    -------
    AcResult
        One :class:`AcPoint` per frequency.  Each point carries the complex
        phasor voltage at every non-ground node.

    Notes
    -----
    - Voltage sources use their ``voltage`` field as AC amplitude.  A DC
      bias source with ``voltage=0.0`` is a short circuit in AC (correct).
    - Capacitors contribute Y = jωC (open circuit at DC).
    - Inductors contribute Y = 1/(jωL) (short circuit at DC → modelled as a
      very large conductance G = 1e12 S to avoid singularity).
    - If the AC MNA matrix is singular (e.g. a floating node at a particular
      frequency), the node voltages for that frequency point are all set to
      zero and the sweep continues.

    Examples
    --------
    RC low-pass filter with cutoff at f_c = 1 / (2πRC)::

        from spice_engine import Circuit, Resistor, Capacitor, VoltageSource
        from spice_engine import ac_sweep
        import math, cmath

        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 1.0))
        c.add(Resistor("R1", "in", "out", 1000.0))
        c.add(Capacitor("C1", "out", "0", 1e-6))

        result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=100)

        # At f_c ≈ 159 Hz, gain ≈ −3 dB
        for pt in result.points:
            gain_db = 20 * math.log10(abs(pt.node_voltages["out"]))
            phase = math.degrees(cmath.phase(pt.node_voltages["out"]))
    """
    # ---- DC operating point --------------------------------------------------
    dc = dc_op(circuit)
    node_to_idx, _nodes = _node_index(circuit)
    vsrcs = _voltage_sources(circuit)
    n_nodes = len(node_to_idx)
    n_vsrcs = len(vsrcs)
    size = n_nodes + n_vsrcs

    # Reconstruct the indexed dc_x vector from the DcResult dict.
    dc_x: list[float] = [0.0] * size
    for name, idx in node_to_idx.items():
        dc_x[idx] = dc.node_voltages.get(name, 0.0)
    for i, vs in enumerate(vsrcs):
        dc_x[n_nodes + i] = dc.branch_currents.get(f"I({vs.name})", 0.0)

    # ---- Frequency grid -------------------------------------------------------
    if n_points < 1:
        return AcResult(points=[])

    if n_points == 1:
        freqs: list[float] = [f_start]
    elif sweep == "log":
        # Log-spaced: start and stop must be positive.
        log_start = math.log10(max(f_start, 1e-300))
        log_stop = math.log10(max(f_stop, f_start, 1e-300))
        step_log = (log_stop - log_start) / (n_points - 1)
        freqs = [10.0 ** (log_start + k * step_log) for k in range(n_points)]
    else:  # "lin"
        step_lin = (f_stop - f_start) / (n_points - 1)
        freqs = [f_start + k * step_lin for k in range(n_points)]

    # ---- Per-frequency solve --------------------------------------------------
    ac_points: list[AcPoint] = []
    for freq in freqs:
        omega = 2.0 * math.pi * freq

        # Build complex MNA matrix — zero initialised.
        G_c: list[list[complex]] = [[0j] * size for _ in range(size)]
        b_c: list[complex] = [0j] * size

        for el in circuit.elements:
            _stamp_ac(el, G_c, b_c, omega, node_to_idx, vsrcs, dc_x)

        try:
            x_c = _solve_complex(G_c, b_c)
        except ZeroDivisionError:
            x_c = [0j] * size  # singular — return zeros for this frequency

        node_v = {name: x_c[idx] for name, idx in node_to_idx.items()}
        ac_points.append(AcPoint(freq=freq, node_voltages=node_v))

    return AcResult(points=ac_points)


# Keep the cmath import visible to callers that ``from spice_engine import cmath``
_ = cmath  # noqa: F841
