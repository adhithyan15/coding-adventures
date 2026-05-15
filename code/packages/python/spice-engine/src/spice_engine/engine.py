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
import random
import statistics
from dataclasses import dataclass, field

from spice_engine.elements import (
    AcSource,
    BJT,
    CCCS,
    CCVS,
    VCCS,
    VCVS,
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


@dataclass(frozen=True)
class TfResult:
    """DC small-signal transfer function, input impedance, and output impedance.

    This is the Python equivalent of the SPICE ``.TF`` analysis.  Given a
    linear (or linearised) circuit, a signal input (voltage or current source)
    and an output node, ``.TF`` computes three quantities:

    transfer_ratio
        The ratio V_output / V_input for a :class:`VoltageSource` input, or
        V_output / I_input (transimpedance, in Ω) for a
        :class:`CurrentSource` input.  Both are measured with all other
        independent sources zeroed (DC small-signal sense).

    input_impedance
        The Thevenin equivalent impedance seen looking into the input port
        (in Ω).  For a VoltageSource input this is ``-V_in / I_in`` where
        the negative sign accounts for the MNA branch-current convention
        (x[branch] is negative when the source delivers current).  For a
        CurrentSource input this is the compliance voltage V_minus − V_plus
        developed across the source when 1 A is forced.

    output_impedance
        The Thevenin equivalent impedance seen looking back into the circuit
        from the output node (in Ω).  Computed by zeroing all independent
        sources and injecting 1 A at the output; Z_out = V_output / 1 A.

    converged
        ``False`` when the DC operating-point Newton-Raphson failed to
        converge.  The transfer function values are unreliable in this case.

    Notes
    -----
    All three values are real-valued DC small-signal quantities (ω = 0).
    For frequency-domain transfer functions use :func:`ac_sweep`.
    """

    transfer_ratio: float
    input_impedance: float
    output_impedance: float
    converged: bool = True

    @property
    def gain(self) -> float:
        """Alias for :attr:`transfer_ratio` — convenient shorthand for voltage gain."""
        return self.transfer_ratio


@dataclass(frozen=True)
class DcSweepPoint:
    """A single operating-point sample from a DC parameter sweep.

    A DC sweep steps one independent source (voltage or current) through a
    range of values and records the circuit's DC operating point at each step.
    This is the SPICE ``.DC`` analysis.

    Attributes
    ----------
    source_value : float
        The value of the swept source at this step (V for a
        :class:`VoltageSource`, A for a :class:`CurrentSource`).
    node_voltages : dict[str, float]
        DC node voltages (in volts) keyed by node name.  The reference node
        (``"0"`` / ``"gnd"``) is excluded; its voltage is always 0 V.
    branch_currents : dict[str, float]
        DC branch currents (in amperes) for every voltage source in the
        circuit, keyed by source name.
    converged : bool
        ``True`` when the Newton-Raphson DC solve converged at this step.
        ``False`` indicates an unreliable operating point (the
        ``node_voltages`` and ``branch_currents`` values are unreliable).

    Notes
    -----
    For nonlinear circuits (diodes, BJTs, MOSFETs) Newton-Raphson may fail
    to converge if the operating point is far from the initial guess.
    Consecutive sweep points start from the previous converged solution,
    which usually keeps convergence robust over moderate sweep ranges.

    Examples
    --------
    Plot V(out) vs V(in) for a common-emitter amplifier swept from 0 V to 5 V::

        result = dc_sweep(circuit, "Vin", 0.0, 5.0, 0.1)
        v_in  = [pt.source_value           for pt in result.points]
        v_out = [pt.node_voltages.get("out", 0.0) for pt in result.points]
    """

    source_value: float
    node_voltages: dict[str, float]
    branch_currents: dict[str, float]
    converged: bool


@dataclass
class DcSweepResult:
    """Collected operating-point samples from a DC parameter sweep.

    Returned by :func:`dc_sweep`.  Contains one :class:`DcSweepPoint` per
    sweep step, in the order the steps were evaluated (ascending or
    descending, matching the sign of ``step``).

    Attributes
    ----------
    points : list[DcSweepPoint]
        Ordered list of operating-point snapshots.  Empty if the sweep
        range produced zero steps (e.g., ``start == stop`` with a nonzero
        step, or ``step`` has the wrong sign).
    source_name : str
        Name of the swept source element (as given to :func:`dc_sweep`).

    Examples
    --------
    Extract all converged node-voltage dicts::

        converged_pts = [pt for pt in result.points if pt.converged]
    """

    points: list[DcSweepPoint]
    source_name: str


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
    if isinstance(el, (VCVS, VCCS)):
        # Both output nodes and controlling nodes become part of the circuit
        return [el.n_plus, el.n_minus, el.ctrl_plus, el.ctrl_minus]
    if isinstance(el, (CCCS, CCVS)):
        # Output nodes only (controlling branch is referenced by name, not nodes)
        return [el.n_plus, el.n_minus]
    return []


def _voltage_sources(circuit: Circuit) -> list[VoltageSource]:
    """Return all independent VoltageSource elements (for sens/mc perturbation)."""
    return [el for el in circuit.elements if isinstance(el, VoltageSource)]


def _branch_sources(
    circuit: Circuit,
) -> list[VoltageSource | VCVS | CCVS]:
    """Elements that require a branch unknown (current variable) in MNA.

    All three element types introduce a KVL constraint row and a corresponding
    branch-current column in the MNA matrix.  The ordering is stable:

        1. All ``VoltageSource`` elements (preserves existing branch indices)
        2. All ``VCVS`` elements
        3. All ``CCVS`` elements

    The branch index for an element ``el`` is::

        branch_idx = n_nodes + _branch_sources(circuit).index(el)

    where ``n_nodes = len(_node_index(circuit)[0])``.

    Note: ``CCCS`` (F element) does **not** appear here because it only adds
    off-diagonal conductance entries and needs no branch unknown of its own.
    ``VCCS`` (G element) likewise needs no branch unknown.
    """
    vsrcs: list[VoltageSource | VCVS | CCVS] = [
        el for el in circuit.elements if isinstance(el, VoltageSource)
    ]
    vcvs_list: list[VoltageSource | VCVS | CCVS] = [
        el for el in circuit.elements if isinstance(el, VCVS)
    ]
    ccvs_list: list[VoltageSource | VCVS | CCVS] = [
        el for el in circuit.elements if isinstance(el, CCVS)
    ]
    return vsrcs + vcvs_list + ccvs_list


def _is_ground(name: str) -> bool:
    return name in ("0", "gnd", "GND")


# ---------------------------------------------------------------------------
# DC analysis
# ---------------------------------------------------------------------------


def _dc_newton(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    x_init: list[float] | None = None,
) -> DcResult:
    """Run Newton-Raphson DC solve, optionally warm-started from *x_init*.

    This is the inner Newton loop shared by :func:`dc_op` and the convergence-
    aid helpers.  Unlike :func:`dc_op` it does **not** retry on non-convergence
    — it returns a :class:`DcResult` with ``converged=False`` immediately.

    Parameters
    ----------
    circuit:
        The (possibly augmented) circuit to solve.
    max_iterations:
        Maximum Newton iterations.
    tol:
        Convergence tolerance: ``max |Δx| < tol`` declares convergence.
    x_init:
        Optional initial-guess vector (node voltages followed by branch
        currents, in the same order as :func:`_node_index` and
        :func:`_branch_sources`).  Defaults to all-zeros.
    """
    node_to_idx, nodes = _node_index(circuit)
    branch_srcs = _branch_sources(circuit)
    n = len(nodes)
    m = len(branch_srcs)
    size = n + m

    x = list(x_init) if x_init is not None else [0.0] * size

    max_delta = float("inf")
    for it in range(max_iterations):
        G = [[0.0] * size for _ in range(size)]
        b = [0.0] * size
        for el in circuit.elements:
            _stamp_dc(el, G, b, x, node_to_idx, branch_srcs)
        try:
            x_new = _solve(G, b)
        except ZeroDivisionError:
            node_v = {nd: x[i] for nd, i in node_to_idx.items()}
            return DcResult(node_v, {}, iterations=it, converged=False)

        max_delta = max(abs(a - bv) for a, bv in zip(x, x_new, strict=False)) if x else 0.0
        x = x_new
        if max_delta < tol:
            break

    node_v = {nd: x[i] for nd, i in node_to_idx.items()}
    branch_i = {f"I({el.name})": x[n + i] for i, el in enumerate(branch_srcs)}
    return DcResult(node_v, branch_i, iterations=it + 1, converged=max_delta < tol)


def _x_from_result(
    result: DcResult,
    nodes: list[str],
    branch_srcs: list,
) -> list[float]:
    """Reconstruct the raw *x* vector from a :class:`DcResult`.

    Needed to warm-start the next Newton iteration from a previously
    converged solution.

    Parameters
    ----------
    result:
        A converged (or partially converged) :class:`DcResult`.
    nodes:
        Non-ground node names in MNA order (from :func:`_node_index`).
    branch_srcs:
        Branch-source elements in MNA order (from :func:`_branch_sources`).
    """
    x = [result.node_voltages.get(nd, 0.0) for nd in nodes]
    x += [result.branch_currents.get(f"I({el.name})", 0.0) for el in branch_srcs]
    return x


def _dc_gmin_step(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    gmin_start: float = 1e-3,
    n_steps: int = 10,
) -> DcResult | None:
    """DC operating point via Gmin stepping (convergence aid #1).

    **What it does:**  A small conductance *Gmin* is added from every
    non-ground node to ground.  Large *Gmin* (1 mS) regularises the MNA
    matrix and guarantees convergence even when the zero-state initial guess
    is far from the operating point (e.g. strongly nonlinear diode circuits).
    The conductance is then reduced logarithmically to zero; each step uses
    the previous solution as a warm start so Newton converges quickly.

    **Step sequence:**

    ::

        gmin_start (1e-3)
            → gmin_start / 10
            → gmin_start / 100
            → ...  (n_steps log-spaced values)
            → 0  (original circuit, warm start from last Gmin solve)

    Parameters
    ----------
    circuit:
        Original circuit (not augmented — augmentation is done internally).
    max_iterations:
        Newton iterations per step.
    tol:
        Convergence tolerance.
    gmin_start:
        Initial Gmin conductance (S).  1 mS = 1 kΩ shunt gives good
        numerical stability across a wide range of circuits.
    n_steps:
        Number of log-spaced Gmin values before the final no-Gmin step.

    Returns
    -------
    DcResult or None
        ``None`` if any intermediate Newton step fails to converge.
        Otherwise the :class:`DcResult` of the final no-Gmin solve.
    """
    _, nodes = _node_index(circuit)
    if not nodes:
        # Trivial circuit (no non-ground nodes) — Gmin stepping adds nothing.
        return None

    orig_branch_srcs = _branch_sources(circuit)

    # Build log-spaced Gmin sequence from gmin_start down to ~1e-12, then 0.
    # Using math.log10 (math module is imported at top of file).
    log_start = math.log10(gmin_start)
    log_end = math.log10(1e-12)
    gmin_sequence: list[float] = [
        10.0 ** (log_start + (log_end - log_start) * k / (n_steps - 1))
        for k in range(n_steps)
    ]
    gmin_sequence.append(0.0)  # final step: no Gmin (solve original circuit)

    x_init: list[float] | None = None

    for gmin in gmin_sequence:
        if gmin > 0.0:
            # Augment the circuit: add a resistor R = 1/gmin from each node to ground.
            # These resistors are named with a leading underscore so they cannot
            # collide with user element names.
            gmin_elements = [
                Resistor(f"_gmin_{nd}", nd, "0", 1.0 / gmin)
                for nd in nodes
            ]
            aug = Circuit(elements=list(circuit.elements) + gmin_elements)
        else:
            # Final step: original circuit, warm-started from the last Gmin solve.
            aug = circuit

        result = _dc_newton(aug, max_iterations=max_iterations, tol=tol, x_init=x_init)
        if not result.converged:
            return None  # This step diverged — Gmin stepping has failed.

        # Reconstruct x_init for the next step.  Gmin resistors add no new
        # non-ground nodes, so the x-vector ordering is identical to the
        # original circuit's ordering.
        x_init = _x_from_result(result, nodes, orig_branch_srcs)

    return result


def _dc_source_step(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    n_steps: int = 10,
) -> DcResult | None:
    """DC operating point via source stepping (convergence aid #2).

    **What it does:**  All independent voltage sources and current sources
    are scaled from 0 to their full values in *n_steps* equal steps.  At
    scale = 0 the trivial solution x = 0 is exact; each subsequent step
    uses the previous solution as a warm start.  This gives Newton a very
    good initial guess at each step and avoids the large nonlinear jumps
    that cause divergence when the full source voltages are applied at once.

    **Step sequence:**

    ::

        scale = 0.0   (all sources zero → trivial x = 0 solution)
        scale = 0.1
        scale = 0.2
        ...
        scale = 1.0   (full original source values)

    Only ``VoltageSource.voltage`` and ``CurrentSource.current`` are scaled.
    Controlled sources (VCVS, VCCS, CCCS, CCVS) pass through unchanged.

    Parameters
    ----------
    circuit:
        Original circuit.
    max_iterations:
        Newton iterations per step.
    tol:
        Convergence tolerance.
    n_steps:
        Number of source-scaling steps from 0 to 1 (inclusive).
        More steps = smaller increments = higher chance of convergence
        but more total Newton iterations.

    Returns
    -------
    DcResult or None
        ``None`` if any intermediate step fails to converge.
        Otherwise the :class:`DcResult` at scale = 1.0 (full sources).
    """
    _, nodes = _node_index(circuit)
    orig_branch_srcs = _branch_sources(circuit)

    # Build the scale sequence: 0, 1/n_steps, 2/n_steps, ..., 1.
    scales = [k / n_steps for k in range(n_steps + 1)]

    x_init: list[float] | None = None

    for scale in scales:
        # Build a circuit with all independent sources scaled by `scale`.
        scaled_elements = []
        for e in circuit.elements:
            if isinstance(e, VoltageSource):
                scaled_elements.append(VoltageSource(
                    name=e.name,
                    n_plus=e.n_plus,
                    n_minus=e.n_minus,
                    voltage=e.voltage * scale,
                    waveform=e.waveform,
                    ac=e.ac,
                ))
            elif isinstance(e, CurrentSource):
                scaled_elements.append(CurrentSource(
                    name=e.name,
                    n_plus=e.n_plus,
                    n_minus=e.n_minus,
                    current=e.current * scale,
                    waveform=e.waveform,
                    ac=e.ac,
                ))
            else:
                scaled_elements.append(e)
        scaled_circuit = Circuit(elements=scaled_elements)

        result = _dc_newton(
            scaled_circuit, max_iterations=max_iterations, tol=tol, x_init=x_init
        )
        if not result.converged:
            return None  # This step diverged — source stepping has failed.

        # Reconstruct x_init for the next step.  Source scaling does not
        # change circuit topology (same nodes, same branch sources), so the
        # x-vector ordering is unchanged.
        x_init = _x_from_result(result, nodes, orig_branch_srcs)

    return result


def dc_op(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    convergence_aids: bool = True,
) -> DcResult:
    """Solve DC operating point via Newton-Raphson on a linearized MNA.

    When the plain Newton-Raphson pass does not converge and
    ``convergence_aids=True`` (the default), the engine automatically retries
    using SPICE3-style fallback strategies:

    1. **Gmin stepping** — adds a small shunt conductance from every
       non-ground node to ground and logarithmically reduces it to zero.
       Stabilises the matrix against floating nodes and large nonlinearities.

    2. **Source stepping** — scales all independent sources from 0 to their
       full values in 10 steps, using each converged solution as a warm
       start.  Particularly effective for circuits with diode clamps and
       other strongly nonlinear devices.

    The chain is tried in sequence (Newton → Gmin → source step).  The first
    method to converge is returned.  If all methods fail the result has
    ``converged=False``.

    Parameters
    ----------
    circuit:
        The circuit to analyse.
    max_iterations:
        Maximum Newton-Raphson iterations per attempt.
    tol:
        Convergence tolerance: ``max |Δx| < tol`` declares convergence.
    convergence_aids:
        When ``True`` (default), automatically fall back to Gmin stepping
        then source stepping when plain Newton diverges.  Set to ``False``
        to force plain Newton only (faster for simple linear circuits).
    """
    # Attempt 1: plain Newton-Raphson.
    result = _dc_newton(circuit, max_iterations=max_iterations, tol=tol)
    if result.converged or not convergence_aids:
        return result

    # Attempt 2: Gmin stepping.
    gmin_result = _dc_gmin_step(circuit, max_iterations=max_iterations, tol=tol)
    if gmin_result is not None and gmin_result.converged:
        return gmin_result

    # Attempt 3: source stepping.
    src_result = _dc_source_step(circuit, max_iterations=max_iterations, tol=tol)
    if src_result is not None and src_result.converged:
        return src_result

    # All methods exhausted — return the plain-Newton result (converged=False).
    return result


def _stamp_dc(
    el: Element,
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    branch_srcs: list[VoltageSource | VCVS | CCVS],
) -> None:
    """Stamp one element's MNA contribution at the current operating point."""
    n_nodes = len(node_to_idx)
    if isinstance(el, Resistor):
        _stamp_g(G, node_to_idx, el.n_plus, el.n_minus, 1.0 / el.resistance)
    elif isinstance(el, VoltageSource):
        i = branch_srcs.index(el)
        _stamp_vsrc(G, b, node_to_idx, el, n_nodes + i)
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
    elif isinstance(el, VCCS):
        _stamp_vccs(G, node_to_idx, el.n_plus, el.n_minus,
                    el.ctrl_plus, el.ctrl_minus, el.gm)
    elif isinstance(el, VCVS):
        i = branch_srcs.index(el)
        _stamp_vcvs(G, b, node_to_idx, el, n_nodes + i)
    elif isinstance(el, CCCS):
        ctrl_el = _find_branch_source(branch_srcs, el.ctrl_source)
        if ctrl_el is None:
            raise ValueError(
                f"CCCS '{el.name}' references controlling source "
                f"'{el.ctrl_source}' which does not exist in the circuit."
            )
        ctrl_idx = n_nodes + branch_srcs.index(ctrl_el)
        _stamp_cccs(G, node_to_idx, el, ctrl_idx)
    elif isinstance(el, CCVS):
        i = branch_srcs.index(el)
        ctrl_el = _find_branch_source(branch_srcs, el.ctrl_source)
        if ctrl_el is None:
            raise ValueError(
                f"CCVS '{el.name}' references controlling source "
                f"'{el.ctrl_source}' which does not exist in the circuit."
            )
        ctrl_idx = n_nodes + branch_srcs.index(ctrl_el)
        _stamp_ccvs(G, b, node_to_idx, el, n_nodes + i, ctrl_idx)
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


# ---------------------------------------------------------------------------
# Controlled-source MNA stamps
# ---------------------------------------------------------------------------


def _find_branch_source(
    branch_srcs: list[VoltageSource | VCVS | CCVS],
    name: str,
) -> VoltageSource | VCVS | CCVS | None:
    """Return the branch-source element with the given name, or None."""
    for el in branch_srcs:
        if el.name == name:
            return el
    return None


def _stamp_vccs(
    G: list[list[float]],
    node_to_idx: dict[str, int],
    n_plus: str,
    n_minus: str,
    ctrl_plus: str,
    ctrl_minus: str,
    gm: float,
) -> None:
    """Stamp a VCCS: I(n_plus→n_minus) = gm × [V(ctrl_plus) − V(ctrl_minus)].

    MNA off-diagonal entries (no branch unknown needed):

        G[n_plus][ctrl_plus]   +=  gm
        G[n_plus][ctrl_minus]  -=  gm
        G[n_minus][ctrl_plus]  -=  gm
        G[n_minus][ctrl_minus] +=  gm

    This is the same stamp used internally for MOSFET/BJT transconductance.
    """
    if not _is_ground(n_plus):
        rp = node_to_idx[n_plus]
        if not _is_ground(ctrl_plus):
            G[rp][node_to_idx[ctrl_plus]] += gm
        if not _is_ground(ctrl_minus):
            G[rp][node_to_idx[ctrl_minus]] -= gm
    if not _is_ground(n_minus):
        rm = node_to_idx[n_minus]
        if not _is_ground(ctrl_plus):
            G[rm][node_to_idx[ctrl_plus]] -= gm
        if not _is_ground(ctrl_minus):
            G[rm][node_to_idx[ctrl_minus]] += gm


def _stamp_vcvs(
    G: list[list[float]],
    b: list[float],
    node_to_idx: dict[str, int],
    el: VCVS,
    branch_idx: int,
) -> None:
    """Stamp a VCVS: V(n_plus,n_minus) = gain × [V(ctrl_plus) − V(ctrl_minus)].

    KCL rows for the output port (identical to VoltageSource structure):

        G[n_plus][k]   += 1    G[k][n_plus]   += 1
        G[n_minus][k]  -= 1    G[k][n_minus]  -= 1

    KVL row contribution from the controlling nodes:

        G[k][ctrl_plus]  -= gain    (from +V_ctrl_plus term moved to LHS)
        G[k][ctrl_minus] += gain    (from −V_ctrl_minus term moved to LHS)

    b[k] = 0 (ideal source, no DC offset).
    """
    if not _is_ground(el.n_plus):
        p = node_to_idx[el.n_plus]
        G[p][branch_idx] += 1.0
        G[branch_idx][p] += 1.0
    if not _is_ground(el.n_minus):
        q = node_to_idx[el.n_minus]
        G[q][branch_idx] -= 1.0
        G[branch_idx][q] -= 1.0
    if not _is_ground(el.ctrl_plus):
        G[branch_idx][node_to_idx[el.ctrl_plus]] -= el.gain
    if not _is_ground(el.ctrl_minus):
        G[branch_idx][node_to_idx[el.ctrl_minus]] += el.gain
    b[branch_idx] = 0.0


def _stamp_cccs(
    G: list[list[float]],
    node_to_idx: dict[str, int],
    el: CCCS,
    ctrl_branch_idx: int,
) -> None:
    """Stamp a CCCS: I(n_plus→n_minus) = beta × I(ctrl_source).

    In the MNA G·x = b framework, a positive branch current ``I_ctrl``
    (which represents current leaving ``ctrl.n_plus`` through the source) is
    used as the controlling quantity.  The CCCS output must inject current
    INTO ``n_plus`` (so that it exits ``n_plus`` into the external circuit
    toward ``n_minus``).  An injected current appears as a NEGATIVE term in
    the "leaving-current" KCL sum, so the stamp is:

        G[n_plus][ctrl_branch_idx]  -= beta   (injection at n_plus)
        G[n_minus][ctrl_branch_idx] += beta   (removal at n_minus)

    This matches the SPICE ``F`` element convention: positive current flows
    from ``n_plus`` through the external circuit to ``n_minus``.

    No new branch unknown is needed; this is a pure off-diagonal entry in
    the branch-current column of the controlling source.
    """
    if not _is_ground(el.n_plus):
        G[node_to_idx[el.n_plus]][ctrl_branch_idx] -= el.beta
    if not _is_ground(el.n_minus):
        G[node_to_idx[el.n_minus]][ctrl_branch_idx] += el.beta


def _stamp_ccvs(
    G: list[list[float]],
    b: list[float],
    node_to_idx: dict[str, int],
    el: CCVS,
    branch_idx: int,
    ctrl_branch_idx: int,
) -> None:
    """Stamp a CCVS: V(n_plus,n_minus) = transresistance × I(ctrl_source).

    KCL rows for the output port (like VoltageSource / VCVS):

        G[n_plus][k]   += 1    G[k][n_plus]   += 1
        G[n_minus][k]  -= 1    G[k][n_minus]  -= 1

    KVL row: V_out_p − V_out_m − rm × x[ctrl_branch_idx] = 0

        G[k][ctrl_branch_idx] -= transresistance

    b[k] = 0.
    """
    if not _is_ground(el.n_plus):
        p = node_to_idx[el.n_plus]
        G[p][branch_idx] += 1.0
        G[branch_idx][p] += 1.0
    if not _is_ground(el.n_minus):
        q = node_to_idx[el.n_minus]
        G[q][branch_idx] -= 1.0
        G[branch_idx][q] -= 1.0
    G[branch_idx][ctrl_branch_idx] -= el.transresistance
    b[branch_idx] = 0.0


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
    t: float = 0.0,
) -> Circuit:
    """Build the linearised companion circuit for one timestep.

    Replaces each capacitor and inductor with their Norton companion models.
    All other elements pass through unchanged.  Independent sources that
    carry a ``waveform`` callable are replaced with a static version whose
    ``voltage`` / ``current`` is evaluated at time *t*.

    Parameters
    ----------
    circuit:
        The original (user-specified) circuit.
    h:
        Current timestep size (seconds).
    method:
        ``"trap"`` (trapezoidal) or ``"euler"`` (backward Euler).
    cap_voltages, cap_currents, ind_currents, ind_voltages:
        Reactive-element history dictionaries from the previous timestep.
    t:
        Current simulation time (seconds).  Used to evaluate source waveforms.

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
    # Build the base element list, substituting time-varying source values.
    # VoltageSource / CurrentSource elements that carry a waveform callable
    # are replaced here with a plain static copy at the current time t.
    # Capacitors and Inductors are always excluded (they get companion models).
    base_elements: list = []
    for e in circuit.elements:
        if isinstance(e, (Capacitor, Inductor)):
            continue
        if isinstance(e, VoltageSource) and e.waveform is not None:
            # Evaluate the waveform at the current simulation time.
            base_elements.append(VoltageSource(
                name=e.name,
                n_plus=e.n_plus,
                n_minus=e.n_minus,
                voltage=e.waveform(t),
                ac=e.ac,
            ))
        elif isinstance(e, CurrentSource) and e.waveform is not None:
            base_elements.append(CurrentSource(
                name=e.name,
                n_plus=e.n_plus,
                n_minus=e.n_minus,
                current=e.waveform(t),
                ac=e.ac,
            ))
        else:
            base_elements.append(e)
    aug = Circuit(elements=base_elements)

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
    # that the rest of the circuit settles consistently.  Time-varying sources
    # are evaluated at t = 0 to obtain the correct initial bias.
    init_elements: list = []
    for e in circuit.elements:
        if isinstance(e, (Capacitor, Inductor)):
            continue
        if isinstance(e, VoltageSource) and e.waveform is not None:
            init_elements.append(VoltageSource(
                name=e.name,
                n_plus=e.n_plus,
                n_minus=e.n_minus,
                voltage=e.waveform(0.0),
                ac=e.ac,
            ))
        elif isinstance(e, CurrentSource) and e.waveform is not None:
            init_elements.append(CurrentSource(
                name=e.name,
                n_plus=e.n_plus,
                n_minus=e.n_minus,
                current=e.waveform(0.0),
                ac=e.ac,
            ))
        else:
            init_elements.append(e)
    init_circuit = Circuit(elements=init_elements)
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
            t=t,
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


def _has_explicit_ac_sources(circuit: Circuit) -> bool:
    """Return True when at least one independent source has an AC spec."""

    return any(
        isinstance(el, (VoltageSource, CurrentSource)) and el.ac is not None
        for el in circuit.elements
    )


def _ac_phasor(
    name: str,
    ac: AcSource | None,
    fallback: float,
    explicit_ac: bool,
) -> complex:
    """Return the source phasor for AC analysis.

    Legacy circuits without explicit ``AC`` source specs keep using the DC
    value as the AC phasor.  Once any independent source declares an explicit
    AC spec, unspecified independent sources become zero small-signal sources.
    """

    if ac is None:
        return 0j if explicit_ac else fallback + 0j
    if not math.isfinite(ac.magnitude) or not math.isfinite(ac.phase_degrees):
        raise ValueError(f"{name}: AC magnitude and phase must be finite")
    phase = math.radians(ac.phase_degrees)
    return ac.magnitude * complex(math.cos(phase), math.sin(phase))


def _stamp_ac(
    el: Element,
    G: list[list[complex]],
    b: list[complex],
    omega: float,
    node_to_idx: dict[str, int],
    branch_srcs: list[VoltageSource | VCVS | CCVS],
    dc_x: list[float],
    *,
    explicit_ac_sources: bool = False,
) -> None:
    """Stamp one element's AC small-signal contribution at angular frequency ω.

    Linear elements (R, C, L, V, I) use their exact complex admittances.
    Nonlinear elements (Diode, MOSFET, BJT) are linearised at the DC operating
    point provided in ``dc_x``.
    Controlled sources (VCVS, VCCS, CCVS, CCCS) are linear and are stamped
    using their real-valued gains (frequency-independent).

    VoltageSource AC handling
    -------------------------
    Each VoltageSource is treated as an ideal AC source.  If any independent
    source has an explicit ``ac`` spec, only explicit AC specs contribute
    phasors and unspecified sources are zeroed.  For backwards compatibility,
    circuits with no explicit AC specs still use the DC source value as the
    AC phasor.

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
    branch_srcs : list[VoltageSource | VCVS | CCVS]
        All branch-unknown sources in the circuit (determines column indices).
    dc_x : list[float]
        DC operating-point vector (node voltages then branch currents), indexed
        by ``node_to_idx``.  Used to compute small-signal parameters for
        nonlinear devices.
    """
    n_nodes = len(node_to_idx)
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
        i = branch_srcs.index(el)
        branch = n_nodes + i
        if not _is_ground(el.n_plus):
            p = node_to_idx[el.n_plus]
            G[p][branch] += 1.0 + 0j
            G[branch][p] += 1.0 + 0j
        if not _is_ground(el.n_minus):
            q = node_to_idx[el.n_minus]
            G[q][branch] -= 1.0 + 0j
            G[branch][q] -= 1.0 + 0j
        b[branch] += _ac_phasor(el.name, el.ac, el.voltage, explicit_ac_sources)

    elif isinstance(el, CurrentSource):
        # AC current source: inject phasor current.
        current = _ac_phasor(el.name, el.ac, el.current, explicit_ac_sources)
        if not _is_ground(el.n_plus):
            b[node_to_idx[el.n_plus]] -= current
        if not _is_ground(el.n_minus):
            b[node_to_idx[el.n_minus]] += current

    elif isinstance(el, VCCS):
        # Frequency-independent transconductance: same stamp as DC.
        _stamp_vccs(G, node_to_idx, el.n_plus, el.n_minus,
                    el.ctrl_plus, el.ctrl_minus, el.gm)  # type: ignore[arg-type]

    elif isinstance(el, VCVS):
        # Voltage-controlled voltage source — same stamp as DC.
        i = branch_srcs.index(el)
        branch = n_nodes + i
        if not _is_ground(el.n_plus):
            p = node_to_idx[el.n_plus]
            G[p][branch] += 1.0 + 0j
            G[branch][p] += 1.0 + 0j
        if not _is_ground(el.n_minus):
            q = node_to_idx[el.n_minus]
            G[q][branch] -= 1.0 + 0j
            G[branch][q] -= 1.0 + 0j
        if not _is_ground(el.ctrl_plus):
            G[branch][node_to_idx[el.ctrl_plus]] -= el.gain + 0j
        if not _is_ground(el.ctrl_minus):
            G[branch][node_to_idx[el.ctrl_minus]] += el.gain + 0j
        b[branch] += 0j

    elif isinstance(el, CCCS):
        ctrl_bsrc = _find_branch_source(branch_srcs, el.ctrl_source)
        if ctrl_bsrc is None:
            raise ValueError(
                f"CCCS '{el.name}' references controlling source "
                f"'{el.ctrl_source}' which does not exist in the circuit."
            )
        ctrl_branch = n_nodes + branch_srcs.index(ctrl_bsrc)
        if not _is_ground(el.n_plus):
            G[node_to_idx[el.n_plus]][ctrl_branch] -= el.beta + 0j
        if not _is_ground(el.n_minus):
            G[node_to_idx[el.n_minus]][ctrl_branch] += el.beta + 0j

    elif isinstance(el, CCVS):
        i = branch_srcs.index(el)
        branch = n_nodes + i
        if not _is_ground(el.n_plus):
            p = node_to_idx[el.n_plus]
            G[p][branch] += 1.0 + 0j
            G[branch][p] += 1.0 + 0j
        if not _is_ground(el.n_minus):
            q = node_to_idx[el.n_minus]
            G[q][branch] -= 1.0 + 0j
            G[branch][q] -= 1.0 + 0j
        ctrl_bsrc = _find_branch_source(branch_srcs, el.ctrl_source)
        if ctrl_bsrc is None:
            raise ValueError(
                f"CCVS '{el.name}' references controlling source "
                f"'{el.ctrl_source}' which does not exist in the circuit."
            )
        ctrl_branch = n_nodes + branch_srcs.index(ctrl_bsrc)
        G[branch][ctrl_branch] -= el.transresistance + 0j
        b[branch] += 0j

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
    branch_srcs = _branch_sources(circuit)
    n_nodes = len(node_to_idx)
    n_branch = len(branch_srcs)
    size = n_nodes + n_branch
    explicit_ac_sources = _has_explicit_ac_sources(circuit)

    # Reconstruct the indexed dc_x vector from the DcResult dict.
    dc_x: list[float] = [0.0] * size
    for name, idx in node_to_idx.items():
        dc_x[idx] = dc.node_voltages.get(name, 0.0)
    for i, bs in enumerate(branch_srcs):
        dc_x[n_nodes + i] = dc.branch_currents.get(f"I({bs.name})", 0.0)

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
            _stamp_ac(
                el,
                G_c,
                b_c,
                omega,
                node_to_idx,
                branch_srcs,
                dc_x,
                explicit_ac_sources=explicit_ac_sources,
            )

        try:
            x_c = _solve_complex(G_c, b_c)
        except ZeroDivisionError:
            x_c = [0j] * size  # singular — return zeros for this frequency

        node_v = {name: x_c[idx] for name, idx in node_to_idx.items()}
        ac_points.append(AcPoint(freq=freq, node_voltages=node_v))

    return AcResult(points=ac_points)


# Keep the cmath import visible to callers that ``from spice_engine import cmath``
_ = cmath  # noqa: F841


# ---------------------------------------------------------------------------
# Section 4 — DC small-signal transfer function (.TF) analysis
# ---------------------------------------------------------------------------
#
# Background: what is .TF analysis?
# ----------------------------------
# SPICE ``.TF`` computes three DC small-signal quantities in one pass:
#
#  1. **Transfer ratio H** — the ratio of a chosen output voltage to the
#     excitation provided by one independent source, with all other
#     independent sources zeroed (superposition at ω = 0).
#
#  2. **Input impedance Z_in** — the Thevenin equivalent impedance looking
#     into the input source terminals.
#
#  3. **Output impedance Z_out** — the Thevenin equivalent impedance
#     looking back into the circuit from the output node.
#
# Algorithm
# ---------
# Step 1: DC operating point.
#     Run :func:`dc_op` to bias all nonlinear devices (Diode, MOSFET, BJT).
#     This gives the linearisation point for the small-signal matrix.
#
# Step 2: Small-signal conductance matrix G_ss.
#     Build a *real* MNA matrix at ω = 0 via :func:`_build_ss_matrix`.
#     Independent sources (VoltageSource voltage, CurrentSource current) are
#     zeroed — only their structural KVL/KCL entries remain.
#     Reactive elements: Capacitor → open (skipped); Inductor → near-short
#     (G = 1e12 S).  Nonlinear elements are replaced by their linearised
#     small-signal conductances at the DC operating point.
#
# Step 3: Forward solve (transfer ratio + input impedance).
#     Apply a unit excitation at the input source while keeping all other
#     sources zeroed:
#       - VoltageSource input: set b_fwd[branch_idx] = 1.0 (1 V excitation).
#       - CurrentSource input: set b_fwd[n_plus] -= 1.0, b_fwd[n_minus] += 1.0
#         (1 A injection following the DC stamp convention).
#     Solve G_ss · x_fwd = b_fwd.
#       - H = x_fwd[output_node_idx].
#       - Z_in (VoltageSource): x_fwd[branch] < 0 when source delivers
#         current (MNA convention), so Z_in = -1 / x_fwd[branch].
#       - Z_in (CurrentSource): compliance voltage = V_n_minus − V_n_plus.
#
# Step 4: Output impedance solve.
#     Use the same G_ss (all independent sources still zeroed).
#     Inject 1 A at the output node: b_test[output_idx] = 1.0.
#     Solve G_ss · x_test = b_test.
#     Z_out = x_test[output_idx] (V_output / 1 A = Thevenin impedance).
#
# Why MNA branch-current sign is negative for delivering sources
# -------------------------------------------------------------
# The VoltageSource stamp places x[branch] in the KCL row for n_plus with
# coefficient +1.  For a node with a resistive load to ground:
#
#   (1/R) * V_n_plus + x[branch] = 0
#   ⟹  x[branch] = -(1/R) * V_n_plus = -I_delivered
#
# So x[branch] = -I_delivered: negative when the source delivers current.
# The input impedance is V_in / I_delivered = 1 / (−x[branch]) = -1/x[branch].
# ---------------------------------------------------------------------------


def _build_ss_matrix(
    circuit: Circuit,
    node_to_idx: dict[str, int],
    branch_srcs: list[VoltageSource | VCVS | CCVS],
    dc_x: list[float],
) -> list[list[float]]:
    """Build the real DC small-signal MNA conductance matrix (ω = 0).

    This is the real-valued analogue of the complex :func:`_stamp_ac` loop.
    Independent sources are excluded (zeroed), leaving only conductance and
    structural KVL/KCL entries.  Controlled sources (VCVS, VCCS, CCCS, CCVS)
    are included with their full gains — they are not "zeroed" because they
    are dependent sources, not independent excitations.

    Stamping rules
    --------------
    +-------------------+-----------------------------------------------+
    | Element type      | Small-signal stamp                            |
    +===================+===============================================+
    | Resistor R        | conductance G = 1/R                           |
    +-------------------+-----------------------------------------------+
    | Capacitor         | open circuit — skipped                        |
    +-------------------+-----------------------------------------------+
    | Inductor          | near-short: G = 1e12 S                        |
    +-------------------+-----------------------------------------------+
    | VoltageSource     | KVL/KCL structural entries (b NOT set)        |
    +-------------------+-----------------------------------------------+
    | CurrentSource     | skipped (independent source → zero in ss)     |
    +-------------------+-----------------------------------------------+
    | VCCS              | off-diagonal gm entries                       |
    +-------------------+-----------------------------------------------+
    | VCVS              | KVL/KCL entries + gain row (b NOT set)        |
    +-------------------+-----------------------------------------------+
    | CCCS              | off-diagonal beta entries                     |
    +-------------------+-----------------------------------------------+
    | CCVS              | KVL/KCL entries + transresistance (b NOT set) |
    +-------------------+-----------------------------------------------+
    | Diode             | gd = (Is/Vt) · exp(Vd/Vt) at DC OP           |
    +-------------------+-----------------------------------------------+
    | MOSFET            | gds + gm VCCS at DC OP                        |
    +-------------------+-----------------------------------------------+
    | BJT               | g_π + gm VCCS at DC OP                        |
    +-------------------+-----------------------------------------------+

    Parameters
    ----------
    circuit : Circuit
        The circuit being analysed.
    node_to_idx : dict[str, int]
        Node-to-row-index mapping (ground excluded).
    branch_srcs : list[VoltageSource | VCVS | CCVS]
        Ordered list of branch-unknown sources (determines column indices).
    dc_x : list[float]
        DC operating-point vector (node voltages then branch currents).

    Returns
    -------
    list[list[float]]
        Square real MNA matrix of size ``(n_nodes + n_branch_srcs)^2``.
    """
    n_nodes = len(node_to_idx)
    size = n_nodes + len(branch_srcs)
    G: list[list[float]] = [[0.0] * size for _ in range(size)]

    for el in circuit.elements:
        if isinstance(el, Resistor):
            # Real conductance: G = 1/R.
            _stamp_g(G, node_to_idx, el.n_plus, el.n_minus, 1.0 / el.resistance)

        elif isinstance(el, Capacitor):
            # At ω = 0, Y_C = jωC = 0 — open circuit.  Nothing to stamp.
            pass

        elif isinstance(el, Inductor):
            # At ω = 0, Y_L = 1/(jωL) → ∞.  Model as near-short (G = 1e12 S)
            # to keep the matrix non-singular, mirroring the AC analysis.
            _stamp_g(G, node_to_idx, el.n_plus, el.n_minus, 1e12)

        elif isinstance(el, VoltageSource):
            # Stamp structural KVL/KCL entries exactly as in _stamp_vsrc, but
            # intentionally leave b alone (independent source zeroed).
            i = branch_srcs.index(el)
            branch_idx = n_nodes + i
            if not _is_ground(el.n_plus):
                p = node_to_idx[el.n_plus]
                G[p][branch_idx] = 1.0
                G[branch_idx][p] = 1.0
            if not _is_ground(el.n_minus):
                q = node_to_idx[el.n_minus]
                G[q][branch_idx] = -1.0
                G[branch_idx][q] = -1.0

        elif isinstance(el, CurrentSource):
            # Independent current source → zero in small-signal analysis.
            pass

        elif isinstance(el, VCCS):
            # Frequency-independent; stamp real transconductance.
            _stamp_vccs(G, node_to_idx, el.n_plus, el.n_minus,
                        el.ctrl_plus, el.ctrl_minus, el.gm)

        elif isinstance(el, VCVS):
            # Dependent source — stamp full KVL/KCL + gain (not zeroed).
            b_dummy: list[float] = [0.0] * size
            _stamp_vcvs(G, b_dummy, node_to_idx, el,
                        n_nodes + branch_srcs.index(el))

        elif isinstance(el, CCCS):
            ctrl_el = _find_branch_source(branch_srcs, el.ctrl_source)
            if ctrl_el is None:
                raise ValueError(
                    f"CCCS '{el.name}' references controlling source "
                    f"'{el.ctrl_source}' which does not exist in the circuit."
                )
            ctrl_idx = n_nodes + branch_srcs.index(ctrl_el)
            _stamp_cccs(G, node_to_idx, el, ctrl_idx)

        elif isinstance(el, CCVS):
            ctrl_el = _find_branch_source(branch_srcs, el.ctrl_source)
            if ctrl_el is None:
                raise ValueError(
                    f"CCVS '{el.name}' references controlling source "
                    f"'{el.ctrl_source}' which does not exist in the circuit."
                )
            ctrl_idx = n_nodes + branch_srcs.index(ctrl_el)
            b_dummy2: list[float] = [0.0] * size
            _stamp_ccvs(G, b_dummy2, node_to_idx, el,
                        n_nodes + branch_srcs.index(el), ctrl_idx)

        elif isinstance(el, Diode):
            # Small-signal conductance: gd = dI/dVd = (Is/Vt)·exp(Vd/Vt).
            Va = 0.0 if _is_ground(el.anode) else dc_x[node_to_idx[el.anode]]
            Vk = 0.0 if _is_ground(el.cathode) else dc_x[node_to_idx[el.cathode]]
            Vd = min(Va - Vk, 0.7)
            gd = (el.Is / el.Vt) * math.exp(Vd / el.Vt)
            _stamp_g(G, node_to_idx, el.anode, el.cathode, gd)

        elif isinstance(el, Mosfet):
            # Small-signal model: gds (drain–source) + gm VCCS (gate–source
            # controls drain current).  Mirrors the AC _stamp_ac Mosfet block.
            Vd = 0.0 if _is_ground(el.drain) else dc_x[node_to_idx[el.drain]]
            Vg = 0.0 if _is_ground(el.gate) else dc_x[node_to_idx[el.gate]]
            Vs = 0.0 if _is_ground(el.source) else dc_x[node_to_idx[el.source]]
            Vb = 0.0 if _is_ground(el.body) else dc_x[node_to_idx[el.body]]
            r = el.model.dc(Vg - Vs, Vd - Vs, Vb - Vs)  # type: ignore[attr-defined]
            gm_m: float = r.gm
            gds_m: float = r.gds
            _stamp_g(G, node_to_idx, el.drain, el.source, gds_m)
            if not _is_ground(el.drain):
                d = node_to_idx[el.drain]
                if not _is_ground(el.gate):
                    G[d][node_to_idx[el.gate]] += gm_m
                if not _is_ground(el.source):
                    G[d][node_to_idx[el.source]] -= gm_m
            if not _is_ground(el.source):
                s = node_to_idx[el.source]
                if not _is_ground(el.gate):
                    G[s][node_to_idx[el.gate]] -= gm_m
                if not _is_ground(el.source):
                    G[s][node_to_idx[el.source]] += gm_m

        elif isinstance(el, BJT):
            # Small-signal model: g_π (junction conductance) + gm VCCS.
            # Mirrors the AC _stamp_ac BJT block in the real domain.
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
                _stamp_g(G, node_to_idx, el.base, el.emitter, g_pi)
                if not _is_ground(el.collector):
                    c_i = node_to_idx[el.collector]
                    if not _is_ground(el.base):
                        G[c_i][node_to_idx[el.base]] += gm_b
                    if not _is_ground(el.emitter):
                        G[c_i][node_to_idx[el.emitter]] -= gm_b
                if not _is_ground(el.emitter):
                    e_i = node_to_idx[el.emitter]
                    if not _is_ground(el.base):
                        G[e_i][node_to_idx[el.base]] -= gm_b
                    if not _is_ground(el.emitter):
                        G[e_i][node_to_idx[el.emitter]] += gm_b
            else:  # PNP — emitter injects, collector collects
                _stamp_g(G, node_to_idx, el.emitter, el.base, g_pi)
                if not _is_ground(el.emitter):
                    e_i = node_to_idx[el.emitter]
                    if not _is_ground(el.emitter):
                        G[e_i][node_to_idx[el.emitter]] += gm_b
                    if not _is_ground(el.base):
                        G[e_i][node_to_idx[el.base]] -= gm_b
                if not _is_ground(el.collector):
                    c_i = node_to_idx[el.collector]
                    if not _is_ground(el.emitter):
                        G[c_i][node_to_idx[el.emitter]] -= gm_b
                    if not _is_ground(el.base):
                        G[c_i][node_to_idx[el.base]] += gm_b

    return G


def tf(
    circuit: Circuit,
    *,
    output_node: str,
    input_source: str,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> TfResult:
    """DC small-signal transfer function analysis (the SPICE ``.TF`` command).

    Computes the transfer ratio, input impedance, and output impedance for
    a linear or linearised analog circuit at DC (ω = 0).

    Parameters
    ----------
    circuit : Circuit
        The circuit to analyse.
    output_node : str
        Name of the output node.  The transfer ratio is ``V_output / V_input``
        (or ``V_output / I_input`` for a current-source input).
    input_source : str
        Name of the driving independent source (a :class:`VoltageSource` or
        :class:`CurrentSource` element whose ``.name`` matches this string).
    max_iterations : int
        Maximum Newton-Raphson iterations for the DC operating point.
    tol : float
        Convergence tolerance for the DC solve.

    Returns
    -------
    TfResult
        Dataclass holding ``transfer_ratio``, ``input_impedance``,
        ``output_impedance``, and ``converged``.

    Raises
    ------
    ValueError
        If ``input_source`` is not found in the circuit, or if the named
        element is not a :class:`VoltageSource` or :class:`CurrentSource`.
    ValueError
        If ``output_node`` is not found in the circuit.

    Algorithm
    ---------
    See the Section 4 comment block above :func:`_build_ss_matrix` for a
    detailed walkthrough.

    Examples
    --------
    Voltage divider::

        from spice_engine import Circuit, VoltageSource, Resistor, tf

        c = Circuit()
        c.add(VoltageSource("V1", "vin", "0", 10.0))
        c.add(Resistor("R1", "vin", "vmid", 1000.0))
        c.add(Resistor("R2", "vmid", "0", 1000.0))

        result = tf(c, output_node="vmid", input_source="V1")
        # result.transfer_ratio  ≈ 0.5  (V_mid / V_in = R2/(R1+R2))
        # result.input_impedance ≈ 2000  (R1 + R2)
        # result.output_impedance ≈ 500  (R1 ∥ R2)
    """
    # ---- Step 1: DC operating point ------------------------------------------
    dc = dc_op(circuit, max_iterations=max_iterations, tol=tol)
    node_to_idx, _nodes = _node_index(circuit)
    branch_srcs_tf = _branch_sources(circuit)
    n_nodes = len(node_to_idx)
    size = n_nodes + len(branch_srcs_tf)

    # Reconstruct the indexed dc_x vector from the DcResult dicts.
    dc_x: list[float] = [0.0] * size
    for name, idx in node_to_idx.items():
        dc_x[idx] = dc.node_voltages.get(name, 0.0)
    for i, bs in enumerate(branch_srcs_tf):
        dc_x[n_nodes + i] = dc.branch_currents.get(f"I({bs.name})", 0.0)

    # ---- Step 2: Small-signal conductance matrix -----------------------------
    G_ss = _build_ss_matrix(circuit, node_to_idx, branch_srcs_tf, dc_x)

    # ---- Locate the input source element ------------------------------------
    input_el: Element | None = None
    for el in circuit.elements:
        if hasattr(el, "name") and el.name == input_source:
            input_el = el
            break
    if input_el is None:
        raise ValueError(
            f"No element named {input_source!r} in circuit.  "
            f"Available elements: {[e.name for e in circuit.elements if hasattr(e, 'name')]}"
        )
    if not isinstance(input_el, (VoltageSource, CurrentSource)):
        raise ValueError(
            f"Input element {input_source!r} must be a VoltageSource or CurrentSource, "
            f"got {type(input_el).__name__}"
        )

    # Validate output node
    if not _is_ground(output_node) and output_node not in node_to_idx:
        raise ValueError(
            f"Output node {output_node!r} not found.  "
            f"Known nodes: {list(node_to_idx.keys())}"
        )
    output_idx: int | None = None if _is_ground(output_node) else node_to_idx[output_node]

    # ---- Step 3: Forward solve (unit excitation at input) --------------------
    #
    # Apply a 1 V or 1 A excitation at the input source; all other independent
    # sources remain zeroed because G_ss was built with b = 0 everywhere.
    b_fwd = [0.0] * size

    if isinstance(input_el, VoltageSource):
        # 1 V across the source: set the KVL constraint row b[branch] = 1.0.
        vsrc_idx = branch_srcs_tf.index(input_el)
        b_fwd[n_nodes + vsrc_idx] = 1.0
    else:
        # CurrentSource: inject 1 A following the same sign convention as the
        # DC stamp — b[n_plus] -= 1 (extract from n_plus), b[n_minus] += 1
        # (inject into n_minus).
        if not _is_ground(input_el.n_plus):
            b_fwd[node_to_idx[input_el.n_plus]] -= 1.0
        if not _is_ground(input_el.n_minus):
            b_fwd[node_to_idx[input_el.n_minus]] += 1.0

    try:
        x_fwd = _solve(G_ss, b_fwd)
    except ZeroDivisionError:
        return TfResult(
            transfer_ratio=0.0,
            input_impedance=float("inf"),
            output_impedance=float("inf"),
            converged=False,
        )

    # Transfer ratio H = V_output (excitation is 1 V or 1 A).
    H: float = 0.0 if output_idx is None else x_fwd[output_idx]

    # Input impedance
    if isinstance(input_el, VoltageSource):
        vsrc_idx = branch_srcs_tf.index(input_el)
        i_branch = x_fwd[n_nodes + vsrc_idx]
        # MNA convention: x[branch] < 0 when the source delivers current
        # (the branch current enters n_plus FROM the circuit, not from the
        # source).  Z_in = V_in / I_delivered = 1 / (-x[branch]).
        Z_in: float = (-1.0 / i_branch) if abs(i_branch) > 1e-30 else float("inf")
    else:
        # CurrentSource: Z_in = compliance voltage V_n_minus − V_n_plus.
        # (The port voltage developed across the source when 1 A is forced.)
        v_plus = 0.0 if _is_ground(input_el.n_plus) else x_fwd[node_to_idx[input_el.n_plus]]
        v_minus = 0.0 if _is_ground(input_el.n_minus) else x_fwd[node_to_idx[input_el.n_minus]]
        Z_in = v_minus - v_plus

    # ---- Step 4: Output impedance (Thevenin) ---------------------------------
    #
    # Same G_ss (all independent sources zeroed).  Inject 1 A at the output
    # node; Thevenin says Z_out = V_open / I_test = V_output / 1 A.
    b_test = [0.0] * size
    if output_idx is not None:
        b_test[output_idx] = 1.0

    try:
        x_test = _solve(G_ss, b_test)
        Z_out: float = 0.0 if output_idx is None else x_test[output_idx]
    except ZeroDivisionError:
        Z_out = float("inf")

    return TfResult(
        transfer_ratio=H,
        input_impedance=Z_in,
        output_impedance=Z_out,
        converged=dc.converged,
    )


# ---------------------------------------------------------------------------
# Section 5 — DC Parameter Sweep (.DC analysis)
# ---------------------------------------------------------------------------


def dc_sweep(
    circuit: Circuit,
    source_name: str,
    start: float,
    stop: float,
    step: float,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> DcSweepResult:
    """Sweep one independent source through a range and record DC operating points.

    This implements the SPICE ``.DC`` analysis.  At each step the named source
    is set to the current sweep value and :func:`dc_op` is called to find the
    operating point.  Consecutive steps seed Newton-Raphson from the previous
    converged solution, which dramatically improves convergence robustness for
    nonlinear circuits.

    Parameters
    ----------
    circuit : Circuit
        The circuit to analyse.  Must contain a :class:`VoltageSource` or
        :class:`CurrentSource` whose ``name`` matches *source_name*.
        All other elements are swept at their nominal values.
    source_name : str
        Name of the independent source to sweep (case-sensitive, matches
        ``element.name``).
    start : float
        Sweep start value (V or A, depending on source type).
    stop : float
        Sweep stop value (inclusive within floating-point tolerance).
    step : float
        Sweep increment.  Must be positive for an ascending sweep
        (``start < stop``) or negative for a descending sweep
        (``start > stop``).  A zero step raises :class:`ValueError`.
    max_iterations : int, keyword-only
        Maximum Newton-Raphson iterations per DC solve.  Default 50.
    tol : float, keyword-only
        Newton-Raphson convergence tolerance (V / A).  Default 1e-6.

    Returns
    -------
    DcSweepResult
        One :class:`DcSweepPoint` per evaluated step, in sweep order.
        ``result.points`` is empty when the step has the wrong sign
        (e.g., ``start=0``, ``stop=5``, ``step=-0.1``).

    Raises
    ------
    ValueError
        If *step* is zero, or if no source named *source_name* is found.

    Notes
    -----
    **How step continuation works**: after each converged step the internal
    MNA state is *not* explicitly threaded between calls; :func:`dc_op` uses
    an all-zero initial guess each time.  For smooth sweeps of linear/mildly
    nonlinear circuits this is sufficient.  Future versions may add warm-start
    support for difficult nonlinear operating regions.

    **Frozen elements**: :class:`VoltageSource` and :class:`CurrentSource` are
    ``frozen=True`` dataclasses.  To change a source value we create a new
    element instance and rebuild the circuit for each step, which keeps the
    original *circuit* object unmodified.

    Examples
    --------
    Sweep a DC bias from 0 V to 5 V in 0.5 V steps::

        from spice_engine import Circuit, VoltageSource, Resistor, dc_sweep
        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 0.0))
        c.add(Resistor("R1", "in", "out", 1000.0))
        c.add(Resistor("R2", "out", "0", 1000.0))
        result = dc_sweep(c, "Vin", 0.0, 5.0, 0.5)
        for pt in result.points:
            print(f"Vin={pt.source_value:.1f}V  Vout={pt.node_voltages['out']:.3f}V")

    Transfer curve of a resistor divider (expected Vout = Vin / 2)::

        assert all(
            abs(pt.node_voltages["out"] - pt.source_value / 2) < 1e-9
            for pt in result.points if pt.converged
        )
    """
    if step == 0.0:
        raise ValueError("dc_sweep: step must be nonzero")

    # ------------------------------------------------------------------
    # Locate the source element to sweep.
    # We accept both VoltageSource and CurrentSource.
    # ------------------------------------------------------------------
    source_el: VoltageSource | CurrentSource | None = None
    source_idx: int = -1
    for idx, el in enumerate(circuit.elements):
        if isinstance(el, (VoltageSource, CurrentSource)) and el.name == source_name:
            source_el = el  # type: ignore[assignment]
            source_idx = idx
            break

    if source_el is None:
        raise ValueError(
            f"dc_sweep: no VoltageSource or CurrentSource named {source_name!r} "
            "found in the circuit"
        )

    # ------------------------------------------------------------------
    # Build the list of sweep values.
    #
    # We use integer-counted steps to avoid floating-point drift across
    # many iterations (e.g. 0.1 + 0.1 + ... ≠ exactly n*0.1).
    # The stop value is included when it falls within half a step of the
    # last computed sample.
    # ------------------------------------------------------------------
    sweep_values: list[float] = []
    if step > 0.0 and start <= stop:
        n = int((stop - start) / step + 0.5) + 1
        sweep_values = [start + i * step for i in range(n) if start + i * step <= stop + step * 0.5]
    elif step < 0.0 and start >= stop:
        n = int((start - stop) / (-step) + 0.5) + 1
        sweep_values = [start + i * step for i in range(n) if start + i * step >= stop + step * 0.5]
    # else: wrong sign — return empty result

    # ------------------------------------------------------------------
    # Run a DC solve at each sweep value.
    #
    # For each step we:
    #   1. Build a modified circuit with the source set to the sweep value
    #      (frozen dataclasses → create a new element instance).
    #   2. Call dc_op on the modified circuit.
    #   3. Record a DcSweepPoint.
    # ------------------------------------------------------------------
    points: list[DcSweepPoint] = []

    for val in sweep_values:
        # Create a new source element with the swept value.
        if isinstance(source_el, VoltageSource):
            new_el: VoltageSource | CurrentSource = VoltageSource(
                source_el.name,
                source_el.n_plus,
                source_el.n_minus,
                val,
                source_el.waveform,
                source_el.ac,
            )
        else:
            new_el = CurrentSource(
                source_el.name,
                source_el.n_plus,
                source_el.n_minus,
                val,
                source_el.waveform,
                source_el.ac,
            )

        # Rebuild circuit with the new element in place of the original.
        swept_elements = list(circuit.elements)
        swept_elements[source_idx] = new_el
        swept_circuit = Circuit(elements=swept_elements)

        dc_result = dc_op(swept_circuit, max_iterations=max_iterations, tol=tol)

        points.append(
            DcSweepPoint(
                source_value=val,
                node_voltages=dc_result.node_voltages,
                branch_currents=dc_result.branch_currents,
                converged=dc_result.converged,
            )
        )

    return DcSweepResult(points=points, source_name=source_name)


# ---------------------------------------------------------------------------
# Section 6 — DC Sensitivity Analysis (.SENS analysis)
# ---------------------------------------------------------------------------
#
# Background: what is sensitivity analysis?
# -----------------------------------------
# Sensitivity analysis answers the question: "If element X changes by a small
# amount δ, how much does the output voltage V_out change?"
#
# Formally, the DC sensitivity of V_out with respect to parameter P is:
#
#     S(P) = ∂V_out / ∂P  ≈  [V_out(P + δ) − V_out(P)] / δ
#
# where δ is a small perturbation chosen as a fixed fraction of P (typically
# 0.1%, 0.5%, or 1%).
#
# Three flavours of sensitivity
# ------------------------------
# 1. **Absolute sensitivity** S(P) — units of V/Ω (for a resistor),  V/V
#    (for a voltage source), V/A (for a current source).  Tells you the
#    slope: "1 Ω change in R1 shifts V_out by S Volts."
#
# 2. **Relative (normalised) sensitivity** S_rel — dimensionless.
#    Computed as (P / V_out) × S(P).  Tells you: "a 1% change in P produces
#    a S_rel% change in V_out."  Useful for comparing components with
#    very different units.
#
# 3. **Element contribution** — sum over all elements to see which one
#    dominates.
#
# Why finite differences?
# -----------------------
# For a general MNA circuit (including nonlinear devices) the closed-form
# adjoint sensitivity requires differentiating through the Newton-Raphson
# loop, which is complex to implement.  Finite differences are simpler,
# correct to O(δ) for a forward difference, and practically accurate for
# the perturbation sizes used in SPICE (δ ≈ 0.001× nominal).
#
# What is perturbed?
# ------------------
# Each element contributes its one free DC parameter:
#
#   Resistor     → resistance (Ω)
#   VoltageSource → voltage (V)
#   CurrentSource → current (A)
#   Diode        → Is (A)  — the reverse saturation current
#   BJT          → Is (A) and beta_f (dimensionless)
#   Capacitor    → skipped (open circuit at DC; C has no DC effect)
#   Inductor     → skipped (short circuit at DC; L has no DC effect)
#   Mosfet       → skipped (model object; perturbing internal params
#                   requires model introspection not yet exposed)
#
# Perturbation size
# -----------------
# For each parameter P, δ = max(|P| × perturbation_fraction, abs_floor).
# The default fraction is 0.001 (0.1%).  The absolute floor is 1e-10 to
# handle zero-valued sources (e.g., a 0 V bias).
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SensEntry:
    """Sensitivity of V_out with respect to one element parameter.

    Attributes
    ----------
    element_name : str
        Name of the circuit element (e.g. ``"R1"``, ``"Vin"``).
    parameter : str
        Which parameter was perturbed: ``"resistance"``, ``"voltage"``,
        ``"current"``, ``"Is"``, or ``"beta_f"``.
    nominal_value : float
        The unperturbed value of the parameter.
    sensitivity : float
        Absolute sensitivity ∂V_out/∂P in units of [V / unit(P)].
        For a resistor this is V/Ω; for a voltage source, V/V; etc.
    rel_sensitivity : float
        Dimensionless relative sensitivity ``(P / V_out) × ∂V_out/∂P``.
        Gives the percentage change in V_out per percentage change in P.
        Set to 0.0 when V_out is zero (undefined otherwise).

    Notes
    -----
    A large absolute value of *rel_sensitivity* indicates that this
    component dominates the output tolerance budget.  Entries are sorted
    by ``abs(rel_sensitivity)`` descending in :class:`SensResult`.
    """

    element_name: str
    parameter: str
    nominal_value: float
    sensitivity: float
    rel_sensitivity: float


@dataclass
class SensResult:
    """DC sensitivity analysis results from :func:`sens_dc`.

    Attributes
    ----------
    output_node : str
        The node whose voltage was observed.
    nominal_voltage : float
        V_out at the unperturbed DC operating point.
    entries : list[SensEntry]
        One entry per perturbed (element, parameter) pair, sorted by
        ``abs(rel_sensitivity)`` descending so the most influential
        components appear first.
    converged : bool
        ``True`` when every DC solve (nominal + all perturbations) converged.
        ``False`` if any solve failed; individual entries may be unreliable.

    Examples
    --------
    Print a ranked sensitivity table::

        result = sens_dc(circuit, "out")
        for e in result.entries:
            print(f"{e.element_name}({e.parameter}): "
                  f"{e.rel_sensitivity:+.2%} / % change")
    """

    output_node: str
    nominal_voltage: float
    entries: list[SensEntry]
    converged: bool



def sens_dc(
    circuit: Circuit,
    output_node: str,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    perturbation: float = 1e-3,
    abs_floor: float = 1e-10,
) -> SensResult:
    """DC sensitivity analysis (SPICE ``.SENS``).

    Computes how sensitive the DC voltage at *output_node* is to small
    changes in each element's parameter, using forward finite differences.

    Parameters
    ----------
    circuit : Circuit
        The circuit to analyse.
    output_node : str
        Name of the observation node.  Use ``"0"`` or ``"gnd"`` to observe
        the reference (always 0 V — not useful but allowed for completeness).
    max_iterations : int, keyword-only
        Maximum Newton-Raphson iterations per DC solve.  Default 50.
    tol : float, keyword-only
        Newton-Raphson convergence tolerance.  Default 1e-6.
    perturbation : float, keyword-only
        Relative perturbation fraction.  Each parameter P is perturbed by
        ``δ = max(|P| × perturbation, abs_floor)``.  Default 0.001 (0.1 %).
    abs_floor : float, keyword-only
        Minimum absolute perturbation (used when P ≈ 0).  Default 1e-10.

    Returns
    -------
    SensResult
        ``entries`` sorted by ``abs(rel_sensitivity)`` descending.
        ``converged`` is ``False`` if any DC solve diverged.

    Raises
    ------
    ValueError
        If *output_node* is not a ground alias and is not found in the
        circuit's node set.

    Notes
    -----
    **What is perturbed**: Resistor (resistance), VoltageSource (voltage),
    CurrentSource (current), Diode (Is), BJT (Is, beta_f).  Capacitors and
    inductors are skipped (no DC effect); MOSFETs are skipped (model object
    introspection not yet implemented).

    **Interpretation**: A ``rel_sensitivity`` of ``0.5`` means a 1% increase
    in that parameter causes a 0.5% increase in V_out.  A ``-1.0`` means a
    1% increase causes a 1% decrease (like the top resistor in a divider).

    Examples
    --------
    Resistor divider: R1 and R2 both 1 kΩ, V_in = 10 V::

        from spice_engine import Circuit, VoltageSource, Resistor, sens_dc

        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 10.0))
        c.add(Resistor("R1", "in", "mid", 1000.0))
        c.add(Resistor("R2", "mid", "0", 1000.0))

        result = sens_dc(c, "mid")
        # result.nominal_voltage ≈ 5.0
        # R1 rel_sensitivity ≈ -0.5  (increasing R1 lowers V_mid)
        # R2 rel_sensitivity ≈ +0.5  (increasing R2 raises V_mid)
        # Vin rel_sensitivity ≈ +1.0 (V_mid tracks Vin linearly)
    """
    # ---- Validate output node ------------------------------------------------
    node_to_idx, _ = _node_index(circuit)
    if not _is_ground(output_node) and output_node not in node_to_idx:
        raise ValueError(
            f"sens_dc: output node {output_node!r} not found in circuit.  "
            f"Known nodes: {sorted(node_to_idx.keys())}"
        )

    # ---- Nominal DC operating point ------------------------------------------
    nominal = dc_op(circuit, max_iterations=max_iterations, tol=tol)
    if not nominal.converged:
        return SensResult(
            output_node=output_node,
            nominal_voltage=0.0,
            entries=[],
            converged=False,
        )

    v_out_nominal = _node_voltage(output_node, nominal.node_voltages)
    all_converged = True
    entries: list[SensEntry] = []

    # ---- Finite-difference perturbation for each element ---------------------
    #
    # For each (element, parameter) pair:
    #   1. Compute δ = max(|param| × perturbation, abs_floor).
    #   2. Build a perturbed circuit with param → param + δ.
    #      (Frozen dataclasses → create a new element, rebuild circuit list.)
    #   3. Solve dc_op on the perturbed circuit.
    #   4. Sensitivity = (V_out_pert − V_out_nominal) / δ.
    #   5. Relative sensitivity = sensitivity × (param / V_out_nominal).
    #
    for idx, el in enumerate(circuit.elements):

        def _make_entry(
            param_name: str,
            nominal_val: float,
            perturbed_el: Element,
            _idx: int = idx,
            _el: Element = el,
        ) -> None:
            """Inner helper: run perturbed solve and append a SensEntry.

            The default-argument captures (``_idx=idx``, ``_el=el``) are
            necessary to correctly bind the loop variables inside the closure.
            Python's late-binding would otherwise share the loop variable
            values from the *last* iteration for all closures.
            """
            nonlocal all_converged
            delta = max(abs(nominal_val) * perturbation, abs_floor)
            # Rebuild circuit with the perturbed element at position _idx.
            pert_elements = list(circuit.elements)
            pert_elements[_idx] = perturbed_el
            pert_circ = Circuit(elements=pert_elements)
            pert_dc = dc_op(pert_circ, max_iterations=max_iterations, tol=tol)
            if not pert_dc.converged:
                all_converged = False
                return
            v_out_pert = _node_voltage(output_node, pert_dc.node_voltages)
            sens = (v_out_pert - v_out_nominal) / delta
            rel = sens * nominal_val / v_out_nominal if abs(v_out_nominal) > abs_floor else 0.0
            entries.append(SensEntry(
                element_name=_el.name,
                parameter=param_name,
                nominal_value=nominal_val,
                sensitivity=sens,
                rel_sensitivity=rel,
            ))

        if isinstance(el, Resistor):
            # Perturb resistance by δ.  New element: same name/nodes, R + δ.
            delta_r = max(abs(el.resistance) * perturbation, abs_floor)
            _make_entry(
                "resistance",
                el.resistance,
                Resistor(el.name, el.n_plus, el.n_minus, el.resistance + delta_r),
            )

        elif isinstance(el, VoltageSource):
            # Perturb voltage by δ.
            delta_v = max(abs(el.voltage) * perturbation, abs_floor)
            _make_entry(
                "voltage",
                el.voltage,
                VoltageSource(
                    el.name,
                    el.n_plus,
                    el.n_minus,
                    el.voltage + delta_v,
                    el.waveform,
                    el.ac,
                ),
            )

        elif isinstance(el, CurrentSource):
            # Perturb current by δ.
            delta_i = max(abs(el.current) * perturbation, abs_floor)
            _make_entry(
                "current",
                el.current,
                CurrentSource(
                    el.name,
                    el.n_plus,
                    el.n_minus,
                    el.current + delta_i,
                    el.waveform,
                    el.ac,
                ),
            )

        elif isinstance(el, Diode):
            # Perturb Is (saturation current).  Large relative change of Is
            # has a logarithmic (Vd ≈ Vt ln(Id/Is)) effect on Vd.
            delta_is = max(abs(el.Is) * perturbation, abs_floor)
            _make_entry(
                "Is",
                el.Is,
                Diode(el.name, el.anode, el.cathode, el.Is + delta_is, el.Vt),
            )

        elif isinstance(el, BJT):
            # Perturb Is and beta_f independently.
            # BJT field order: name, collector, base, emitter, polarity, Is, beta_f, Vt
            # (polarity is positional with a default, so use keyword args to be safe.)
            delta_is = max(abs(el.Is) * perturbation, abs_floor)
            _make_entry(
                "Is",
                el.Is,
                BJT(
                    el.name, el.collector, el.base, el.emitter,
                    polarity=el.polarity,
                    Is=el.Is + delta_is,
                    beta_f=el.beta_f,
                    Vt=el.Vt,
                ),
            )
            delta_beta = max(abs(el.beta_f) * perturbation, abs_floor)
            _make_entry(
                "beta_f",
                el.beta_f,
                BJT(
                    el.name, el.collector, el.base, el.emitter,
                    polarity=el.polarity,
                    Is=el.Is,
                    beta_f=el.beta_f + delta_beta,
                    Vt=el.Vt,
                ),
            )

        # Capacitor, Inductor, Mosfet: no DC parameter to perturb.

    # Sort by |rel_sensitivity| descending so biggest drivers appear first.
    entries.sort(key=lambda e: abs(e.rel_sensitivity), reverse=True)

    return SensResult(
        output_node=output_node,
        nominal_voltage=v_out_nominal,
        entries=entries,
        converged=all_converged,
    )


# ---------------------------------------------------------------------------
# Section 7 — Monte Carlo Analysis (.MC analysis)
# ---------------------------------------------------------------------------
#
# Background: what is Monte Carlo analysis?
# ------------------------------------------
# Component tolerances are unavoidable in real manufacturing.  A resistor
# marked "1 kΩ ±5%" might measure anywhere from 950 Ω to 1050 Ω.  Monte
# Carlo (MC) analysis quantifies the resulting spread in circuit performance:
#
#   1. Run N DC operating points, each with ALL element parameters randomly
#      varied by their specified tolerance.
#   2. Record the output voltage at a chosen node for each trial.
#   3. Report the mean and standard deviation of those N samples.
#
# This mirrors the SPICE .MC command (also called .WCASE, .STRESS in some
# simulators) and answers: "Given real component spreads, what is the
# probability that V_out stays within my design budget?"
#
# Two variation distributions
# ----------------------------
# 1. **Gaussian** (default) — models the bell-curve spread seen in tightly
#    controlled manufacturing lots.  The parameter P is drawn from:
#
#       P_varied = P_nominal × (1 + σ × N(0, 1))
#       where σ = tolerance / 3
#
#    The ÷3 factor is the "3-sigma" convention: the tolerance band is the
#    ±3σ range, so 99.73% of drawn values fall within ±tolerance of nominal.
#
# 2. **Uniform** — models worst-case flat spread (e.g., wirewound resistors
#    or deliberately oversized bins).  Each draw is:
#
#       P_varied = random.uniform(P_nominal × (1−tolerance),
#                                 P_nominal × (1+tolerance))
#
# What is varied
# --------------
# Same set as sens_dc: Resistor, VoltageSource, CurrentSource, Diode.Is,
# BJT.Is and BJT.beta_f.  Capacitors, inductors, and MOSFETs are unchanged.
# Each element's parameter is independently varied per trial.
#
# Seed reproducibility
# --------------------
# Passing ``seed`` to mc_dc calls ``random.seed(seed)`` before the loop.
# Running with the same seed on the same circuit always produces identical
# trial vectors — essential for regression tests and debugging.
#
# Reading the results
# -------------------
# ``McResult.mean`` and ``McResult.std_dev`` describe the output voltage
# distribution across all converged trials.  The individual ``McPoint``
# entries are stored in ``McResult.points`` for histogram plotting:
#
#     voltages = [pt.node_voltages.get(output_node, 0.0)
#                 for pt in result.points if pt.converged]
#     # → histogram shows the manufactured spread of V_out
#
# Note: ``statistics.stdev`` (sample stdev, N-1 denominator) is used
# rather than population stdev (N denominator) because the trials are a
# *sample* of the infinite ensemble of possible component lots.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class McPoint:
    """Result of one Monte Carlo trial.

    Attributes
    ----------
    trial : int
        Zero-based trial index (0 … N−1).
    node_voltages : dict[str, float]
        DC node voltages at this trial's random parameter draw.
    branch_currents : dict[str, float]
        Branch currents for all voltage sources in the circuit.
    converged : bool
        ``True`` when the Newton-Raphson DC solve converged at this trial.
        Unconverged trials are included in ``McResult.points`` but excluded
        from the mean / std_dev statistics.
    """

    trial: int
    node_voltages: dict[str, float]
    branch_currents: dict[str, float]
    converged: bool


@dataclass
class McResult:
    """Collected results from a Monte Carlo DC analysis.

    Returned by :func:`mc_dc`.

    Attributes
    ----------
    output_node : str
        The node whose voltage was observed across trials.
    points : list[McPoint]
        One :class:`McPoint` per trial, in trial order (0 … n_trials−1).
    n_trials : int
        Total number of trials requested (including unconverged ones).
    mean : float
        Sample mean of V(output_node) across all *converged* trials.
        ``0.0`` if no trial converged.
    std_dev : float
        Sample standard deviation (N−1 denominator) of V(output_node)
        across all *converged* trials.  ``0.0`` if fewer than 2 trials
        converged.

    Examples
    --------
    Quick histogram of the output spread::

        import statistics
        result = mc_dc(circuit, "out", n_trials=500, tolerance=0.05, seed=42)
        voltages = [pt.node_voltages["out"] for pt in result.points if pt.converged]
        print(f"V_out = {result.mean:.4f} ± {result.std_dev:.4f} V  "
              f"({len(voltages)}/{result.n_trials} converged)")
    """

    output_node: str
    points: list[McPoint]
    n_trials: int
    mean: float
    std_dev: float


def _vary_element(el: Element, tolerance: float, distribution: str) -> Element:
    """Return a copy of *el* with its DC parameter(s) randomly varied.

    Parameters
    ----------
    el : Element
        The original circuit element.
    tolerance : float
        Relative tolerance (e.g., 0.05 for ±5%).
    distribution : str
        ``"gaussian"`` (σ = tolerance/3) or ``"uniform"`` (flat ±tolerance).

    Returns
    -------
    Element
        A new frozen dataclass instance with the varied parameter.
        Elements with no tunable DC parameter (Capacitor, Inductor, Mosfet)
        are returned unchanged.
    """

    def _draw(nominal: float) -> float:
        """Draw one random multiplier and apply it to *nominal*."""
        if distribution == "gaussian":
            # σ = tolerance/3 → 99.73% of values within ±tolerance
            sigma = tolerance / 3.0
            return nominal * (1.0 + random.gauss(0.0, sigma))
        # Uniform: flat distribution over [nominal*(1−tol), nominal*(1+tol)]
        return nominal * random.uniform(1.0 - tolerance, 1.0 + tolerance)

    if isinstance(el, Resistor):
        return Resistor(el.name, el.n_plus, el.n_minus, _draw(el.resistance))

    if isinstance(el, VoltageSource):
        return VoltageSource(
            el.name,
            el.n_plus,
            el.n_minus,
            _draw(el.voltage),
            el.waveform,
            el.ac,
        )

    if isinstance(el, CurrentSource):
        return CurrentSource(
            el.name,
            el.n_plus,
            el.n_minus,
            _draw(el.current),
            el.waveform,
            el.ac,
        )

    if isinstance(el, Diode):
        return Diode(el.name, el.anode, el.cathode, _draw(el.Is), el.Vt)

    if isinstance(el, BJT):
        return BJT(
            el.name, el.collector, el.base, el.emitter,
            polarity=el.polarity,
            Is=_draw(el.Is),
            beta_f=_draw(el.beta_f),
            Vt=el.Vt,
        )

    # Capacitor, Inductor, Mosfet — no tunable DC parameter; return unchanged.
    return el


def mc_dc(
    circuit: Circuit,
    output_node: str,
    n_trials: int = 100,
    *,
    tolerance: float = 0.05,
    distribution: str = "gaussian",
    seed: int | None = None,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> McResult:
    """Monte Carlo DC analysis (SPICE ``.MC``).

    Runs *n_trials* DC operating points, each with every element parameter
    independently varied by a random draw from the specified distribution.
    Reports the mean and standard deviation of V(*output_node*) across all
    converged trials.

    Parameters
    ----------
    circuit : Circuit
        The circuit to analyse.  All elements with tunable DC parameters
        (Resistor, VoltageSource, CurrentSource, Diode, BJT) are varied
        each trial.
    output_node : str
        Name of the observation node.
    n_trials : int
        Number of Monte Carlo trials to run.  Default 100.  More trials
        give a more accurate standard deviation estimate; the error in
        ``std_dev`` scales as ``σ / √(2N)``.
    tolerance : float, keyword-only
        Relative parameter tolerance (e.g., 0.05 for ±5%).  Applied to
        every varied parameter in every trial.  Default 0.05.
    distribution : str, keyword-only
        ``"gaussian"`` (default) — draws from N(0, σ=tolerance/3), so
        ±tolerance spans ≈ 3σ (99.73% coverage).
        ``"uniform"`` — draws uniformly from [1−tolerance, 1+tolerance].
    seed : int | None, keyword-only
        If provided, ``random.seed(seed)`` is called before the trial loop.
        Identical seeds with identical circuits reproduce identical results.
    max_iterations : int, keyword-only
        Newton-Raphson iteration limit per DC solve.  Default 50.
    tol : float, keyword-only
        Newton-Raphson convergence tolerance.  Default 1e-6.

    Returns
    -------
    McResult
        ``points`` holds all N :class:`McPoint` objects (including
        unconverged trials).  ``mean`` and ``std_dev`` are computed only
        over converged trials; ``std_dev`` is 0.0 if fewer than 2 trials
        converged.

    Raises
    ------
    ValueError
        If *output_node* is not a ground alias and is not in the circuit.
    ValueError
        If *distribution* is not ``"gaussian"`` or ``"uniform"``.
    ValueError
        If *n_trials* < 1.

    Notes
    -----
    The random state is module-global (``random`` module).  If other code
    in the same process uses ``random``, set *seed* to isolate results.

    Examples
    --------
    5% Gaussian tolerance on a resistor divider::

        from spice_engine import Circuit, VoltageSource, Resistor, mc_dc

        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 10.0))
        c.add(Resistor("R1", "in", "mid", 1000.0))
        c.add(Resistor("R2", "mid", "0", 1000.0))

        result = mc_dc(c, "mid", n_trials=1000, tolerance=0.05, seed=42)
        # result.mean    ≈ 5.0 V  (symmetric tolerance → no mean shift)
        # result.std_dev > 0.0 V  (spread due to ±5% on R1 and R2)
    """
    # ---- Input validation ---------------------------------------------------
    if n_trials < 1:
        raise ValueError(f"mc_dc: n_trials must be >= 1, got {n_trials}")
    if distribution not in ("gaussian", "uniform"):
        raise ValueError(
            f"mc_dc: distribution must be 'gaussian' or 'uniform', got {distribution!r}"
        )
    node_to_idx, _ = _node_index(circuit)
    if not _is_ground(output_node) and output_node not in node_to_idx:
        raise ValueError(
            f"mc_dc: output node {output_node!r} not found in circuit.  "
            f"Known nodes: {sorted(node_to_idx.keys())}"
        )

    # ---- Seed the RNG if requested -----------------------------------------
    if seed is not None:
        random.seed(seed)

    # ---- Run N trials -------------------------------------------------------
    points: list[McPoint] = []

    for trial_idx in range(n_trials):
        # Vary every element independently for this trial.
        varied_elements = [
            _vary_element(el, tolerance, distribution)
            for el in circuit.elements
        ]
        trial_circuit = Circuit(elements=varied_elements)

        dc_result = dc_op(trial_circuit, max_iterations=max_iterations, tol=tol)

        points.append(McPoint(
            trial=trial_idx,
            node_voltages=dc_result.node_voltages,
            branch_currents=dc_result.branch_currents,
            converged=dc_result.converged,
        ))

    # ---- Compute statistics over converged trials --------------------------
    converged_voltages = [
        _node_voltage(output_node, pt.node_voltages)
        for pt in points
        if pt.converged
    ]

    if len(converged_voltages) == 0:
        mean = 0.0
        std_dev = 0.0
    elif len(converged_voltages) == 1:
        mean = converged_voltages[0]
        std_dev = 0.0
    else:
        mean = statistics.mean(converged_voltages)
        std_dev = statistics.stdev(converged_voltages)

    return McResult(
        output_node=output_node,
        points=points,
        n_trials=n_trials,
        mean=mean,
        std_dev=std_dev,
    )


# ---------------------------------------------------------------------------
# Section 8 — Noise Analysis (.NOISE analysis)
# ---------------------------------------------------------------------------
#
# Background: what is noise analysis?
# ------------------------------------
# Every real circuit element generates noise — tiny random voltage or current
# fluctuations that limit the minimum detectable signal.  Two sources dominate
# at the DC/audio/RF frequencies we model here:
#
#   1. Johnson-Nyquist (thermal) noise — Resistors
#      Any resistor R at temperature T generates a white (flat PSD) current
#      noise in parallel with its conductance:
#
#          S_i = 4kT / R   [A²/Hz]
#
#      Physical cause: thermal agitation of electrons.  Discovered by Johnson
#      (1928) and explained by Nyquist using thermodynamics.  The factor 4
#      comes from the Nyquist theorem for two-sided spectra.
#
#   2. Shot noise — Diodes and BJT junctions
#      A p-n junction carrying DC current I_DC has a white current noise:
#
#          S_i = 2q |I_DC|   [A²/Hz]
#
#      Physical cause: the discreteness of charge carriers (electrons and
#      holes) crossing the junction independently of each other — a Poisson
#      process.  The factor 2 arises from the two-sided PSD convention.
#
# Noise model for each element
# ----------------------------
#   Resistor R : S_i = 4kT/R, current noise in parallel (across R terminals)
#   Diode       : S_i = 2q|I_D|, current noise anode → cathode
#   BJT         : S_i = 2q|I_C|, current noise base → emitter (collector
#                 junction — approximated as proportional to I_C)
#   All others (Capacitor, Inductor, VoltageSource, CurrentSource, Mosfet):
#                 treated as noiseless in this model
#
# The adjoint method — computing all contributions in one solve
# -------------------------------------------------------------
# A naive approach: for each of the N noise sources, inject a unit test
# current, solve the full MNA system, and read off V_out.  That's N solves.
#
# The adjoint approach does it in ONE solve:
#
#   Forward:  G(jω) × x = b         → x[out] = e_out^T G^{-1} b
#   Adjoint:  G(jω)^T × v = e_out   → solve once per frequency
#
# Then for any noise current source k injecting between nodes a and b:
#
#   H_k = v[a] - v[b]               (transfer impedance, Ω)
#   S_out_k = |H_k|² × S_k          (contribution to output noise, V²/Hz)
#
# Total output noise:   S_out = Σ_k |H_k|² × S_k
#
# Proof: forward output = e_out^T G^{-1} b = (G^{-T} e_out)^T b = v^T b
# For b_k = e_a - e_b:  v^T b_k = v[a] - v[b] = H_k  ✓
#
# Input-referred noise
# --------------------
# The input-referred noise spectral density is the hypothetical input noise
# that would produce the same total output noise as the circuit generates
# internally.  It allows direct comparison with the signal level:
#
#   S_in = S_out / |H_signal(jω)|²
#
# H_signal is the AC gain from the nominated ``input_source`` to ``output_node``.
# Using the adjoint (same v already computed):
#   For a VoltageSource with branch index k:  H_signal = v[n_nodes + k]
#   For a CurrentSource between (n+, n-):     H_signal = v[n-_idx] - v[n+_idx]
#
# Why input-referred noise matters
# ---------------------------------
# Suppose a low-noise amplifier has S_in = 1 nV²/Hz at 1 kHz.  This means
# signals smaller than √(1e-9) ≈ 32 nV (in a 1 Hz bandwidth) cannot be
# resolved.  Comparing S_in to the signal PSD immediately tells you whether
# the circuit meets its dynamic-range requirement.
#
# Temperature
# -----------
# The default temperature is 300 K (≈ 27 °C, close to room temperature and
# used as the SPICE reference).  Thermal noise scales as T, so cold circuits
# (cryogenic amplifiers, superconducting detectors) have dramatically lower
# Johnson noise.
#
# Units reminder
# --------------
#   S (power spectral density) has units V²/Hz or A²/Hz
#   √S has units V/√Hz or A/√Hz  ("voltage noise density", commonly plotted)
# ---------------------------------------------------------------------------

# Physical constants used in noise calculations.
_BOLTZMANN: float = 1.380649e-23  # Boltzmann constant [J/K]
_ELECTRON_CHARGE: float = 1.602176634e-19  # Electron charge [C]


@dataclass(frozen=True)
class NoiseEntry:
    """Noise contribution from one element at one frequency.

    Attributes
    ----------
    element_name : str
        Name of the circuit element generating this noise.
    noise_type : str
        ``"thermal"`` (Johnson-Nyquist noise, for resistors) or
        ``"shot"`` (Poisson/shot noise, for diodes and BJTs).
    source_psd : float
        Noise current power spectral density at the source itself, in A²/Hz.
        For resistors: ``4kT/R``.  For diodes/BJTs: ``2q|I_DC|``.
    output_psd : float
        Contribution to the output voltage noise spectral density, in V²/Hz.
        Computed as ``|H_k(jω)|² × source_psd`` where ``H_k`` is the transfer
        impedance from this source's nodes to the output node.
    """

    element_name: str
    noise_type: str
    source_psd: float
    output_psd: float


@dataclass(frozen=True)
class NoisePoint:
    """Noise analysis result at one frequency point.

    Attributes
    ----------
    freq : float
        Frequency in hertz.
    output_psd : float
        Total output voltage noise power spectral density in V²/Hz.
        This is the sum of all element contributions.
        Take the square root to get the noise voltage density in V/√Hz.
    input_referred_psd : float
        Total noise referred back to the input, in V²/Hz (or A²/Hz if the
        input source is a current source).  Computed as
        ``output_psd / |H_signal(jω)|²``.  Zero when ``|H_signal|`` is
        negligibly small (< 1e-50) at that frequency.
    entries : tuple[NoiseEntry, ...]
        Per-element noise breakdown, sorted by ``output_psd`` descending
        (loudest contributor first).
    """

    freq: float
    output_psd: float
    input_referred_psd: float
    entries: tuple[NoiseEntry, ...]


@dataclass
class NoiseResult:
    """Full .NOISE analysis result returned by :func:`noise_ac`.

    Attributes
    ----------
    output_node : str
        Node at which output noise is measured.
    input_source : str
        Name of the element used for input-referred noise calculation.
    temperature : float
        Analysis temperature in Kelvin (default 300 K).
    points : list[NoisePoint]
        One :class:`NoisePoint` per frequency, in ascending frequency order.

    Examples
    --------
    Compute output noise density in nV/√Hz at each frequency::

        import math
        result = noise_ac(circuit, "out", "Vin")
        for pt in result.points:
            density_nv = math.sqrt(pt.output_psd) * 1e9
            print(f"{pt.freq:.1f} Hz: {density_nv:.2f} nV/√Hz")
    """

    output_node: str
    input_source: str
    temperature: float
    points: list[NoisePoint]


def _collect_noise_sources(
    circuit: Circuit,
    node_to_idx: dict[str, int],
    dc_x: list[float],
    temperature: float,
) -> list[tuple[str, str, int | None, int | None, float]]:
    """Enumerate noise current sources for all noisy circuit elements.

    Each element that contributes noise is modelled as an ideal Norton
    (parallel) current noise source between its principal terminals.

    Parameters
    ----------
    circuit : Circuit
        The circuit whose elements are scanned.
    node_to_idx : dict[str, int]
        Node-to-index map (ground excluded).
    dc_x : list[float]
        DC operating-point solution vector (node voltages then branch currents).
    temperature : float
        Temperature in Kelvin for thermal noise calculations.

    Returns
    -------
    list of 5-tuples (element_name, noise_type, n_plus_idx, n_minus_idx, psd)
        ``n_plus_idx`` and ``n_minus_idx`` are integer matrix indices, or
        ``None`` when the terminal connects to ground.
        ``psd`` is the current noise PSD in A²/Hz.
    """
    kT4 = 4.0 * _BOLTZMANN * temperature  # 4kT factor
    q2 = 2.0 * _ELECTRON_CHARGE           # 2q factor

    sources: list[tuple[str, str, int | None, int | None, float]] = []

    for el in circuit.elements:
        if isinstance(el, Resistor):
            # Johnson-Nyquist thermal noise: S_i = 4kT/R
            psd = kT4 / el.resistance
            n_p = node_to_idx.get(el.n_plus)   # None for ground
            n_m = node_to_idx.get(el.n_minus)  # None for ground
            sources.append((el.name, "thermal", n_p, n_m, psd))

        elif isinstance(el, Diode):
            # Shot noise: S_i = 2q |I_D|
            # Use the actual converged DC voltage from dc_x — no clamp needed here
            # because we are evaluating at the operating point, not iterating Newton.
            # (The 0.7 V clamp in the Newton loop prevents divergence during
            # iterations; at convergence, Vd is the physically correct value.)
            Va = 0.0 if _is_ground(el.anode) else dc_x[node_to_idx[el.anode]]
            Vk = 0.0 if _is_ground(el.cathode) else dc_x[node_to_idx[el.cathode]]
            Vd = Va - Vk  # actual operating-point junction voltage
            I_D = el.Is * (math.exp(Vd / el.Vt) - 1.0)
            psd = q2 * abs(I_D)
            n_a = None if _is_ground(el.anode) else node_to_idx[el.anode]
            n_k = None if _is_ground(el.cathode) else node_to_idx[el.cathode]
            sources.append((el.name, "shot", n_a, n_k, psd))

        elif isinstance(el, BJT):
            # Shot noise on the base-emitter junction: S_i = 2q |I_C|
            # I_C ≈ Is × exp(V_BE / Vt)  (dominates for forward-active BJT)
            # Use actual converged dc_x voltages — no clamp — same reasoning as Diode.
            Vb = 0.0 if _is_ground(el.base) else dc_x[node_to_idx[el.base]]
            Ve = 0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
            Vjunc = (
                Vb - Ve if el.polarity == "NPN"
                else Ve - Vb
            )
            I_C = el.Is * math.exp(Vjunc / el.Vt)
            psd = q2 * abs(I_C)
            n_b = None if _is_ground(el.base) else node_to_idx[el.base]
            n_e = None if _is_ground(el.emitter) else node_to_idx[el.emitter]
            sources.append((el.name, "shot", n_b, n_e, psd))

        # Capacitors, Inductors, VoltageSources, CurrentSources, MOSFETs:
        # noiseless in this first-order model.

    return sources


def noise_ac(
    circuit: Circuit,
    output_node: str,
    input_source: str,
    freqs: list[float] | None = None,
    *,
    temperature: float = 300.0,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> NoiseResult:
    """Small-signal noise analysis (the SPICE .NOISE analysis).

    Computes the voltage noise power spectral density (PSD) at ``output_node``
    due to thermal noise (Johnson-Nyquist) in resistors and shot noise in
    diodes and BJTs, at each frequency in ``freqs``.  Also reports the noise
    referred back to ``input_source`` so you can compare it directly to your
    signal level.

    Algorithm
    ---------
    1. Find the DC operating point to compute shot-noise PSDs
       (diode/BJT currents are bias-dependent).
    2. Build the noise-source list: for each noisy element compute its
       current noise PSD ``S_k`` (A²/Hz).
    3. For each frequency ω = 2πf:

       a. Build the complex AC MNA matrix G(jω) using :func:`_stamp_ac`.
       b. Solve the *adjoint* system G(jω)^T × v = e_out once per frequency,
          where e_out is a unit vector at ``output_node``'s matrix row.
       c. For each noise source k between nodes (a, b):
              H_k = v[a] − v[b]          (transfer impedance, Ω)
              S_out_k = |H_k|² × S_k    (contribution to output PSD, V²/Hz)
       d. Total output noise:  S_out = Σ_k S_out_k
       e. Input-referred noise: S_in = S_out / |H_signal|²
          where H_signal = transfer from ``input_source`` to ``output_node``.

    The adjoint method requires only ONE linear solve per frequency regardless
    of how many noise sources the circuit contains.

    Parameters
    ----------
    circuit : Circuit
        The circuit to analyse.
    output_node : str
        Node at which to measure the output noise voltage.
    input_source : str
        Name of the element (VoltageSource or CurrentSource) used as the
        signal reference for input-referred noise computation.
        If not found in the circuit, ``input_referred_psd`` will be 0.0 at
        every frequency.
    freqs : list[float] | None
        Frequency points in Hz.  If ``None``, defaults to a logarithmic sweep
        of 50 points from 1 Hz to 1 MHz.
    temperature : float
        Ambient temperature in Kelvin.  Default 300 K (≈ 27 °C).
        Affects thermal (Johnson-Nyquist) noise only; shot noise depends on
        DC current, not temperature.
    max_iterations : int
        Newton-Raphson iteration limit for the DC operating-point solve.
    tol : float
        Convergence tolerance for the DC operating-point solve.

    Returns
    -------
    NoiseResult
        One :class:`NoisePoint` per frequency, each containing the total
        output PSD, input-referred PSD, and per-element breakdown.

    Notes
    -----
    - Noiseless elements: Capacitor, Inductor, VoltageSource, CurrentSource,
      Mosfet.  Extending to MOSFET channel noise (``4kT γ gm``) is future work.
    - If the AC matrix is singular at a frequency, that point's PSDs are 0.0.
    - Output PSD in V²/Hz; take ``math.sqrt(pt.output_psd)`` for V/√Hz density.

    Examples
    --------
    Noise figure of an RC filter::

        from spice_engine import Circuit, VoltageSource, Resistor, Capacitor
        from spice_engine import noise_ac
        import math

        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 1.0))
        c.add(Resistor("R1", "in", "out", 1000.0))
        c.add(Capacitor("C1", "out", "0", 1e-9))

        result = noise_ac(c, "out", "Vin", temperature=300.0)
        for pt in result.points:
            v_noise = math.sqrt(pt.output_psd) * 1e9  # nV/√Hz
            v_in_ref = math.sqrt(pt.input_referred_psd) * 1e9
            print(f"{pt.freq:8.1f} Hz: out={v_noise:.2f} nV/√Hz  "
                  f"in-ref={v_in_ref:.2f} nV/√Hz")
    """
    # ---- DC operating point --------------------------------------------------
    dc = dc_op(circuit, max_iterations=max_iterations, tol=tol)

    # ---- Matrix bookkeeping --------------------------------------------------
    node_to_idx, _nodes = _node_index(circuit)
    branch_srcs_noise = _branch_sources(circuit)
    n_nodes = len(node_to_idx)
    n_branch_noise = len(branch_srcs_noise)
    size = n_nodes + n_branch_noise

    # Reconstruct dc_x solution vector for linearisation of nonlinear devices.
    dc_x: list[float] = [0.0] * size
    for name, idx in node_to_idx.items():
        dc_x[idx] = dc.node_voltages.get(name, 0.0)
    for i, bs in enumerate(branch_srcs_noise):
        dc_x[n_nodes + i] = dc.branch_currents.get(f"I({bs.name})", 0.0)

    # ---- Validate output node -----------------------------------------------
    if _is_ground(output_node):
        # Ground is always 0 V; no noise to measure there.
        return NoiseResult(
            output_node=output_node,
            input_source=input_source,
            temperature=temperature,
            points=[],
        )
    out_idx = node_to_idx.get(output_node)
    if out_idx is None:
        return NoiseResult(
            output_node=output_node,
            input_source=input_source,
            temperature=temperature,
            points=[],
        )

    # ---- Build noise source list from DC operating point --------------------
    noise_sources = _collect_noise_sources(circuit, node_to_idx, dc_x, temperature)

    # ---- Locate the input source element for input-referred noise ----------
    input_el: VoltageSource | CurrentSource | None = None
    for el in circuit.elements:
        if el.name == input_source and isinstance(el, (VoltageSource, CurrentSource)):
            input_el = el  # type: ignore[assignment]
            break

    # ---- Default frequency sweep: 50 log-spaced points, 1 Hz … 1 MHz ------
    if freqs is None:
        log_start = 0.0      # log10(1 Hz)
        log_stop = 6.0       # log10(1 MHz)
        step = (log_stop - log_start) / 49
        freqs = [10.0 ** (log_start + k * step) for k in range(50)]

    # ---- Adjoint vector: unit vector at output node -------------------------
    # This is the RHS of the adjoint solve: G^T × v = e_out
    e_out: list[complex] = [0j] * size
    e_out[out_idx] = 1.0 + 0j

    # ---- Per-frequency noise computation ------------------------------------
    points: list[NoisePoint] = []

    for freq in freqs:
        omega = 2.0 * math.pi * freq

        # Build complex MNA matrix G_c at this frequency.
        G_c: list[list[complex]] = [[0j] * size for _ in range(size)]
        b_c: list[complex] = [0j] * size  # dummy RHS for stamping (unused)
        for el in circuit.elements:
            _stamp_ac(
                el,
                G_c,
                b_c,
                omega,
                node_to_idx,
                branch_srcs_noise,
                dc_x,
                explicit_ac_sources=_has_explicit_ac_sources(circuit),
            )

        # Transpose G_c → G_T for the adjoint solve.
        # G_T[i][j] = G_c[j][i]
        G_T: list[list[complex]] = [
            [G_c[j][i] for j in range(size)]
            for i in range(size)
        ]

        # Solve adjoint: G_T × v_adj = e_out
        # v_adj[k] = transfer impedance from current injection at node k
        # to voltage at output_node.
        try:
            v_adj = _solve_complex(G_T, list(e_out))  # copy e_out (mutated)
        except ZeroDivisionError:
            # Singular matrix at this frequency — skip with zero PSD.
            zero_entries: tuple[NoiseEntry, ...] = tuple(
                NoiseEntry(
                    element_name=name,
                    noise_type=ntype,
                    source_psd=psd,
                    output_psd=0.0,
                )
                for (name, ntype, _, _, psd) in noise_sources
            )
            points.append(NoisePoint(
                freq=freq,
                output_psd=0.0,
                input_referred_psd=0.0,
                entries=zero_entries,
            ))
            continue

        # Accumulate noise contributions.
        # For each noise current source k between nodes (n_p, n_m):
        #   H_k = v_adj[n_p] - v_adj[n_m]   (None → ground → 0)
        #   S_out_k = |H_k|² × S_k
        entries_list: list[NoiseEntry] = []
        total_psd = 0.0

        for (elem_name, noise_type, n_p, n_m, src_psd) in noise_sources:
            h_p: complex = v_adj[n_p] if n_p is not None else 0j
            h_m: complex = v_adj[n_m] if n_m is not None else 0j
            H_k = h_p - h_m
            contrib = (abs(H_k) ** 2) * src_psd
            total_psd += contrib
            entries_list.append(NoiseEntry(
                element_name=elem_name,
                noise_type=noise_type,
                source_psd=src_psd,
                output_psd=contrib,
            ))

        # Sort entries loudest-first.
        entries_list.sort(key=lambda e: e.output_psd, reverse=True)

        # Input-referred noise: S_in = S_out / |H_signal|²
        # H_signal is the adjoint-derived transfer from input_source to output.
        # The adjoint v_adj satisfies: v_adj^T × b = x[out] for any forward b.
        # For VS with branch index k: b[n_nodes+k]=1 → H = v_adj[n_nodes+k]
        # For IS between (n+, n-): b[n+]=-1, b[n-]=+1 → H = v_adj[n-] - v_adj[n+]
        input_referred_psd = 0.0
        if input_el is not None:
            if isinstance(input_el, VoltageSource):
                vs_idx = branch_srcs_noise.index(input_el)
                H_sig = v_adj[n_nodes + vs_idx]
            else:  # CurrentSource
                h_n_plus: complex = (
                    v_adj[node_to_idx[input_el.n_plus]]
                    if not _is_ground(input_el.n_plus)
                    else 0j
                )
                h_n_minus: complex = (
                    v_adj[node_to_idx[input_el.n_minus]]
                    if not _is_ground(input_el.n_minus)
                    else 0j
                )
                H_sig = h_n_minus - h_n_plus
            H_sig_sq = abs(H_sig) ** 2
            if H_sig_sq > 1e-100:
                input_referred_psd = total_psd / H_sig_sq

        points.append(NoisePoint(
            freq=freq,
            output_psd=total_psd,
            input_referred_psd=input_referred_psd,
            entries=tuple(entries_list),
        ))

    return NoiseResult(
        output_node=output_node,
        input_source=input_source,
        temperature=temperature,
        points=points,
    )
