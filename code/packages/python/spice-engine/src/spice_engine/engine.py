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

For transient: three integration methods are supported:

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

3. **Gear-2 / BDF2** (``method="gear2"``):
   Second-order, numerically damped method.  The first step bootstraps with
   backward Euler, then capacitor and inductor companions use two-step
   history.

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

import ast
import cmath
import json
import math
import random
import re
import statistics
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field, replace
from typing import Literal

from mosfet_models import (
    MOSFET,
    Level1Model,
    Level1Params,
    MosfetType,
    bulk_junction_capacitance,
)

from spice_engine.compatibility import (
    DeckAnalysisPlan,
    DeckFourierCard,
    DeckInitialConditionSummary,
    DeckMeasurementCard,
    DeckNodeCondition,
    analyze_deck_controls,
    resolve_deck_analyses,
    resolve_deck_fourier,
    resolve_deck_measurements,
    select_deck_analysis_plan,
    select_deck_output_directive_analysis_kinds,
    select_deck_output_directive_lines,
    select_deck_output_directives,
    select_deck_output_probe_lines,
    select_deck_output_probes,
)
from spice_engine.elements import (
    BJT,
    CCCS,
    CCVS,
    JFET,
    VCCS,
    VCVS,
    AcSource,
    BSource,
    Capacitor,
    CurrentSource,
    CustomModel,
    CustomModelContext,
    CustomModelEvaluation,
    Diode,
    Element,
    Inductor,
    Mosfet,
    MutualInductor,
    PwlWaveform,
    Resistor,
    SubcircuitDefinition,
    TransmissionLine,
    VoltageSource,
    XInstance,
    waveform_period,
)

_DEFAULT_NEWTON_STEP_LIMIT = 5.0


@dataclass
class Circuit:
    elements: list[Element] = field(default_factory=list)
    subcircuits: dict[str, SubcircuitDefinition] = field(default_factory=dict)

    def __post_init__(self) -> None:
        initial_elements = self.elements
        self.elements = []
        for element in initial_elements:
            self.add(element)

    def add(self, element: Element | XInstance) -> None:
        if isinstance(element, XInstance):
            self.instantiate(element)
            return
        self.elements.append(element)

    def define_subcircuit(self, definition: SubcircuitDefinition) -> None:
        key = definition.name.lower()
        if key in self.subcircuits:
            raise ValueError(f"duplicate subcircuit definition {definition.name!r}")
        self.subcircuits[key] = definition

    def instantiate(self, instance: XInstance) -> None:
        self.elements.extend(_expand_xinstance(instance, self.subcircuits, ()))


@dataclass(frozen=True)
class CustomModelDiagnostic:
    """Stable diagnostic emitted by the custom-model source subset analyzer."""

    code: str
    message: str
    severity: Literal["error", "warning"] = "error"


@dataclass(frozen=True)
class CustomModelSourceAnalysis:
    """Result of checking the accepted Verilog-A/custom-model source subset."""

    accepted: bool
    subset: str
    module_name: str | None
    terminals: tuple[str, ...]
    contribution: tuple[str, str] | None
    diagnostics: list[CustomModelDiagnostic]


CUSTOM_MODEL_SUBSET = "two-terminal-current-contribution-v0"
_CUSTOM_MODEL_FORBIDDEN_PATTERNS: tuple[tuple[str, str], ...] = (
    ("ddt", "dynamic charge operators are not accepted in this custom-model subset"),
    ("idt", "dynamic integration operators are not accepted in this custom-model subset"),
    ("laplace", "Laplace-domain operators are not accepted in this custom-model subset"),
    ("cross", "event crossing operators are not accepted in this custom-model subset"),
    ("timer", "timer events are not accepted in this custom-model subset"),
    ("@(", "event controls are not accepted in this custom-model subset"),
    ("$finish", "system tasks are not accepted in this custom-model subset"),
    ("$stop", "system tasks are not accepted in this custom-model subset"),
    ("$display", "system tasks are not accepted in this custom-model subset"),
    ("initial", "procedural initial blocks are not accepted in this custom-model subset"),
    ("always", "procedural always blocks are not accepted in this custom-model subset"),
    ("analog function", "analog functions are not accepted in this custom-model subset"),
    ("discipline", "discipline declarations are not accepted in this custom-model subset"),
    ("branch ", "named branch declarations are not accepted in this custom-model subset"),
)


def analyze_custom_model_source(source: str) -> CustomModelSourceAnalysis:
    """Check the first portable Verilog-A/custom-model subset.

    This is a diagnostic foothold, not a compiler.  Accepted sources define a
    module with at least two ports and one current contribution shaped like
    ``I(p,n) <+ expression``.  Dynamic/event/system constructs are rejected so
    TypeScript/web callers can use the same subset without runtime code eval.
    """

    diagnostics: list[CustomModelDiagnostic] = []
    stripped = source.strip()
    if not stripped:
        diagnostics.append(
            CustomModelDiagnostic(
                code="CUSTOM_MODEL_EMPTY_SOURCE",
                message="custom model source is empty",
            )
        )
        return CustomModelSourceAnalysis(
            accepted=False,
            subset=CUSTOM_MODEL_SUBSET,
            module_name=None,
            terminals=(),
            contribution=None,
            diagnostics=diagnostics,
        )

    lowered = stripped.lower()
    for token, message in _CUSTOM_MODEL_FORBIDDEN_PATTERNS:
        if token in lowered:
            diagnostics.append(
                CustomModelDiagnostic(
                    code="CUSTOM_MODEL_FORBIDDEN_CONSTRUCT",
                    message=message,
                )
            )

    module_match = re.search(
        r"\bmodule\s+([A-Za-z_][A-Za-z0-9_$]*)\s*\(([^)]*)\)\s*;",
        stripped,
        flags=re.IGNORECASE,
    )
    module_name: str | None = None
    terminals: tuple[str, ...] = ()
    if module_match is None:
        diagnostics.append(
            CustomModelDiagnostic(
                code="CUSTOM_MODEL_MISSING_MODULE",
                message="custom model source must declare a module with a port list",
            )
        )
    else:
        module_name = module_match.group(1)
        terminals = tuple(
            port.strip()
            for port in module_match.group(2).split(",")
            if port.strip()
        )
        if len(terminals) < 2:
            diagnostics.append(
                CustomModelDiagnostic(
                    code="CUSTOM_MODEL_PORT_COUNT",
                    message="custom model module must expose at least two terminals",
                )
            )

    contribution_match = re.search(
        r"\bI\s*\(\s*([A-Za-z_][A-Za-z0-9_$]*)\s*,\s*([A-Za-z_][A-Za-z0-9_$]*)\s*\)\s*<\+",
        stripped,
        flags=re.IGNORECASE,
    )
    contribution: tuple[str, str] | None = None
    if contribution_match is None:
        diagnostics.append(
            CustomModelDiagnostic(
                code="CUSTOM_MODEL_MISSING_CONTRIBUTION",
                message="custom model source must contain a two-terminal I(p,n) <+ contribution",
            )
        )
    else:
        contribution = (contribution_match.group(1), contribution_match.group(2))
        terminal_set = set(terminals)
        if terminals and any(node not in terminal_set for node in contribution):
            diagnostics.append(
                CustomModelDiagnostic(
                    code="CUSTOM_MODEL_UNKNOWN_TERMINAL",
                    message="current contribution terminals must be declared module ports",
                )
            )

    return CustomModelSourceAnalysis(
        accepted=not any(diagnostic.severity == "error" for diagnostic in diagnostics),
        subset=CUSTOM_MODEL_SUBSET,
        module_name=module_name,
        terminals=terminals,
        contribution=contribution,
        diagnostics=diagnostics,
    )


def diode_at_temperature(
    diode: Diode,
    temperature_kelvin: float,
    *,
    nominal_temperature_kelvin: float = 300.15,
    energy_gap_ev: float = 1.11,
) -> Diode:
    """Return a diode adjusted from its nominal model temperature.

    This SPICE-style foothold scales thermal voltage linearly with absolute
    temperature and scales saturation current with a silicon energy-gap term.
    """

    if not math.isfinite(temperature_kelvin) or temperature_kelvin <= 0.0:
        raise ValueError("temperature_kelvin must be finite and positive")
    if not math.isfinite(nominal_temperature_kelvin) or nominal_temperature_kelvin <= 0.0:
        raise ValueError("nominal_temperature_kelvin must be finite and positive")
    if not math.isfinite(energy_gap_ev) or energy_gap_ev <= 0.0:
        raise ValueError("energy_gap_ev must be finite and positive")
    effective_n = diode.N
    if not math.isfinite(effective_n) or effective_n <= 0.0:
        raise ValueError(f"{diode.name}: diode emission coefficient must be finite and positive")
    if not math.isfinite(diode.Xti):
        raise ValueError(
            f"{diode.name}: diode saturation-current temperature exponent must be finite"
        )
    ratio = temperature_kelvin / nominal_temperature_kelvin
    exponent = (
        energy_gap_ev
        * _ELECTRON_CHARGE
        / (effective_n * _BOLTZMANN)
        * (1.0 / nominal_temperature_kelvin - 1.0 / temperature_kelvin)
    )
    saturation_scale = ratio**diode.Xti * math.exp(max(-100.0, min(100.0, exponent)))
    return replace(
        diode,
        Is=diode.Is * saturation_scale,
        Vt=diode.Vt * ratio,
    )


def bjt_at_temperature(
    bjt: BJT,
    temperature_kelvin: float,
    *,
    nominal_temperature_kelvin: float = 300.15,
    energy_gap_ev: float = 1.11,
) -> BJT:
    """Return a BJT adjusted from its nominal model temperature."""

    if not math.isfinite(temperature_kelvin) or temperature_kelvin <= 0.0:
        raise ValueError("temperature_kelvin must be finite and positive")
    nominal_temperature_kelvin = bjt.Tnom if bjt.Tnom is not None else nominal_temperature_kelvin
    if not math.isfinite(nominal_temperature_kelvin) or nominal_temperature_kelvin <= 0.0:
        raise ValueError("nominal_temperature_kelvin must be finite and positive")
    if not math.isfinite(energy_gap_ev) or energy_gap_ev <= 0.0:
        raise ValueError("energy_gap_ev must be finite and positive")
    ratio = temperature_kelvin / nominal_temperature_kelvin
    exponent = (
        energy_gap_ev
        * _ELECTRON_CHARGE
        / _BOLTZMANN
        * (1.0 / nominal_temperature_kelvin - 1.0 / temperature_kelvin)
    )
    if not math.isfinite(bjt.Xti):
        raise ValueError(
            f"{bjt.name}: BJT saturation-current temperature exponent must be finite"
        )
    if not math.isfinite(bjt.Xtb):
        raise ValueError(f"{bjt.name}: BJT beta temperature exponent must be finite")
    if math.isnan(bjt.beta_r) or bjt.beta_r <= 0.0:
        raise ValueError(f"{bjt.name}: BJT reverse beta must be positive")
    saturation_scale = ratio**bjt.Xti * math.exp(max(-100.0, min(100.0, exponent)))
    return replace(
        bjt,
        Is=bjt.Is * saturation_scale,
        Ise=bjt.Ise * saturation_scale,
        Isc=bjt.Isc * saturation_scale,
        beta_f=bjt.beta_f * ratio**bjt.Xtb,
        beta_r=bjt.beta_r * ratio**bjt.Xtb,
        Vt=bjt.Vt * ratio,
    )


def mosfet_at_temperature(
    mosfet: Mosfet,
    temperature_kelvin: float,
    *,
    nominal_temperature_kelvin: float = 300.15,
) -> Mosfet:
    """Return a Level-1 MOSFET adjusted from its nominal model temperature."""

    if not math.isfinite(temperature_kelvin) or temperature_kelvin <= 0.0:
        raise ValueError("temperature_kelvin must be finite and positive")
    if not math.isfinite(nominal_temperature_kelvin) or nominal_temperature_kelvin <= 0.0:
        raise ValueError("nominal_temperature_kelvin must be finite and positive")
    model = mosfet.model
    if not isinstance(model, MOSFET) or not isinstance(model.model, Level1Model):
        raise ValueError(f"{mosfet.name}: only Level-1 MOSFET temperature scaling is supported")
    params = model.model.params
    if not isinstance(params, Level1Params):
        raise ValueError(f"{mosfet.name}: only Level-1 MOSFET parameters are supported")
    ratio = temperature_kelvin / nominal_temperature_kelvin
    threshold_shift = -2.0e-3 * (temperature_kelvin - nominal_temperature_kelvin)
    adjusted_params = replace(
        params,
        VT0=params.VT0 + threshold_shift,
        KP=params.KP * ratio**-1.5,
        T_NOM=temperature_kelvin,
    )
    return replace(
        mosfet,
        model=MOSFET(model.type, Level1Model(adjusted_params)),
    )


def circuit_at_temperature(
    circuit: Circuit,
    temperature_kelvin: float,
    *,
    nominal_temperature_kelvin: float = 300.15,
    energy_gap_ev: float = 1.11,
) -> Circuit:
    """Return a circuit with semiconductor models adjusted for temperature."""

    def _adjust_element(element: Element) -> Element:
        if isinstance(element, Diode):
            return diode_at_temperature(
                element,
                temperature_kelvin,
                nominal_temperature_kelvin=nominal_temperature_kelvin,
                energy_gap_ev=element.Eg,
            )
        if isinstance(element, BJT):
            return bjt_at_temperature(
                element,
                temperature_kelvin,
                nominal_temperature_kelvin=nominal_temperature_kelvin,
                energy_gap_ev=element.Eg,
            )
        if isinstance(element, Mosfet):
            return mosfet_at_temperature(
                element,
                temperature_kelvin,
                nominal_temperature_kelvin=nominal_temperature_kelvin,
            )
        return element

    return Circuit(
        elements=[_adjust_element(element) for element in circuit.elements],
        subcircuits=dict(circuit.subcircuits),
    )


def _is_integer_multiple(candidate: float, period: float, tolerance: float) -> bool:
    ratio = candidate / period
    nearest = round(ratio)
    return nearest >= 1 and math.isclose(
        ratio, nearest, rel_tol=tolerance, abs_tol=tolerance
    )


def estimate_period(circuit: Circuit, *, tolerance: float = 1.0e-9) -> float | None:
    """Estimate a PSS source period from periodic independent-source waveforms.

    Static independent sources do not constrain the period.  If any time-varying
    independent source is non-periodic, or if periodic source periods are not
    harmonic multiples of a common candidate, no reliable period is reported.
    """
    periods: list[float] = []
    for element in circuit.elements:
        if (
            isinstance(element, (VoltageSource, CurrentSource))
            and element.waveform is not None
        ):
            period = waveform_period(element.waveform)
            if period is None:
                return None
            periods.append(period)
    if not periods:
        return None

    candidate = max(periods)
    if not math.isfinite(candidate) or candidate <= 0.0:
        return None
    if all(_is_integer_multiple(candidate, period, tolerance) for period in periods):
        return candidate
    return None


def _expand_xinstance(
    instance: XInstance,
    subcircuits: dict[str, SubcircuitDefinition],
    stack: tuple[str, ...],
) -> list[Element]:
    definition = subcircuits.get(instance.subckt.lower())
    if definition is None:
        raise ValueError(f"unknown subcircuit {instance.subckt!r}")
    definition_key = definition.name.lower()
    if definition_key in stack:
        cycle = " -> ".join((*stack, definition_key))
        raise ValueError(f"recursive subcircuit expansion is not supported: {cycle}")
    if len(instance.nodes) != len(definition.pins):
        raise ValueError(
            f"subcircuit {definition.name!r} expects {len(definition.pins)} pins, "
            f"got {len(instance.nodes)}"
        )

    node_map = {pin: node for pin, node in zip(definition.pins, instance.nodes, strict=True)}
    node_map.update(
        {pin.lower(): node for pin, node in zip(definition.pins, instance.nodes, strict=True)}
    )
    expanded: list[Element] = []
    next_stack = (*stack, definition_key)
    for element in definition.elements:
        if isinstance(element, XInstance):
            nested_nodes = tuple(_map_subckt_node(node, instance.name, node_map) for node in element.nodes)
            expanded.extend(
                _expand_xinstance(
                    XInstance(f"{instance.name}.{element.name}", nested_nodes, element.subckt, element.parameters),
                    subcircuits,
                    next_stack,
                )
            )
        else:
            expanded.append(_clone_subckt_element(element, instance.name, node_map))
    return expanded


def _map_subckt_node(node: str, instance_name: str, node_map: dict[str, str]) -> str:
    if node.lower() in {"0", "gnd"}:
        return node
    if node in node_map:
        return node_map[node]
    if node.lower() in node_map:
        return node_map[node.lower()]
    return f"{instance_name}.{node}"


def _map_subckt_source_ref(source_name: str, instance_name: str) -> str:
    if "." in source_name:
        return source_name
    return f"{instance_name}.{source_name}"


def _map_bsource_expr_nodes(expr: str | None, instance_name: str, node_map: dict[str, str]) -> str | None:
    if expr is None:
        return None
    result: list[str] = []
    index = 0
    while index < len(expr):
        if expr[index] == "V" and index + 1 < len(expr) and expr[index + 1] == "(":
            close = expr.find(")", index + 2)
            if close != -1:
                args = expr[index + 2 : close].split(",")
                if 1 <= len(args) <= 2:
                    mapped_args = [
                        _map_subckt_node(arg.strip(), instance_name, node_map) for arg in args
                    ]
                    result.append(f"V({','.join(mapped_args)})")
                    index = close + 1
                    continue
        result.append(expr[index])
        index += 1
    return "".join(result)


def _clone_subckt_element(element: Element, instance_name: str, node_map: dict[str, str]) -> Element:
    name = f"{instance_name}.{element.name}"
    if isinstance(element, Resistor):
        return Resistor(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), element.resistance)
    if isinstance(element, Capacitor):
        return Capacitor(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), element.capacitance, element.initial_voltage)
    if isinstance(element, Inductor):
        return Inductor(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), element.inductance, element.initial_current)
    if isinstance(element, MutualInductor):
        return MutualInductor(name, _map_subckt_source_ref(element.primary, instance_name), _map_subckt_source_ref(element.secondary, instance_name), element.coupling)
    if isinstance(element, TransmissionLine):
        return TransmissionLine(name, _map_subckt_node(element.n1, instance_name, node_map), _map_subckt_node(element.n2, instance_name, node_map), _map_subckt_node(element.n3, instance_name, node_map), _map_subckt_node(element.n4, instance_name, node_map), element.characteristic_impedance, element.delay)
    if isinstance(element, VoltageSource):
        return VoltageSource(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), element.voltage, element.waveform, element.ac)
    if isinstance(element, CurrentSource):
        return CurrentSource(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), element.current, element.waveform, element.ac)
    if isinstance(element, BSource):
        return BSource(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), _map_bsource_expr_nodes(element.voltage_expr, instance_name, node_map), _map_bsource_expr_nodes(element.current_expr, instance_name, node_map))
    if isinstance(element, CustomModel):
        return CustomModel(
            name,
            _map_subckt_node(element.n_plus, instance_name, node_map),
            _map_subckt_node(element.n_minus, instance_name, node_map),
            element.model_name,
            element.parameters,
            element.evaluator,
            element.conductance_siemens,
            element.current_offset_amps,
        )
    if isinstance(element, Diode):
        return Diode(
            name,
            _map_subckt_node(element.anode, instance_name, node_map),
            _map_subckt_node(element.cathode, instance_name, node_map),
            element.Is,
            element.Vt,
            element.N,
            element.BV,
            element.IBV,
            element.Cjo,
            element.Tt,
            element.Vj,
            element.M,
            element.Fc,
            element.Xti,
            element.Eg,
            element.Rs,
            element.Kf,
            element.Af,
        )
    if isinstance(element, JFET):
        return JFET(name, _map_subckt_node(element.drain, instance_name, node_map), _map_subckt_node(element.gate, instance_name, node_map), _map_subckt_node(element.source, instance_name, node_map), element.polarity, element.beta, element.vto, element.lambda_, element.Cgs, element.Cgd, element.Kf, element.Af)
    if isinstance(element, Mosfet):
        return Mosfet(name, _map_subckt_node(element.drain, instance_name, node_map), _map_subckt_node(element.gate, instance_name, node_map), _map_subckt_node(element.source, instance_name, node_map), _map_subckt_node(element.body, instance_name, node_map), element.model)
    if isinstance(element, BJT):
        return BJT(name, _map_subckt_node(element.collector, instance_name, node_map), _map_subckt_node(element.base, instance_name, node_map), _map_subckt_node(element.emitter, instance_name, node_map), element.polarity, element.Is, element.beta_f, element.Vt, element.Cje, element.Cjc, element.Tf, element.Tr, element.Xti, element.Eg, element.Vaf, element.Nf, element.Nr, element.Vje, element.Mje, element.Vjc, element.Mjc, element.Fc, element.Var, element.Ikf, element.Ise, element.Ne, element.Isc, element.Nc, element.Xtb, element.beta_r, element.Ikr, element.Tnom, element.Kf, element.Af, element.Ptf, element.Xtf, element.Itf, element.Vtf, element.Re, element.Rc, element.Rb, element.Rbm, element.Irb, element.Xcjc)
    if isinstance(element, VCVS):
        return VCVS(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), _map_subckt_node(element.ctrl_plus, instance_name, node_map), _map_subckt_node(element.ctrl_minus, instance_name, node_map), element.gain)
    if isinstance(element, VCCS):
        return VCCS(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), _map_subckt_node(element.ctrl_plus, instance_name, node_map), _map_subckt_node(element.ctrl_minus, instance_name, node_map), element.gm)
    if isinstance(element, CCCS):
        return CCCS(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), _map_subckt_source_ref(element.ctrl_source, instance_name), element.beta)
    if isinstance(element, CCVS):
        return CCVS(name, _map_subckt_node(element.n_plus, instance_name, node_map), _map_subckt_node(element.n_minus, instance_name, node_map), _map_subckt_source_ref(element.ctrl_source, instance_name), element.transresistance)
    raise TypeError(f"unsupported subcircuit element {type(element).__name__}")


@dataclass
class LinearSolverProfile:
    """Auditable profile for one real-valued linear solve."""

    matrix_size: int
    solver: str
    backend: str
    structural_nonzeros: int
    density: float
    fill_in_nonzeros: int = 0
    fallback_reason: str | None = None


@dataclass
class DcSolverDiagnostics:
    """Stable DC solve metadata for downstream comparison."""

    matrix_size: int
    solver: str
    tolerance: float
    max_delta: float
    convergence_aid: str
    newton_step_limit: float | None = None
    limited_newton_steps: int = 0
    minimum_damping_factor: float = 1.0
    solver_profile: LinearSolverProfile = field(
        default_factory=lambda: LinearSolverProfile(
            matrix_size=0,
            solver="none",
            backend="none",
            structural_nonzeros=0,
            density=0.0,
        )
    )


@dataclass
class DcResult:
    """Operating-point voltages by node + extra branch currents."""

    node_voltages: dict[str, float]
    branch_currents: dict[str, float]
    iterations: int
    converged: bool
    convergence_aid: str = "newton"
    diagnostics: DcSolverDiagnostics = field(
        default_factory=lambda: DcSolverDiagnostics(
            matrix_size=0,
            solver="none",
            tolerance=0.0,
            max_delta=0.0,
            convergence_aid="newton",
        )
    )


@dataclass(frozen=True)
class CornerOverride:
    """One element-parameter override for a named analysis corner."""

    element_name: str
    parameter: str
    value: float


@dataclass(frozen=True)
class CornerSpec:
    """Named DC analysis corner."""

    name: str
    overrides: tuple[CornerOverride, ...] = ()


@dataclass(frozen=True)
class CornerPoint:
    """DC operating-point result for one named corner."""

    corner_name: str
    result: DcResult


@dataclass(frozen=True)
class CornerSweepResult:
    """Multi-corner DC operating-point sweep result."""

    points: list[CornerPoint]


@dataclass(frozen=True)
class TemperatureDcPoint:
    """DC operating-point result for one analysis temperature."""

    temperature_kelvin: float
    result: DcResult


@dataclass(frozen=True)
class TemperatureDcResult:
    """DC operating-point sweep across explicit analysis temperatures."""

    points: list[TemperatureDcPoint]


@dataclass(frozen=True)
class CornerTemperatureDcPoint:
    """Temperature DC sweep result for one named corner."""

    corner_name: str
    points: list[TemperatureDcPoint]


@dataclass(frozen=True)
class CornerTemperatureDcResult:
    """Named-corner DC temperature sweep result."""

    points: list[CornerTemperatureDcPoint]


@dataclass
class TransientPoint:
    time: float
    node_voltages: dict[str, float]
    branch_currents: dict[str, float] = field(default_factory=dict)


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
        Integration method that was used: ``"trap"``, ``"euler"``, or
        ``"gear2"``.
    steps_rejected:
        Number of timesteps rejected by the LTE adaptive controller.
        Always 0 when ``adaptive=False``.
    """

    points: list[TransientPoint]
    converged: bool
    method: str = "trap"
    steps_rejected: int = 0


@dataclass(frozen=True)
class ProbeMeasurement:
    """Stable scalar result for a SPICE-style probe measurement."""

    name: str
    analysis: str
    probe: str
    mode: str
    value: float
    from_value: float | None = None
    to_value: float | None = None


@dataclass(frozen=True)
class CornerTransientPoint:
    """Transient waveform result for one named analysis corner."""

    corner_name: str
    points: list[TransientPoint]


@dataclass(frozen=True)
class CornerTransientResult:
    """Multi-corner transient waveform result."""

    points: list[CornerTransientPoint]


@dataclass(frozen=True)
class CornerAdaptiveTransientPoint:
    """Adaptive transient waveform result for one named analysis corner."""

    corner_name: str
    result: TransientResult


@dataclass(frozen=True)
class CornerAdaptiveTransientResult:
    """Multi-corner adaptive transient waveform result."""

    points: list[CornerAdaptiveTransientPoint]


DigitalState = Literal["low", "high"]


@dataclass(frozen=True)
class DigitalEvent:
    """One hardware-VM-facing digital value change."""

    time_seconds: float
    state: DigitalState


@dataclass(frozen=True)
class DigitalEventStream:
    """Named digital event stream used at the SPICE/VM boundary."""

    signal_name: str
    events: list[DigitalEvent]


@dataclass(frozen=True)
class DigitalTransientBridgeResult:
    """Transient result plus thresholded output streams."""

    points: list[TransientPoint]
    output_streams: list[DigitalEventStream]


@dataclass(frozen=True)
class CornerDigitalTransientBridgePoint:
    """Digital bridge result for one named corner."""

    corner_name: str
    result: DigitalTransientBridgeResult


@dataclass(frozen=True)
class CornerDigitalTransientBridgeResult:
    """Multi-corner digital bridge result."""

    points: list[CornerDigitalTransientBridgePoint]


@dataclass(frozen=True)
class AdaptiveDigitalTransientBridgeResult:
    """Adaptive transient result plus thresholded output streams."""

    result: TransientResult
    output_streams: list[DigitalEventStream]


@dataclass(frozen=True)
class CornerAdaptiveDigitalTransientBridgePoint:
    """Adaptive digital bridge result for one named corner."""

    corner_name: str
    result: AdaptiveDigitalTransientBridgeResult


@dataclass(frozen=True)
class CornerAdaptiveDigitalTransientBridgeResult:
    """Multi-corner adaptive digital bridge result."""

    points: list[CornerAdaptiveDigitalTransientBridgePoint]


@dataclass(frozen=True)
class DigitalBridgeSchedule:
    """Hardware VM breakpoint schedule derived from digital event streams."""

    stop_time: float
    breakpoints: list[float]


@dataclass(frozen=True)
class DigitalLogicLevels:
    """Analog voltages used when driving digital events into SPICE."""

    low_voltage: float
    high_voltage: float
    transition_seconds: float

    @classmethod
    def cmos_1v8(cls, transition_seconds: float) -> DigitalLogicLevels:
        return cls(0.0, 1.8, transition_seconds)

    def voltage_for(self, state: DigitalState) -> float:
        return self.low_voltage if _normalize_digital_state(state) == "low" else self.high_voltage


@dataclass(frozen=True)
class DigitalThresholds:
    """Analog thresholds used when sampling SPICE probes back to logic."""

    low_max_voltage: float
    high_min_voltage: float

    @classmethod
    def cmos_1v8(cls) -> DigitalThresholds:
        return cls(0.6, 1.2)

    def classify(self, voltage: float) -> DigitalState | None:
        if voltage <= self.low_max_voltage:
            return "low"
        if voltage >= self.high_min_voltage:
            return "high"
        return None


@dataclass(frozen=True)
class FourierHarmonic:
    harmonic: int
    frequency: float
    cosine: float
    sine: float
    magnitude: float
    phase_degrees: float


@dataclass(frozen=True)
class FourierProbeResult:
    probe: str
    dc: float
    harmonics: list[FourierHarmonic]
    total_harmonic_distortion: float


@dataclass(frozen=True)
class FourierResult:
    fundamental_frequency: float
    start_time: float
    end_time: float
    probes: list[FourierProbeResult]


@dataclass(frozen=True)
class CornerFourierPoint:
    """Fourier result for one named analysis corner."""

    corner_name: str
    result: FourierResult


@dataclass(frozen=True)
class CornerFourierResult:
    """Multi-corner Fourier analysis result."""

    fundamental_frequency: float
    points: list[CornerFourierPoint]


@dataclass(frozen=True)
class DistortionHarmonic:
    harmonic: int
    frequency: float
    magnitude: float
    phase_degrees: float


@dataclass(frozen=True)
class DistortionPoint:
    frequency: float
    fundamental_magnitude: float
    harmonics: list[DistortionHarmonic]
    total_harmonic_distortion: float


@dataclass(frozen=True)
class DistortionResult:
    input_source: str
    output_probe: str
    points: list[DistortionPoint]


@dataclass(frozen=True)
class CornerDistortionPoint:
    """Distortion result for one named analysis corner."""

    corner_name: str
    result: DistortionResult


@dataclass(frozen=True)
class CornerDistortionResult:
    """Multi-corner distortion analysis result."""

    input_source: str
    output_probe: str
    points: list[CornerDistortionPoint]


@dataclass(frozen=True)
class PoleZeroEntry:
    kind: str
    real: float
    imaginary: float
    frequency: float
    damping: float


@dataclass(frozen=True)
class PoleZeroResult:
    input_source: str
    output_node: str
    entries: list[PoleZeroEntry]


@dataclass(frozen=True)
class CornerPoleZeroPoint:
    """Pole-zero result for one named analysis corner."""

    corner_name: str
    result: PoleZeroResult


@dataclass(frozen=True)
class CornerPoleZeroResult:
    """Multi-corner pole-zero analysis result."""

    input_source: str
    output_node: str
    topology: str
    points: list[CornerPoleZeroPoint]


_POLE_ZERO_TOPOLOGIES: dict[str, Callable[[Circuit, str, str], PoleZeroResult]] = {}


def _normalize_pole_zero_topology(topology: str) -> str:
    text = topology.replace("_", "-").strip().lower()
    aliases = {
        "rc-lowpass": "rc-lowpass",
        "rclowpass": "rc-lowpass",
        "rclow-pass": "rc-lowpass",
        "rc-highpass": "rc-highpass",
        "rchighpass": "rc-highpass",
        "rchigh-pass": "rc-highpass",
        "rlc-lowpass": "rlc-lowpass",
        "rlclowpass": "rlc-lowpass",
        "rlclow-pass": "rlc-lowpass",
        "rlc-highpass": "rlc-highpass",
        "rlchighpass": "rlc-highpass",
        "rlchigh-pass": "rlc-highpass",
        "rlc-bandpass": "rlc-bandpass",
        "rlcbandpass": "rlc-bandpass",
        "rlcband-pass": "rlc-bandpass",
        "rlc-notch": "rlc-notch",
        "rlcnotch": "rlc-notch",
    }
    if text not in aliases:
        supported = ", ".join(sorted(_POLE_ZERO_TOPOLOGIES))
        raise ValueError(f"pole_zero_corners: unsupported topology {topology!r}; expected {supported}")
    return aliases[text]


def pole_zero_rc_lowpass(
    circuit: Circuit,
    input_source: str,
    output_node: str,
) -> PoleZeroResult:
    """Return the one-pole result for a conservative RC low-pass fixture."""

    source = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, VoltageSource) and element.name == input_source
        ),
        None,
    )
    if source is None:
        raise ValueError(f"pole_zero_rc_lowpass: missing input source {input_source!r}")
    if not _is_ground(source.n_minus):
        raise ValueError("pole_zero_rc_lowpass: input source negative terminal must be ground")

    resistor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Resistor)
            and {
                element.n_plus,
                element.n_minus,
            }
            == {source.n_plus, output_node}
        ),
        None,
    )
    capacitor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Capacitor)
            and output_node in {element.n_plus, element.n_minus}
            and (_is_ground(element.n_plus) or _is_ground(element.n_minus))
        ),
        None,
    )
    if resistor is None or capacitor is None:
        raise ValueError(
            "pole_zero_rc_lowpass: expected one resistor from input to output "
            "and one grounded output capacitor"
        )
    if not math.isfinite(resistor.resistance) or resistor.resistance <= 0.0:
        raise ValueError("pole_zero_rc_lowpass: resistance must be finite and positive")
    if not math.isfinite(capacitor.capacitance) or capacitor.capacitance <= 0.0:
        raise ValueError("pole_zero_rc_lowpass: capacitance must be finite and positive")

    real = -1.0 / (resistor.resistance * capacitor.capacitance)
    return PoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        entries=[
            PoleZeroEntry(
                kind="pole",
                real=real,
                imaginary=0.0,
                frequency=abs(real) / (2.0 * math.pi),
                damping=1.0,
            )
        ],
    )


def pole_zero_rc_highpass(
    circuit: Circuit,
    input_source: str,
    output_node: str,
) -> PoleZeroResult:
    """Return the zero-at-origin and one-pole result for an RC high-pass fixture."""

    source = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, VoltageSource) and element.name == input_source
        ),
        None,
    )
    if source is None:
        raise ValueError(f"pole_zero_rc_highpass: missing input source {input_source!r}")
    if not _is_ground(source.n_minus):
        raise ValueError("pole_zero_rc_highpass: input source negative terminal must be ground")

    capacitor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Capacitor)
            and {
                element.n_plus,
                element.n_minus,
            }
            == {source.n_plus, output_node}
        ),
        None,
    )
    resistor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Resistor)
            and output_node in {element.n_plus, element.n_minus}
            and (_is_ground(element.n_plus) or _is_ground(element.n_minus))
        ),
        None,
    )
    if capacitor is None or resistor is None:
        raise ValueError(
            "pole_zero_rc_highpass: expected one capacitor from input to output "
            "and one grounded output resistor"
        )
    if not math.isfinite(resistor.resistance) or resistor.resistance <= 0.0:
        raise ValueError("pole_zero_rc_highpass: resistance must be finite and positive")
    if not math.isfinite(capacitor.capacitance) or capacitor.capacitance <= 0.0:
        raise ValueError("pole_zero_rc_highpass: capacitance must be finite and positive")

    real = -1.0 / (resistor.resistance * capacitor.capacitance)
    return PoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        entries=[
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=0.0,
                frequency=0.0,
                damping=1.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=real,
                imaginary=0.0,
                frequency=abs(real) / (2.0 * math.pi),
                damping=1.0,
            ),
        ],
    )


def pole_zero_rlc_lowpass(
    circuit: Circuit,
    input_source: str,
    output_node: str,
) -> PoleZeroResult:
    """Return the two-pole result for a series R-L, shunt-C low-pass fixture."""

    source = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, VoltageSource) and element.name == input_source
        ),
        None,
    )
    if source is None:
        raise ValueError(f"pole_zero_rlc_lowpass: missing input source {input_source!r}")
    if not _is_ground(source.n_minus):
        raise ValueError("pole_zero_rlc_lowpass: input source negative terminal must be ground")

    resistor: Resistor | None = None
    intermediate: str | None = None
    for element in circuit.elements:
        if isinstance(element, Resistor) and source.n_plus in {element.n_plus, element.n_minus}:
            other = element.n_minus if element.n_plus == source.n_plus else element.n_plus
            if other != output_node and not _is_ground(other):
                resistor = element
                intermediate = other
                break
    inductor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Inductor)
            and intermediate is not None
            and {element.n_plus, element.n_minus} == {intermediate, output_node}
        ),
        None,
    )
    capacitor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Capacitor)
            and output_node in {element.n_plus, element.n_minus}
            and (_is_ground(element.n_plus) or _is_ground(element.n_minus))
        ),
        None,
    )
    if resistor is None or inductor is None or capacitor is None:
        raise ValueError(
            "pole_zero_rlc_lowpass: expected series resistor and inductor from "
            "input to output plus one grounded output capacitor"
        )
    if not math.isfinite(resistor.resistance) or resistor.resistance <= 0.0:
        raise ValueError("pole_zero_rlc_lowpass: resistance must be finite and positive")
    if not math.isfinite(inductor.inductance) or inductor.inductance <= 0.0:
        raise ValueError("pole_zero_rlc_lowpass: inductance must be finite and positive")
    if not math.isfinite(capacitor.capacitance) or capacitor.capacitance <= 0.0:
        raise ValueError("pole_zero_rlc_lowpass: capacitance must be finite and positive")

    alpha = resistor.resistance / (2.0 * inductor.inductance)
    omega0 = 1.0 / math.sqrt(inductor.inductance * capacitor.capacitance)
    discriminant = alpha * alpha - omega0 * omega0
    if discriminant >= 0.0:
        root = math.sqrt(discriminant)
        entries = [
            PoleZeroEntry(
                kind="pole",
                real=-alpha + root,
                imaginary=0.0,
                frequency=abs(-alpha + root) / (2.0 * math.pi),
                damping=1.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha - root,
                imaginary=0.0,
                frequency=abs(-alpha - root) / (2.0 * math.pi),
                damping=1.0,
            ),
        ]
    else:
        imaginary = math.sqrt(-discriminant)
        damping = alpha / omega0
        frequency = omega0 / (2.0 * math.pi)
        entries = [
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=imaginary,
                frequency=frequency,
                damping=damping,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=-imaginary,
                frequency=frequency,
                damping=damping,
            ),
        ]

    return PoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        entries=entries,
    )


def pole_zero_rlc_highpass(
    circuit: Circuit,
    input_source: str,
    output_node: str,
) -> PoleZeroResult:
    """Return the double-zero and two-pole result for a series R-C, shunt-L high-pass fixture."""

    source = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, VoltageSource) and element.name == input_source
        ),
        None,
    )
    if source is None:
        raise ValueError(f"pole_zero_rlc_highpass: missing input source {input_source!r}")
    if not _is_ground(source.n_minus):
        raise ValueError("pole_zero_rlc_highpass: input source negative terminal must be ground")

    resistor: Resistor | None = None
    intermediate: str | None = None
    for element in circuit.elements:
        if isinstance(element, Resistor) and source.n_plus in {element.n_plus, element.n_minus}:
            other = element.n_minus if element.n_plus == source.n_plus else element.n_plus
            if other != output_node and not _is_ground(other):
                resistor = element
                intermediate = other
                break
    capacitor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Capacitor)
            and intermediate is not None
            and {element.n_plus, element.n_minus} == {intermediate, output_node}
        ),
        None,
    )
    inductor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Inductor)
            and output_node in {element.n_plus, element.n_minus}
            and (_is_ground(element.n_plus) or _is_ground(element.n_minus))
        ),
        None,
    )
    if resistor is None or capacitor is None or inductor is None:
        raise ValueError(
            "pole_zero_rlc_highpass: expected series resistor and capacitor from "
            "input to output plus one grounded output inductor"
        )
    if not math.isfinite(resistor.resistance) or resistor.resistance <= 0.0:
        raise ValueError("pole_zero_rlc_highpass: resistance must be finite and positive")
    if not math.isfinite(capacitor.capacitance) or capacitor.capacitance <= 0.0:
        raise ValueError("pole_zero_rlc_highpass: capacitance must be finite and positive")
    if not math.isfinite(inductor.inductance) or inductor.inductance <= 0.0:
        raise ValueError("pole_zero_rlc_highpass: inductance must be finite and positive")

    alpha = resistor.resistance / (2.0 * inductor.inductance)
    omega0 = 1.0 / math.sqrt(inductor.inductance * capacitor.capacitance)
    discriminant = alpha * alpha - omega0 * omega0
    entries = [
        PoleZeroEntry(kind="zero", real=0.0, imaginary=0.0, frequency=0.0, damping=1.0),
        PoleZeroEntry(kind="zero", real=0.0, imaginary=0.0, frequency=0.0, damping=1.0),
    ]
    if discriminant >= 0.0:
        root = math.sqrt(discriminant)
        entries.extend(
            [
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha + root,
                    imaginary=0.0,
                    frequency=abs(-alpha + root) / (2.0 * math.pi),
                    damping=1.0,
                ),
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha - root,
                    imaginary=0.0,
                    frequency=abs(-alpha - root) / (2.0 * math.pi),
                    damping=1.0,
                ),
            ]
        )
    else:
        imaginary = math.sqrt(-discriminant)
        damping = alpha / omega0
        frequency = omega0 / (2.0 * math.pi)
        entries.extend(
            [
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha,
                    imaginary=imaginary,
                    frequency=frequency,
                    damping=damping,
                ),
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha,
                    imaginary=-imaginary,
                    frequency=frequency,
                    damping=damping,
                ),
            ]
        )

    return PoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        entries=entries,
    )


def pole_zero_rlc_bandpass(
    circuit: Circuit,
    input_source: str,
    output_node: str,
) -> PoleZeroResult:
    """Return the single-zero and two-pole result for a series L-C, shunt-R band-pass fixture."""

    source = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, VoltageSource) and element.name == input_source
        ),
        None,
    )
    if source is None:
        raise ValueError(f"pole_zero_rlc_bandpass: missing input source {input_source!r}")
    if not _is_ground(source.n_minus):
        raise ValueError("pole_zero_rlc_bandpass: input source negative terminal must be ground")

    inductor: Inductor | None = None
    intermediate: str | None = None
    for element in circuit.elements:
        if isinstance(element, Inductor) and source.n_plus in {element.n_plus, element.n_minus}:
            other = element.n_minus if element.n_plus == source.n_plus else element.n_plus
            if other != output_node and not _is_ground(other):
                inductor = element
                intermediate = other
                break
    capacitor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Capacitor)
            and intermediate is not None
            and {element.n_plus, element.n_minus} == {intermediate, output_node}
        ),
        None,
    )
    resistor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Resistor)
            and output_node in {element.n_plus, element.n_minus}
            and (_is_ground(element.n_plus) or _is_ground(element.n_minus))
        ),
        None,
    )
    if inductor is None or capacitor is None or resistor is None:
        raise ValueError(
            "pole_zero_rlc_bandpass: expected series inductor and capacitor from "
            "input to output plus one grounded output resistor"
        )
    if not math.isfinite(inductor.inductance) or inductor.inductance <= 0.0:
        raise ValueError("pole_zero_rlc_bandpass: inductance must be finite and positive")
    if not math.isfinite(capacitor.capacitance) or capacitor.capacitance <= 0.0:
        raise ValueError("pole_zero_rlc_bandpass: capacitance must be finite and positive")
    if not math.isfinite(resistor.resistance) or resistor.resistance <= 0.0:
        raise ValueError("pole_zero_rlc_bandpass: resistance must be finite and positive")

    alpha = resistor.resistance / (2.0 * inductor.inductance)
    omega0 = 1.0 / math.sqrt(inductor.inductance * capacitor.capacitance)
    discriminant = alpha * alpha - omega0 * omega0
    entries = [
        PoleZeroEntry(kind="zero", real=0.0, imaginary=0.0, frequency=0.0, damping=1.0),
    ]
    if discriminant >= 0.0:
        root = math.sqrt(discriminant)
        entries.extend(
            [
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha + root,
                    imaginary=0.0,
                    frequency=abs(-alpha + root) / (2.0 * math.pi),
                    damping=1.0,
                ),
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha - root,
                    imaginary=0.0,
                    frequency=abs(-alpha - root) / (2.0 * math.pi),
                    damping=1.0,
                ),
            ]
        )
    else:
        imaginary = math.sqrt(-discriminant)
        damping = alpha / omega0
        frequency = omega0 / (2.0 * math.pi)
        entries.extend(
            [
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha,
                    imaginary=imaginary,
                    frequency=frequency,
                    damping=damping,
                ),
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha,
                    imaginary=-imaginary,
                    frequency=frequency,
                    damping=damping,
                ),
            ]
        )

    return PoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        entries=entries,
    )


def pole_zero_rlc_notch(
    circuit: Circuit,
    input_source: str,
    output_node: str,
) -> PoleZeroResult:
    """Return the notch zeros and two-pole result for a series R, shunt series-L-C fixture."""

    source = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, VoltageSource) and element.name == input_source
        ),
        None,
    )
    if source is None:
        raise ValueError(f"pole_zero_rlc_notch: missing input source {input_source!r}")
    if not _is_ground(source.n_minus):
        raise ValueError("pole_zero_rlc_notch: input source negative terminal must be ground")

    resistor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Resistor)
            and source.n_plus in {element.n_plus, element.n_minus}
            and output_node in {element.n_plus, element.n_minus}
        ),
        None,
    )
    inductor: Inductor | None = None
    intermediate: str | None = None
    for element in circuit.elements:
        if isinstance(element, Inductor) and output_node in {element.n_plus, element.n_minus}:
            other = element.n_minus if element.n_plus == output_node else element.n_plus
            if not _is_ground(other):
                inductor = element
                intermediate = other
                break
    capacitor = next(
        (
            element
            for element in circuit.elements
            if isinstance(element, Capacitor)
            and intermediate is not None
            and intermediate in {element.n_plus, element.n_minus}
            and (_is_ground(element.n_plus) or _is_ground(element.n_minus))
        ),
        None,
    )
    if resistor is None or inductor is None or capacitor is None:
        raise ValueError(
            "pole_zero_rlc_notch: expected series resistor from input to output "
            "plus a grounded series inductor-capacitor branch at output"
        )
    if not math.isfinite(resistor.resistance) or resistor.resistance <= 0.0:
        raise ValueError("pole_zero_rlc_notch: resistance must be finite and positive")
    if not math.isfinite(inductor.inductance) or inductor.inductance <= 0.0:
        raise ValueError("pole_zero_rlc_notch: inductance must be finite and positive")
    if not math.isfinite(capacitor.capacitance) or capacitor.capacitance <= 0.0:
        raise ValueError("pole_zero_rlc_notch: capacitance must be finite and positive")

    alpha = resistor.resistance / (2.0 * inductor.inductance)
    omega0 = 1.0 / math.sqrt(inductor.inductance * capacitor.capacitance)
    discriminant = alpha * alpha - omega0 * omega0
    entries = [
        PoleZeroEntry(
            kind="zero",
            real=0.0,
            imaginary=omega0,
            frequency=omega0 / (2.0 * math.pi),
            damping=0.0,
        ),
        PoleZeroEntry(
            kind="zero",
            real=0.0,
            imaginary=-omega0,
            frequency=omega0 / (2.0 * math.pi),
            damping=0.0,
        ),
    ]
    if discriminant >= 0.0:
        root = math.sqrt(discriminant)
        entries.extend(
            [
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha + root,
                    imaginary=0.0,
                    frequency=abs(-alpha + root) / (2.0 * math.pi),
                    damping=1.0,
                ),
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha - root,
                    imaginary=0.0,
                    frequency=abs(-alpha - root) / (2.0 * math.pi),
                    damping=1.0,
                ),
            ]
        )
    else:
        imaginary = math.sqrt(-discriminant)
        damping = alpha / omega0
        frequency = omega0 / (2.0 * math.pi)
        entries.extend(
            [
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha,
                    imaginary=imaginary,
                    frequency=frequency,
                    damping=damping,
                ),
                PoleZeroEntry(
                    kind="pole",
                    real=-alpha,
                    imaginary=-imaginary,
                    frequency=frequency,
                    damping=damping,
                ),
            ]
        )

    return PoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        entries=entries,
    )


_POLE_ZERO_TOPOLOGIES = {
    "rc-lowpass": pole_zero_rc_lowpass,
    "rc-highpass": pole_zero_rc_highpass,
    "rlc-lowpass": pole_zero_rlc_lowpass,
    "rlc-highpass": pole_zero_rlc_highpass,
    "rlc-bandpass": pole_zero_rlc_bandpass,
    "rlc-notch": pole_zero_rlc_notch,
}


def pole_zero_corners(
    circuit: Circuit,
    input_source: str,
    output_node: str,
    topology: str,
    corners: list[CornerSpec],
) -> CornerPoleZeroResult:
    """Run a selected pole-zero fixture helper at each named corner."""
    normalized_topology = _normalize_pole_zero_topology(topology)
    helper = _POLE_ZERO_TOPOLOGIES[normalized_topology]
    return CornerPoleZeroResult(
        input_source=input_source,
        output_node=output_node,
        topology=normalized_topology,
        points=[
            CornerPoleZeroPoint(
                corner_name=corner.name,
                result=helper(_circuit_with_corner(circuit, corner), input_source, output_node),
            )
            for corner in corners
        ],
    )


def distortion_from_fourier(
    result: FourierResult,
    input_source: str,
    output_probe: str,
) -> DistortionResult:
    """Project a Fourier probe result into the Phase-8 distortion shape."""

    probe_result = next(
        (probe for probe in result.probes if probe.probe == output_probe),
        None,
    )
    if probe_result is None:
        raise ValueError(f"distortion_from_fourier: missing probe {output_probe!r}")
    if not probe_result.harmonics:
        raise ValueError("distortion_from_fourier: Fourier result has no harmonics")
    fundamental = probe_result.harmonics[0]
    return DistortionResult(
        input_source=input_source,
        output_probe=output_probe,
        points=[
            DistortionPoint(
                frequency=fundamental.frequency,
                fundamental_magnitude=fundamental.magnitude,
                harmonics=[
                    DistortionHarmonic(
                        harmonic=harmonic.harmonic,
                        frequency=harmonic.frequency,
                        magnitude=harmonic.magnitude,
                        phase_degrees=harmonic.phase_degrees,
                    )
                    for harmonic in probe_result.harmonics[1:]
                ],
                total_harmonic_distortion=probe_result.total_harmonic_distortion,
            )
        ],
    )


def distortion_from_transient(
    transient_result: TransientResult | list[TransientPoint],
    fundamental_frequency: float,
    input_source: str,
    output_probe: str,
    *,
    harmonics: int = 9,
    start_time: float | None = None,
) -> DistortionResult:
    """Compute the Phase-8 distortion shape directly from transient samples."""

    return distortion_from_fourier(
        fourier(
            transient_result,
            fundamental_frequency,
            [output_probe],
            harmonics=harmonics,
            start_time=start_time,
        ),
        input_source=input_source,
        output_probe=output_probe,
    )


def distortion_from_transient_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    t_stop: float,
    t_step: float,
    fundamental_frequency: float,
    input_source: str,
    output_probe: str,
    harmonics: int = 9,
    start_time: float | None = None,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerDistortionResult:
    """Run transient distortion projection at each named corner."""
    points: list[CornerDistortionPoint] = []
    for corner in corners:
        transient_result = transient(
            _circuit_with_corner(circuit, corner),
            t_stop=t_stop,
            t_step=t_step,
            method=method,
            max_iterations=max_iterations,
            tol=tol,
        )
        points.append(
            CornerDistortionPoint(
                corner_name=corner.name,
                result=distortion_from_transient(
                    transient_result,
                    fundamental_frequency,
                    input_source,
                    output_probe,
                    harmonics=harmonics,
                    start_time=start_time,
                ),
            )
        )
    return CornerDistortionResult(
        input_source=input_source,
        output_probe=output_probe,
        points=points,
    )


def format_dc_table(result: DcResult, probes: list[str] | None = None) -> str:
    """Format a DC operating point as a stable SPICE-style text table."""
    selected_probes = probes or _default_output_probes(
        result.node_voltages,
        result.branch_currents,
    )
    row = [
        _format_table_number(
            _table_probe_value(
                result.node_voltages,
                result.branch_currents,
                probe,
                "format_dc_table",
            )
        )
        for probe in selected_probes
    ]
    return "\n".join(
        [
            "\t".join(["Index", *selected_probes]),
            "\t".join(["0", *row]),
            "",
        ]
    )


def format_corner_dc_table(
    result: CornerSweepResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner DC operating points as a stable SPICE-style table."""
    selected_probes = probes or next(
        (
            _default_output_probes(
                point.result.node_voltages,
                point.result.branch_currents,
            )
            for point in result.points
        ),
        [],
    )
    rows = ["\t".join(["Corner", "Index", *selected_probes])]
    for index, point in enumerate(result.points):
        values = [
            _format_table_number(
                _table_probe_value(
                    point.result.node_voltages,
                    point.result.branch_currents,
                    probe,
                    "format_corner_dc_table",
                )
            )
            for probe in selected_probes
        ]
        rows.append("\t".join([point.corner_name, str(index), *values]))
    rows.append("")
    return "\n".join(rows)


def format_temperature_dc_table(
    result: TemperatureDcResult,
    probes: list[str] | None = None,
) -> str:
    """Format a DC temperature sweep as a stable SPICE-style text table."""
    selected_probes = probes or next(
        (
            _default_output_probes(
                point.result.node_voltages,
                point.result.branch_currents,
            )
            for point in result.points
        ),
        [],
    )
    rows = ["\t".join(["Index", "TemperatureKelvin", *selected_probes])]
    for index, point in enumerate(result.points):
        values = [
            _format_table_number(
                _table_probe_value(
                    point.result.node_voltages,
                    point.result.branch_currents,
                    probe,
                    "format_temperature_dc_table",
                )
            )
            for probe in selected_probes
        ]
        rows.append(
            "\t".join(
                [
                    str(index),
                    _format_table_number(point.temperature_kelvin),
                    *values,
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_corner_temperature_dc_table(
    result: CornerTemperatureDcResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner DC temperature sweeps as a stable SPICE-style table."""
    selected_probes = probes or next(
        (
            _default_output_probes(
                point.result.node_voltages,
                point.result.branch_currents,
            )
            for corner in result.points
            for point in corner.points
        ),
        [],
    )
    rows = ["\t".join(["Corner", "Index", "TemperatureKelvin", *selected_probes])]
    for corner in result.points:
        for index, point in enumerate(corner.points):
            values = [
                _format_table_number(
                    _table_probe_value(
                        point.result.node_voltages,
                        point.result.branch_currents,
                        probe,
                        "format_corner_temperature_dc_table",
                    )
                )
                for probe in selected_probes
            ]
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        str(index),
                        _format_table_number(point.temperature_kelvin),
                        *values,
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_dc_sweep_table(
    result: DcSweepResult,
    probes: list[str] | None = None,
) -> str:
    """Format a DC source sweep as a stable SPICE-style text table."""
    selected_probes = probes or next(
        (
            _default_output_probes(
                point.node_voltages,
                point.branch_currents,
            )
            for point in result.points
        ),
        [],
    )
    rows = ["\t".join(["Index", "Source", "Value", *selected_probes])]
    for index, point in enumerate(result.points):
        values = [
            _format_table_number(
                _table_probe_value(
                    point.node_voltages,
                    point.branch_currents,
                    probe,
                    "format_dc_sweep_table",
                )
            )
            for probe in selected_probes
        ]
        rows.append(
            "\t".join(
                [
                    str(index),
                    result.source_name,
                    _format_table_number(point.source_value),
                    *values,
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_corner_dc_sweep_table(
    result: CornerDcSweepResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner DC source sweeps as a stable SPICE-style table."""
    selected_probes = probes or next(
        (
            _default_output_probes(
                point.node_voltages,
                point.branch_currents,
            )
            for corner in result.points
            for point in corner.result.points
        ),
        [],
    )
    rows = ["\t".join(["Corner", "Index", "Source", "Value", *selected_probes])]
    for corner in result.points:
        for index, point in enumerate(corner.result.points):
            values = [
                _format_table_number(
                    _table_probe_value(
                        point.node_voltages,
                        point.branch_currents,
                        probe,
                        "format_corner_dc_sweep_table",
                    )
                )
                for probe in selected_probes
            ]
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        str(index),
                        result.source_name,
                        _format_table_number(point.source_value),
                        *values,
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_transient_table(
    transient_result: TransientResult | list[TransientPoint],
    probes: list[str] | None = None,
) -> str:
    """Format transient samples as a stable SPICE-style text table."""
    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    selected_probes = probes or _default_transient_output_probes(points)
    rows = ["\t".join(["Index", "Time", *selected_probes])]
    for index, point in enumerate(points):
        values = [
            _format_table_number(
                _table_probe_value(
                    point.node_voltages,
                    point.branch_currents,
                    probe,
                    "format_transient_table",
                )
            )
            for probe in selected_probes
        ]
        rows.append("\t".join([str(index), _format_table_number(point.time), *values]))
    rows.append("")
    return "\n".join(rows)


def format_corner_transient_table(
    result: CornerTransientResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner transient samples as a stable SPICE-style table."""
    selected_probes = probes or next(
        (
            _default_transient_output_probes(corner.points)
            for corner in result.points
            if corner.points
        ),
        [],
    )
    rows = ["\t".join(["Corner", "Index", "Time", *selected_probes])]
    for corner in result.points:
        for index, point in enumerate(corner.points):
            values = [
                _format_table_number(
                    _table_probe_value(
                        point.node_voltages,
                        point.branch_currents,
                        probe,
                        "format_corner_transient_table",
                    )
                )
                for probe in selected_probes
            ]
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        str(index),
                        _format_table_number(point.time),
                        *values,
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_corner_adaptive_transient_table(
    result: CornerAdaptiveTransientResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner adaptive transient samples as a stable table."""
    selected_probes = probes or next(
        (
            _default_transient_output_probes(corner.result.points)
            for corner in result.points
            if corner.result.points
        ),
        [],
    )
    rows = [
        "\t".join(
            [
                "Corner",
                "Method",
                "StepsRejected",
                "Converged",
                "Index",
                "Time",
                *selected_probes,
            ]
        )
    ]
    for corner in result.points:
        for index, point in enumerate(corner.result.points):
            values = [
                _format_table_number(
                    _table_probe_value(
                        point.node_voltages,
                        point.branch_currents,
                        probe,
                        "format_corner_adaptive_transient_table",
                    )
                )
                for probe in selected_probes
            ]
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        corner.result.method,
                        str(corner.result.steps_rejected),
                        str(corner.result.converged).lower(),
                        str(index),
                        _format_table_number(point.time),
                        *values,
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_pss_table(
    result: PssResult,
    probes: list[str] | None = None,
) -> str:
    """Format one-period PSS steady-state samples as a stable text table."""
    selected_probes = probes or _default_transient_output_probes(result.steady_state.points)
    rows = [
        "\t".join(
            [
                "Index",
                "Period",
                "TimeStep",
                "Converged",
                "Iterations",
                "ResidualL2",
                "Time",
                *selected_probes,
            ]
        )
    ]
    for index, point in enumerate(result.steady_state.points):
        values = [
            _format_table_number(
                _table_probe_value(
                    point.node_voltages,
                    point.branch_currents,
                    probe,
                    "format_pss_table",
                )
            )
            for probe in selected_probes
        ]
        rows.append(
            "\t".join(
                [
                    str(index),
                    _format_table_number(result.period),
                    _format_table_number(result.time_step),
                    str(result.converged).lower(),
                    str(result.solve.iteration_count),
                    _format_table_number(result.solve.final_residual.residual_l2_norm),
                    _format_table_number(point.time),
                    *values,
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_corner_pss_table(
    result: CornerPssResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner PSS steady-state samples as a stable text table."""
    selected_probes = probes or next(
        (
            _default_transient_output_probes(corner.result.steady_state.points)
            for corner in result.points
            if corner.result.steady_state.points
        ),
        [],
    )
    rows = [
        "\t".join(
            [
                "Corner",
                "Index",
                "Period",
                "TimeStep",
                "Converged",
                "Iterations",
                "ResidualL2",
                "Time",
                *selected_probes,
            ]
        )
    ]
    for corner in result.points:
        for index, point in enumerate(corner.result.steady_state.points):
            values = [
                _format_table_number(
                    _table_probe_value(
                        point.node_voltages,
                        point.branch_currents,
                        probe,
                        "format_corner_pss_table",
                    )
                )
                for probe in selected_probes
            ]
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        str(index),
                        _format_table_number(corner.result.period),
                        _format_table_number(corner.result.time_step),
                        str(corner.result.converged).lower(),
                        str(corner.result.solve.iteration_count),
                        _format_table_number(
                            corner.result.solve.final_residual.residual_l2_norm
                        ),
                        _format_table_number(point.time),
                        *values,
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_ac_table(result: AcResult | list[AcPoint], probes: list[str] | None = None) -> str:
    """Format AC phasors as a stable SPICE-style text table."""
    points = result.points if isinstance(result, AcResult) else result
    selected_probes = probes or _default_ac_output_probes(points)
    rows = ["Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase"]
    for index, point in enumerate(points):
        for probe in selected_probes:
            value = _table_complex_probe_value(
                point.node_voltages,
                point.branch_currents,
                probe,
                "format_ac_table",
            )
            rows.append(
                "\t".join(
                    [
                        str(index),
                        _format_table_number(point.freq),
                        probe,
                        _format_table_number(value.real),
                        _format_table_number(value.imag),
                        _format_table_number(abs(value)),
                        _format_table_number(math.degrees(cmath.phase(value))),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_deck_op_table(result: DcResult, netlist: str) -> str:
    """Format a DC operating point using parsed deck output cards."""

    return format_dc_table(result, select_deck_output_probes(netlist, "op"))


def format_deck_dc_sweep_table(result: DcSweepResult, netlist: str) -> str:
    """Format a DC sweep using parsed deck output cards."""

    return format_dc_sweep_table(result, select_deck_output_probes(netlist, "dc"))


def format_deck_ac_table(result: AcResult | list[AcPoint], netlist: str) -> str:
    """Format AC phasors using parsed deck output cards."""

    return format_ac_table(result, select_deck_output_probes(netlist, "ac"))


def format_deck_transient_table(
    transient_result: TransientResult | list[TransientPoint],
    netlist: str,
) -> str:
    """Format transient samples using parsed deck output cards."""

    return format_transient_table(
        transient_result,
        select_deck_output_probes(netlist, "tran"),
    )


def format_deck_tf_table(result: TfResult) -> str:
    """Format a deck-selected transfer-function result."""

    return format_tf_table(result)


def format_deck_sens_table(result: SensResult) -> str:
    """Format a deck-selected sensitivity result."""

    return format_sens_table(result)


def format_deck_noise_table(result: NoiseResult) -> str:
    """Format a deck-selected AC noise result."""

    return format_noise_table(result)


@dataclass(frozen=True)
class DeckRunArtifact:
    """Stable metadata for one selected deck analysis execution."""

    analysis: str
    directive: str
    analysis_directive_count: int
    analysis_directives: list[str]
    deck_analysis_kind_count: int
    deck_analysis_kinds: list[str]
    deck_analysis_directive_count: int
    deck_analysis_directives: list[str]
    line_number: int
    source_name: str | None
    output_node: str | None
    sweep_kind: str | None
    start_value: float | None
    stop_value: float | None
    step_value: float | None
    point_count: int | None
    start_frequency_hz: float | None
    stop_frequency_hz: float | None
    step_time: float | None
    stop_time: float | None
    start_time: float | None
    max_step: float | None
    use_initial_conditions: bool | None
    result_rows: int
    result_column_count: int
    result_columns: list[str]
    table_count: int
    tables: list[str]
    output_probe_count: int
    output_probes: list[str]
    output_directive_count: int
    output_directives: list[str]
    measurement_count: int
    measurement_names: list[str]
    fourier_count: int
    fourier_probes: list[str]
    control_line_count: int
    control_lines: list[str]
    write_marker_count: int
    write_markers: list[str]
    rawfile_option_count: int
    rawfile_options: list[str]
    control_policy_artifact_count: int
    control_policy_categories: list[str]
    control_policy_codes: list[str]
    control_policy_severities: list[str]
    diagnostic_count: int
    diagnostic_codes: list[str]


@dataclass(frozen=True)
class DeckTableArtifact:
    """Stable text and structured exports for one selected deck output table."""

    name: str
    table: str
    csv: str
    json: str
    records: list[dict[str, str]]


@dataclass(frozen=True)
class DeckOutputPlanArtifact:
    """Stable inventory of one selected deck execution output plan."""

    analysis: str
    directive: str
    line_number: int
    source_name: str | None
    output_node: str | None
    sweep_kind: str | None
    start_value: float | None
    stop_value: float | None
    step_value: float | None
    point_count: int | None
    start_frequency_hz: float | None
    stop_frequency_hz: float | None
    step_time: float | None
    stop_time: float | None
    start_time: float | None
    max_step: float | None
    use_initial_conditions: bool | None
    result_row_count: int
    result_column_count: int
    result_columns: list[str]
    output_probe_count: int
    output_probes: list[str]
    output_probe_line_count: int
    output_probe_lines: list[int]
    output_directive_count: int
    output_directives: list[str]
    output_directive_kind_count: int
    output_directive_kinds: list[str]
    output_directive_analysis_kind_count: int
    output_directive_analysis_kinds: list[str]
    output_directive_line_count: int
    output_directive_lines: list[int]
    table_count: int
    tables: list[str]


@dataclass(frozen=True)
class DeckControlPolicyArtifact:
    """Stable metadata for one policy-blocked control command."""

    line_number: int
    category: str
    command: str
    code: str
    severity: str
    message: str


@dataclass(frozen=True)
class DeckControlPolicySummaryArtifact:
    """Stable per-category summary for policy-blocked control commands."""

    category: str
    artifact_count: int
    line_numbers: list[int]
    commands: list[str]
    codes: list[str]
    severities: list[str]


@dataclass(frozen=True)
class DeckRawfileArtifact:
    """In-memory ASCII rawfile content for an accepted control write marker."""

    target: str
    marker: str
    probe_count: int
    probes: list[str]
    matched_probe_count: int
    matched_probes: list[str]
    unmatched_probe_count: int
    unmatched_probes: list[str]
    option_count: int
    options: list[str]
    rawfile: str


@dataclass(frozen=True)
class DeckWrdataArtifact:
    """In-memory ASCII data-file content for an accepted control wrdata marker."""

    target: str
    marker: str
    probe_count: int
    probes: list[str]
    matched_probe_count: int
    matched_probes: list[str]
    unmatched_probe_count: int
    unmatched_probes: list[str]
    option_count: int
    options: list[str]
    datafile: str


@dataclass(frozen=True)
class DeckAnalysisExecution:
    """A selected deck analysis plan plus its executed solver output."""

    plan: DeckAnalysisPlan
    result: DcResult | DcSweepResult | AcResult | TransientResult | TfResult | SensResult | NoiseResult
    table: str
    output_probes: list[str]
    output_directives: list[str]
    analysis_directives: list[str]
    deck_analysis_kind_count: int
    deck_analysis_kinds: list[str]
    deck_analysis_directive_count: int
    deck_analysis_directives: list[str]
    output_plan_artifact_count: int
    output_plan_artifacts: list[DeckOutputPlanArtifact]
    output_plan_artifact_table: str
    output_plan_artifact_csv: str
    output_plan_artifact_json: str
    output_plan_artifact_records: list[dict[str, str]]
    control_line_count: int
    control_lines: list[str]
    write_marker_count: int
    write_markers: list[str]
    rawfile_option_count: int
    rawfile_options: list[str]
    diagnostic_count: int
    diagnostic_codes: list[str]
    table_count: int
    tables: list[str]
    table_artifacts: list[DeckTableArtifact]
    measurements: list[ProbeMeasurement]
    measurement_table: str
    fourier: list[FourierResult]
    fourier_table: str
    run_artifacts: list[DeckRunArtifact]
    run_artifact_table: str
    control_policy_artifact_count: int = 0
    control_policy_artifacts: list[DeckControlPolicyArtifact] = field(default_factory=list)
    control_policy_artifact_table: str = ""
    control_policy_artifact_csv: str = ""
    control_policy_artifact_json: str = ""
    control_policy_artifact_records: list[dict[str, str]] = field(default_factory=list)
    control_policy_summary_artifact_count: int = 0
    control_policy_summary_artifacts: list[DeckControlPolicySummaryArtifact] = field(
        default_factory=list
    )
    control_policy_summary_artifact_table: str = ""
    control_policy_summary_artifact_csv: str = ""
    control_policy_summary_artifact_json: str = ""
    control_policy_summary_artifact_records: list[dict[str, str]] = field(
        default_factory=list
    )
    rawfile_artifact_count: int = 0
    rawfile_artifacts: list[DeckRawfileArtifact] = field(default_factory=list)
    rawfile_artifact_table: str = ""
    rawfile_artifact_csv: str = ""
    rawfile_artifact_json: str = ""
    rawfile_artifact_records: list[dict[str, str]] = field(default_factory=list)
    wrdata_artifact_count: int = 0
    wrdata_artifacts: list[DeckWrdataArtifact] = field(default_factory=list)
    wrdata_artifact_table: str = ""
    wrdata_artifact_csv: str = ""
    wrdata_artifact_json: str = ""
    wrdata_artifact_records: list[dict[str, str]] = field(default_factory=list)

    def __post_init__(self) -> None:
        control_policy_artifacts = list(self.control_policy_artifacts)
        object.__setattr__(
            self,
            "control_policy_artifact_count",
            len(control_policy_artifacts),
        )
        object.__setattr__(
            self,
            "control_policy_artifacts",
            control_policy_artifacts,
        )
        object.__setattr__(
            self,
            "control_policy_artifact_table",
            self.control_policy_artifact_table
            or format_deck_control_policy_artifact_table(control_policy_artifacts),
        )
        object.__setattr__(
            self,
            "control_policy_artifact_csv",
            self.control_policy_artifact_csv
            or format_deck_control_policy_artifact_csv(control_policy_artifacts),
        )
        object.__setattr__(
            self,
            "control_policy_artifact_json",
            self.control_policy_artifact_json
            or format_deck_control_policy_artifact_json(control_policy_artifacts),
        )
        object.__setattr__(
            self,
            "control_policy_artifact_records",
            self.control_policy_artifact_records
            or deck_control_policy_artifact_records(control_policy_artifacts),
        )
        control_policy_summary_artifacts = list(
            self.control_policy_summary_artifacts
        ) or _deck_control_policy_summary_artifacts(control_policy_artifacts)
        object.__setattr__(
            self,
            "control_policy_summary_artifact_count",
            len(control_policy_summary_artifacts),
        )
        object.__setattr__(
            self,
            "control_policy_summary_artifacts",
            control_policy_summary_artifacts,
        )
        object.__setattr__(
            self,
            "control_policy_summary_artifact_table",
            self.control_policy_summary_artifact_table
            or format_deck_control_policy_summary_artifact_table(
                control_policy_summary_artifacts
            ),
        )
        object.__setattr__(
            self,
            "control_policy_summary_artifact_csv",
            self.control_policy_summary_artifact_csv
            or format_deck_control_policy_summary_artifact_csv(
                control_policy_summary_artifacts
            ),
        )
        object.__setattr__(
            self,
            "control_policy_summary_artifact_json",
            self.control_policy_summary_artifact_json
            or format_deck_control_policy_summary_artifact_json(
                control_policy_summary_artifacts
            ),
        )
        object.__setattr__(
            self,
            "control_policy_summary_artifact_records",
            self.control_policy_summary_artifact_records
            or deck_control_policy_summary_artifact_records(
                control_policy_summary_artifacts
            ),
        )
        rawfile_artifacts = list(self.rawfile_artifacts) or _deck_rawfile_artifacts(
            self.plan,
            self.table,
            self.write_markers,
            self.rawfile_options,
        )
        object.__setattr__(
            self, "rawfile_artifact_count", len(rawfile_artifacts)
        )
        object.__setattr__(self, "rawfile_artifacts", rawfile_artifacts)
        object.__setattr__(
            self,
            "rawfile_artifact_table",
            self.rawfile_artifact_table
            or format_deck_rawfile_artifact_table(rawfile_artifacts),
        )
        object.__setattr__(
            self,
            "rawfile_artifact_csv",
            self.rawfile_artifact_csv
            or format_deck_rawfile_artifact_csv(rawfile_artifacts),
        )
        object.__setattr__(
            self,
            "rawfile_artifact_json",
            self.rawfile_artifact_json
            or format_deck_rawfile_artifact_json(rawfile_artifacts),
        )
        object.__setattr__(
            self,
            "rawfile_artifact_records",
            self.rawfile_artifact_records
            or deck_rawfile_artifact_records(rawfile_artifacts),
        )
        wrdata_artifacts = list(self.wrdata_artifacts) or _deck_wrdata_artifacts(
            self.table,
            self.write_markers,
            self.rawfile_options,
        )
        object.__setattr__(
            self, "wrdata_artifact_count", len(wrdata_artifacts)
        )
        object.__setattr__(self, "wrdata_artifacts", wrdata_artifacts)
        object.__setattr__(
            self,
            "wrdata_artifact_table",
            self.wrdata_artifact_table
            or format_deck_wrdata_artifact_table(wrdata_artifacts),
        )
        object.__setattr__(
            self,
            "wrdata_artifact_csv",
            self.wrdata_artifact_csv
            or format_deck_wrdata_artifact_csv(wrdata_artifacts),
        )
        object.__setattr__(
            self,
            "wrdata_artifact_json",
            self.wrdata_artifact_json
            or format_deck_wrdata_artifact_json(wrdata_artifacts),
        )
        object.__setattr__(
            self,
            "wrdata_artifact_records",
            self.wrdata_artifact_records
            or deck_wrdata_artifact_records(wrdata_artifacts),
        )


@dataclass(frozen=True)
class DeckExecution:
    """A full deck execution sequence in source analysis order."""

    execution_count: int
    analysis_order: list[str]
    analysis_directives: list[str]
    executions: list[DeckAnalysisExecution]
    run_artifact_count: int
    run_artifacts: list[DeckRunArtifact]
    run_artifact_table: str
    run_artifact_csv: str
    run_artifact_json: str
    run_artifact_records: list[dict[str, str]]


def _select_deck_measurement_cards_for_analysis(
    netlist: str,
    analysis: str,
) -> list[DeckMeasurementCard]:
    summary = resolve_deck_measurements(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"run_deck_analysis: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return [
        measurement
        for measurement in summary.measurements
        if measurement.analysis == analysis
        or (analysis == "tran" and measurement.analysis == "transient")
    ]


def _select_deck_fourier_cards_for_analysis(
    netlist: str,
    analysis: str,
) -> list[DeckFourierCard]:
    summary = resolve_deck_fourier(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"run_deck_analysis: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return list(summary.fourier) if analysis == "tran" else []


def _deck_result_row_count(
    result: DcResult | DcSweepResult | AcResult | TransientResult | TfResult | SensResult | NoiseResult,
) -> int:
    if isinstance(result, (DcResult, TfResult, SensResult)):
        return 1
    return len(result.points)


def _format_deck_artifact_float(value: float | None) -> str:
    return "" if value is None else f"{value:.6e}"


def _format_deck_artifact_bool(value: bool | None) -> str:
    return "" if value is None else str(value).lower()


_DECK_RUN_ARTIFACT_COLUMNS = [
    "Analysis",
    "Directive",
    "AnalysisDirectives",
    "AnalysisDirectiveList",
    "Line",
    "SourceName",
    "OutputNode",
    "SweepKind",
    "StartValue",
    "StopValue",
    "StepValue",
    "PointCount",
    "StartFrequencyHz",
    "StopFrequencyHz",
    "StepTime",
    "StopTime",
    "StartTime",
    "MaxStep",
    "UseInitialConditions",
    "ResultRows",
    "ResultColumns",
    "ResultColumnList",
    "Tables",
    "TableList",
    "OutputProbes",
    "OutputProbeList",
    "OutputDirectives",
    "OutputDirectiveList",
    "Measurements",
    "MeasurementList",
    "Fourier",
    "FourierList",
    "ControlLines",
    "ControlLineList",
    "WriteMarkers",
    "WriteMarkerList",
    "RawfileOptions",
    "RawfileOptionList",
    "ControlPolicyArtifacts",
    "ControlPolicyCategoryList",
    "ControlPolicyCodeList",
    "ControlPolicySeverityList",
    "Diagnostics",
    "DiagnosticCodeList",
    "DeckAnalysisKinds",
    "DeckAnalysisKindList",
    "DeckAnalysisDirectives",
    "DeckAnalysisDirectiveList",
]


def _deck_table_columns(table: str) -> list[str]:
    header = table.splitlines()[0] if table else ""
    return header.split("\t") if header else []


def _deck_analysis_directives(plan: DeckAnalysisPlan) -> list[str]:
    return [plan.directive] if plan.directive else []


def _deck_analysis_inventory(netlist: str) -> tuple[list[str], list[str]]:
    summary = resolve_deck_analyses(netlist)
    analysis_kinds: list[str] = []
    seen_kinds: set[str] = set()
    directives: list[str] = []
    for plan in summary.analyses:
        if plan.analysis and plan.analysis not in seen_kinds:
            seen_kinds.add(plan.analysis)
            analysis_kinds.append(plan.analysis)
        if plan.directive:
            directives.append(plan.directive)
    return analysis_kinds, directives


def _deck_stable_tables(
    measurements: list[ProbeMeasurement],
    fourier: list[FourierResult],
    control_policy_artifacts: list[DeckControlPolicyArtifact],
) -> list[str]:
    tables = ["result"]
    if measurements:
        tables.append("measurement")
    if fourier:
        tables.append("fourier")
    if control_policy_artifacts:
        tables.extend(["control-policy", "control-policy-summary"])
    tables.append("output-plan")
    tables.append("run-artifact")
    return tables


def _deck_output_directive_kind(directive: str) -> str:
    token = directive.strip().split(maxsplit=1)[0].lower()
    return token[1:] if token.startswith(".") else token


def _deck_output_directive_kinds(output_directives: Iterable[str]) -> list[str]:
    selected: list[str] = []
    seen: set[str] = set()
    for directive in output_directives:
        kind = _deck_output_directive_kind(directive)
        if not kind or kind in seen:
            continue
        seen.add(kind)
        selected.append(kind)
    return selected


def _deck_run_artifacts(
    plan: DeckAnalysisPlan,
    result: DcResult | DcSweepResult | AcResult | TransientResult | TfResult | SensResult | NoiseResult,
    result_columns: list[str],
    output_probes: list[str],
    output_directives: list[str],
    measurements: list[ProbeMeasurement],
    fourier: list[FourierResult],
    control_lines: list[str],
    write_markers: list[str],
    rawfile_options: list[str],
    diagnostic_codes: list[str],
    control_policy_artifacts: list[DeckControlPolicyArtifact],
    deck_analysis_kinds: list[str],
    deck_analysis_directives: list[str],
) -> list[DeckRunArtifact]:
    is_transient = plan.analysis == "tran"
    analysis_directives = _deck_analysis_directives(plan)
    tables = _deck_stable_tables(measurements, fourier, control_policy_artifacts)
    control_policy_summaries = _deck_control_policy_summary_artifacts(
        control_policy_artifacts
    )
    control_policy_categories = [
        artifact.category for artifact in control_policy_summaries
    ]
    control_policy_codes = [
        code for artifact in control_policy_summaries for code in artifact.codes
    ]
    control_policy_severities: list[str] = []
    for artifact in control_policy_summaries:
        for severity in artifact.severities:
            if severity not in control_policy_severities:
                control_policy_severities.append(severity)
    return [
        DeckRunArtifact(
            analysis=plan.analysis,
            directive=plan.directive,
            analysis_directive_count=len(analysis_directives),
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            line_number=plan.line_number,
            source_name=plan.source_name,
            output_node=plan.output_node,
            sweep_kind=plan.sweep_kind,
            start_value=plan.start_value,
            stop_value=plan.stop_value,
            step_value=plan.step_value,
            point_count=plan.point_count,
            start_frequency_hz=plan.start_frequency,
            stop_frequency_hz=plan.stop_frequency,
            step_time=plan.step_time if is_transient else None,
            stop_time=plan.stop_time if is_transient else None,
            start_time=plan.start_time if is_transient else None,
            max_step=plan.max_step if is_transient else None,
            use_initial_conditions=(
                plan.use_initial_conditions if is_transient else None
            ),
            result_rows=_deck_result_row_count(result),
            result_column_count=len(result_columns),
            result_columns=list(result_columns),
            table_count=len(tables),
            tables=tables,
            output_probe_count=len(output_probes),
            output_probes=list(output_probes),
            output_directive_count=len(output_directives),
            output_directives=list(output_directives),
            measurement_count=len(measurements),
            measurement_names=[measurement.name for measurement in measurements],
            fourier_count=len(fourier),
            fourier_probes=[
                probe.probe for result in fourier for probe in result.probes
            ],
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            control_policy_artifact_count=len(control_policy_artifacts),
            control_policy_categories=control_policy_categories,
            control_policy_codes=control_policy_codes,
            control_policy_severities=control_policy_severities,
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
        )
    ]


def _deck_output_plan_artifacts(
    plan: DeckAnalysisPlan,
    result_row_count: int,
    result_columns: list[str],
    output_probes: list[str],
    output_probe_lines: list[int],
    output_directives: list[str],
    output_directive_analysis_kinds: list[str],
    output_directive_lines: list[int],
    tables: list[str],
) -> list[DeckOutputPlanArtifact]:
    output_directive_kinds = _deck_output_directive_kinds(output_directives)
    is_transient = plan.analysis == "tran"
    return [
        DeckOutputPlanArtifact(
            analysis=plan.analysis,
            directive=plan.directive,
            line_number=plan.line_number,
            source_name=plan.source_name,
            output_node=plan.output_node,
            sweep_kind=plan.sweep_kind,
            start_value=plan.start_value,
            stop_value=plan.stop_value,
            step_value=plan.step_value,
            point_count=plan.point_count,
            start_frequency_hz=plan.start_frequency,
            stop_frequency_hz=plan.stop_frequency,
            step_time=plan.step_time if is_transient else None,
            stop_time=plan.stop_time if is_transient else None,
            start_time=plan.start_time if is_transient else None,
            max_step=plan.max_step if is_transient else None,
            use_initial_conditions=(
                plan.use_initial_conditions if is_transient else None
            ),
            result_row_count=result_row_count,
            result_column_count=len(result_columns),
            result_columns=list(result_columns),
            output_probe_count=len(output_probes),
            output_probes=list(output_probes),
            output_probe_line_count=len(output_probe_lines),
            output_probe_lines=list(output_probe_lines),
            output_directive_count=len(output_directives),
            output_directives=list(output_directives),
            output_directive_kind_count=len(output_directive_kinds),
            output_directive_kinds=output_directive_kinds,
            output_directive_analysis_kind_count=len(
                output_directive_analysis_kinds
            ),
            output_directive_analysis_kinds=list(output_directive_analysis_kinds),
            output_directive_line_count=len(output_directive_lines),
            output_directive_lines=list(output_directive_lines),
            table_count=len(tables),
            tables=list(tables),
        )
    ]


_DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS = [
    "Analysis",
    "Directive",
    "Line",
    "SourceName",
    "OutputNode",
    "SweepKind",
    "StartValue",
    "StopValue",
    "StepValue",
    "PointCount",
    "StartFrequencyHz",
    "StopFrequencyHz",
    "StepTime",
    "StopTime",
    "StartTime",
    "MaxStep",
    "UseInitialConditions",
    "ResultRows",
    "ResultColumns",
    "ResultColumnList",
    "OutputProbes",
    "OutputProbeList",
    "OutputProbeLines",
    "OutputProbeLineList",
    "OutputDirectives",
    "OutputDirectiveList",
    "OutputDirectiveKinds",
    "OutputDirectiveKindList",
    "OutputDirectiveAnalysisKinds",
    "OutputDirectiveAnalysisKindList",
    "OutputDirectiveLines",
    "OutputDirectiveLineList",
    "Tables",
    "TableList",
]


def _deck_output_plan_artifact_cells(artifact: DeckOutputPlanArtifact) -> list[str]:
    return [
        artifact.analysis,
        artifact.directive,
        str(artifact.line_number),
        artifact.source_name or "",
        artifact.output_node or "",
        artifact.sweep_kind or "",
        _format_deck_artifact_float(artifact.start_value),
        _format_deck_artifact_float(artifact.stop_value),
        _format_deck_artifact_float(artifact.step_value),
        "" if artifact.point_count is None else str(artifact.point_count),
        _format_deck_artifact_float(artifact.start_frequency_hz),
        _format_deck_artifact_float(artifact.stop_frequency_hz),
        _format_deck_artifact_float(artifact.step_time),
        _format_deck_artifact_float(artifact.stop_time),
        _format_deck_artifact_float(artifact.start_time),
        _format_deck_artifact_float(artifact.max_step),
        _format_deck_artifact_bool(artifact.use_initial_conditions),
        str(artifact.result_row_count),
        str(artifact.result_column_count),
        ";".join(artifact.result_columns),
        str(artifact.output_probe_count),
        ";".join(artifact.output_probes),
        str(artifact.output_probe_line_count),
        ";".join(str(line) for line in artifact.output_probe_lines),
        str(artifact.output_directive_count),
        ";".join(artifact.output_directives),
        str(artifact.output_directive_kind_count),
        ";".join(artifact.output_directive_kinds),
        str(artifact.output_directive_analysis_kind_count),
        ";".join(artifact.output_directive_analysis_kinds),
        str(artifact.output_directive_line_count),
        ";".join(str(line) for line in artifact.output_directive_lines),
        str(artifact.table_count),
        ";".join(artifact.tables),
    ]


def deck_output_plan_artifact_records(
    artifacts: Iterable[DeckOutputPlanArtifact],
) -> list[dict[str, str]]:
    """Format selected output-plan artifacts as header-keyed records."""

    return [
        dict(
            zip(
                _DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS,
                _deck_output_plan_artifact_cells(artifact),
                strict=True,
            )
        )
        for artifact in artifacts
    ]


def format_deck_output_plan_artifact_table(
    artifacts: Iterable[DeckOutputPlanArtifact],
) -> str:
    """Format selected output-plan artifacts as a stable summary table."""

    rows = ["\t".join(_DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append("\t".join(_deck_output_plan_artifact_cells(artifact)))
    return "\n".join(rows) + "\n"


def format_deck_output_plan_artifact_csv(
    artifacts: Iterable[DeckOutputPlanArtifact],
) -> str:
    """Format selected output-plan artifacts as stable RFC 4180-style CSV."""

    rows = [",".join(_DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append(
            ",".join(
                _format_csv_cell(cell)
                for cell in _deck_output_plan_artifact_cells(artifact)
            )
        )
    return "\n".join(rows) + "\n"


def format_deck_output_plan_artifact_json(
    artifacts: Iterable[DeckOutputPlanArtifact],
) -> str:
    """Format selected output-plan artifacts as stable compact JSON records."""

    return json.dumps(
        deck_output_plan_artifact_records(artifacts),
        separators=(",", ":"),
    ) + "\n"


def _deck_output_plan_artifact_bundle(
    plan: DeckAnalysisPlan,
    result_table: str,
    output_probes: list[str],
    output_probe_lines: list[int],
    output_directives: list[str],
    output_directive_analysis_kinds: list[str],
    output_directive_lines: list[int],
    tables: list[str],
) -> tuple[list[DeckOutputPlanArtifact], str, str, str, list[dict[str, str]]]:
    artifacts = _deck_output_plan_artifacts(
        plan,
        _deck_table_row_count(result_table),
        _deck_table_columns(result_table),
        output_probes,
        output_probe_lines,
        output_directives,
        output_directive_analysis_kinds,
        output_directive_lines,
        tables,
    )
    return (
        artifacts,
        format_deck_output_plan_artifact_table(artifacts),
        format_deck_output_plan_artifact_csv(artifacts),
        format_deck_output_plan_artifact_json(artifacts),
        deck_output_plan_artifact_records(artifacts),
    )


def _deck_table_row_count(table: str) -> int:
    rows = table.splitlines()
    return max(len(rows) - 1, 0) if rows else 0


def _deck_analysis_diagnostic_codes(
    netlist: str,
    plan: DeckAnalysisPlan,
) -> list[str]:
    summary = resolve_deck_analyses(netlist)
    return [
        diagnostic.code
        for diagnostic in summary.diagnostics
        if diagnostic.line_number == plan.line_number
        and diagnostic.directive == plan.directive
    ]


def _deck_control_diagnostic_codes(netlist: str) -> list[str]:
    summary = analyze_deck_controls(netlist)
    return [
        diagnostic.code
        for diagnostic in summary.diagnostics
        if diagnostic.code.startswith("SPICE_DECK_CONTROL_")
    ]


def _deck_control_lines(netlist: str) -> list[str]:
    return list(analyze_deck_controls(netlist).control_lines)


def _deck_control_write_markers(netlist: str) -> list[str]:
    return list(analyze_deck_controls(netlist).write_markers)


def _deck_control_rawfile_options(netlist: str) -> list[str]:
    return list(analyze_deck_controls(netlist).rawfile_options)


def _deck_run_diagnostic_codes(netlist: str, plan: DeckAnalysisPlan) -> list[str]:
    return [
        *_deck_analysis_diagnostic_codes(netlist, plan),
        *_deck_control_diagnostic_codes(netlist),
    ]


def format_deck_run_artifact_table(artifacts: Iterable[DeckRunArtifact]) -> str:
    """Format selected deck-run artifacts as a stable summary table."""

    rows = ["\t".join(_DECK_RUN_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append("\t".join(_deck_run_artifact_cells(artifact)))
    return "\n".join(rows) + "\n"


def _deck_run_artifact_cells(artifact: DeckRunArtifact) -> list[str]:
    return [
        artifact.analysis,
        artifact.directive,
        str(artifact.analysis_directive_count),
        ";".join(artifact.analysis_directives),
        str(artifact.line_number),
        artifact.source_name or "",
        artifact.output_node or "",
        artifact.sweep_kind or "",
        _format_deck_artifact_float(artifact.start_value),
        _format_deck_artifact_float(artifact.stop_value),
        _format_deck_artifact_float(artifact.step_value),
        "" if artifact.point_count is None else str(artifact.point_count),
        _format_deck_artifact_float(artifact.start_frequency_hz),
        _format_deck_artifact_float(artifact.stop_frequency_hz),
        _format_deck_artifact_float(artifact.step_time),
        _format_deck_artifact_float(artifact.stop_time),
        _format_deck_artifact_float(artifact.start_time),
        _format_deck_artifact_float(artifact.max_step),
        _format_deck_artifact_bool(artifact.use_initial_conditions),
        str(artifact.result_rows),
        str(artifact.result_column_count),
        ";".join(artifact.result_columns),
        str(artifact.table_count),
        ";".join(artifact.tables),
        str(artifact.output_probe_count),
        ";".join(artifact.output_probes),
        str(artifact.output_directive_count),
        ";".join(artifact.output_directives),
        str(artifact.measurement_count),
        ";".join(artifact.measurement_names),
        str(artifact.fourier_count),
        ";".join(artifact.fourier_probes),
        str(artifact.control_line_count),
        ";".join(artifact.control_lines),
        str(artifact.write_marker_count),
        ";".join(artifact.write_markers),
        str(artifact.rawfile_option_count),
        ";".join(artifact.rawfile_options),
        str(artifact.control_policy_artifact_count),
        ";".join(artifact.control_policy_categories),
        ";".join(artifact.control_policy_codes),
        ";".join(artifact.control_policy_severities),
        str(artifact.diagnostic_count),
        ";".join(artifact.diagnostic_codes),
        str(artifact.deck_analysis_kind_count),
        ";".join(artifact.deck_analysis_kinds),
        str(artifact.deck_analysis_directive_count),
        ";".join(artifact.deck_analysis_directives),
    ]


def _deck_run_artifact_record(artifact: DeckRunArtifact) -> dict[str, str]:
    return dict(
        zip(
            _DECK_RUN_ARTIFACT_COLUMNS,
            _deck_run_artifact_cells(artifact),
            strict=True,
        )
    )


def deck_run_artifact_records(
    artifacts: Iterable[DeckRunArtifact],
) -> list[dict[str, str]]:
    """Return stable run-artifact records keyed by exported column name."""

    return [_deck_run_artifact_record(artifact) for artifact in artifacts]


def _format_csv_cell(value: str) -> str:
    if any(character in value for character in [",", '"', "\n", "\r"]):
        return '"' + value.replace('"', '""') + '"'
    return value


def format_deck_table_csv(table: str) -> str:
    """Format a stable tab-separated deck table as RFC 4180-style CSV."""

    rows = table.splitlines()
    if not rows:
        return ""
    return (
        "\n".join(
            ",".join(_format_csv_cell(cell) for cell in row.split("\t"))
            for row in rows
        )
        + "\n"
    )


def deck_table_records(table: str) -> list[dict[str, str]]:
    """Parse a stable tab-separated deck table into header-keyed records."""

    rows = table.splitlines()
    if not rows:
        return []
    columns = rows[0].split("\t")
    records: list[dict[str, str]] = []
    for row in rows[1:]:
        cells = row.split("\t")
        records.append(
            {
                column: cells[index] if index < len(cells) else ""
                for index, column in enumerate(columns)
            }
        )
    return records


def format_deck_table_json(table: str) -> str:
    """Format a stable tab-separated deck table as compact JSON records."""

    return json.dumps(deck_table_records(table), separators=(",", ":")) + "\n"


def _deck_table_artifact(name: str, table: str) -> DeckTableArtifact:
    return DeckTableArtifact(
        name=name,
        table=table,
        csv=format_deck_table_csv(table),
        json=format_deck_table_json(table),
        records=deck_table_records(table),
    )


def _deck_table_artifacts(
    plan: DeckAnalysisPlan,
    result_table: str,
    measurement_table: str,
    fourier_table: str,
    run_artifact_table: str,
    measurements: list[ProbeMeasurement],
    fourier: list[FourierResult],
    control_policy_artifacts: list[DeckControlPolicyArtifact],
    control_policy_artifact_table: str,
    control_policy_summary_artifacts: list[DeckControlPolicySummaryArtifact],
    control_policy_summary_artifact_table: str,
    output_probes: list[str],
    output_probe_lines: list[int],
    output_directives: list[str],
    output_directive_analysis_kinds: list[str],
    output_directive_lines: list[int],
    tables: list[str],
) -> list[DeckTableArtifact]:
    artifacts = [_deck_table_artifact("result", result_table)]
    if measurements:
        artifacts.append(_deck_table_artifact("measurement", measurement_table))
    if fourier:
        artifacts.append(_deck_table_artifact("fourier", fourier_table))
    if control_policy_artifacts:
        artifacts.append(
            _deck_table_artifact("control-policy", control_policy_artifact_table)
        )
    if control_policy_summary_artifacts:
        artifacts.append(
            _deck_table_artifact(
                "control-policy-summary",
                control_policy_summary_artifact_table,
            )
        )
    output_plan_artifact_table = _deck_output_plan_artifact_bundle(
        plan,
        result_table,
        output_probes,
        output_probe_lines,
        output_directives,
        output_directive_analysis_kinds,
        output_directive_lines,
        tables,
    )[1]
    artifacts.append(_deck_table_artifact("output-plan", output_plan_artifact_table))
    artifacts.append(_deck_table_artifact("run-artifact", run_artifact_table))
    return artifacts


_DECK_CONTROL_POLICY_CODES = {
    "SPICE_DECK_CONTROL_SCRIPT_COMMAND": "script",
    "SPICE_DECK_CONTROL_WORKDIR_COMMAND": "workdir",
    "SPICE_DECK_CONTROL_FLOW_COMMAND": "control-flow",
    "SPICE_DECK_CONTROL_VARIABLE_COMMAND": "variable",
}


def _deck_control_policy_artifacts(netlist: str) -> list[DeckControlPolicyArtifact]:
    summary = analyze_deck_controls(netlist)
    lines = netlist.splitlines()
    artifacts: list[DeckControlPolicyArtifact] = []
    for diagnostic in summary.diagnostics:
        category = _DECK_CONTROL_POLICY_CODES.get(diagnostic.code)
        if category is None:
            continue
        command = (
            lines[diagnostic.line_number - 1].strip()
            if 0 < diagnostic.line_number <= len(lines)
            else ""
        )
        artifacts.append(
            DeckControlPolicyArtifact(
                line_number=diagnostic.line_number,
                category=category,
                command=command,
                code=diagnostic.code,
                severity=diagnostic.severity,
                message=diagnostic.message,
            )
        )
    return artifacts


_DECK_CONTROL_POLICY_ARTIFACT_COLUMNS = [
    "Line",
    "Category",
    "Command",
    "Code",
    "Severity",
    "Message",
]


def _deck_control_policy_artifact_cells(
    artifact: DeckControlPolicyArtifact,
) -> list[str]:
    return [
        str(artifact.line_number),
        artifact.category,
        artifact.command,
        artifact.code,
        artifact.severity,
        artifact.message,
    ]


def deck_control_policy_artifact_records(
    artifacts: Iterable[DeckControlPolicyArtifact],
) -> list[dict[str, str]]:
    """Format selected control-policy artifacts as header-keyed records."""

    return [
        dict(
            zip(
                _DECK_CONTROL_POLICY_ARTIFACT_COLUMNS,
                _deck_control_policy_artifact_cells(artifact),
                strict=True,
            )
        )
        for artifact in artifacts
    ]


def format_deck_control_policy_artifact_table(
    artifacts: Iterable[DeckControlPolicyArtifact],
) -> str:
    """Format selected control-policy artifacts as a stable summary table."""

    rows = ["\t".join(_DECK_CONTROL_POLICY_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append("\t".join(_deck_control_policy_artifact_cells(artifact)))
    return "\n".join(rows) + "\n"


def format_deck_control_policy_artifact_csv(
    artifacts: Iterable[DeckControlPolicyArtifact],
) -> str:
    """Format selected control-policy artifacts as stable RFC 4180-style CSV."""

    rows = [",".join(_DECK_CONTROL_POLICY_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append(
            ",".join(
                _format_csv_cell(cell)
                for cell in _deck_control_policy_artifact_cells(artifact)
            )
        )
    return "\n".join(rows) + "\n"


def format_deck_control_policy_artifact_json(
    artifacts: Iterable[DeckControlPolicyArtifact],
) -> str:
    """Format selected control-policy artifacts as stable compact JSON records."""

    return json.dumps(
        deck_control_policy_artifact_records(artifacts),
        separators=(",", ":"),
    ) + "\n"


def _append_unique_string(values: list[str], value: str) -> None:
    if value not in values:
        values.append(value)


def _deck_control_policy_summary_artifacts(
    artifacts: Iterable[DeckControlPolicyArtifact],
) -> list[DeckControlPolicySummaryArtifact]:
    categories: list[str] = []
    line_numbers_by_category: dict[str, list[int]] = {}
    commands_by_category: dict[str, list[str]] = {}
    codes_by_category: dict[str, list[str]] = {}
    severities_by_category: dict[str, list[str]] = {}
    for artifact in artifacts:
        category = artifact.category
        if category not in line_numbers_by_category:
            categories.append(category)
            line_numbers_by_category[category] = []
            commands_by_category[category] = []
            codes_by_category[category] = []
            severities_by_category[category] = []
        line_numbers_by_category[category].append(artifact.line_number)
        commands_by_category[category].append(artifact.command)
        _append_unique_string(codes_by_category[category], artifact.code)
        _append_unique_string(severities_by_category[category], artifact.severity)
    return [
        DeckControlPolicySummaryArtifact(
            category=category,
            artifact_count=len(line_numbers_by_category[category]),
            line_numbers=line_numbers_by_category[category],
            commands=commands_by_category[category],
            codes=codes_by_category[category],
            severities=severities_by_category[category],
        )
        for category in categories
    ]


_DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS = [
    "Category",
    "Artifacts",
    "LineList",
    "CommandList",
    "CodeList",
    "SeverityList",
]


def _deck_control_policy_summary_artifact_cells(
    artifact: DeckControlPolicySummaryArtifact,
) -> list[str]:
    return [
        artifact.category,
        str(artifact.artifact_count),
        ";".join(str(line_number) for line_number in artifact.line_numbers),
        ";".join(artifact.commands),
        ";".join(artifact.codes),
        ";".join(artifact.severities),
    ]


def deck_control_policy_summary_artifact_records(
    artifacts: Iterable[DeckControlPolicySummaryArtifact],
) -> list[dict[str, str]]:
    """Format selected control-policy summary artifacts as header-keyed records."""

    return [
        dict(
            zip(
                _DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS,
                _deck_control_policy_summary_artifact_cells(artifact),
                strict=True,
            )
        )
        for artifact in artifacts
    ]


def format_deck_control_policy_summary_artifact_table(
    artifacts: Iterable[DeckControlPolicySummaryArtifact],
) -> str:
    """Format selected control-policy summary artifacts as a stable summary table."""

    rows = ["\t".join(_DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append("\t".join(_deck_control_policy_summary_artifact_cells(artifact)))
    return "\n".join(rows) + "\n"


def format_deck_control_policy_summary_artifact_csv(
    artifacts: Iterable[DeckControlPolicySummaryArtifact],
) -> str:
    """Format selected control-policy summary artifacts as RFC 4180-style CSV."""

    rows = [",".join(_DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append(
            ",".join(
                _format_csv_cell(cell)
                for cell in _deck_control_policy_summary_artifact_cells(artifact)
            )
        )
    return "\n".join(rows) + "\n"


def format_deck_control_policy_summary_artifact_json(
    artifacts: Iterable[DeckControlPolicySummaryArtifact],
) -> str:
    """Format selected control-policy summary artifacts as compact JSON records."""

    return json.dumps(
        deck_control_policy_summary_artifact_records(artifacts),
        separators=(",", ":"),
    ) + "\n"


def format_deck_rawfile_ascii(
    table: str,
    analysis: str,
    rawfile_options: Iterable[str] = (),
) -> str:
    """Format a selected deck table as deterministic in-memory ASCII rawfile text."""

    return _format_deck_rawfile_ascii(table, analysis, rawfile_options, ())


def _format_deck_rawfile_ascii(
    table: str,
    analysis: str,
    rawfile_options: Iterable[str],
    probes: Iterable[str],
) -> str:
    rows = table.splitlines()
    if not rows:
        return ""
    probe_list = list(probes)
    projected_rows = _deck_rawfile_project_rows(rows, probe_list)
    columns = projected_rows[0].split("\t")
    data_rows = [row.split("\t") for row in projected_rows[1:]]
    lines = [
        f"Title: SPICE deck {analysis} result",
        "Date: deterministic",
        f"Plotname: {analysis}",
        "Flags: real",
        f"No. Variables: {len(columns)}",
        f"No. Points: {len(data_rows)}",
        "Options: " + ";".join(rawfile_options),
        "Variables:",
    ]
    for index, column in enumerate(columns):
        lines.append(f"\t{index}\t{column}\treal")
    lines.append("Values:")
    for index, row in enumerate(data_rows):
        padded = [*row, *([""] * max(0, len(columns) - len(row)))]
        lines.append(f"{index}\t" + "\t".join(padded[: len(columns)]))
    return "\n".join(lines) + "\n"


def _deck_rawfile_project_rows(rows: list[str], probes: list[str]) -> list[str]:
    columns = rows[0].split("\t")
    if not probes:
        return rows
    selected_indices, _, _ = _deck_rawfile_probe_inventory(columns, probes)
    projected_rows: list[str] = []
    for row in rows:
        cells = row.split("\t")
        projected_rows.append(
            "\t".join(
                cells[index] if index < len(cells) else ""
                for index in selected_indices
            )
        )
    return projected_rows


def _deck_rawfile_probe_inventory(
    columns: list[str],
    probes: list[str],
) -> tuple[list[int], list[str], list[str]]:
    selected_indices: list[int] = []
    matched_probes: list[str] = []
    unmatched_probes: list[str] = []
    if columns:
        selected_indices.append(0)
    normalized_columns = [column.casefold() for column in columns]
    for probe in probes:
        normalized_probe = probe.casefold()
        if normalized_probe not in normalized_columns:
            unmatched_probes.append(probe)
            continue
        index = normalized_columns.index(normalized_probe)
        if index not in selected_indices:
            selected_indices.append(index)
            matched_probes.append(columns[index])
    return selected_indices, matched_probes, unmatched_probes


def _deck_write_marker_parts(marker: str) -> tuple[str, list[str]] | None:
    parts = marker.split()
    if len(parts) < 2 or parts[0] != "write":
        return None
    return parts[1], parts[2:]


def _deck_rawfile_artifacts(
    plan: DeckAnalysisPlan,
    table: str,
    write_markers: Iterable[str],
    rawfile_options: Iterable[str],
) -> list[DeckRawfileArtifact]:
    options = list(rawfile_options)
    rows = table.splitlines()
    columns = rows[0].split("\t") if rows else []
    artifacts: list[DeckRawfileArtifact] = []
    for marker in write_markers:
        parts = _deck_write_marker_parts(marker)
        if parts is None:
            continue
        target, probes = parts
        _, matched_probes, unmatched_probes = _deck_rawfile_probe_inventory(
            columns, probes
        )
        artifacts.append(
            DeckRawfileArtifact(
                target=target,
                marker=marker,
                probe_count=len(probes),
                probes=probes,
                matched_probe_count=len(matched_probes),
                matched_probes=matched_probes,
                unmatched_probe_count=len(unmatched_probes),
                unmatched_probes=unmatched_probes,
                option_count=len(options),
                options=list(options),
                rawfile=_format_deck_rawfile_ascii(table, plan.analysis, options, probes),
            )
        )
    return artifacts


_DECK_RAWFILE_ARTIFACT_COLUMNS = [
    "Target",
    "Marker",
    "Probes",
    "ProbeList",
    "MatchedProbes",
    "MatchedProbeList",
    "UnmatchedProbes",
    "UnmatchedProbeList",
    "Options",
    "RawfileOptionList",
    "Bytes",
]


def _deck_rawfile_artifact_cells(artifact: DeckRawfileArtifact) -> list[str]:
    return [
        artifact.target,
        artifact.marker,
        str(artifact.probe_count),
        ";".join(artifact.probes),
        str(artifact.matched_probe_count),
        ";".join(artifact.matched_probes),
        str(artifact.unmatched_probe_count),
        ";".join(artifact.unmatched_probes),
        str(artifact.option_count),
        ";".join(artifact.options),
        str(len(artifact.rawfile.encode())),
    ]


def deck_rawfile_artifact_records(
    artifacts: Iterable[DeckRawfileArtifact],
) -> list[dict[str, str]]:
    """Format selected rawfile artifacts as header-keyed summary records."""

    return [
        dict(
            zip(
                _DECK_RAWFILE_ARTIFACT_COLUMNS,
                _deck_rawfile_artifact_cells(artifact),
                strict=True,
            )
        )
        for artifact in artifacts
    ]


def format_deck_rawfile_artifact_table(
    artifacts: Iterable[DeckRawfileArtifact],
) -> str:
    """Format selected rawfile artifacts as a stable summary table."""

    rows = ["\t".join(_DECK_RAWFILE_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append("\t".join(_deck_rawfile_artifact_cells(artifact)))
    return "\n".join(rows) + "\n"


def format_deck_rawfile_artifact_csv(
    artifacts: Iterable[DeckRawfileArtifact],
) -> str:
    """Format selected rawfile artifacts as stable RFC 4180-style CSV."""

    rows = [",".join(_DECK_RAWFILE_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append(
            ",".join(
                _format_csv_cell(cell)
                for cell in _deck_rawfile_artifact_cells(artifact)
            )
        )
    return "\n".join(rows) + "\n"


def format_deck_rawfile_artifact_json(
    artifacts: Iterable[DeckRawfileArtifact],
) -> str:
    """Format selected rawfile artifacts as stable compact JSON records."""

    return json.dumps(
        deck_rawfile_artifact_records(artifacts),
        separators=(",", ":"),
    ) + "\n"


def format_deck_wrdata_ascii(
    table: str,
    probes: Iterable[str] = (),
    rawfile_options: Iterable[str] = (),
) -> str:
    """Format a selected deck table as deterministic in-memory WRDATA text."""

    rows = table.splitlines()
    if not rows:
        return ""
    probe_list = list(probes)
    projected_rows = _deck_wrdata_project_rows(rows, probe_list)
    columns = projected_rows[0].split("\t")
    options = list(rawfile_options)
    lines = [
        "# SPICE deck wrdata artifact",
        "Probes: " + ";".join(probe_list),
    ]
    if options:
        lines.append("Options: " + ";".join(options))
    normalized_options = {option.casefold() for option in options}
    if "set wr_vecnames" in normalized_options:
        lines.append("VectorNames: " + ";".join(columns))
    if "set wr_singlescale" in normalized_options and columns:
        lines.append("Scale: " + columns[0])
    lines.extend(projected_rows)
    return "\n".join(lines) + "\n"


def _deck_wrdata_project_rows(rows: list[str], probes: list[str]) -> list[str]:
    columns = rows[0].split("\t")
    if not probes:
        return rows
    selected_indices, _, _ = _deck_wrdata_probe_inventory(columns, probes)
    projected_rows: list[str] = []
    for row in rows:
        cells = row.split("\t")
        projected_rows.append(
            "\t".join(
                cells[index] if index < len(cells) else ""
                for index in selected_indices
            )
        )
    return projected_rows


def _deck_wrdata_probe_inventory(
    columns: list[str],
    probes: list[str],
) -> tuple[list[int], list[str], list[str]]:
    selected_indices: list[int] = []
    matched_probes: list[str] = []
    unmatched_probes: list[str] = []
    if columns:
        selected_indices.append(0)
    normalized_columns = [column.casefold() for column in columns]
    for probe in probes:
        normalized_probe = probe.casefold()
        if normalized_probe not in normalized_columns:
            unmatched_probes.append(probe)
            continue
        index = normalized_columns.index(normalized_probe)
        if index not in selected_indices:
            selected_indices.append(index)
            matched_probes.append(columns[index])
    return selected_indices, matched_probes, unmatched_probes


def _deck_wrdata_marker_parts(marker: str) -> tuple[str, list[str]] | None:
    parts = marker.split()
    if len(parts) < 2 or parts[0] != "wrdata":
        return None
    return parts[1], parts[2:]


def _deck_wrdata_artifacts(
    table: str,
    write_markers: Iterable[str],
    rawfile_options: Iterable[str] = (),
) -> list[DeckWrdataArtifact]:
    artifacts: list[DeckWrdataArtifact] = []
    options = list(rawfile_options)
    rows = table.splitlines()
    columns = rows[0].split("\t") if rows else []
    for marker in write_markers:
        parts = _deck_wrdata_marker_parts(marker)
        if parts is None:
            continue
        target, probes = parts
        _, matched_probes, unmatched_probes = _deck_wrdata_probe_inventory(columns, probes)
        artifacts.append(
            DeckWrdataArtifact(
                target=target,
                marker=marker,
                probe_count=len(probes),
                probes=probes,
                matched_probe_count=len(matched_probes),
                matched_probes=matched_probes,
                unmatched_probe_count=len(unmatched_probes),
                unmatched_probes=unmatched_probes,
                option_count=len(options),
                options=list(options),
                datafile=format_deck_wrdata_ascii(table, probes, options),
            )
        )
    return artifacts


_DECK_WRDATA_ARTIFACT_COLUMNS = [
    "Target",
    "Marker",
    "Probes",
    "ProbeList",
    "MatchedProbes",
    "MatchedProbeList",
    "UnmatchedProbes",
    "UnmatchedProbeList",
    "Options",
    "RawfileOptionList",
    "Bytes",
]


def _deck_wrdata_artifact_cells(artifact: DeckWrdataArtifact) -> list[str]:
    return [
        artifact.target,
        artifact.marker,
        str(artifact.probe_count),
        ";".join(artifact.probes),
        str(artifact.matched_probe_count),
        ";".join(artifact.matched_probes),
        str(artifact.unmatched_probe_count),
        ";".join(artifact.unmatched_probes),
        str(artifact.option_count),
        ";".join(artifact.options),
        str(len(artifact.datafile.encode())),
    ]


def deck_wrdata_artifact_records(
    artifacts: Iterable[DeckWrdataArtifact],
) -> list[dict[str, str]]:
    """Format selected WRDATA artifacts as header-keyed summary records."""

    return [
        dict(
            zip(
                _DECK_WRDATA_ARTIFACT_COLUMNS,
                _deck_wrdata_artifact_cells(artifact),
                strict=True,
            )
        )
        for artifact in artifacts
    ]


def format_deck_wrdata_artifact_table(
    artifacts: Iterable[DeckWrdataArtifact],
) -> str:
    """Format selected WRDATA artifacts as a stable summary table."""

    rows = ["\t".join(_DECK_WRDATA_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append("\t".join(_deck_wrdata_artifact_cells(artifact)))
    return "\n".join(rows) + "\n"


def format_deck_wrdata_artifact_csv(
    artifacts: Iterable[DeckWrdataArtifact],
) -> str:
    """Format selected WRDATA artifacts as stable RFC 4180-style CSV."""

    rows = [",".join(_DECK_WRDATA_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append(
            ",".join(
                _format_csv_cell(cell)
                for cell in _deck_wrdata_artifact_cells(artifact)
            )
        )
    return "\n".join(rows) + "\n"


def format_deck_wrdata_artifact_json(
    artifacts: Iterable[DeckWrdataArtifact],
) -> str:
    """Format selected WRDATA artifacts as stable compact JSON records."""

    return json.dumps(
        deck_wrdata_artifact_records(artifacts),
        separators=(",", ":"),
    ) + "\n"


def format_deck_run_artifact_csv(artifacts: Iterable[DeckRunArtifact]) -> str:
    """Format selected deck-run artifacts as stable RFC 4180-style CSV."""

    rows = [",".join(_DECK_RUN_ARTIFACT_COLUMNS)]
    for artifact in artifacts:
        rows.append(
            ",".join(_format_csv_cell(cell) for cell in _deck_run_artifact_cells(artifact))
        )
    return "\n".join(rows) + "\n"


def format_deck_run_artifact_json(artifacts: Iterable[DeckRunArtifact]) -> str:
    """Format selected deck-run artifacts as stable compact JSON records."""

    records = [_deck_run_artifact_record(artifact) for artifact in artifacts]
    return json.dumps(records, separators=(",", ":")) + "\n"


def run_deck_analysis(
    circuit: Circuit,
    netlist: str,
    analysis: str | None = None,
) -> DeckAnalysisExecution:
    """Select one deck analysis card, execute it, and format deck-selected output."""

    plan = select_deck_analysis_plan(netlist, analysis)
    return _run_deck_analysis_plan(circuit, netlist, plan)


def run_deck(circuit: Circuit, netlist: str) -> DeckExecution:
    """Execute every parsed deck analysis card in source order."""

    plans = _deck_analysis_plans_for_execution(netlist, "run_deck")
    executions = [
        _run_deck_analysis_plan(circuit, netlist, plan) for plan in plans
    ]
    run_artifacts = [
        artifact for execution in executions for artifact in execution.run_artifacts
    ]
    return DeckExecution(
        execution_count=len(executions),
        analysis_order=[plan.analysis for plan in plans],
        analysis_directives=[plan.directive for plan in plans if plan.directive],
        executions=executions,
        run_artifact_count=len(run_artifacts),
        run_artifacts=run_artifacts,
        run_artifact_table=format_deck_run_artifact_table(run_artifacts),
        run_artifact_csv=format_deck_run_artifact_csv(run_artifacts),
        run_artifact_json=format_deck_run_artifact_json(run_artifacts),
        run_artifact_records=deck_run_artifact_records(run_artifacts),
    )


def _deck_analysis_plans_for_execution(
    netlist: str,
    context: str,
) -> list[DeckAnalysisPlan]:
    summary = resolve_deck_analyses(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"{context}: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return list(summary.analyses) or [DeckAnalysisPlan(".op", "op", 0)]


def _run_deck_analysis_plan(
    circuit: Circuit,
    netlist: str,
    plan: DeckAnalysisPlan,
) -> DeckAnalysisExecution:
    """Execute an already resolved deck analysis plan."""

    diagnostic_codes = _deck_run_diagnostic_codes(netlist, plan)
    control_lines = _deck_control_lines(netlist)
    write_markers = _deck_control_write_markers(netlist)
    rawfile_options = _deck_control_rawfile_options(netlist)
    control_policy_artifacts = _deck_control_policy_artifacts(netlist)
    control_policy_artifact_table = format_deck_control_policy_artifact_table(
        control_policy_artifacts
    )
    control_policy_summary_artifacts = _deck_control_policy_summary_artifacts(
        control_policy_artifacts
    )
    control_policy_summary_artifact_table = (
        format_deck_control_policy_summary_artifact_table(
            control_policy_summary_artifacts
        )
    )
    analysis_directives = _deck_analysis_directives(plan)
    deck_analysis_kinds, deck_analysis_directives = _deck_analysis_inventory(netlist)
    if plan.analysis == "op":
        result = dc_op(circuit)
        table = format_deck_op_table(result, netlist)
        _select_deck_measurement_cards_for_analysis(netlist, plan.analysis)
        fourier: list[FourierResult] = []
        _select_deck_fourier_cards_for_analysis(netlist, plan.analysis)
        measurements: list[ProbeMeasurement] = []
        output_probes = select_deck_output_probes(netlist, plan.analysis)
        output_probe_lines = select_deck_output_probe_lines(netlist, plan.analysis)
        output_directives = select_deck_output_directives(netlist, plan.analysis)
        output_directive_analysis_kinds = select_deck_output_directive_analysis_kinds(
            netlist,
            plan.analysis,
        )
        output_directive_lines = select_deck_output_directive_lines(
            netlist,
            plan.analysis,
        )
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    if plan.analysis == "dc":
        source_name = _require_deck_plan_string(plan, "source_name")
        start = _require_deck_plan_number(plan, "start_value")
        stop = _require_deck_plan_number(plan, "stop_value")
        step = _require_deck_plan_number(plan, "step_value")
        result = dc_sweep(circuit, source_name, start, stop, step)
        table = format_deck_dc_sweep_table(result, netlist)
        measurements = measure_dc_sweep_cards(
            result,
            _select_deck_measurement_cards_for_analysis(netlist, plan.analysis),
        )
        fourier = []
        _select_deck_fourier_cards_for_analysis(netlist, plan.analysis)
        output_probes = select_deck_output_probes(netlist, plan.analysis)
        output_probe_lines = select_deck_output_probe_lines(netlist, plan.analysis)
        output_directives = select_deck_output_directives(netlist, plan.analysis)
        output_directive_analysis_kinds = select_deck_output_directive_analysis_kinds(
            netlist,
            plan.analysis,
        )
        output_directive_lines = select_deck_output_directive_lines(
            netlist,
            plan.analysis,
        )
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    if plan.analysis == "ac":
        sweep_kind = _require_deck_plan_string(plan, "sweep_kind")
        point_count = _require_deck_plan_int(plan, "point_count")
        start_frequency = _require_deck_plan_number(plan, "start_frequency")
        stop_frequency = _require_deck_plan_number(plan, "stop_frequency")
        result = _run_deck_ac_sweep(
            circuit, plan, sweep_kind, point_count, start_frequency, stop_frequency
        )
        table = format_deck_ac_table(result, netlist)
        measurements = measure_ac_sweep_cards(
            result,
            _select_deck_measurement_cards_for_analysis(netlist, plan.analysis),
        )
        fourier = []
        _select_deck_fourier_cards_for_analysis(netlist, plan.analysis)
        output_probes = select_deck_output_probes(netlist, plan.analysis)
        output_probe_lines = select_deck_output_probe_lines(netlist, plan.analysis)
        output_directives = select_deck_output_directives(netlist, plan.analysis)
        output_directive_analysis_kinds = select_deck_output_directive_analysis_kinds(
            netlist,
            plan.analysis,
        )
        output_directive_lines = select_deck_output_directive_lines(
            netlist,
            plan.analysis,
        )
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    if plan.analysis == "tran":
        step_time = _require_deck_plan_number(plan, "step_time")
        stop_time = _require_deck_plan_number(plan, "stop_time")
        start_time = _optional_deck_plan_number(plan, "start_time")
        max_step = _optional_deck_plan_number(plan, "max_step")
        run_step = min(step_time, max_step) if max_step is not None else step_time
        result = _sample_transient_result_print_step(
            transient(circuit, t_step=run_step, t_stop=stop_time, method="euler"),
            step_time,
            start_time=start_time,
            stop_time=stop_time,
        )
        measurements = measure_transient_cards(
            result,
            _select_deck_measurement_cards_for_analysis(netlist, plan.analysis),
        )
        table = format_deck_transient_table(result, netlist)
        fourier = fourier_transient_cards(
            result,
            _select_deck_fourier_cards_for_analysis(netlist, plan.analysis),
        )
        output_probes = select_deck_output_probes(netlist, plan.analysis)
        output_probe_lines = select_deck_output_probe_lines(netlist, plan.analysis)
        output_directives = select_deck_output_directives(netlist, plan.analysis)
        output_directive_analysis_kinds = select_deck_output_directive_analysis_kinds(
            netlist,
            plan.analysis,
        )
        output_directive_lines = select_deck_output_directive_lines(
            netlist,
            plan.analysis,
        )
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    if plan.analysis == "tf":
        output_node = _require_deck_plan_string(plan, "output_node")
        input_source = _require_deck_plan_string(plan, "source_name")
        result = tf(circuit, output_node=output_node, input_source=input_source)
        _select_deck_measurement_cards_for_analysis(netlist, plan.analysis)
        measurements = []
        _select_deck_fourier_cards_for_analysis(netlist, plan.analysis)
        fourier = []
        output_probes = [f"V({output_node})"]
        output_probe_lines: list[int] = []
        output_directives: list[str] = []
        output_directive_analysis_kinds: list[str] = []
        output_directive_lines: list[int] = []
        table = format_deck_tf_table(result)
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    if plan.analysis == "sens":
        output_node = _require_deck_plan_string(plan, "output_node")
        result = sens_dc(circuit, output_node=output_node)
        _select_deck_measurement_cards_for_analysis(netlist, plan.analysis)
        measurements = []
        _select_deck_fourier_cards_for_analysis(netlist, plan.analysis)
        fourier = []
        output_probes = [f"V({output_node})"]
        output_probe_lines: list[int] = []
        output_directives = []
        output_directive_analysis_kinds: list[str] = []
        output_directive_lines: list[int] = []
        table = format_deck_sens_table(result)
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    if plan.analysis == "noise":
        output_node = _require_deck_plan_string(plan, "output_node")
        input_source = _require_deck_plan_string(plan, "source_name")
        frequencies: list[float] | None = None
        if plan.sweep_kind is not None:
            sweep_kind = _require_deck_plan_string(plan, "sweep_kind")
            point_count = _require_deck_plan_int(plan, "point_count")
            start_frequency = _require_deck_plan_number(plan, "start_frequency")
            stop_frequency = _require_deck_plan_number(plan, "stop_frequency")
            frequencies = _deck_ac_frequencies(
                plan,
                sweep_kind,
                point_count,
                start_frequency,
                stop_frequency,
            )
        result = noise_ac(
            circuit,
            output_node=output_node,
            input_source=input_source,
            freqs=frequencies,
        )
        _select_deck_measurement_cards_for_analysis(netlist, plan.analysis)
        measurements = []
        _select_deck_fourier_cards_for_analysis(netlist, plan.analysis)
        fourier = []
        output_probes = [f"V({output_node})"]
        output_probe_lines: list[int] = []
        output_directives = []
        output_directive_analysis_kinds: list[str] = []
        output_directive_lines: list[int] = []
        table = format_deck_noise_table(result)
        run_artifacts = _deck_run_artifacts(
            plan,
            result,
            _deck_table_columns(table),
            output_probes,
            output_directives,
            measurements,
            fourier,
            control_lines,
            write_markers,
            rawfile_options,
            diagnostic_codes,
            control_policy_artifacts,
            deck_analysis_kinds,
            deck_analysis_directives,
        )
        measurement_table = format_measurement_table(measurements)
        fourier_table = format_deck_fourier_table(fourier)
        run_artifact_table = format_deck_run_artifact_table(run_artifacts)
        tables = _deck_stable_tables(
            measurements,
            fourier,
            control_policy_artifacts,
        )
        table_artifacts = _deck_table_artifacts(
            plan,
            table,
            measurement_table,
            fourier_table,
            run_artifact_table,
            measurements,
            fourier,
            control_policy_artifacts,
            control_policy_artifact_table,
            control_policy_summary_artifacts,
            control_policy_summary_artifact_table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        (
            output_plan_artifacts,
            output_plan_artifact_table,
            output_plan_artifact_csv,
            output_plan_artifact_json,
            output_plan_artifact_records,
        ) = _deck_output_plan_artifact_bundle(
            plan,
            table,
            output_probes,
            output_probe_lines,
            output_directives,
            output_directive_analysis_kinds,
            output_directive_lines,
            tables,
        )
        return DeckAnalysisExecution(
            plan=plan,
            result=result,
            table=table,
            output_probes=output_probes,
            output_directives=output_directives,
            analysis_directives=analysis_directives,
            deck_analysis_kind_count=len(deck_analysis_kinds),
            deck_analysis_kinds=list(deck_analysis_kinds),
            deck_analysis_directive_count=len(deck_analysis_directives),
            deck_analysis_directives=list(deck_analysis_directives),
            output_plan_artifact_count=len(output_plan_artifacts),
            output_plan_artifacts=output_plan_artifacts,
            output_plan_artifact_table=output_plan_artifact_table,
            output_plan_artifact_csv=output_plan_artifact_csv,
            output_plan_artifact_json=output_plan_artifact_json,
            output_plan_artifact_records=output_plan_artifact_records,
            control_line_count=len(control_lines),
            control_lines=list(control_lines),
            write_marker_count=len(write_markers),
            write_markers=list(write_markers),
            rawfile_option_count=len(rawfile_options),
            rawfile_options=list(rawfile_options),
            diagnostic_count=len(diagnostic_codes),
            diagnostic_codes=list(diagnostic_codes),
            table_count=len(tables),
            tables=tables,
            table_artifacts=table_artifacts,
            measurements=measurements,
            measurement_table=measurement_table,
            fourier=fourier,
            fourier_table=fourier_table,
            run_artifacts=run_artifacts,
            run_artifact_table=run_artifact_table,
            control_policy_artifacts=control_policy_artifacts,
        )
    raise ValueError(f"run_deck_analysis: unsupported analysis {plan.analysis!r}")


def _require_deck_plan_string(plan: DeckAnalysisPlan, field_name: str) -> str:
    value = getattr(plan, field_name)
    if isinstance(value, str) and value:
        return value
    raise ValueError(
        f"run_deck_analysis: line {plan.line_number}: "
        f"{plan.directive} analysis missing {field_name}"
    )


def _require_deck_plan_number(plan: DeckAnalysisPlan, field_name: str) -> float:
    value = getattr(plan, field_name)
    if isinstance(value, (float, int)):
        return float(value)
    raise ValueError(
        f"run_deck_analysis: line {plan.line_number}: "
        f"{plan.directive} analysis missing {field_name}"
    )


def _optional_deck_plan_number(plan: DeckAnalysisPlan, field_name: str) -> float | None:
    value = getattr(plan, field_name)
    if value is None:
        return None
    if isinstance(value, (float, int)):
        return float(value)
    raise ValueError(
        f"run_deck_analysis: line {plan.line_number}: "
        f"{plan.directive} analysis has invalid {field_name}"
    )


def _require_deck_plan_int(plan: DeckAnalysisPlan, field_name: str) -> int:
    value = getattr(plan, field_name)
    if isinstance(value, int):
        return value
    raise ValueError(
        f"run_deck_analysis: line {plan.line_number}: "
        f"{plan.directive} analysis missing {field_name}"
    )


def _sample_transient_result_print_step(
    result: TransientResult,
    print_step: float,
    *,
    start_time: float | None,
    stop_time: float,
) -> TransientResult:
    if not result.points:
        return result
    epsilon = max(abs(stop_time), abs(print_step), 1.0) * 1.0e-12
    if start_time is not None and start_time > 0.0:
        report_start = start_time
    elif abs(result.points[0].time) <= epsilon:
        report_start = 0.0
    else:
        report_start = print_step

    sampled_points: list[TransientPoint] = []
    index = 0
    while True:
        sample_time = report_start + index * print_step
        if sample_time > stop_time + epsilon:
            break
        sampled_points.append(_interpolate_transient_point(result.points, sample_time))
        index += 1

    return TransientResult(
        points=sampled_points,
        converged=result.converged,
        method=result.method,
        steps_rejected=result.steps_rejected,
    )


def _interpolate_transient_point(
    points: list[TransientPoint],
    time: float,
) -> TransientPoint:
    epsilon = max(abs(time), 1.0) * 1.0e-12
    for point in points:
        if abs(point.time - time) <= epsilon:
            return TransientPoint(
                time=time,
                node_voltages=dict(point.node_voltages),
                branch_currents=dict(point.branch_currents),
            )
    for left, right in zip(points, points[1:], strict=False):
        if left.time - epsilon <= time <= right.time + epsilon:
            span = right.time - left.time
            if span <= 0.0:
                return TransientPoint(
                    time=time,
                    node_voltages=dict(left.node_voltages),
                    branch_currents=dict(left.branch_currents),
                )
            alpha = (time - left.time) / span
            return TransientPoint(
                time=time,
                node_voltages=_interpolate_value_map(
                    left.node_voltages, right.node_voltages, alpha
                ),
                branch_currents=_interpolate_value_map(
                    left.branch_currents, right.branch_currents, alpha
                ),
            )
    raise ValueError("run_deck_analysis: transient print point is outside output")


def _interpolate_value_map(
    left: dict[str, float],
    right: dict[str, float],
    alpha: float,
) -> dict[str, float]:
    values: dict[str, float] = {}
    for key in set(left) | set(right):
        left_value = left.get(key, right.get(key, 0.0))
        right_value = right.get(key, left_value)
        values[key] = (1.0 - alpha) * left_value + alpha * right_value
    return values


def _run_deck_ac_sweep(
    circuit: Circuit,
    plan: DeckAnalysisPlan,
    sweep_kind: str,
    point_count: int,
    start_frequency: float,
    stop_frequency: float,
) -> AcResult:
    frequencies = _deck_ac_frequencies(
        plan, sweep_kind, point_count, start_frequency, stop_frequency
    )
    points: list[AcPoint] = []
    for frequency in frequencies:
        points.extend(
            ac_sweep(
                circuit,
                f_start=frequency,
                f_stop=frequency,
                n_points=1,
                sweep="lin",
            ).points
        )
    return AcResult(points=points)


def _deck_ac_frequencies(
    plan: DeckAnalysisPlan,
    sweep_kind: str,
    point_count: int,
    start_frequency: float,
    stop_frequency: float,
) -> list[float]:
    if point_count <= 0:
        raise ValueError(
            f"run_deck_analysis: line {plan.line_number}: "
            ".ac point_count must be positive"
        )
    if sweep_kind == "lin":
        if point_count == 1:
            return [start_frequency]
        step = (stop_frequency - start_frequency) / (point_count - 1)
        return [start_frequency + index * step for index in range(point_count)]
    if sweep_kind in {"dec", "oct"}:
        base = 10.0 if sweep_kind == "dec" else 2.0
        ratio = base ** (1.0 / point_count)
        epsilon = stop_frequency * 1.0e-12
        frequencies: list[float] = []
        frequency = start_frequency
        while frequency <= stop_frequency + epsilon:
            frequencies.append(frequency)
            frequency *= ratio
        return frequencies
    raise ValueError(
        f"run_deck_analysis: line {plan.line_number}: "
        f".ac {sweep_kind.upper()} execution is not supported yet"
    )


def format_corner_ac_table(
    result: CornerAcSweepResult,
    probes: list[str] | None = None,
) -> str:
    """Format named-corner AC phasors as a stable SPICE-style text table."""
    selected_probes = probes or next(
        (
            _default_ac_output_probes(corner.result.points)
            for corner in result.points
            if corner.result.points
        ),
        [],
    )
    rows = ["Corner\tIndex\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase"]
    for corner in result.points:
        for index, point in enumerate(corner.result.points):
            for probe in selected_probes:
                value = _table_complex_probe_value(
                    point.node_voltages,
                    point.branch_currents,
                    probe,
                    "format_corner_ac_table",
                )
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            str(index),
                            _format_table_number(point.freq),
                            probe,
                            _format_table_number(value.real),
                            _format_table_number(value.imag),
                            _format_table_number(abs(value)),
                            _format_table_number(math.degrees(cmath.phase(value))),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def format_tf_table(result: TfResult) -> str:
    """Format a transfer-function result as a stable SPICE-style text table."""
    return "\n".join(
        [
            "TransferRatio\tInputImpedance\tOutputImpedance",
            "\t".join(
                [
                    _format_table_number(result.transfer_ratio),
                    _format_table_number(result.input_impedance),
                    _format_table_number(result.output_impedance),
                ]
            ),
            "",
        ]
    )


def format_corner_tf_table(result: CornerTfResult) -> str:
    """Format named-corner transfer-function results as a stable text table."""
    rows = ["Corner\tTransferRatio\tInputImpedance\tOutputImpedance"]
    for point in result.points:
        rows.append(
            "\t".join(
                [
                    point.corner_name,
                    _format_table_number(point.result.transfer_ratio),
                    _format_table_number(point.result.input_impedance),
                    _format_table_number(point.result.output_impedance),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_mc_table(result: McResult) -> str:
    """Format Monte Carlo DC trials as a stable SPICE-style text table."""
    rows = ["Trial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged"]
    for point in result.points:
        output_value = (
            _format_table_number(_node_voltage(result.output_node, point.node_voltages))
            if point.converged
            else ""
        )
        rows.append(
            "\t".join(
                [
                    str(point.trial),
                    result.output_node,
                    output_value,
                    _format_table_number(result.mean),
                    _format_table_number(result.std_dev),
                    str(point.converged).lower(),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_corner_mc_table(result: CornerMcResult) -> str:
    """Format multi-corner Monte Carlo DC trials as a stable text table."""
    rows = ["Corner\tTrial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged"]
    for corner in result.points:
        for point in corner.result.points:
            output_value = (
                _format_table_number(_node_voltage(result.output_node, point.node_voltages))
                if point.converged
                else ""
            )
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        str(point.trial),
                        result.output_node,
                        output_value,
                        _format_table_number(corner.result.mean),
                        _format_table_number(corner.result.std_dev),
                        str(point.converged).lower(),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_sens_table(result: SensResult) -> str:
    """Format DC sensitivity entries as a stable SPICE-style text table."""
    rows = [
        "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity"
    ]
    for entry in result.entries:
        rows.append(
            "\t".join(
                [
                    result.output_node,
                    _format_table_number(result.nominal_voltage),
                    entry.element_name,
                    entry.parameter,
                    _format_table_number(entry.nominal_value),
                    _format_table_number(entry.sensitivity),
                    _format_table_number(entry.rel_sensitivity),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_corner_sens_table(result: CornerSensResult) -> str:
    """Format multi-corner DC sensitivity entries as a stable text table."""
    rows = [
        "Corner\tOutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity"
    ]
    for corner in result.points:
        for entry in corner.result.entries:
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        result.output_node,
                        _format_table_number(corner.result.nominal_voltage),
                        entry.element_name,
                        entry.parameter,
                        _format_table_number(entry.nominal_value),
                        _format_table_number(entry.sensitivity),
                        _format_table_number(entry.rel_sensitivity),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_s_parameter_table(result: SParameterResult) -> str:
    """Format S-parameters as a stable SPICE-style text table."""
    rows = ["Index\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase"]
    for index, point in enumerate(result.points):
        for parameter, value in [
            ("S11", point.s11),
            ("S21", point.s21),
            ("S12", point.s12),
            ("S22", point.s22),
        ]:
            rows.append(
                "\t".join(
                    [
                        str(index),
                        _format_table_number(point.freq),
                        result.port1_source,
                        result.port2_source,
                        parameter,
                        _format_table_number(value.real),
                        _format_table_number(value.imag),
                        _format_table_number(abs(value)),
                        _format_table_number(math.degrees(cmath.phase(value))),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_corner_s_parameter_table(result: CornerSParameterResult) -> str:
    """Format multi-corner S-parameters as a stable text table."""
    rows = [
        "Corner\tIndex\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase"
    ]
    for corner in result.points:
        for index, point in enumerate(corner.result.points):
            for parameter, value in [
                ("S11", point.s11),
                ("S21", point.s21),
                ("S12", point.s12),
                ("S22", point.s22),
            ]:
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            str(index),
                            _format_table_number(point.freq),
                            result.port1_source,
                            result.port2_source,
                            parameter,
                            _format_table_number(value.real),
                            _format_table_number(value.imag),
                            _format_table_number(abs(value)),
                            _format_table_number(math.degrees(cmath.phase(value))),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def format_noise_table(result: NoiseResult) -> str:
    """Format AC noise entries as a stable SPICE-style text table."""
    rows = [
        "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD"
    ]
    for index, point in enumerate(result.points):
        if not point.entries:
            rows.append(
                "\t".join(
                    [
                        str(index),
                        _format_table_number(point.freq),
                        result.output_node,
                        result.input_source,
                        _format_table_number(point.output_psd),
                        _format_table_number(point.input_referred_psd),
                        "",
                        "",
                        "",
                        "",
                    ]
                )
            )
            continue
        for entry in point.entries:
            rows.append(
                "\t".join(
                    [
                        str(index),
                        _format_table_number(point.freq),
                        result.output_node,
                        result.input_source,
                        _format_table_number(point.output_psd),
                        _format_table_number(point.input_referred_psd),
                        entry.element_name,
                        entry.noise_type,
                        _format_table_number(entry.source_psd),
                        _format_table_number(entry.output_psd),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_corner_noise_table(result: CornerNoiseResult) -> str:
    """Format multi-corner AC noise entries as a stable text table."""
    rows = [
        "Corner\tIndex\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD"
    ]
    for corner in result.points:
        for index, point in enumerate(corner.result.points):
            if not point.entries:
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            str(index),
                            _format_table_number(point.freq),
                            result.output_node,
                            result.input_source,
                            _format_table_number(point.output_psd),
                            _format_table_number(point.input_referred_psd),
                            "",
                            "",
                            "",
                            "",
                        ]
                    )
                )
                continue
            for entry in point.entries:
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            str(index),
                            _format_table_number(point.freq),
                            result.output_node,
                            result.input_source,
                            _format_table_number(point.output_psd),
                            _format_table_number(point.input_referred_psd),
                            entry.element_name,
                            entry.noise_type,
                            _format_table_number(entry.source_psd),
                            _format_table_number(entry.output_psd),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def format_pole_zero_table(result: PoleZeroResult) -> str:
    """Format pole-zero entries as a stable SPICE-style text table."""
    rows = ["Index\tKind\tReal\tImaginary\tFrequency\tDamping"]
    for index, entry in enumerate(result.entries):
        rows.append(
            "\t".join(
                [
                    str(index),
                    entry.kind,
                    _format_table_number(entry.real),
                    _format_table_number(entry.imaginary),
                    _format_table_number(entry.frequency),
                    _format_table_number(entry.damping),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_corner_pole_zero_table(result: CornerPoleZeroResult) -> str:
    """Format named-corner pole-zero entries as a stable text table."""
    rows = ["Corner\tIndex\tKind\tReal\tImaginary\tFrequency\tDamping"]
    for corner in result.points:
        for index, entry in enumerate(corner.result.entries):
            rows.append(
                "\t".join(
                    [
                        corner.corner_name,
                        str(index),
                        entry.kind,
                        _format_table_number(entry.real),
                        _format_table_number(entry.imaginary),
                        _format_table_number(entry.frequency),
                        _format_table_number(entry.damping),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_distortion_table(result: DistortionResult) -> str:
    """Format distortion harmonics as a stable SPICE-style text table."""
    rows = ["Frequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD"]
    for point in result.points:
        for harmonic in point.harmonics:
            rows.append(
                "\t".join(
                    [
                        _format_table_number(point.frequency),
                        result.input_source,
                        result.output_probe,
                        str(harmonic.harmonic),
                        _format_table_number(harmonic.magnitude),
                        _format_table_number(harmonic.phase_degrees),
                        _format_table_number(point.total_harmonic_distortion),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_corner_distortion_table(result: CornerDistortionResult) -> str:
    """Format named-corner distortion harmonics as a stable text table."""
    rows = ["Corner\tFrequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD"]
    for corner in result.points:
        for point in corner.result.points:
            for harmonic in point.harmonics:
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            _format_table_number(point.frequency),
                            result.input_source,
                            result.output_probe,
                            str(harmonic.harmonic),
                            _format_table_number(harmonic.magnitude),
                            _format_table_number(harmonic.phase_degrees),
                            _format_table_number(point.total_harmonic_distortion),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def format_fourier_table(result: FourierResult) -> str:
    """Format Fourier harmonics as a stable SPICE-style text table."""
    rows = ["Probe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD"]
    for probe in result.probes:
        for harmonic in probe.harmonics:
            rows.append(
                "\t".join(
                    [
                        probe.probe,
                        str(harmonic.harmonic),
                        _format_table_number(harmonic.frequency),
                        _format_table_number(harmonic.cosine),
                        _format_table_number(harmonic.sine),
                        _format_table_number(harmonic.magnitude),
                        _format_table_number(harmonic.phase_degrees),
                        _format_table_number(probe.dc),
                        _format_table_number(probe.total_harmonic_distortion),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_deck_fourier_table(results: Iterable[FourierResult]) -> str:
    """Format selected deck ``.four`` artifacts as stable text tables."""

    return "\n".join(format_fourier_table(result) for result in results)


def format_corner_fourier_table(result: CornerFourierResult) -> str:
    """Format named-corner Fourier harmonics as a stable text table."""
    rows = ["Corner\tProbe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD"]
    for corner in result.points:
        for probe in corner.result.probes:
            for harmonic in probe.harmonics:
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            probe.probe,
                            str(harmonic.harmonic),
                            _format_table_number(harmonic.frequency),
                            _format_table_number(harmonic.cosine),
                            _format_table_number(harmonic.sine),
                            _format_table_number(harmonic.magnitude),
                            _format_table_number(harmonic.phase_degrees),
                            _format_table_number(probe.dc),
                            _format_table_number(probe.total_harmonic_distortion),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def _default_output_probes(
    node_voltages: dict[str, float],
    branch_currents: dict[str, float],
) -> list[str]:
    return [
        *(f"V({name})" for name in sorted(node_voltages)),
        *sorted(branch_currents),
    ]


def _default_transient_output_probes(points: list[TransientPoint]) -> list[str]:
    node_names: set[str] = set()
    branch_names: set[str] = set()
    for point in points:
        node_names.update(point.node_voltages)
        branch_names.update(point.branch_currents)
    return [
        *(f"V({name})" for name in sorted(node_names)),
        *sorted(branch_names),
    ]


def _default_ac_output_probes(points: list[AcPoint]) -> list[str]:
    node_names: set[str] = set()
    branch_names: set[str] = set()
    for point in points:
        node_names.update(point.node_voltages)
        branch_names.update(point.branch_currents)
    return [
        *(f"V({name})" for name in sorted(node_names)),
        *sorted(branch_names),
    ]


def _format_table_number(value: float) -> str:
    return f"{value:.6e}"


def measure_transient_probe(
    transient_result: TransientResult | list[TransientPoint],
    name: str,
    probe: str,
    mode: str,
    *,
    from_time: float | None = None,
    to_time: float | None = None,
) -> ProbeMeasurement:
    """Measure one transient probe over an optional time window.

    Supported modes are ``max``, ``min``, ``avg``, ``rms``, ``pp``/``p2p`` /
    ``peak-to-peak``, and ``last``/``final``.
    """

    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    normalized_mode = _normalize_measurement_mode(mode)
    if from_time is not None and not math.isfinite(from_time):
        raise ValueError("measure_transient_probe: from_time must be finite")
    if to_time is not None and not math.isfinite(to_time):
        raise ValueError("measure_transient_probe: to_time must be finite")
    if from_time is not None and to_time is not None and from_time > to_time:
        raise ValueError("measure_transient_probe: from_time must be <= to_time")

    selected = [
        point
        for point in points
        if (from_time is None or point.time >= from_time)
        and (to_time is None or point.time <= to_time)
    ]
    if not selected:
        raise ValueError("measure_transient_probe: no transient samples in window")

    values = [
        _table_probe_value(
            point.node_voltages,
            point.branch_currents,
            probe,
            "measure_transient_probe",
        )
        for point in selected
    ]
    value = _measure_values(values, normalized_mode)
    return ProbeMeasurement(
        name=name,
        analysis="tran",
        probe=probe,
        mode=normalized_mode,
        value=value,
        from_value=from_time,
        to_value=to_time,
    )


def measure_transient_find_at_probe(
    transient_result: TransientResult | list[TransientPoint],
    name: str,
    probe: str,
    at_time: float,
) -> ProbeMeasurement:
    """Measure one transient probe at an exact or interpolated time."""

    if not math.isfinite(at_time):
        raise ValueError("measure_transient_find_at_probe: at_time must be finite")
    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    value = _transient_probe_value_at(
        points,
        probe,
        at_time,
        "measure_transient_find_at_probe",
    )
    return ProbeMeasurement(
        name=name,
        analysis="tran",
        probe=probe,
        mode="find",
        value=value,
        from_value=at_time,
        to_value=at_time,
    )


def measure_transient_when_probe(
    transient_result: TransientResult | list[TransientPoint],
    name: str,
    probe: str,
    target_value: float,
    *,
    from_time: float | None = None,
    to_time: float | None = None,
) -> ProbeMeasurement:
    """Measure the first transient crossing time for ``probe == target_value``."""

    if not math.isfinite(target_value):
        raise ValueError("measure_transient_when_probe: target_value must be finite")
    if from_time is not None and not math.isfinite(from_time):
        raise ValueError("measure_transient_when_probe: from_time must be finite")
    if to_time is not None and not math.isfinite(to_time):
        raise ValueError("measure_transient_when_probe: to_time must be finite")
    if from_time is not None and to_time is not None and from_time > to_time:
        raise ValueError("measure_transient_when_probe: from_time must be <= to_time")
    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    value = _transient_probe_crossing_time(
        points,
        probe,
        target_value,
        "cross",
        1,
        from_time,
        to_time,
        "measure_transient_when_probe",
    )
    return ProbeMeasurement(
        name=name,
        analysis="tran",
        probe=probe,
        mode="when",
        value=value,
        from_value=from_time,
        to_value=to_time,
    )


def measure_transient_when_probe_counted(
    transient_result: TransientResult | list[TransientPoint],
    name: str,
    probe: str,
    target_value: float,
    crossing_kind: str,
    crossing_count: int,
    *,
    from_time: float | None = None,
    to_time: float | None = None,
) -> ProbeMeasurement:
    """Measure a counted transient RISE, FALL, or CROSS threshold occurrence."""

    context = "measure_transient_when_probe_counted"
    if not math.isfinite(target_value):
        raise ValueError(f"{context}: target_value must be finite")
    crossing_kind = _normalize_transient_crossing_kind(crossing_kind, context)
    if not isinstance(crossing_count, int) or crossing_count < 1:
        raise ValueError(f"{context}: crossing_count must be a positive integer")
    if from_time is not None and not math.isfinite(from_time):
        raise ValueError(f"{context}: from_time must be finite")
    if to_time is not None and not math.isfinite(to_time):
        raise ValueError(f"{context}: to_time must be finite")
    if from_time is not None and to_time is not None and from_time > to_time:
        raise ValueError(f"{context}: from_time must be <= to_time")
    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    value = _transient_probe_crossing_time(
        points,
        probe,
        target_value,
        crossing_kind,
        crossing_count,
        from_time,
        to_time,
        context,
    )
    return ProbeMeasurement(
        name=name,
        analysis="tran",
        probe=probe,
        mode="when",
        value=value,
        from_value=from_time,
        to_value=to_time,
    )


def measure_transient_delay_between_probes(
    transient_result: TransientResult | list[TransientPoint],
    name: str,
    trigger_probe: str,
    trigger_value: float,
    trigger_crossing_kind: str,
    trigger_crossing_count: int,
    target_probe: str,
    target_value: float,
    target_crossing_kind: str,
    target_crossing_count: int,
    *,
    from_time: float | None = None,
    to_time: float | None = None,
) -> ProbeMeasurement:
    """Measure target crossing delay relative to a counted trigger crossing."""

    context = "measure_transient_delay_between_probes"
    if not math.isfinite(trigger_value):
        raise ValueError(f"{context}: trigger_value must be finite")
    if not math.isfinite(target_value):
        raise ValueError(f"{context}: target_value must be finite")
    trigger_crossing_kind = _normalize_transient_crossing_kind(
        trigger_crossing_kind,
        context,
    )
    target_crossing_kind = _normalize_transient_crossing_kind(
        target_crossing_kind,
        context,
    )
    if (
        not isinstance(trigger_crossing_count, int)
        or not isinstance(target_crossing_count, int)
        or trigger_crossing_count < 1
        or target_crossing_count < 1
    ):
        raise ValueError(f"{context}: crossing counts must be positive integers")
    if from_time is not None and not math.isfinite(from_time):
        raise ValueError(f"{context}: from_time must be finite")
    if to_time is not None and not math.isfinite(to_time):
        raise ValueError(f"{context}: to_time must be finite")
    if from_time is not None and to_time is not None and from_time > to_time:
        raise ValueError(f"{context}: from_time must be <= to_time")
    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    trigger_time = _transient_probe_crossing_time(
        points,
        trigger_probe,
        trigger_value,
        trigger_crossing_kind,
        trigger_crossing_count,
        from_time,
        to_time,
        context,
    )
    target_from_time = max(from_time, trigger_time) if from_time is not None else trigger_time
    target_time = _transient_probe_crossing_time(
        points,
        target_probe,
        target_value,
        target_crossing_kind,
        target_crossing_count,
        target_from_time,
        to_time,
        context,
    )
    return ProbeMeasurement(
        name=name,
        analysis="tran",
        probe=f"{trigger_probe}->{target_probe}",
        mode="delay",
        value=target_time - trigger_time,
        from_value=from_time,
        to_value=to_time,
    )


def _normalize_transient_crossing_kind(crossing_kind: str, context: str) -> str:
    normalized = crossing_kind.strip().lower()
    if normalized not in {"rise", "fall", "cross"}:
        raise ValueError(f"{context}: crossing_kind must be rise, fall, or cross")
    return normalized


def _transient_probe_value_at(
    points: list[TransientPoint],
    probe: str,
    at_time: float,
    context: str,
) -> float:
    previous: tuple[float, float] | None = None
    for point in points:
        value = _table_probe_value(
            point.node_voltages,
            point.branch_currents,
            probe,
            context,
        )
        if point.time == at_time:
            return value
        if point.time > at_time:
            if previous is None:
                raise ValueError(f"{context}: at_time is outside transient sample range")
            previous_time, previous_value = previous
            if point.time == previous_time:
                raise ValueError(
                    f"{context}: duplicate transient sample times around AT value"
                )
            fraction = (at_time - previous_time) / (point.time - previous_time)
            return previous_value + (value - previous_value) * fraction
        previous = (point.time, value)
    raise ValueError(f"{context}: at_time is outside transient sample range")


def _transient_probe_crossing_time(
    points: list[TransientPoint],
    probe: str,
    target_value: float,
    crossing_kind: str,
    crossing_count: int,
    from_time: float | None,
    to_time: float | None,
    context: str,
) -> float:
    previous: tuple[float, float, float] | None = None
    selected_count = 0
    matched_count = 0
    for point in points:
        if (from_time is not None and point.time < from_time) or (
            to_time is not None and point.time > to_time
        ):
            continue
        selected_count += 1
        value = _table_probe_value(
            point.node_voltages,
            point.branch_currents,
            probe,
            context,
        )
        delta = value - target_value
        crossing_time: float | None = None
        if previous is not None:
            previous_time, previous_value, previous_delta = previous
            if delta == 0.0:
                if (
                    crossing_kind == "cross"
                    or (crossing_kind == "rise" and previous_delta < 0.0)
                    or (crossing_kind == "fall" and previous_delta > 0.0)
                ):
                    crossing_time = point.time
            elif (
                previous_delta < 0.0
                and delta > 0.0
                and crossing_kind != "fall"
            ) or (
                previous_delta > 0.0
                and delta < 0.0
                and crossing_kind != "rise"
            ):
                if point.time == previous_time:
                    raise ValueError(
                        f"{context}: duplicate transient sample times around WHEN crossing"
                    )
                fraction = (target_value - previous_value) / (value - previous_value)
                crossing_time = previous_time + (point.time - previous_time) * fraction
        elif delta == 0.0 and crossing_kind == "cross":
            crossing_time = point.time
        if crossing_time is not None:
            matched_count += 1
            if matched_count == crossing_count:
                return crossing_time
        previous = (point.time, value, delta)
    if selected_count == 0:
        raise ValueError(f"{context}: no transient samples in window")
    raise ValueError(f"{context}: no transient crossing in window")


def measure_transient_cards(
    transient_result: TransientResult | list[TransientPoint],
    measurements: Iterable[DeckMeasurementCard],
) -> list[ProbeMeasurement]:
    """Execute parsed transient ``.measure`` / ``.meas`` cards."""

    results: list[ProbeMeasurement] = []
    for measurement in measurements:
        if measurement.analysis not in {"tran", "transient"}:
            raise ValueError(
                "measure_transient_cards: only transient measurement cards are supported"
            )
        if measurement.mode == "find":
            if measurement.at_value is None:
                raise ValueError(
                    "measure_transient_cards: FIND measurement cards require an AT value"
                )
            results.append(
                measure_transient_find_at_probe(
                    transient_result,
                    measurement.name,
                    measurement.probe,
                    measurement.at_value,
                )
            )
        elif measurement.mode == "when":
            if measurement.target_value is None:
                raise ValueError(
                    "measure_transient_cards: WHEN measurement cards require a target value"
                )
            results.append(
                measure_transient_when_probe_counted(
                    transient_result,
                    measurement.name,
                    measurement.probe,
                    measurement.target_value,
                    measurement.crossing_kind
                    if measurement.crossing_kind is not None
                    else "cross",
                    measurement.crossing_count
                    if measurement.crossing_count is not None
                    else 1,
                    from_time=measurement.from_value,
                    to_time=measurement.to_value,
                )
            )
        elif measurement.mode == "delay":
            if measurement.trigger_probe is None:
                raise ValueError(
                    "measure_transient_cards: delay measurement cards require a trigger probe"
                )
            if measurement.trigger_value is None:
                raise ValueError(
                    "measure_transient_cards: delay measurement cards require a trigger value"
                )
            if measurement.target_value is None:
                raise ValueError(
                    "measure_transient_cards: delay measurement cards require a target value"
                )
            results.append(
                measure_transient_delay_between_probes(
                    transient_result,
                    measurement.name,
                    measurement.trigger_probe,
                    measurement.trigger_value,
                    measurement.trigger_crossing_kind
                    if measurement.trigger_crossing_kind is not None
                    else "cross",
                    measurement.trigger_crossing_count
                    if measurement.trigger_crossing_count is not None
                    else 1,
                    measurement.probe,
                    measurement.target_value,
                    measurement.crossing_kind
                    if measurement.crossing_kind is not None
                    else "cross",
                    measurement.crossing_count
                    if measurement.crossing_count is not None
                    else 1,
                    from_time=measurement.from_value,
                    to_time=measurement.to_value,
                )
            )
        else:
            results.append(
                measure_transient_probe(
                    transient_result,
                    measurement.name,
                    measurement.probe,
                    measurement.mode,
                    from_time=measurement.from_value,
                    to_time=measurement.to_value,
                )
            )
    return results


def measure_transient_deck(
    transient_result: TransientResult | list[TransientPoint],
    netlist: str,
) -> list[ProbeMeasurement]:
    """Parse and execute supported transient measurements from a SPICE deck."""

    summary = resolve_deck_measurements(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"measure_transient_deck: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return measure_transient_cards(transient_result, summary.measurements)


def measure_dc_sweep_probe(
    dc_sweep_result: DcSweepResult | list[DcSweepPoint],
    name: str,
    probe: str,
    mode: str,
    *,
    from_value: float | None = None,
    to_value: float | None = None,
) -> ProbeMeasurement:
    """Measure one DC sweep probe over an optional source-value window."""

    points = (
        dc_sweep_result.points
        if isinstance(dc_sweep_result, DcSweepResult)
        else dc_sweep_result
    )
    normalized_mode = _normalize_measurement_mode(
        mode,
        context="measure_dc_sweep_probe",
    )
    if from_value is not None and not math.isfinite(from_value):
        raise ValueError("measure_dc_sweep_probe: from_value must be finite")
    if to_value is not None and not math.isfinite(to_value):
        raise ValueError("measure_dc_sweep_probe: to_value must be finite")
    if from_value is not None and to_value is not None and from_value > to_value:
        raise ValueError("measure_dc_sweep_probe: from_value must be <= to_value")

    selected = [
        point
        for point in points
        if (from_value is None or point.source_value >= from_value)
        and (to_value is None or point.source_value <= to_value)
    ]
    if not selected:
        raise ValueError("measure_dc_sweep_probe: no dc sweep samples in window")

    values = [
        _table_probe_value(
            point.node_voltages,
            point.branch_currents,
            probe,
            "measure_dc_sweep_probe",
        )
        for point in selected
    ]
    value = _measure_values(
        values,
        normalized_mode,
        context="measure_dc_sweep_probe",
    )
    return ProbeMeasurement(
        name=name,
        analysis="dc",
        probe=probe,
        mode=normalized_mode,
        value=value,
        from_value=from_value,
        to_value=to_value,
    )


def measure_dc_sweep_cards(
    dc_sweep_result: DcSweepResult | list[DcSweepPoint],
    measurements: Iterable[DeckMeasurementCard],
) -> list[ProbeMeasurement]:
    """Execute parsed DC sweep ``.measure`` / ``.meas`` cards."""

    results: list[ProbeMeasurement] = []
    for measurement in measurements:
        if measurement.analysis != "dc":
            raise ValueError(
                "measure_dc_sweep_cards: only dc measurement cards are supported"
            )
        results.append(
            measure_dc_sweep_probe(
                dc_sweep_result,
                measurement.name,
                measurement.probe,
                measurement.mode,
                from_value=measurement.from_value,
                to_value=measurement.to_value,
            )
        )
    return results


def measure_dc_sweep_deck(
    dc_sweep_result: DcSweepResult | list[DcSweepPoint],
    netlist: str,
) -> list[ProbeMeasurement]:
    """Parse and execute supported DC sweep measurements from a SPICE deck."""

    summary = resolve_deck_measurements(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"measure_dc_sweep_deck: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return measure_dc_sweep_cards(dc_sweep_result, summary.measurements)


def measure_ac_sweep_probe(
    ac_result: AcResult | list[AcPoint],
    name: str,
    probe: str,
    mode: str,
    *,
    from_frequency: float | None = None,
    to_frequency: float | None = None,
) -> ProbeMeasurement:
    """Measure one AC probe magnitude over an optional frequency window."""

    points = ac_result.points if isinstance(ac_result, AcResult) else ac_result
    normalized_mode = _normalize_measurement_mode(
        mode,
        context="measure_ac_sweep_probe",
    )
    if from_frequency is not None and not math.isfinite(from_frequency):
        raise ValueError("measure_ac_sweep_probe: from_frequency must be finite")
    if to_frequency is not None and not math.isfinite(to_frequency):
        raise ValueError("measure_ac_sweep_probe: to_frequency must be finite")
    if (
        from_frequency is not None
        and to_frequency is not None
        and from_frequency > to_frequency
    ):
        raise ValueError(
            "measure_ac_sweep_probe: from_frequency must be <= to_frequency"
        )

    selected = [
        point
        for point in points
        if (from_frequency is None or point.freq >= from_frequency)
        and (to_frequency is None or point.freq <= to_frequency)
    ]
    if not selected:
        raise ValueError("measure_ac_sweep_probe: no ac sweep samples in window")

    values = [
        abs(
            _table_complex_probe_value(
                point.node_voltages,
                point.branch_currents,
                probe,
                "measure_ac_sweep_probe",
            )
        )
        for point in selected
    ]
    value = _measure_values(
        values,
        normalized_mode,
        context="measure_ac_sweep_probe",
    )
    return ProbeMeasurement(
        name=name,
        analysis="ac",
        probe=probe,
        mode=normalized_mode,
        value=value,
        from_value=from_frequency,
        to_value=to_frequency,
    )


def measure_ac_sweep_cards(
    ac_result: AcResult | list[AcPoint],
    measurements: Iterable[DeckMeasurementCard],
) -> list[ProbeMeasurement]:
    """Execute parsed AC sweep ``.measure`` / ``.meas`` cards."""

    results: list[ProbeMeasurement] = []
    for measurement in measurements:
        if measurement.analysis != "ac":
            raise ValueError(
                "measure_ac_sweep_cards: only ac measurement cards are supported"
            )
        results.append(
            measure_ac_sweep_probe(
                ac_result,
                measurement.name,
                measurement.probe,
                measurement.mode,
                from_frequency=measurement.from_value,
                to_frequency=measurement.to_value,
            )
        )
    return results


def measure_ac_sweep_deck(
    ac_result: AcResult | list[AcPoint],
    netlist: str,
) -> list[ProbeMeasurement]:
    """Parse and execute supported AC sweep measurements from a SPICE deck."""

    summary = resolve_deck_measurements(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"measure_ac_sweep_deck: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return measure_ac_sweep_cards(ac_result, summary.measurements)


def format_measurement_table(measurements: Iterable[ProbeMeasurement]) -> str:
    """Format scalar probe measurements as a stable tab-separated table."""

    rows = ["Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue"]
    for measurement in measurements:
        rows.append(
            "\t".join(
                [
                    measurement.name,
                    measurement.analysis,
                    measurement.probe,
                    measurement.mode,
                    _format_optional_table_number(measurement.from_value),
                    _format_optional_table_number(measurement.to_value),
                    _format_table_number(measurement.value),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def _normalize_measurement_mode(
    mode: str,
    *,
    context: str = "measure_transient_probe",
) -> str:
    normalized = mode.strip().lower().replace("_", "-")
    aliases = {
        "average": "avg",
        "mean": "avg",
        "root-mean-square": "rms",
        "p-p": "pp",
        "p2p": "pp",
        "peak-to-peak": "pp",
        "peak2peak": "pp",
        "final": "last",
    }
    normalized = aliases.get(normalized, normalized)
    if normalized not in {"max", "min", "avg", "rms", "pp", "last"}:
        raise ValueError(f"{context}: unsupported mode {mode!r}")
    return normalized


def _measure_values(
    values: list[float],
    mode: str,
    *,
    context: str = "measure_transient_probe",
) -> float:
    if mode == "max":
        return max(values)
    if mode == "min":
        return min(values)
    if mode == "avg":
        return sum(values) / len(values)
    if mode == "rms":
        return math.sqrt(sum(value * value for value in values) / len(values))
    if mode == "pp":
        return max(values) - min(values)
    if mode == "last":
        return values[-1]
    raise ValueError(f"{context}: unsupported mode {mode!r}")


def _format_optional_table_number(value: float | None) -> str:
    return "" if value is None else _format_table_number(value)


def _table_probe_value(
    node_voltages: dict[str, float],
    branch_currents: dict[str, float],
    probe: str,
    context: str,
) -> float:
    text = probe.strip()
    lower = text.lower()
    if lower.startswith("v(") and text.endswith(")"):
        args = [arg.strip() for arg in text[2:-1].split(",")]
        if len(args) == 1:
            return _table_voltage(node_voltages, args[0], context)
        if len(args) == 2:
            return _table_voltage(node_voltages, args[0], context) - _table_voltage(
                node_voltages,
                args[1],
                context,
            )
    if lower.startswith("i(") and text.endswith(")"):
        key = f"I({text[2:-1].strip()})"
        if key in branch_currents:
            return branch_currents[key]
        raise ValueError(f"{context}: missing branch current probe {probe!r}")
    if text:
        return _table_voltage(node_voltages, text, context)
    raise ValueError(f"{context}: empty probe")


def _table_complex_probe_value(
    node_voltages: dict[str, complex],
    branch_currents: dict[str, complex],
    probe: str,
    context: str,
) -> complex:
    text = probe.strip()
    lower = text.lower()
    if lower.startswith("v(") and text.endswith(")"):
        args = [arg.strip() for arg in text[2:-1].split(",")]
        if len(args) == 1:
            return _table_complex_voltage(node_voltages, args[0], context)
        if len(args) == 2:
            return _table_complex_voltage(
                node_voltages,
                args[0],
                context,
            ) - _table_complex_voltage(node_voltages, args[1], context)
    if lower.startswith("i(") and text.endswith(")"):
        key = f"I({text[2:-1].strip()})"
        if key not in branch_currents:
            raise ValueError(f"{context}: missing branch current probe {probe}")
        return branch_currents[key]
    if text:
        return _table_complex_voltage(node_voltages, text, context)
    raise ValueError(f"{context}: empty probe")


def _table_complex_voltage(
    node_voltages: dict[str, complex],
    node: str,
    context: str,
) -> complex:
    if _is_ground(node):
        return 0.0 + 0.0j
    if node not in node_voltages:
        raise ValueError(f"{context}: missing node voltage {node}")
    return node_voltages[node]


def _table_voltage(
    node_voltages: dict[str, float],
    node: str,
    context: str,
) -> float:
    if node.lower() in {"0", "gnd"}:
        return 0.0
    if node in node_voltages:
        return node_voltages[node]
    raise ValueError(f"{context}: missing node voltage {node!r}")


def fourier(
    transient_result: TransientResult | list[TransientPoint],
    fundamental_frequency: float,
    probes: list[str],
    *,
    harmonics: int = 9,
    start_time: float | None = None,
) -> FourierResult:
    """Compute SPICE-style Fourier components from transient samples.

    By default the latest full fundamental period in the transient output is
    used, matching the common ``.four`` workflow of ignoring startup transients.
    Probe strings accept ``V(node)``, ``V(node,ref)``, ``I(source)``, or a bare
    node name.
    """
    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    if not math.isfinite(fundamental_frequency) or fundamental_frequency <= 0.0:
        raise ValueError("fourier: fundamental_frequency must be finite and positive")
    if harmonics < 1:
        raise ValueError("fourier: harmonics must be positive")
    if not probes:
        raise ValueError("fourier: at least one probe is required")
    if len(points) < 2:
        raise ValueError("fourier: at least two transient points are required")

    sorted_points = sorted(points, key=lambda point: point.time)
    period = 1.0 / fundamental_frequency
    end_time = sorted_points[-1].time
    window_start = end_time - period if start_time is None else start_time
    if not math.isfinite(window_start) or window_start < sorted_points[0].time:
        raise ValueError("fourier: transient output does not contain a full analysis window")
    if window_start >= end_time:
        raise ValueError("fourier: analysis window must have positive duration")

    results = [
        _fourier_probe(
            sorted_points,
            probe,
            fundamental_frequency,
            harmonics,
            window_start,
            end_time,
        )
        for probe in probes
    ]
    return FourierResult(
        fundamental_frequency=fundamental_frequency,
        start_time=window_start,
        end_time=end_time,
        probes=results,
    )


def fourier_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    t_stop: float,
    t_step: float,
    fundamental_frequency: float,
    probes: list[str],
    harmonics: int = 9,
    start_time: float | None = None,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerFourierResult:
    """Run transient Fourier analysis at each named corner."""
    points: list[CornerFourierPoint] = []
    for corner in corners:
        transient_result = transient(
            _circuit_with_corner(circuit, corner),
            t_stop=t_stop,
            t_step=t_step,
            method=method,
            max_iterations=max_iterations,
            tol=tol,
        )
        points.append(
            CornerFourierPoint(
                corner_name=corner.name,
                result=fourier(
                    transient_result,
                    fundamental_frequency,
                    probes,
                    harmonics=harmonics,
                    start_time=start_time,
                ),
            )
        )
    return CornerFourierResult(
        fundamental_frequency=fundamental_frequency,
        points=points,
    )


def fourier_transient_cards(
    transient_result: TransientResult | list[TransientPoint],
    fourier_cards: Iterable[DeckFourierCard],
) -> list[FourierResult]:
    """Route parsed ``.four`` cards into Fourier transient analysis."""

    points = (
        transient_result.points
        if isinstance(transient_result, TransientResult)
        else transient_result
    )
    return [
        fourier(
            points,
            card.fundamental_frequency,
            list(card.probes),
            harmonics=card.harmonics or 9,
            start_time=card.from_value,
        )
        for card in fourier_cards
    ]


def fourier_transient_deck(
    transient_result: TransientResult | list[TransientPoint],
    netlist: str,
) -> list[FourierResult]:
    """Resolve transient ``.four`` cards from a deck and run Fourier analyses."""

    summary = resolve_deck_fourier(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"fourier_transient_deck: line {diagnostic.line_number}: {diagnostic.message}"
        )
    return fourier_transient_cards(transient_result, summary.fourier)


def _fourier_probe(
    points: list[TransientPoint],
    probe: str,
    fundamental_frequency: float,
    harmonics: int,
    start_time: float,
    end_time: float,
) -> FourierProbeResult:
    samples: list[tuple[float, float]] = [
        (start_time, _interpolate_probe(points, probe, start_time))
    ]
    for point in points:
        if start_time < point.time < end_time:
            samples.append((point.time, _probe_value(point, probe)))
    samples.append((end_time, _interpolate_probe(points, probe, end_time)))
    samples.sort(key=lambda sample: sample[0])

    duration = end_time - start_time
    dc = _integrate(samples, lambda _time: 1.0) / duration
    components: list[FourierHarmonic] = []
    omega = 2.0 * math.pi * fundamental_frequency
    for harmonic in range(1, harmonics + 1):
        frequency = harmonic * fundamental_frequency
        cosine = 2.0 / duration * _integrate(
            samples, lambda time, n=harmonic: math.cos(n * omega * time)
        )
        sine = 2.0 / duration * _integrate(
            samples, lambda time, n=harmonic: math.sin(n * omega * time)
        )
        magnitude = math.hypot(cosine, sine)
        components.append(
            FourierHarmonic(
                harmonic=harmonic,
                frequency=frequency,
                cosine=cosine,
                sine=sine,
                magnitude=magnitude,
                phase_degrees=math.degrees(math.atan2(cosine, sine)),
            )
        )
    fundamental = components[0].magnitude
    distortion = math.sqrt(
        sum(component.magnitude ** 2 for component in components[1:])
    )
    thd = (
        math.inf
        if fundamental == 0.0 and distortion > 0.0
        else (0.0 if fundamental == 0.0 else distortion / fundamental)
    )
    return FourierProbeResult(
        probe=probe,
        dc=dc,
        harmonics=components,
        total_harmonic_distortion=thd,
    )


def _integrate(
    samples: list[tuple[float, float]], weight: Callable[[float], float]
) -> float:
    total = 0.0
    for (left_time, left_value), (right_time, right_value) in zip(
        samples, samples[1:], strict=False
    ):
        left_weight = weight(left_time)
        right_weight = weight(right_time)
        total += 0.5 * (right_time - left_time) * (
            left_value * left_weight + right_value * right_weight
        )
    return total


def _interpolate_probe(points: list[TransientPoint], probe: str, time: float) -> float:
    for point in points:
        if math.isclose(point.time, time, rel_tol=0.0, abs_tol=1.0e-15):
            return _probe_value(point, probe)
    for left, right in zip(points, points[1:], strict=False):
        if left.time <= time <= right.time:
            span = right.time - left.time
            if span <= 0.0:
                return _probe_value(left, probe)
            alpha = (time - left.time) / span
            return (1.0 - alpha) * _probe_value(
                left, probe
            ) + alpha * _probe_value(right, probe)
    raise ValueError("fourier: analysis window is outside transient output")


def _probe_value(point: TransientPoint, probe: str) -> float:
    text = probe.strip()
    lower = text.lower()
    if lower.startswith("v(") and text.endswith(")"):
        args = [arg.strip() for arg in text[2:-1].split(",")]
        if len(args) == 1:
            return _point_voltage(point, args[0])
        if len(args) == 2:
            return _point_voltage(point, args[0]) - _point_voltage(point, args[1])
    if lower.startswith("i(") and text.endswith(")"):
        key = f"I({text[2:-1].strip()})"
        if key in point.branch_currents:
            return point.branch_currents[key]
        raise ValueError(f"fourier: missing branch current probe {probe!r}")
    if text:
        return _point_voltage(point, text)
    raise ValueError("fourier: empty probe")


def _point_voltage(point: TransientPoint, node: str) -> float:
    if node.lower() in {"0", "gnd"}:
        return 0.0
    if node in point.node_voltages:
        return point.node_voltages[node]
    raise ValueError(f"fourier: missing node voltage {node!r}")


@dataclass
class PssResidualEntry:
    """One ordered entry in the PSS state-closure residual vector."""

    kind: str
    name: str
    value: float


@dataclass
class PssResidualResult:
    """One-period state-closure residual for a periodic source period."""

    period: float
    time_step: float
    node_residuals: dict[str, float]
    branch_residuals: dict[str, float]
    residual_vector: list[PssResidualEntry]
    max_abs_branch_residual: float
    max_abs_residual: float
    residual_l2_norm: float
    residual_rms_norm: float
    residual_tol: float
    within_tolerance: bool
    converged: bool


@dataclass
class PssStateEntry:
    """One ordered PSS shooting-state entry."""

    kind: str
    name: str
    value: float


@dataclass
class PssResidualJacobianColumn:
    """Finite-difference derivatives for one PSS shooting-state entry."""

    state: PssStateEntry
    residual_derivatives: list[PssResidualEntry]


@dataclass
class PssResidualJacobianResult:
    """Forward finite-difference Jacobian for the ordered PSS residual vector."""

    residual: PssResidualResult
    state_vector: list[PssStateEntry]
    perturbation: float
    columns: list[PssResidualJacobianColumn]
    jacobian: list[list[float]]


@dataclass
class PssNewtonUpdateResult:
    """Least-squares Newton correction for the PSS reactive state vector."""

    jacobian: PssResidualJacobianResult
    state_updates: list[PssStateEntry]
    next_state_vector: list[PssStateEntry]
    update_l2_norm: float


@dataclass
class PssNewtonCandidateResult:
    """Candidate PSS circuit after applying one Newton reactive-state update."""

    update: PssNewtonUpdateResult
    candidate_circuit: Circuit
    candidate_state_vector: list[PssStateEntry]
    candidate_residual: PssResidualResult


@dataclass
class PssNewtonIterationResult:
    """Accepted or rejected single PSS Newton shooting iteration."""

    candidate: PssNewtonCandidateResult
    accepted: bool
    residual_l2_reduction: float
    residual_l2_ratio: float
    next_circuit: Circuit
    next_state_vector: list[PssStateEntry]
    next_residual: PssResidualResult
    converged: bool


@dataclass
class PssNewtonSolveResult:
    """Bounded PSS shooting-Newton solve over accepted iterations."""

    iterations: list[PssNewtonIterationResult]
    final_circuit: Circuit
    final_state_vector: list[PssStateEntry]
    final_residual: PssResidualResult
    converged: bool
    iteration_count: int


@dataclass
class PssResult:
    """Periodic steady-state analysis result over one solved source period."""

    solve: PssNewtonSolveResult
    steady_state: TransientResult
    period: float
    time_step: float
    converged: bool


@dataclass(frozen=True)
class CornerPssPoint:
    """PSS result for one named analysis corner."""

    corner_name: str
    result: PssResult


@dataclass(frozen=True)
class CornerPssResult:
    """Multi-corner periodic steady-state analysis result."""

    points: list[CornerPssPoint]


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
    branch_currents: dict[str, complex] = field(default_factory=dict)


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
class SParameterPoint:
    """Two-port scattering parameters at one frequency point."""

    freq: float
    s11: complex
    s21: complex
    s12: complex
    s22: complex


@dataclass(frozen=True)
class SParameterResult:
    """Two-port S-parameter sweep result."""

    port1_source: str
    port2_source: str
    reference_impedance: float
    points: list[SParameterPoint]


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


@dataclass(frozen=True)
class CornerDcSweepPoint:
    """DC source sweep result for one named analysis corner."""

    corner_name: str
    result: DcSweepResult


@dataclass(frozen=True)
class CornerDcSweepResult:
    """Multi-corner DC source sweep result."""

    points: list[CornerDcSweepPoint]
    source_name: str


@dataclass(frozen=True)
class CornerAcSweepPoint:
    """AC frequency sweep result for one named analysis corner."""

    corner_name: str
    result: AcResult


@dataclass(frozen=True)
class CornerAcSweepResult:
    """Multi-corner AC frequency sweep result."""

    points: list[CornerAcSweepPoint]


@dataclass(frozen=True)
class CornerTfPoint:
    """Transfer-function result for one named analysis corner."""

    corner_name: str
    result: TfResult


@dataclass(frozen=True)
class CornerTfResult:
    """Multi-corner transfer-function analysis result."""

    points: list[CornerTfPoint]
    input_source: str
    output_node: str


@dataclass(frozen=True)
class CornerMcPoint:
    """Monte Carlo DC result for one named analysis corner."""

    corner_name: str
    result: McResult


@dataclass(frozen=True)
class CornerMcResult:
    """Multi-corner Monte Carlo DC analysis result."""

    points: list[CornerMcPoint]
    output_node: str


@dataclass(frozen=True)
class CornerSensPoint:
    """DC sensitivity result for one named analysis corner."""

    corner_name: str
    result: SensResult


@dataclass(frozen=True)
class CornerSensResult:
    """Multi-corner DC sensitivity analysis result."""

    points: list[CornerSensPoint]
    output_node: str


@dataclass(frozen=True)
class CornerNoisePoint:
    """AC noise result for one named analysis corner."""

    corner_name: str
    result: NoiseResult


@dataclass(frozen=True)
class CornerNoiseResult:
    """Multi-corner AC noise analysis result."""

    points: list[CornerNoisePoint]
    output_node: str
    input_source: str


@dataclass(frozen=True)
class CornerSParameterPoint:
    """S-parameter result for one named analysis corner."""

    corner_name: str
    result: SParameterResult


@dataclass(frozen=True)
class CornerSParameterResult:
    """Multi-corner S-parameter extraction result."""

    points: list[CornerSParameterPoint]
    port1_source: str
    port2_source: str
    reference_impedance: float


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
    if isinstance(el, BSource):
        nodes = [el.n_plus, el.n_minus]
        expr = el.voltage_expr if el.voltage_expr is not None else el.current_expr
        if expr is not None:
            nodes.extend(_bsource_expr_nodes(expr))
        return nodes
    if isinstance(el, CustomModel):
        return [el.n_plus, el.n_minus]
    if isinstance(el, Diode):
        nodes = [el.anode, el.cathode]
        if el.Rs > 0.0:
            nodes.append(_diode_intrinsic_anode_node(el))
        return nodes
    if isinstance(el, JFET):
        return [el.drain, el.gate, el.source]
    if isinstance(el, Mosfet):
        return [el.drain, el.gate, el.source, el.body]
    if isinstance(el, BJT):
        nodes = [el.collector, el.base, el.emitter]
        if el.Re > 0.0:
            nodes.append(_bjt_intrinsic_emitter_node(el))
        if el.Rc > 0.0:
            nodes.append(_bjt_intrinsic_collector_node(el))
        if el.Rb > 0.0:
            nodes.append(_bjt_intrinsic_base_node(el))
        return nodes
    if isinstance(el, TransmissionLine):
        return [el.n1, el.n2, el.n3, el.n4]
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
) -> list[VoltageSource | VCVS | CCVS | BSource]:
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
    ccvs_list: list[VoltageSource | VCVS | CCVS | BSource] = [
        el for el in circuit.elements if isinstance(el, CCVS)
    ]
    bsources: list[VoltageSource | VCVS | CCVS | BSource] = [
        el for el in circuit.elements if isinstance(el, BSource) and el.voltage_expr is not None
    ]
    return vsrcs + vcvs_list + ccvs_list + bsources


def _is_ground(name: str) -> bool:
    return name in ("0", "gnd", "GND")


def _real_solver_kind(matrix_size: int) -> str:
    if matrix_size == 0:
        return "none"
    if matrix_size >= _SPARSE_SOLVER_THRESHOLD:
        return "sparse_real"
    return "dense_real"


def _complex_solver_kind(matrix_size: int) -> str:
    if matrix_size == 0:
        return "none"
    if matrix_size >= _SPARSE_SOLVER_THRESHOLD:
        return "sparse_complex"
    return "dense_complex"


def _dc_diagnostics(
    matrix_size: int,
    *,
    tol: float,
    max_delta: float,
    convergence_aid: str,
    solver_profile: LinearSolverProfile | None = None,
    newton_step_limit: float | None = None,
    limited_newton_steps: int = 0,
    minimum_damping_factor: float = 1.0,
) -> DcSolverDiagnostics:
    return DcSolverDiagnostics(
        matrix_size=matrix_size,
        solver=_real_solver_kind(matrix_size),
        tolerance=tol,
        max_delta=max_delta,
        convergence_aid=convergence_aid,
        newton_step_limit=newton_step_limit,
        limited_newton_steps=limited_newton_steps,
        minimum_damping_factor=minimum_damping_factor,
        solver_profile=solver_profile
        if solver_profile is not None
        else _empty_solver_profile(matrix_size),
    )


def _has_nonlinear_element(circuit: Circuit) -> bool:
    return any(
        isinstance(el, (BSource, CustomModel, Diode, JFET, Mosfet, BJT))
        for el in circuit.elements
    )


def _validate_newton_step_limit(value: float | None) -> float | None:
    if value is None:
        return None
    if not math.isfinite(value) or value <= 0.0:
        raise ValueError("newton_step_limit must be finite and positive")
    return value


def _limit_newton_step(
    previous: list[float],
    candidate: list[float],
    step_limit: float | None,
) -> tuple[list[float], float, float, bool]:
    if step_limit is None or not previous:
        max_delta = (
            max(abs(a - bv) for a, bv in zip(previous, candidate, strict=False))
            if previous
            else 0.0
        )
        return candidate, max_delta, 1.0, False

    raw_delta = max(abs(a - bv) for a, bv in zip(previous, candidate, strict=False))
    if raw_delta <= step_limit:
        return candidate, raw_delta, 1.0, False

    if not math.isfinite(raw_delta):
        limited = []
        for old, new in zip(previous, candidate, strict=False):
            delta = new - old
            if math.isfinite(delta):
                limited.append(old + math.copysign(step_limit, delta))
            else:
                limited.append(old)
        return limited, step_limit, 0.0, True

    damping_factor = step_limit / raw_delta
    limited = [
        old + (new - old) * damping_factor
        for old, new in zip(previous, candidate, strict=False)
    ]
    return limited, step_limit, damping_factor, True


# ---------------------------------------------------------------------------
# DC analysis
# ---------------------------------------------------------------------------


def _dc_newton(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    x_init: list[float] | None = None,
    newton_step_limit: float | None = _DEFAULT_NEWTON_STEP_LIMIT,
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
    active_step_limit = (
        _validate_newton_step_limit(newton_step_limit)
        if _has_nonlinear_element(circuit)
        else None
    )
    limited_newton_steps = 0
    minimum_damping_factor = 1.0

    max_delta = float("inf")
    for it in range(max_iterations):
        G = [[0.0] * size for _ in range(size)]
        b = [0.0] * size
        for el in circuit.elements:
            _stamp_dc(el, G, b, x, node_to_idx, branch_srcs)
        solver_profile = _real_solver_profile(G, backend="pending")
        try:
            x_new, solver_profile = _solve_with_profile(G, b)
        except ZeroDivisionError as exc:
            if isinstance(exc, _LinearSolveFailure):
                solver_profile = exc.solver_profile
            node_v = {nd: x[i] for nd, i in node_to_idx.items()}
            return DcResult(
                node_v,
                {},
                iterations=it,
                converged=False,
                diagnostics=_dc_diagnostics(
                    size,
                    tol=tol,
                    max_delta=float("inf"),
                    convergence_aid="newton",
                    solver_profile=solver_profile,
                    newton_step_limit=active_step_limit,
                    limited_newton_steps=limited_newton_steps,
                    minimum_damping_factor=minimum_damping_factor,
                ),
            )

        x_new, max_delta, damping_factor, was_limited = _limit_newton_step(
            x,
            x_new,
            active_step_limit,
        )
        if was_limited:
            limited_newton_steps += 1
            minimum_damping_factor = min(minimum_damping_factor, damping_factor)
        x = x_new
        if max_delta < tol:
            break

    node_v = {nd: x[i] for nd, i in node_to_idx.items()}
    branch_i = {f"I({el.name})": x[n + i] for i, el in enumerate(branch_srcs)}
    return DcResult(
        node_v,
        branch_i,
        iterations=it + 1,
        converged=max_delta < tol,
        diagnostics=_dc_diagnostics(
            size,
            tol=tol,
            max_delta=max_delta,
            convergence_aid="newton",
            solver_profile=solver_profile,
            newton_step_limit=active_step_limit,
            limited_newton_steps=limited_newton_steps,
            minimum_damping_factor=minimum_damping_factor,
        ),
    )


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
    newton_step_limit: float | None = _DEFAULT_NEWTON_STEP_LIMIT,
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

        result = _dc_newton(
            aug,
            max_iterations=max_iterations,
            tol=tol,
            x_init=x_init,
            newton_step_limit=newton_step_limit,
        )
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
    newton_step_limit: float | None = _DEFAULT_NEWTON_STEP_LIMIT,
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
            scaled_circuit,
            max_iterations=max_iterations,
            tol=tol,
            x_init=x_init,
            newton_step_limit=newton_step_limit,
        )
        if not result.converged:
            return None  # This step diverged — source stepping has failed.

        # Reconstruct x_init for the next step.  Source scaling does not
        # change circuit topology (same nodes, same branch sources), so the
        # x-vector ordering is unchanged.
        x_init = _x_from_result(result, nodes, orig_branch_srcs)

    return result


def _dc_pseudo_transient(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    n_steps: int = 20,
    shunt_conductance: float = 1e-3,
    newton_step_limit: float | None = _DEFAULT_NEWTON_STEP_LIMIT,
) -> DcResult | None:
    """DC continuation through artificial backward-Euler node capacitors."""
    if n_steps <= 0 or not math.isfinite(shunt_conductance) or shunt_conductance <= 0.0:
        return None

    _, nodes = _node_index(circuit)
    if not nodes:
        return None

    branch_srcs = _branch_sources(circuit)
    previous_node_voltages = {node: 0.0 for node in nodes}
    x_init: list[float] | None = None
    last_result: DcResult | None = None

    for step in range(n_steps):
        pseudo_elements = list(circuit.elements)
        for node in nodes:
            pseudo_elements.append(Resistor(
                f"_ptran_g_{step}_{node}",
                node,
                "0",
                1.0 / shunt_conductance,
            ))
            history_current = shunt_conductance * previous_node_voltages[node]
            if history_current != 0.0:
                pseudo_elements.append(CurrentSource(
                    f"_ptran_i_{step}_{node}",
                    "0",
                    node,
                    history_current,
                ))

        result = _dc_newton(
            Circuit(elements=pseudo_elements),
            max_iterations=max_iterations,
            tol=tol,
            x_init=x_init,
            newton_step_limit=newton_step_limit,
        )
        if not result.converged:
            return None

        delta = max(
            abs(result.node_voltages.get(node, 0.0) - previous_node_voltages[node])
            for node in nodes
        )
        previous_node_voltages = {
            node: result.node_voltages.get(node, 0.0)
            for node in nodes
        }
        x_init = _x_from_result(result, nodes, branch_srcs)
        last_result = result
        if delta < tol:
            break

    if last_result is None:
        return None

    final = _dc_newton(
        circuit,
        max_iterations=max_iterations,
        tol=tol,
        x_init=x_init,
    )
    return final if final.converged else None


def dc_initial_vector_from_conditions(
    circuit: Circuit,
    initial_conditions: Iterable[DeckNodeCondition],
    nodesets: Iterable[DeckNodeCondition] = (),
) -> list[float]:
    """Build a DC Newton warm-start vector from parsed ``.ic``/``.nodeset`` hints.

    The vector follows the engine's internal MNA ordering: non-ground node
    voltages first, then branch currents initialised to zero.  ``.nodeset``
    values are applied first, and ``.ic`` values override them when both
    mention the same node.
    """

    node_to_idx, nodes = _node_index(circuit)
    branch_srcs = _branch_sources(circuit)
    vector = [0.0] * (len(nodes) + len(branch_srcs))

    def apply(condition: DeckNodeCondition) -> None:
        if not math.isfinite(condition.value):
            msg = f"{condition.directive} V({condition.node}) must be finite"
            raise ValueError(msg)
        if _is_ground(condition.node):
            if condition.value != 0.0:
                msg = f"{condition.directive} V({condition.node}) conflicts with ground"
                raise ValueError(msg)
            return
        index = node_to_idx.get(condition.node)
        if index is None:
            msg = f"{condition.directive} references unknown node {condition.node!r}"
            raise ValueError(msg)
        vector[index] = condition.value

    for condition in nodesets:
        apply(condition)
    for condition in initial_conditions:
        apply(condition)
    return vector


def _validate_dc_initial_vector(circuit: Circuit, initial_vector: list[float]) -> None:
    _, nodes = _node_index(circuit)
    branch_srcs = _branch_sources(circuit)
    expected_len = len(nodes) + len(branch_srcs)
    if len(initial_vector) != expected_len:
        msg = (
            "dc_initial_vector: expected "
            f"{expected_len} entries for circuit MNA ordering, got {len(initial_vector)}"
        )
        raise ValueError(msg)
    if any(not math.isfinite(value) for value in initial_vector):
        msg = "dc_initial_vector: all entries must be finite"
        raise ValueError(msg)


def dc_op(
    circuit: Circuit,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    convergence_aids: bool = True,
    pseudo_transient_steps: int = 20,
    pseudo_transient_shunt_conductance: float = 1e-3,
    pseudo_transient_max_iterations: int | None = None,
    initial_vector: list[float] | None = None,
    newton_step_limit: float | None = _DEFAULT_NEWTON_STEP_LIMIT,
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
    pseudo_transient_steps:
        Maximum artificial backward-Euler continuation steps after Newton,
        Gmin stepping, and source stepping fail.  Set to 0 to disable this
        final convergence aid.
    pseudo_transient_shunt_conductance:
        Artificial node-to-ground conductance (S) used by the continuation
        companion.  Larger values damp Newton more aggressively.
    pseudo_transient_max_iterations:
        Optional Newton iteration cap for each pseudo-transient step and final
        polish solve.  Defaults to ``max_iterations``.
    initial_vector:
        Optional MNA warm-start vector.  Prefer
        :func:`dc_op_with_initial_conditions` for parsed deck hints.
    newton_step_limit:
        Maximum absolute Newton update per unknown for nonlinear circuits.
        Set to ``None`` to disable damping.
    """
    if initial_vector is not None:
        _validate_dc_initial_vector(circuit, initial_vector)
    newton_step_limit = _validate_newton_step_limit(newton_step_limit)

    # Attempt 1: plain Newton-Raphson.
    result = _dc_newton(
        circuit,
        max_iterations=max_iterations,
        tol=tol,
        x_init=initial_vector,
        newton_step_limit=newton_step_limit,
    )
    if result.converged:
        return replace(
            result,
            convergence_aid="newton",
            diagnostics=replace(result.diagnostics, convergence_aid="newton"),
        )
    if not convergence_aids:
        return replace(
            result,
            convergence_aid="none",
            diagnostics=replace(result.diagnostics, convergence_aid="none"),
        )

    # Attempt 2: Gmin stepping.
    gmin_result = _dc_gmin_step(
        circuit,
        max_iterations=max_iterations,
        tol=tol,
        newton_step_limit=newton_step_limit,
    )
    if gmin_result is not None and gmin_result.converged:
        return replace(
            gmin_result,
            convergence_aid="gmin",
            diagnostics=replace(gmin_result.diagnostics, convergence_aid="gmin"),
        )

    # Attempt 3: source stepping.
    src_result = _dc_source_step(
        circuit,
        max_iterations=max_iterations,
        tol=tol,
        newton_step_limit=newton_step_limit,
    )
    if src_result is not None and src_result.converged:
        return replace(
            src_result,
            convergence_aid="source",
            diagnostics=replace(src_result.diagnostics, convergence_aid="source"),
        )

    # Attempt 4: artificial pseudo-transient continuation.
    pseudo_result = _dc_pseudo_transient(
        circuit,
        max_iterations=(
            pseudo_transient_max_iterations
            if pseudo_transient_max_iterations is not None
            else max_iterations
        ),
        tol=tol,
        n_steps=pseudo_transient_steps,
        shunt_conductance=pseudo_transient_shunt_conductance,
        newton_step_limit=newton_step_limit,
    )
    if pseudo_result is not None and pseudo_result.converged:
        return replace(
            pseudo_result,
            convergence_aid="pseudo_transient",
            diagnostics=replace(
                pseudo_result.diagnostics,
                convergence_aid="pseudo_transient",
            ),
        )

    # All methods exhausted — return the plain-Newton result (converged=False).
    return replace(
        result,
        convergence_aid="none",
        diagnostics=replace(result.diagnostics, convergence_aid="none"),
    )


def dc_op_with_initial_conditions(
    circuit: Circuit,
    summary: DeckInitialConditionSummary,
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    convergence_aids: bool = True,
    pseudo_transient_steps: int = 20,
    pseudo_transient_shunt_conductance: float = 1e-3,
    pseudo_transient_max_iterations: int | None = None,
    newton_step_limit: float | None = _DEFAULT_NEWTON_STEP_LIMIT,
) -> DcResult:
    """Solve DC operating point using parsed ``.ic``/``.nodeset`` node hints."""

    initial_vector = dc_initial_vector_from_conditions(
        circuit,
        summary.initial_conditions,
        summary.nodesets,
    )
    return dc_op(
        circuit,
        max_iterations=max_iterations,
        tol=tol,
        convergence_aids=convergence_aids,
        pseudo_transient_steps=pseudo_transient_steps,
        pseudo_transient_shunt_conductance=pseudo_transient_shunt_conductance,
        pseudo_transient_max_iterations=pseudo_transient_max_iterations,
        initial_vector=initial_vector,
        newton_step_limit=newton_step_limit,
    )


def _apply_corner_override(element: Element, override: CornerOverride) -> Element:
    if not math.isfinite(override.value):
        raise ValueError("dc_corners: override values must be finite")

    if isinstance(element, Resistor) and override.parameter == "resistance":
        if override.value <= 0.0:
            raise ValueError("dc_corners: resistance overrides must be positive")
        return replace(element, resistance=override.value)
    if isinstance(element, Capacitor) and override.parameter == "capacitance":
        if override.value <= 0.0:
            raise ValueError("dc_corners: capacitance overrides must be positive")
        return replace(element, capacitance=override.value)
    if isinstance(element, Inductor) and override.parameter == "inductance":
        if override.value <= 0.0:
            raise ValueError("dc_corners: inductance overrides must be positive")
        return replace(element, inductance=override.value)
    if isinstance(element, VoltageSource) and override.parameter == "voltage":
        return replace(element, voltage=override.value)
    if isinstance(element, CurrentSource) and override.parameter == "current":
        return replace(element, current=override.value)
    raise ValueError(
        f"dc_corners: unsupported override {override.element_name!r}.{override.parameter!r}"
    )


def _circuit_with_corner(circuit: Circuit, corner: CornerSpec) -> Circuit:
    overrides_by_name: dict[str, list[CornerOverride]] = {}
    for override in corner.overrides:
        overrides_by_name.setdefault(override.element_name, []).append(override)

    elements: list[Element] = []
    seen: set[str] = set()
    for element in circuit.elements:
        name = getattr(element, "name", None)
        if name in overrides_by_name:
            seen.add(name)
            for override in overrides_by_name[name]:
                element = _apply_corner_override(element, override)
        elements.append(element)

    missing = sorted(set(overrides_by_name) - seen)
    if missing:
        raise ValueError(f"dc_corners: missing element(s) for corner overrides: {missing}")
    return Circuit(elements)


def dc_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    convergence_aids: bool = True,
) -> CornerSweepResult:
    """Run DC operating point at each named corner.

    Each corner clones the circuit with explicit element-parameter overrides,
    then reuses :func:`dc_op`.  Supported override parameters are
    ``resistance``, ``capacitance``, ``inductance``, ``voltage`` and
    ``current``.
    """
    points = [
        CornerPoint(
            corner_name=corner.name,
            result=dc_op(
                _circuit_with_corner(circuit, corner),
                max_iterations=max_iterations,
                tol=tol,
                convergence_aids=convergence_aids,
            ),
        )
        for corner in corners
    ]
    return CornerSweepResult(points=points)


def dc_temperature_sweep(
    circuit: Circuit,
    temperatures_kelvin: list[float],
    *,
    nominal_temperature_kelvin: float = 300.15,
    energy_gap_ev: float = 1.11,
    max_iterations: int = 50,
    tol: float = 1e-6,
    convergence_aids: bool = True,
) -> TemperatureDcResult:
    """Run DC operating points at explicit semiconductor analysis temperatures."""
    return TemperatureDcResult(
        points=[
            TemperatureDcPoint(
                temperature_kelvin=temperature_kelvin,
                result=dc_op(
                    circuit_at_temperature(
                        circuit,
                        temperature_kelvin,
                        nominal_temperature_kelvin=nominal_temperature_kelvin,
                        energy_gap_ev=energy_gap_ev,
                    ),
                    max_iterations=max_iterations,
                    tol=tol,
                    convergence_aids=convergence_aids,
                ),
            )
            for temperature_kelvin in temperatures_kelvin
        ]
    )


def dc_temperature_sweep_corners(
    circuit: Circuit,
    temperatures_kelvin: list[float],
    corners: list[CornerSpec],
    *,
    nominal_temperature_kelvin: float = 300.15,
    energy_gap_ev: float = 1.11,
    max_iterations: int = 50,
    tol: float = 1e-6,
    convergence_aids: bool = True,
) -> CornerTemperatureDcResult:
    """Run DC temperature sweeps at each named corner."""
    return CornerTemperatureDcResult(
        points=[
            CornerTemperatureDcPoint(
                corner_name=corner.name,
                points=dc_temperature_sweep(
                    _circuit_with_corner(circuit, corner),
                    temperatures_kelvin,
                    nominal_temperature_kelvin=nominal_temperature_kelvin,
                    energy_gap_ev=energy_gap_ev,
                    max_iterations=max_iterations,
                    tol=tol,
                    convergence_aids=convergence_aids,
                ).points,
            )
            for corner in corners
        ]
    )


def _stamp_dc(
    el: Element,
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    branch_srcs: list[VoltageSource | VCVS | CCVS | BSource],
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
    elif isinstance(el, BSource):
        _stamp_bsource(G, b, x, node_to_idx, branch_srcs, el)
    elif isinstance(el, CustomModel):
        _stamp_custom_model(G, b, x, node_to_idx, el)
    elif isinstance(el, Diode):
        _stamp_diode(G, b, x, node_to_idx, el)
    elif isinstance(el, JFET):
        _stamp_jfet(G, b, x, node_to_idx, el)
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


def _bsource_expr_nodes(expr: str) -> list[str]:
    nodes: list[str] = []
    for match in re.finditer(r"V\s*\(([^)]*)\)", expr):
        args = [arg.strip() for arg in match.group(1).split(",")]
        if len(args) not in (1, 2):
            raise ValueError("V() expects one or two node arguments")
        for node_name in args:
            if node_name and not _is_ground(node_name):
                nodes.append(node_name)
    return nodes


def _normalize_bsource_expr(expr: str) -> str:
    def replace(match: re.Match[str]) -> str:
        args = [arg.strip() for arg in match.group(1).split(",")]
        if len(args) not in (1, 2):
            raise ValueError("V() expects one or two node arguments")
        return "V(" + ", ".join(repr(arg) for arg in args) + ")"

    return re.sub(r"V\s*\(([^)]*)\)", replace, expr)


def _ast_call_name(node: ast.Call) -> str | None:
    return node.func.id if isinstance(node.func, ast.Name) else None


def _ast_node_name(node: ast.AST) -> str:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Constant) and isinstance(node.value, (str, int)):
        return str(node.value)
    raise ValueError("V() node references must be bare node names or 0")


def _bsource_voltage(name: str, node_to_idx: dict[str, int], x: list[float]) -> float:
    return 0.0 if _is_ground(name) else x[node_to_idx[name]]


def _eval_bsource_expr(expr: str, node_to_idx: dict[str, int], x: list[float]) -> float:
    normalized_expr = _normalize_bsource_expr(expr)
    tree = ast.parse(normalized_expr, mode="eval")

    def eval_node(node: ast.AST) -> float:
        if isinstance(node, ast.Expression):
            return eval_node(node.body)
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
            return float(node.value)
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.UAdd, ast.USub)):
            value = eval_node(node.operand)
            return value if isinstance(node.op, ast.UAdd) else -value
        if isinstance(node, ast.BinOp) and isinstance(node.op, (ast.Add, ast.Sub, ast.Mult, ast.Div)):
            left = eval_node(node.left)
            right = eval_node(node.right)
            if isinstance(node.op, ast.Add):
                return left + right
            if isinstance(node.op, ast.Sub):
                return left - right
            if isinstance(node.op, ast.Mult):
                return left * right
            return left / right
        if isinstance(node, ast.Call) and _ast_call_name(node) == "V":
            if len(node.args) == 1:
                return _bsource_voltage(_ast_node_name(node.args[0]), node_to_idx, x)
            if len(node.args) == 2:
                return (
                    _bsource_voltage(_ast_node_name(node.args[0]), node_to_idx, x)
                    - _bsource_voltage(_ast_node_name(node.args[1]), node_to_idx, x)
                )
        raise ValueError(f"unsupported behavioral source expression in '{expr}'")

    value = eval_node(tree)
    if not math.isfinite(value):
        raise ValueError(f"behavioral source expression produced non-finite value: {expr}")
    return value


def _bsource_linearization(
    expr: str,
    node_to_idx: dict[str, int],
    x: list[float],
) -> tuple[float, dict[str, float], float]:
    value = _eval_bsource_expr(expr, node_to_idx, x)
    derivatives: dict[str, float] = {}
    for node, idx in node_to_idx.items():
        h = max(1e-6, abs(x[idx]) * 1e-6)
        xp = list(x)
        xm = list(x)
        xp[idx] += h
        xm[idx] -= h
        derivatives[node] = (
            _eval_bsource_expr(expr, node_to_idx, xp)
            - _eval_bsource_expr(expr, node_to_idx, xm)
        ) / (2.0 * h)
    offset = value - sum(derivatives[node] * x[idx] for node, idx in node_to_idx.items())
    return value, derivatives, offset


def _stamp_bsource(
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    branch_srcs: list[VoltageSource | VCVS | CCVS | BSource],
    el: BSource,
) -> None:
    has_voltage = el.voltage_expr is not None
    has_current = el.current_expr is not None
    if has_voltage == has_current:
        raise ValueError(f"BSource '{el.name}' must define exactly one voltage_expr or current_expr.")

    if el.current_expr is not None:
        _, derivatives, offset = _bsource_linearization(el.current_expr, node_to_idx, x)
        if not _is_ground(el.n_plus):
            row = node_to_idx[el.n_plus]
            for node, derivative in derivatives.items():
                G[row][node_to_idx[node]] += derivative
            b[row] -= offset
        if not _is_ground(el.n_minus):
            row = node_to_idx[el.n_minus]
            for node, derivative in derivatives.items():
                G[row][node_to_idx[node]] -= derivative
            b[row] += offset
        return

    assert el.voltage_expr is not None
    _, derivatives, offset = _bsource_linearization(el.voltage_expr, node_to_idx, x)
    branch_idx = len(node_to_idx) + branch_srcs.index(el)
    if not _is_ground(el.n_plus):
        p = node_to_idx[el.n_plus]
        G[p][branch_idx] += 1.0
        G[branch_idx][p] += 1.0
    if not _is_ground(el.n_minus):
        q = node_to_idx[el.n_minus]
        G[q][branch_idx] -= 1.0
        G[branch_idx][q] -= 1.0
    for node, derivative in derivatives.items():
        G[branch_idx][node_to_idx[node]] -= derivative
    b[branch_idx] += offset


def _custom_model_voltage(
    el: CustomModel,
    node_to_idx: dict[str, int],
    x: list[float],
) -> float:
    v_plus = 0.0 if _is_ground(el.n_plus) else x[node_to_idx[el.n_plus]]
    v_minus = 0.0 if _is_ground(el.n_minus) else x[node_to_idx[el.n_minus]]
    return v_plus - v_minus


def _evaluate_custom_model(el: CustomModel, voltage: float) -> CustomModelEvaluation:
    if el.evaluator is not None:
        result = el.evaluator(
            CustomModelContext(voltage=voltage, parameters=el.parameters)
        )
    else:
        if el.conductance_siemens is None:
            raise ValueError(
                f"CustomModel '{el.name}' must define an evaluator or conductance_siemens."
            )
        result = CustomModelEvaluation(
            current_amps=el.conductance_siemens * voltage + el.current_offset_amps,
            conductance_siemens=el.conductance_siemens,
        )
    if not math.isfinite(result.current_amps):
        raise ValueError(f"CustomModel '{el.name}' produced non-finite current")
    if not math.isfinite(result.conductance_siemens):
        raise ValueError(f"CustomModel '{el.name}' produced non-finite conductance")
    return result


def _custom_model_conductance(
    el: CustomModel,
    node_to_idx: dict[str, int],
    x: list[float],
) -> float:
    return _evaluate_custom_model(el, _custom_model_voltage(el, node_to_idx, x)).conductance_siemens


def _stamp_custom_model(
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    el: CustomModel,
) -> None:
    voltage = _custom_model_voltage(el, node_to_idx, x)
    evaluation = _evaluate_custom_model(el, voltage)
    _stamp_g(G, node_to_idx, el.n_plus, el.n_minus, evaluation.conductance_siemens)
    equivalent_current = evaluation.current_amps - evaluation.conductance_siemens * voltage
    if not _is_ground(el.n_plus):
        b[node_to_idx[el.n_plus]] -= equivalent_current
    if not _is_ground(el.n_minus):
        b[node_to_idx[el.n_minus]] += equivalent_current


# ---------------------------------------------------------------------------
# Controlled-source MNA stamps
# ---------------------------------------------------------------------------


def _find_branch_source(
    branch_srcs: list[VoltageSource | VCVS | CCVS | BSource],
    name: str,
) -> VoltageSource | VCVS | CCVS | BSource | None:
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
    """Linearized diode: I = Is*(exp(Vd/(N*Vt)) - 1).

    Newton: I0 = Is*(exp(Vd0/(N*Vt)) - 1), gd = (Is/(N*Vt))*exp(Vd0/(N*Vt)).
    Stamp gd as conductance + (gd*Vd0 - I0) as current source from cathode."""
    intrinsic_anode = _diode_intrinsic_anode_node(el)
    Va = 0.0 if _is_ground(intrinsic_anode) else x[node_to_idx[intrinsic_anode]]
    Vk = 0.0 if _is_ground(el.cathode) else x[node_to_idx[el.cathode]]
    Vd = Va - Vk
    # Clamp to avoid exp overflow
    Vd = min(Vd, 0.7 * el.N)
    I0, gd = _diode_current_conductance(el, Vd)

    _stamp_g(G, node_to_idx, intrinsic_anode, el.cathode, gd)
    Ieq = I0 - gd * Vd
    if not _is_ground(intrinsic_anode):
        b[node_to_idx[intrinsic_anode]] -= Ieq
    if not _is_ground(el.cathode):
        b[node_to_idx[el.cathode]] += Ieq
    if el.Rs > 0.0:
        _stamp_g(G, node_to_idx, el.anode, intrinsic_anode, 1.0 / el.Rs)


def _diode_effective_vt(el: Diode) -> float:
    if not math.isfinite(el.Kf) or el.Kf < 0.0:
        raise ValueError(
            f"{el.name}: diode flicker-noise coefficient must be finite and non-negative"
        )
    if not math.isfinite(el.Af) or el.Af < 0.0:
        raise ValueError(
            f"{el.name}: diode flicker-noise exponent must be finite and non-negative"
        )
    if not math.isfinite(el.Rs) or el.Rs < 0.0:
        raise ValueError(f"{el.name}: diode series resistance must be finite and non-negative")
    if not math.isfinite(el.N) or el.N <= 0.0:
        raise ValueError(f"{el.name}: diode emission coefficient must be finite and positive")
    if not math.isfinite(el.Vt) or el.Vt <= 0.0:
        raise ValueError(f"{el.name}: diode thermal voltage must be finite and positive")
    if el.BV is not None and (not math.isfinite(el.BV) or el.BV <= 0.0):
        raise ValueError(f"{el.name}: diode breakdown voltage must be finite and positive")
    if not math.isfinite(el.IBV) or el.IBV <= 0.0:
        raise ValueError(f"{el.name}: diode breakdown current must be finite and positive")
    if not math.isfinite(el.Cjo) or el.Cjo < 0.0:
        raise ValueError(f"{el.name}: diode junction capacitance must be finite and non-negative")
    if not math.isfinite(el.Vj) or el.Vj <= 0.0:
        raise ValueError(f"{el.name}: diode junction potential must be finite and positive")
    if not math.isfinite(el.M) or el.M < 0.0:
        raise ValueError(f"{el.name}: diode grading coefficient must be finite and non-negative")
    if not math.isfinite(el.Fc) or el.Fc < 0.0 or el.Fc >= 1.0:
        raise ValueError(
            f"{el.name}: diode forward-bias depletion coefficient must be finite and in [0, 1)"
        )
    if not math.isfinite(el.Xti):
        raise ValueError(
            f"{el.name}: diode saturation-current temperature exponent must be finite"
        )
    if not math.isfinite(el.Eg) or el.Eg <= 0.0:
        raise ValueError(f"{el.name}: diode energy gap must be finite and positive")
    if not math.isfinite(el.Tt) or el.Tt < 0.0:
        raise ValueError(f"{el.name}: diode transit time must be finite and non-negative")
    return el.Vt * el.N


def _diode_current_conductance(el: Diode, vd: float, *, clamp_forward: bool = True) -> tuple[float, float]:
    vt_eff = _diode_effective_vt(el)
    forward_vd = min(vd, 0.7 * el.N) if clamp_forward else vd
    exp_term = math.exp(max(-40.0, min(40.0, forward_vd / vt_eff)))
    current = el.Is * (exp_term - 1.0)
    conductance = (el.Is / vt_eff) * exp_term
    if el.BV is not None and vd <= -el.BV:
        breakdown_exp = math.exp(max(-40.0, min(40.0, ((-vd) - el.BV) / vt_eff)))
        current -= el.IBV * breakdown_exp
        conductance += (el.IBV / vt_eff) * breakdown_exp
    return current, conductance


def _diode_charge_state_name(el: Diode) -> str:
    return f"_D_{el.name}_charge"


def _diode_intrinsic_anode_node(el: Diode) -> str:
    return el.anode if el.Rs == 0.0 else f"_D_{el.name}_anode"


def _diode_has_charge_storage(el: Diode) -> bool:
    return el.Cjo > 0.0 or el.Tt > 0.0


def _diode_dynamic_capacitance(el: Diode, vd: float) -> float:
    _, gd = _diode_current_conductance(el, vd)
    return _diode_depletion_capacitance(el, vd) + el.Tt * gd


def _diode_depletion_capacitance(el: Diode, vd: float) -> float:
    if el.Cjo <= 0.0 or el.M == 0.0:
        return el.Cjo
    normalized_voltage = vd / el.Vj
    if normalized_voltage < el.Fc:
        return el.Cjo / ((1.0 - normalized_voltage) ** el.M)
    transition_scale = (1.0 - el.Fc) ** (1.0 + el.M)
    continuation = 1.0 - el.Fc * (1.0 + el.M) + el.M * normalized_voltage
    return el.Cjo * continuation / transition_scale


def _diode_charge_voltage(el: Diode, node_voltages: dict[str, float]) -> float:
    return _node_voltage(_diode_intrinsic_anode_node(el), node_voltages) - _node_voltage(
        el.cathode, node_voltages
    )


def _bjt_base_emitter_charge_state_name(el: BJT) -> str:
    return f"_Q_{el.name}_be_charge"


def _bjt_base_collector_charge_state_name(el: BJT) -> str:
    return f"_Q_{el.name}_bc_charge"


def _bjt_external_base_collector_charge_state_name(el: BJT) -> str:
    return f"_Q_{el.name}_bx_charge"


def _bjt_intrinsic_emitter_node(el: BJT) -> str:
    return el.emitter if el.Re == 0.0 else f"__spice_{el.name}_emitter"


def _bjt_intrinsic_collector_node(el: BJT) -> str:
    return el.collector if el.Rc == 0.0 else f"__spice_{el.name}_collector"


def _bjt_intrinsic_base_node(el: BJT) -> str:
    return el.base if el.Rb == 0.0 else f"__spice_{el.name}_base"


def _bjt_junction_transconductance(el: BJT, voltage: float, emission_coefficient: float) -> float:
    effective_thermal_voltage = el.Vt * emission_coefficient
    exponent = max(-40.0, min(40.0, voltage / effective_thermal_voltage))
    return (el.Is / effective_thermal_voltage) * math.exp(exponent)


def _bjt_forward_transit_time_scale(
    el: BJT,
    voltage: float,
    reverse_junction_voltage: float,
) -> float:
    effective_thermal_voltage = el.Vt * el.Nf
    forward_current = max(
        el.Is * (math.exp(max(-40.0, min(40.0, voltage / effective_thermal_voltage))) - 1.0),
        0.0,
    )
    current_factor = 1.0
    if el.Itf > 0.0:
        ratio = forward_current / (forward_current + el.Itf)
        current_factor = ratio * ratio
    voltage_factor = 1.0
    if el.Vtf > 0.0:
        voltage_exponent = max(-40.0, min(40.0, reverse_junction_voltage / (1.44 * el.Vtf)))
        voltage_factor = math.exp(voltage_exponent)
    return 1.0 + el.Xtf * current_factor * voltage_factor


def _bjt_charge_dynamic_capacitance(
    el: BJT,
    state_kind: str,
    voltage: float,
    reverse_junction_voltage: float,
) -> float:
    if state_kind == "be":
        conductance = _bjt_junction_transconductance(el, voltage, el.Nf)
        return (
            _bjt_base_emitter_depletion_capacitance(el, voltage)
            + el.Tf
            * _bjt_forward_transit_time_scale(el, voltage, reverse_junction_voltage)
            * conductance
        )
    depletion_capacitance = _bjt_base_collector_depletion_capacitance(el, voltage)
    if state_kind == "bx":
        return (1.0 - el.Xcjc) * depletion_capacitance
    conductance = _bjt_junction_transconductance(el, voltage, el.Nr)
    return el.Xcjc * depletion_capacitance + el.Tr * conductance


def _bjt_base_emitter_depletion_capacitance(el: BJT, voltage: float) -> float:
    if el.Cje <= 0.0 or el.Mje == 0.0:
        return el.Cje
    normalized_voltage = voltage / el.Vje
    coefficient = el.Fc
    if normalized_voltage < coefficient:
        return el.Cje / ((1.0 - normalized_voltage) ** el.Mje)
    transition_scale = (1.0 - coefficient) ** (1.0 + el.Mje)
    continuation = 1.0 - coefficient * (1.0 + el.Mje) + el.Mje * normalized_voltage
    return el.Cje * continuation / transition_scale


def _bjt_base_collector_depletion_capacitance(el: BJT, voltage: float) -> float:
    if el.Cjc <= 0.0 or el.Mjc == 0.0:
        return el.Cjc
    normalized_voltage = voltage / el.Vjc
    coefficient = el.Fc
    if normalized_voltage < coefficient:
        return el.Cjc / ((1.0 - normalized_voltage) ** el.Mjc)
    transition_scale = (1.0 - coefficient) ** (1.0 + el.Mjc)
    continuation = 1.0 - coefficient * (1.0 + el.Mjc) + el.Mjc * normalized_voltage
    return el.Cjc * continuation / transition_scale


def _bjt_charge_state_specs(el: BJT) -> list[tuple[str, str, str, str]]:
    specs: list[tuple[str, str, str, str]] = []
    emitter = _bjt_intrinsic_emitter_node(el)
    collector = _bjt_intrinsic_collector_node(el)
    base = _bjt_intrinsic_base_node(el)
    if el.Cje > 0.0 or el.Tf > 0.0:
        if el.polarity == "NPN":
            specs.append((_bjt_base_emitter_charge_state_name(el), base, emitter, "be"))
        else:
            specs.append((_bjt_base_emitter_charge_state_name(el), emitter, base, "be"))
    if el.Cjc > 0.0 or el.Tr > 0.0 or (el.Tf > 0.0 and el.Xtf > 0.0 and el.Vtf > 0.0):
        if el.polarity == "NPN":
            specs.append((_bjt_base_collector_charge_state_name(el), base, collector, "bc"))
        else:
            specs.append((_bjt_base_collector_charge_state_name(el), collector, base, "bc"))
    if el.Cjc > 0.0 and el.Xcjc < 1.0:
        if el.polarity == "NPN":
            specs.append(
                (
                    _bjt_external_base_collector_charge_state_name(el),
                    el.base,
                    collector,
                    "bx",
                )
            )
        else:
            specs.append(
                (
                    _bjt_external_base_collector_charge_state_name(el),
                    collector,
                    el.base,
                    "bx",
                )
            )
    return specs


def _bjt_charge_state_voltage(n_plus: str, n_minus: str, node_voltages: dict[str, float]) -> float:
    return _node_voltage(n_plus, node_voltages) - _node_voltage(n_minus, node_voltages)


def _jfet_gate_source_charge_state_name(el: JFET) -> str:
    return f"_J_{el.name}_gs_charge"


def _jfet_gate_drain_charge_state_name(el: JFET) -> str:
    return f"_J_{el.name}_gd_charge"


def _jfet_charge_state_specs(el: JFET) -> list[tuple[str, str, str, float]]:
    specs: list[tuple[str, str, str, float]] = []
    if el.Cgs > 0.0:
        specs.append((_jfet_gate_source_charge_state_name(el), el.gate, el.source, el.Cgs))
    if el.Cgd > 0.0:
        specs.append((_jfet_gate_drain_charge_state_name(el), el.gate, el.drain, el.Cgd))
    return specs


def _jfet_charge_state_voltage(n_plus: str, n_minus: str, node_voltages: dict[str, float]) -> float:
    return _node_voltage(n_plus, node_voltages) - _node_voltage(n_minus, node_voltages)


def _mosfet_gate_source_charge_state_name(el: Mosfet) -> str:
    return f"_M_{el.name}_gs_charge"


def _mosfet_gate_drain_charge_state_name(el: Mosfet) -> str:
    return f"_M_{el.name}_gd_charge"


def _mosfet_gate_body_charge_state_name(el: Mosfet) -> str:
    return f"_M_{el.name}_gb_charge"


def _mosfet_source_body_charge_state_name(el: Mosfet) -> str:
    return f"_M_{el.name}_sb_charge"


def _mosfet_drain_body_charge_state_name(el: Mosfet) -> str:
    return f"_M_{el.name}_db_charge"


def _mosfet_charge_state_specs(el: Mosfet) -> list[tuple[str, str, str, float]]:
    params = getattr(getattr(el.model, "model", None), "params", None)
    if params is None:
        return []
    width = getattr(params, "W", 0.0)
    length = getattr(params, "L", 0.0)
    specs: list[tuple[str, str, str, float]] = []
    cgs = getattr(params, "CGSO", 0.0) * width
    cgd = getattr(params, "CGDO", 0.0) * width
    cgb = getattr(params, "CGBO", 0.0) * length
    cbs = getattr(params, "CBS", 0.0)
    cbd = getattr(params, "CBD", 0.0)
    if cgs > 0.0:
        specs.append((_mosfet_gate_source_charge_state_name(el), el.gate, el.source, cgs))
    if cgd > 0.0:
        specs.append((_mosfet_gate_drain_charge_state_name(el), el.gate, el.drain, cgd))
    if cgb > 0.0:
        specs.append((_mosfet_gate_body_charge_state_name(el), el.gate, el.body, cgb))
    if cbs > 0.0:
        specs.append((_mosfet_source_body_charge_state_name(el), el.source, el.body, cbs))
    if cbd > 0.0:
        specs.append((_mosfet_drain_body_charge_state_name(el), el.drain, el.body, cbd))
    return specs


def _mosfet_charge_state_voltage(n_plus: str, n_minus: str, node_voltages: dict[str, float]) -> float:
    return _node_voltage(n_plus, node_voltages) - _node_voltage(n_minus, node_voltages)


def _mosfet_charge_dynamic_capacitance(
    el: Mosfet,
    state_name: str,
    zero_bias_capacitance: float,
    state_voltage: float,
) -> float:
    params = getattr(getattr(el.model, "model", None), "params", None)
    if params is None or zero_bias_capacitance <= 0.0:
        return zero_bias_capacitance
    if state_name not in {
        _mosfet_source_body_charge_state_name(el),
        _mosfet_drain_body_charge_state_name(el),
    }:
        return zero_bias_capacitance
    junction_potential = getattr(params, "PB", 0.8)
    grading_coefficient = getattr(params, "MJ", 0.5)
    if not math.isfinite(junction_potential) or junction_potential <= 0.0:
        raise ValueError(f"{el.name}: MOSFET PB must be finite and positive")
    if not math.isfinite(grading_coefficient) or grading_coefficient < 0.0:
        raise ValueError(f"{el.name}: MOSFET MJ must be finite and non-negative")
    mosfet_type = getattr(el.model, "type", None)
    junction_voltage = state_voltage if mosfet_type == MosfetType.PMOS else -state_voltage
    return bulk_junction_capacitance(
        zero_bias_capacitance,
        junction_voltage,
        junction_potential,
        grading_coefficient,
    )


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


def _eval_jfet(el: JFET, vgs: float, vds: float) -> tuple[float, float, float]:
    if not math.isfinite(el.beta) or el.beta <= 0.0:
        raise ValueError(f"JFET '{el.name}' beta must be finite and positive")
    if not math.isfinite(el.vto):
        raise ValueError(f"JFET '{el.name}' VTO must be finite")
    if not math.isfinite(el.lambda_):
        raise ValueError(f"JFET '{el.name}' LAMBDA must be finite")
    if not math.isfinite(el.Cgs) or el.Cgs < 0.0:
        raise ValueError(f"JFET '{el.name}' CGS must be finite and non-negative")
    if not math.isfinite(el.Cgd) or el.Cgd < 0.0:
        raise ValueError(f"JFET '{el.name}' CGD must be finite and non-negative")
    if not math.isfinite(el.Kf) or el.Kf < 0.0:
        raise ValueError(f"JFET '{el.name}' flicker-noise coefficient must be finite and non-negative")
    if not math.isfinite(el.Af) or el.Af < 0.0:
        raise ValueError(f"JFET '{el.name}' flicker-noise exponent must be finite and non-negative")
    if el.polarity == "PJF":
        ids, gm, gds = _eval_njf(-vgs, -vds, -el.vto, el.beta, el.lambda_)
        return -ids, gm, gds
    if el.polarity != "NJF":
        raise ValueError(f"JFET '{el.name}' polarity must be 'NJF' or 'PJF'")
    return _eval_njf(vgs, vds, el.vto, el.beta, el.lambda_)


def _eval_njf(
    vgs: float, vds: float, vto: float, beta: float, lambda_: float
) -> tuple[float, float, float]:
    overdrive = vgs - vto
    if overdrive <= 0.0 or vds < 0.0:
        return (0.0, 0.0, 0.0)
    if vds < overdrive:
        channel = 2.0 * overdrive * vds - vds * vds
        modulation = 1.0 + lambda_ * vds
        ids = beta * channel * modulation
        gm = 2.0 * beta * vds * modulation
        gds = beta * (2.0 * overdrive - 2.0 * vds) * modulation + beta * channel * lambda_
        return (ids, gm, gds)
    ids = beta * overdrive * overdrive * (1.0 + lambda_ * vds)
    gm = 2.0 * beta * overdrive * (1.0 + lambda_ * vds)
    gds = beta * overdrive * overdrive * lambda_
    return (ids, gm, gds)


def _stamp_jfet(
    G: list[list[float]],
    b: list[float],
    x: list[float],
    node_to_idx: dict[str, int],
    el: JFET,
) -> None:
    Vd = 0.0 if _is_ground(el.drain) else x[node_to_idx[el.drain]]
    Vg = 0.0 if _is_ground(el.gate) else x[node_to_idx[el.gate]]
    Vs = 0.0 if _is_ground(el.source) else x[node_to_idx[el.source]]
    vgs = Vg - Vs
    vds = Vd - Vs
    ids, gm, gds = _eval_jfet(el, vgs, vds)

    _stamp_g(G, node_to_idx, el.drain, el.source, gds)
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
    Ieq = ids - gm * vgs - gds * vds
    if not _is_ground(el.drain):
        b[node_to_idx[el.drain]] -= Ieq
    if not _is_ground(el.source):
        b[node_to_idx[el.source]] += Ieq


def _bjt_early_factor(el: BJT, junction_voltage: float, output_voltage: float) -> float:
    forward_term = 0.0 if el.Vaf == 0.0 else output_voltage / el.Vaf
    reverse_term = 0.0 if el.Var == 0.0 else junction_voltage / el.Var
    return 1.0 + forward_term - reverse_term


def _bjt_forward_transconductance(
    el: BJT,
    base_collector_current: float,
    base_gm: float,
    early_factor: float,
) -> float:
    reverse_early_conductance = 0.0 if el.Var == 0.0 else base_collector_current / el.Var
    return base_gm * early_factor - reverse_early_conductance


def _bjt_forward_transport(
    el: BJT,
    base_collector_current: float,
    base_gm: float,
    early_factor: float,
) -> tuple[float, float, float]:
    low_current_gm = _bjt_forward_transconductance(
        el, base_collector_current, base_gm, early_factor
    )
    if el.Ikf == 0.0 or base_collector_current <= 0.0:
        return base_collector_current * early_factor, low_current_gm, 1.0
    root = math.sqrt(1.0 + 4.0 * base_collector_current / el.Ikf)
    charge_factor = 0.5 * (1.0 + root)
    charge_derivative = base_gm / (el.Ikf * root)
    collector_current = base_collector_current * early_factor / charge_factor
    gm = (
        low_current_gm / charge_factor
        - base_collector_current * early_factor * charge_derivative
        / charge_factor**2
    )
    return collector_current, gm, charge_factor


def _bjt_base_emitter_leakage(el: BJT, junction_voltage: float) -> tuple[float, float]:
    if el.Ise == 0.0:
        return 0.0, 0.0
    thermal_voltage = el.Vt * el.Ne
    exponent = max(-40.0, min(40.0, junction_voltage / thermal_voltage))
    exp_value = math.exp(exponent)
    return el.Ise * (exp_value - 1.0), el.Ise / thermal_voltage * exp_value


def _bjt_base_collector_leakage(el: BJT, junction_voltage: float) -> tuple[float, float]:
    if el.Isc == 0.0:
        return 0.0, 0.0
    thermal_voltage = el.Vt * el.Nc
    exponent = max(-40.0, min(40.0, junction_voltage / thermal_voltage))
    exp_value = math.exp(exponent)
    return el.Isc * (exp_value - 1.0), el.Isc / thermal_voltage * exp_value


def _bjt_reverse_base_current(el: BJT, junction_voltage: float) -> tuple[float, float]:
    if math.isinf(el.beta_r):
        return 0.0, 0.0
    thermal_voltage = el.Vt * el.Nr
    exponent = max(-40.0, min(40.0, junction_voltage / thermal_voltage))
    exp_value = math.exp(exponent)
    diffusion_current = el.Is * (exp_value - 1.0)
    diffusion_conductance = el.Is / thermal_voltage * exp_value
    if el.Ikr == 0.0 or diffusion_current <= 0.0:
        return diffusion_current / el.beta_r, diffusion_conductance / el.beta_r
    root = math.sqrt(1.0 + 4.0 * diffusion_current / el.Ikr)
    charge_factor = 0.5 * (1.0 + root)
    charge_derivative = diffusion_conductance / (el.Ikr * root)
    return (
        diffusion_current * charge_factor / el.beta_r,
        (diffusion_conductance * charge_factor + diffusion_current * charge_derivative)
        / el.beta_r,
    )


def _bjt_effective_base_resistance(
    el: BJT,
    base_voltage: float,
    emitter_voltage: float,
    collector_voltage: float,
) -> float:
    minimum = el.Rb if el.Rbm is None else el.Rbm
    if minimum == el.Rb:
        return el.Rb
    junction_voltage = (
        base_voltage - emitter_voltage
        if el.polarity == "NPN"
        else emitter_voltage - base_voltage
    )
    reverse_voltage = (
        base_voltage - collector_voltage
        if el.polarity == "NPN"
        else collector_voltage - base_voltage
    )
    forward_thermal_voltage = el.Vt * el.Nf
    exp_value = math.exp(
        max(-40.0, min(40.0, junction_voltage / forward_thermal_voltage))
    )
    diffusion_current = el.Is * (exp_value - 1.0)
    diffusion_conductance = el.Is / forward_thermal_voltage * exp_value
    output_voltage = (
        collector_voltage - emitter_voltage
        if el.polarity == "NPN"
        else emitter_voltage - collector_voltage
    )
    early_factor = _bjt_early_factor(el, junction_voltage, output_voltage)
    _, _, charge_factor = _bjt_forward_transport(
        el, diffusion_current, diffusion_conductance, early_factor
    )
    leakage_current, _ = _bjt_base_emitter_leakage(el, junction_voltage)
    collector_leakage_current, _ = _bjt_base_collector_leakage(el, reverse_voltage)
    reverse_base_current, _ = _bjt_reverse_base_current(el, reverse_voltage)
    base_current = (
        diffusion_current / el.beta_f
        + leakage_current
        + collector_leakage_current
        + reverse_base_current
    )
    variable_resistance = el.Rb - minimum
    if el.Irb == 0.0:
        return minimum + variable_resistance / charge_factor
    ratio = max(base_current / el.Irb, 1.0e-9)
    angle = (
        (-1.0 + math.sqrt(1.0 + 14.59025 * ratio))
        / (2.4317 * math.sqrt(ratio))
    )
    tangent = math.tan(angle)
    transition = 3.0 * (tangent - angle) / (angle * tangent * tangent)
    return minimum + variable_resistance * transition


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
    _validate_bjt(el)
    if el.Re > 0.0:
        intrinsic_emitter = _bjt_intrinsic_emitter_node(el)
        _stamp_g(G, node_to_idx, el.emitter, intrinsic_emitter, 1.0 / el.Re)
        el = replace(el, emitter=intrinsic_emitter, Re=0.0)
    if el.Rc > 0.0:
        intrinsic_collector = _bjt_intrinsic_collector_node(el)
        _stamp_g(G, node_to_idx, el.collector, intrinsic_collector, 1.0 / el.Rc)
        el = replace(el, collector=intrinsic_collector, Rc=0.0)
    if el.Rb > 0.0:
        intrinsic_base = _bjt_intrinsic_base_node(el)
        Vb_rb = 0.0 if _is_ground(intrinsic_base) else x[node_to_idx[intrinsic_base]]
        Ve_rb = 0.0 if _is_ground(el.emitter) else x[node_to_idx[el.emitter]]
        Vc_rb = 0.0 if _is_ground(el.collector) else x[node_to_idx[el.collector]]
        base_resistance = _bjt_effective_base_resistance(el, Vb_rb, Ve_rb, Vc_rb)
        _stamp_g(G, node_to_idx, el.base, intrinsic_base, 1.0 / base_resistance)
        el = replace(el, base=intrinsic_base, Rb=0.0, Rbm=None, Irb=0.0)
    # --- Resolve node voltages at the current Newton iterate -----------------
    Vb = 0.0 if _is_ground(el.base) else x[node_to_idx[el.base]]
    Ve = 0.0 if _is_ground(el.emitter) else x[node_to_idx[el.emitter]]
    Vc = 0.0 if _is_ground(el.collector) else x[node_to_idx[el.collector]]

    # --- Controlling junction voltage (clamped to avoid exp overflow) --------
    Vjunc = min(Vb - Ve, 0.7) if el.polarity == "NPN" else min(Ve - Vb, 0.7)
    Vreverse = Vb - Vc if el.polarity == "NPN" else Vc - Vb

    forward_thermal_voltage = el.Vt * el.Nf
    exp_term = math.exp(Vjunc / forward_thermal_voltage)
    base_collector_current = el.Is * (exp_term - 1.0)
    base_gm = (el.Is / forward_thermal_voltage) * exp_term
    output_voltage = Vc - Ve if el.polarity == "NPN" else Ve - Vc
    early_factor = _bjt_early_factor(el, Vjunc, output_voltage)
    Ic0, gm, charge_factor = _bjt_forward_transport(
        el, base_collector_current, base_gm, early_factor
    )
    output_conductance = (
        0.0 if el.Vaf == 0.0 else base_collector_current / el.Vaf / charge_factor
    )
    leakage_current, leakage_conductance = _bjt_base_emitter_leakage(el, Vjunc)
    g_pi = base_gm / el.beta_f + leakage_conductance
    Ib0 = base_collector_current / el.beta_f + leakage_current

    Ieq_junc = Ib0 - g_pi * Vjunc      # junction Norton offset
    Ieq_coll = Ic0 - gm * Vjunc - output_conductance * output_voltage
    collector_leakage_current, collector_leakage_conductance = (
        _bjt_base_collector_leakage(el, Vreverse)
    )
    reverse_base_current, reverse_base_conductance = _bjt_reverse_base_current(
        el, Vreverse
    )
    base_collector_current = collector_leakage_current + reverse_base_current
    base_collector_conductance = (
        collector_leakage_conductance + reverse_base_conductance
    )
    Ieq_collector_leakage = (
        base_collector_current - base_collector_conductance * Vreverse
    )

    _stamp_g(G, node_to_idx, el.collector, el.emitter, output_conductance)
    _stamp_g(G, node_to_idx, el.base, el.collector, base_collector_conductance)

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
        if not _is_ground(el.base):
            b[node_to_idx[el.base]] -= Ieq_collector_leakage
        if not _is_ground(el.collector):
            b[node_to_idx[el.collector]] += Ieq_collector_leakage

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
        if not _is_ground(el.collector):
            b[node_to_idx[el.collector]] -= Ieq_collector_leakage
        if not _is_ground(el.base):
            b[node_to_idx[el.base]] += Ieq_collector_leakage


def _validate_bjt(el: BJT) -> None:
    if el.polarity not in {"NPN", "PNP"}:
        raise ValueError(f"{el.name}: BJT polarity must be NPN or PNP")
    if not math.isfinite(el.Is) or el.Is <= 0.0:
        raise ValueError(f"{el.name}: BJT saturation current must be finite and positive")
    if not math.isfinite(el.beta_f) or el.beta_f <= 0.0:
        raise ValueError(f"{el.name}: BJT forward beta must be finite and positive")
    if math.isnan(el.beta_r) or el.beta_r <= 0.0:
        raise ValueError(f"{el.name}: BJT reverse beta must be positive")
    if not math.isfinite(el.Vt) or el.Vt <= 0.0:
        raise ValueError(f"{el.name}: BJT thermal voltage must be finite and positive")
    if not math.isfinite(el.Cje) or el.Cje < 0.0:
        raise ValueError(f"{el.name}: BJT base-emitter capacitance must be finite and non-negative")
    if not math.isfinite(el.Cjc) or el.Cjc < 0.0:
        raise ValueError(f"{el.name}: BJT base-collector capacitance must be finite and non-negative")
    if not math.isfinite(el.Tf) or el.Tf < 0.0:
        raise ValueError(f"{el.name}: BJT forward transit time must be finite and non-negative")
    if not math.isfinite(el.Tr) or el.Tr < 0.0:
        raise ValueError(f"{el.name}: BJT reverse transit time must be finite and non-negative")
    if not math.isfinite(el.Xti):
        raise ValueError(
            f"{el.name}: BJT saturation-current temperature exponent must be finite"
        )
    if not math.isfinite(el.Xtb):
        raise ValueError(f"{el.name}: BJT beta temperature exponent must be finite")
    if not math.isfinite(el.Eg) or el.Eg <= 0.0:
        raise ValueError(f"{el.name}: BJT energy gap must be finite and positive")
    if not math.isfinite(el.Vaf) or el.Vaf < 0.0:
        raise ValueError(f"{el.name}: BJT forward Early voltage must be finite and non-negative")
    if not math.isfinite(el.Var) or el.Var < 0.0:
        raise ValueError(f"{el.name}: BJT reverse Early voltage must be finite and non-negative")
    if not math.isfinite(el.Ikf) or el.Ikf < 0.0:
        raise ValueError(
            f"{el.name}: BJT forward beta roll-off current must be finite and non-negative"
        )
    if not math.isfinite(el.Ikr) or el.Ikr < 0.0:
        raise ValueError(
            f"{el.name}: BJT reverse beta roll-off current must be finite and non-negative"
        )
    if el.Tnom is not None and (not math.isfinite(el.Tnom) or el.Tnom <= 0.0):
        raise ValueError(f"{el.name}: BJT nominal temperature must be finite and positive")
    if not math.isfinite(el.Kf) or el.Kf < 0.0:
        raise ValueError(f"{el.name}: BJT flicker noise coefficient must be finite and non-negative")
    if not math.isfinite(el.Af) or el.Af < 0.0:
        raise ValueError(f"{el.name}: BJT flicker noise exponent must be finite and non-negative")
    if not math.isfinite(el.Ptf) or el.Ptf < 0.0:
        raise ValueError(f"{el.name}: BJT forward excess phase must be finite and non-negative")
    if not math.isfinite(el.Xtf) or el.Xtf < 0.0:
        raise ValueError(
            f"{el.name}: BJT forward transit-time bias coefficient must be finite and non-negative"
        )
    if not math.isfinite(el.Itf) or el.Itf < 0.0:
        raise ValueError(
            f"{el.name}: BJT forward transit-time current must be finite and non-negative"
        )
    if not math.isfinite(el.Vtf) or el.Vtf < 0.0:
        raise ValueError(
            f"{el.name}: BJT forward transit-time voltage must be finite and non-negative"
        )
    if not math.isfinite(el.Re) or el.Re < 0.0:
        raise ValueError(
            f"{el.name}: BJT emitter resistance must be finite and non-negative"
        )
    if not math.isfinite(el.Rc) or el.Rc < 0.0:
        raise ValueError(
            f"{el.name}: BJT collector resistance must be finite and non-negative"
        )
    if not math.isfinite(el.Rb) or el.Rb < 0.0:
        raise ValueError(
            f"{el.name}: BJT base resistance must be finite and non-negative"
        )
    if el.Rbm is not None and (not math.isfinite(el.Rbm) or el.Rbm < 0.0):
        raise ValueError(
            f"{el.name}: BJT minimum base resistance must be finite and non-negative"
        )
    if not math.isfinite(el.Irb) or el.Irb < 0.0:
        raise ValueError(
            f"{el.name}: BJT base-resistance half-current must be finite and non-negative"
        )
    if not math.isfinite(el.Xcjc) or not 0.0 <= el.Xcjc <= 1.0:
        raise ValueError(
            f"{el.name}: BJT base-collector capacitance fraction must be between zero and one"
        )
    if not math.isfinite(el.Ise) or el.Ise < 0.0:
        raise ValueError(
            f"{el.name}: BJT base-emitter leakage saturation current must be finite and non-negative"
        )
    if not math.isfinite(el.Ne) or el.Ne <= 0.0:
        raise ValueError(
            f"{el.name}: BJT base-emitter leakage emission coefficient must be finite and positive"
        )
    if not math.isfinite(el.Isc) or el.Isc < 0.0:
        raise ValueError(
            f"{el.name}: BJT base-collector leakage saturation current must be finite and non-negative"
        )
    if not math.isfinite(el.Nc) or el.Nc <= 0.0:
        raise ValueError(
            f"{el.name}: BJT base-collector leakage emission coefficient must be finite and positive"
        )
    if not math.isfinite(el.Nf) or el.Nf <= 0.0:
        raise ValueError(f"{el.name}: BJT forward emission coefficient must be finite and positive")
    if not math.isfinite(el.Nr) or el.Nr <= 0.0:
        raise ValueError(f"{el.name}: BJT reverse emission coefficient must be finite and positive")
    if not math.isfinite(el.Vje) or el.Vje <= 0.0:
        raise ValueError(f"{el.name}: BJT base-emitter junction potential must be finite and positive")
    if not math.isfinite(el.Mje) or not 0.0 <= el.Mje < 1.0:
        raise ValueError(f"{el.name}: BJT base-emitter grading coefficient must be finite and in [0, 1)")
    if not math.isfinite(el.Vjc) or el.Vjc <= 0.0:
        raise ValueError(f"{el.name}: BJT base-collector junction potential must be finite and positive")
    if not math.isfinite(el.Mjc) or not 0.0 <= el.Mjc < 1.0:
        raise ValueError(f"{el.name}: BJT base-collector grading coefficient must be finite and in [0, 1)")
    if not math.isfinite(el.Fc) or not 0.0 <= el.Fc < 1.0:
        raise ValueError(f"{el.name}: BJT forward-bias depletion coefficient must be finite and in [0, 1)")


# ---------------------------------------------------------------------------
# Linear solver
# ---------------------------------------------------------------------------

_SPARSE_SOLVER_THRESHOLD = 30


class _LinearSolveFailure(ZeroDivisionError):
    def __init__(self, message: str, solver_profile: LinearSolverProfile):
        super().__init__(message)
        self.solver_profile = solver_profile


def _solve(A: list[list[float]], b: list[float]) -> list[float]:
    return _solve_with_profile(A, b)[0]


def _empty_solver_profile(matrix_size: int = 0) -> LinearSolverProfile:
    return LinearSolverProfile(
        matrix_size=matrix_size,
        solver=_real_solver_kind(matrix_size),
        backend="none",
        structural_nonzeros=0,
        density=0.0,
    )


def _real_matrix_nonzeros(A: list[list[float]]) -> int:
    return sum(1 for row in A for value in row if value != 0.0)


def _real_matrix_density(matrix_size: int, structural_nonzeros: int) -> float:
    if matrix_size == 0:
        return 0.0
    return structural_nonzeros / float(matrix_size * matrix_size)


def _real_solver_profile(
    A: list[list[float]],
    *,
    backend: str,
    fill_in_nonzeros: int = 0,
    fallback_reason: str | None = None,
) -> LinearSolverProfile:
    matrix_size = len(A)
    structural_nonzeros = _real_matrix_nonzeros(A)
    return LinearSolverProfile(
        matrix_size=matrix_size,
        solver=_real_solver_kind(matrix_size),
        backend=backend,
        structural_nonzeros=structural_nonzeros,
        density=_real_matrix_density(matrix_size, structural_nonzeros),
        fill_in_nonzeros=fill_in_nonzeros,
        fallback_reason=fallback_reason,
    )


def _solve_with_profile(
    A: list[list[float]], b: list[float]
) -> tuple[list[float], LinearSolverProfile]:
    if len(A) >= _SPARSE_SOLVER_THRESHOLD:
        return _solve_sparse_with_profile(A, b)
    profile = _real_solver_profile(A, backend="dense_gaussian")
    try:
        return _solve_dense(A, b), profile
    except ZeroDivisionError as exc:
        raise _LinearSolveFailure(str(exc), profile) from exc


def _solve_dense(A: list[list[float]], b: list[float]) -> list[float]:
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


def _solve_sparse(A: list[list[float]], b: list[float]) -> list[float]:
    return _solve_sparse_with_profile(A, b)[0]


def _solve_sparse_with_profile(
    A: list[list[float]],
    b: list[float],
    *,
    fallback_reason: str | None = None,
) -> tuple[list[float], LinearSolverProfile]:
    scipy_result = _solve_sparse_scipy(A, b)
    if scipy_result is not None:
        return scipy_result
    return _solve_sparse_native_with_profile(
        A,
        b,
        fallback_reason=fallback_reason or "scipy_unavailable",
    )


def _solve_sparse_scipy(
    A: list[list[float]], b: list[float]
) -> tuple[list[float], LinearSolverProfile] | None:
    try:
        from scipy.sparse import csc_matrix
        from scipy.sparse.linalg import splu
    except Exception:
        return None

    matrix_size = len(A)
    structural_nonzeros = _real_matrix_nonzeros(A)
    profile = LinearSolverProfile(
        matrix_size=matrix_size,
        solver=_real_solver_kind(matrix_size),
        backend="scipy_sparse_lu",
        structural_nonzeros=structural_nonzeros,
        density=_real_matrix_density(matrix_size, structural_nonzeros),
    )
    try:
        sparse_matrix = csc_matrix(A, dtype=float)
        factorization = splu(sparse_matrix)
        solution = factorization.solve(b)
        fill_in_nonzeros = max(
            0,
            int(factorization.L.nnz + factorization.U.nnz) - structural_nonzeros,
        )
        return (
            [float(value) for value in solution],
            replace(profile, fill_in_nonzeros=fill_in_nonzeros),
        )
    except Exception as exc:
        try:
            return _solve_sparse_native_with_profile(
                A,
                b,
                fallback_reason=f"scipy_sparse_lu:{type(exc).__name__}",
            )
        except ZeroDivisionError as native_exc:
            raise _LinearSolveFailure(str(native_exc), profile) from native_exc


def _solve_sparse_native_with_profile(
    A: list[list[float]],
    b: list[float],
    *,
    fallback_reason: str | None = None,
) -> tuple[list[float], LinearSolverProfile]:
    """Sparse-row Gaussian elimination with partial pivoting.

    The MNA matrix is assembled densely today, but each device stamp touches
    only a few columns. Converting rows to dictionaries keeps the large-circuit
    solve path from doing work on structural zeros.
    """

    n = len(A)
    if n == 0:
        return [], _empty_solver_profile()
    initial_nonzeros = _real_matrix_nonzeros(A)
    peak_nonzeros = initial_nonzeros
    profile = LinearSolverProfile(
        matrix_size=n,
        solver=_real_solver_kind(n),
        backend="native_sparse_gaussian",
        structural_nonzeros=initial_nonzeros,
        density=_real_matrix_density(n, initial_nonzeros),
        fallback_reason=fallback_reason,
    )
    rows = [
        {col: value for col, value in enumerate(row) if value != 0.0}
        for row in A
    ]
    rhs = list(b)

    for pivot_col in range(n):
        pivot_row = max(
            range(pivot_col, n),
            key=lambda row: abs(rows[row].get(pivot_col, 0.0)),
        )
        pivot_abs = abs(rows[pivot_row].get(pivot_col, 0.0))
        if pivot_abs < 1e-15:
            raise _LinearSolveFailure(f"singular matrix at row {pivot_col}", profile)

        rows[pivot_col], rows[pivot_row] = rows[pivot_row], rows[pivot_col]
        rhs[pivot_col], rhs[pivot_row] = rhs[pivot_row], rhs[pivot_col]

        pivot_value = rows[pivot_col][pivot_col]
        pivot_entries = [
            (col, value)
            for col, value in rows[pivot_col].items()
            if col > pivot_col
        ]
        for row_index in range(pivot_col + 1, n):
            value = rows[row_index].get(pivot_col, 0.0)
            if value == 0.0:
                continue
            factor = value / pivot_value
            rows[row_index].pop(pivot_col, None)
            for col, pivot_entry in pivot_entries:
                next_value = rows[row_index].get(col, 0.0) - factor * pivot_entry
                if abs(next_value) < 1e-15:
                    rows[row_index].pop(col, None)
                else:
                    rows[row_index][col] = next_value
            rhs[row_index] -= factor * rhs[pivot_col]
        peak_nonzeros = max(peak_nonzeros, sum(len(row) for row in rows))

    x = [0.0] * n
    for row_index in range(n - 1, -1, -1):
        diag = rows[row_index].get(row_index, 0.0)
        if abs(diag) < 1e-15:
            raise _LinearSolveFailure(f"singular matrix at row {row_index}", profile)
        total = rhs[row_index]
        for col, value in rows[row_index].items():
            if col > row_index:
                total -= value * x[col]
        x[row_index] = total / diag
    return x, replace(profile, fill_in_nonzeros=max(0, peak_nonzeros - initial_nonzeros))


# ---------------------------------------------------------------------------
# Transient analysis — companion-model builders and helpers
# ---------------------------------------------------------------------------


def _node_voltage(name: str, node_voltages: dict[str, float]) -> float:
    """Return the solved node voltage, 0 V for any ground alias."""
    return 0.0 if _is_ground(name) else node_voltages.get(name, 0.0)


TransmissionLineSample = tuple[float, float, float, float, float]


def _transmission_line_port_voltage(
    line: TransmissionLine,
    node_voltages: dict[str, float],
    first_port: bool,
) -> float:
    if first_port:
        return (
            _node_voltage(line.n1, node_voltages)
            - _node_voltage(line.n2, node_voltages)
        )
    return (
        _node_voltage(line.n3, node_voltages)
        - _node_voltage(line.n4, node_voltages)
    )


def _transmission_line_sample_at(
    samples: list[TransmissionLineSample],
    target_time: float,
) -> tuple[float, float, float, float]:
    if not samples or target_time < samples[0][0] - 1.0e-18:
        return (0.0, 0.0, 0.0, 0.0)
    if target_time <= samples[0][0]:
        _, v1, i1, v2, i2 = samples[0]
        return (v1, i1, v2, i2)
    for left, right in zip(samples, samples[1:], strict=False):
        if target_time <= right[0]:
            t0, v10, i10, v20, i20 = left
            t1, v11, i11, v21, i21 = right
            if t1 <= t0:
                return (v11, i11, v21, i21)
            alpha = (target_time - t0) / (t1 - t0)
            return (
                v10 + alpha * (v11 - v10),
                i10 + alpha * (i11 - i10),
                v20 + alpha * (v21 - v20),
                i20 + alpha * (i21 - i20),
            )
    _, v1, i1, v2, i2 = samples[-1]
    return (v1, i1, v2, i2)


def _transmission_line_history_terms(
    line: TransmissionLine,
    line_history: dict[str, list[TransmissionLineSample]],
    time: float,
) -> tuple[float, float]:
    _validate_transmission_line(line)
    v1_d, i1_d, v2_d, i2_d = _transmission_line_sample_at(
        line_history.get(line.name, []),
        time - line.delay,
    )
    z0 = line.characteristic_impedance
    return (v2_d / z0 + i2_d, v1_d / z0 + i1_d)


def _add_transmission_line_companion(
    circuit: Circuit,
    line: TransmissionLine,
    history_1: float,
    history_2: float,
) -> None:
    _validate_transmission_line(line)
    circuit.add(Resistor(
        name=f"_T_{line.name}_P1_R",
        n_plus=line.n1,
        n_minus=line.n2,
        resistance=line.characteristic_impedance,
    ))
    circuit.add(Resistor(
        name=f"_T_{line.name}_P2_R",
        n_plus=line.n3,
        n_minus=line.n4,
        resistance=line.characteristic_impedance,
    ))
    circuit.add(CurrentSource(
        name=f"_T_{line.name}_P1_I",
        n_plus=line.n1,
        n_minus=line.n2,
        current=-history_1,
    ))
    circuit.add(CurrentSource(
        name=f"_T_{line.name}_P2_I",
        n_plus=line.n3,
        n_minus=line.n4,
        current=-history_2,
    ))


def _transmission_line_branch_currents(
    circuit: Circuit,
    line_history: dict[str, list[TransmissionLineSample]],
    time: float,
    node_voltages: dict[str, float],
) -> dict[str, float]:
    currents: dict[str, float] = {}
    for el in circuit.elements:
        if not isinstance(el, TransmissionLine):
            continue
        history_1, history_2 = _transmission_line_history_terms(el, line_history, time)
        v1 = _transmission_line_port_voltage(el, node_voltages, True)
        v2 = _transmission_line_port_voltage(el, node_voltages, False)
        currents[f"I({el.name}:1)"] = v1 / el.characteristic_impedance - history_1
        currents[f"I({el.name}:2)"] = v2 / el.characteristic_impedance - history_2
    return currents


def _append_transmission_line_history(
    circuit: Circuit,
    line_history: dict[str, list[TransmissionLineSample]],
    time: float,
    node_voltages: dict[str, float],
) -> dict[str, float]:
    currents = _transmission_line_branch_currents(
        circuit,
        line_history,
        time,
        node_voltages,
    )
    for el in circuit.elements:
        if isinstance(el, TransmissionLine):
            v1 = _transmission_line_port_voltage(el, node_voltages, True)
            v2 = _transmission_line_port_voltage(el, node_voltages, False)
            line_history.setdefault(el.name, []).append((
                time,
                v1,
                currents[f"I({el.name}:1)"],
                v2,
                currents[f"I({el.name}:2)"],
            ))
    return currents


def _build_transient_companions(
    circuit: Circuit,
    h: float,
    method: str,
    cap_voltages: dict[str, float],
    cap_voltages_older: dict[str, float],
    cap_currents: dict[str, float],
    ind_currents: dict[str, float],
    ind_currents_older: dict[str, float],
    ind_voltages: dict[str, float],
    line_history: dict[str, list[TransmissionLineSample]],
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
        ``"trap"`` (trapezoidal), ``"euler"`` (backward Euler), or
        ``"gear2"`` (BDF2).
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
    coupled_names = _coupled_inductor_names(circuit)

    # Build the base element list, substituting time-varying source values.
    # VoltageSource / CurrentSource elements that carry a waveform callable
    # are replaced here with a plain static copy at the current time t.
    # Capacitors and Inductors are always excluded (they get companion models).
    base_elements: list = []
    for e in circuit.elements:
        if isinstance(e, (Capacitor, Inductor, TransmissionLine)):
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
        if isinstance(el, TransmissionLine):
            history_1, history_2 = _transmission_line_history_terms(
                el,
                line_history,
                t,
            )
            _add_transmission_line_companion(aug, el, history_1, history_2)

    inductors = _inductor_by_name(circuit)
    for el in circuit.elements:
        if isinstance(el, MutualInductor):
            primary, secondary, mutual_inductance = _validate_mutual_inductor(
                el,
                inductors,
            )
            determinant = primary.inductance * secondary.inductance - mutual_inductance**2
            if determinant <= 0.0 or not math.isfinite(determinant):
                raise ValueError(f"{el.name}: coupled inductance matrix is singular")

            if method == "trap":
                scale = h / (2.0 * determinant)
                v_primary_prev = ind_voltages.get(primary.name, 0.0)
                v_secondary_prev = ind_voltages.get(secondary.name, 0.0)
            else:
                scale = h / determinant
                v_primary_prev = 0.0
                v_secondary_prev = 0.0
            g11 = secondary.inductance * scale
            g12 = -mutual_inductance * scale
            g22 = primary.inductance * scale
            i_primary_eq = (
                ind_currents.get(primary.name, primary.initial_current)
                + g11 * v_primary_prev
                + g12 * v_secondary_prev
            )
            i_secondary_eq = (
                ind_currents.get(secondary.name, secondary.initial_current)
                + g12 * v_primary_prev
                + g22 * v_secondary_prev
            )

            aug.elements.append(Resistor(
                name=f"_K_{el.name}_{primary.name}_R",
                n_plus=primary.n_plus,
                n_minus=primary.n_minus,
                resistance=1.0 / g11,
            ))
            aug.elements.append(Resistor(
                name=f"_K_{el.name}_{secondary.name}_R",
                n_plus=secondary.n_plus,
                n_minus=secondary.n_minus,
                resistance=1.0 / g22,
            ))
            aug.elements.append(VCCS(
                name=f"_K_{el.name}_{primary.name}_from_{secondary.name}",
                n_plus=primary.n_plus,
                n_minus=primary.n_minus,
                ctrl_plus=secondary.n_plus,
                ctrl_minus=secondary.n_minus,
                gm=g12,
            ))
            aug.elements.append(VCCS(
                name=f"_K_{el.name}_{secondary.name}_from_{primary.name}",
                n_plus=secondary.n_plus,
                n_minus=secondary.n_minus,
                ctrl_plus=primary.n_plus,
                ctrl_minus=primary.n_minus,
                gm=g12,
            ))
            aug.elements.append(CurrentSource(
                name=f"_K_{el.name}_{primary.name}_I",
                n_plus=primary.n_plus,
                n_minus=primary.n_minus,
                current=i_primary_eq,
            ))
            aug.elements.append(CurrentSource(
                name=f"_K_{el.name}_{secondary.name}_I",
                n_plus=secondary.n_plus,
                n_minus=secondary.n_minus,
                current=i_secondary_eq,
            ))

    for el in circuit.elements:
        # ---- Capacitor companion ------------------------------------------
        if isinstance(el, Capacitor):
            v_prev = cap_voltages.get(el.name, el.initial_voltage)
            if method == "trap":
                g_eq = 2.0 * el.capacitance / h
                I_eq = g_eq * v_prev + cap_currents.get(el.name, 0.0)
            elif method == "gear2":
                v_older = cap_voltages_older.get(el.name, el.initial_voltage)
                g_eq = 3.0 * el.capacitance / (2.0 * h)
                I_eq = el.capacitance * (4.0 * v_prev - v_older) / (2.0 * h)
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

        # ---- Diode model-card charge companion ----------------------------
        elif isinstance(el, Diode) and _diode_has_charge_storage(el):
            state_name = _diode_charge_state_name(el)
            v_prev = cap_voltages.get(state_name, 0.0)
            capacitance = _diode_dynamic_capacitance(el, v_prev)
            if capacitance > 0.0:
                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_eq = g_eq * v_prev + cap_currents.get(state_name, 0.0)
                elif method == "gear2":
                    v_older = cap_voltages_older.get(state_name, v_prev)
                    g_eq = 3.0 * capacitance / (2.0 * h)
                    I_eq = capacitance * (4.0 * v_prev - v_older) / (2.0 * h)
                else:
                    g_eq = capacitance / h
                    I_eq = g_eq * v_prev

                aug.elements.append(Resistor(
                    name=f"_D_{el.name}_charge_R",
                    n_plus=el.anode,
                    n_minus=el.cathode,
                    resistance=1.0 / g_eq,
                ))
                aug.elements.append(CurrentSource(
                    name=f"_D_{el.name}_charge_I",
                    n_plus=el.cathode,
                    n_minus=el.anode,
                    current=I_eq,
                ))

        # ---- JFET model-card charge companions -----------------------------
        elif isinstance(el, JFET):
            for state_name, n_plus, n_minus, capacitance in _jfet_charge_state_specs(el):
                v_prev = cap_voltages.get(state_name, 0.0)
                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_eq = g_eq * v_prev + cap_currents.get(state_name, 0.0)
                elif method == "gear2":
                    v_older = cap_voltages_older.get(state_name, v_prev)
                    g_eq = 3.0 * capacitance / (2.0 * h)
                    I_eq = capacitance * (4.0 * v_prev - v_older) / (2.0 * h)
                else:
                    g_eq = capacitance / h
                    I_eq = g_eq * v_prev

                aug.elements.append(Resistor(
                    name=f"{state_name}_R",
                    n_plus=n_plus,
                    n_minus=n_minus,
                    resistance=1.0 / g_eq,
                ))
                aug.elements.append(CurrentSource(
                    name=f"{state_name}_I",
                    n_plus=n_minus,
                    n_minus=n_plus,
                    current=I_eq,
                ))

        # ---- MOS Level-1 overlap charge companions -------------------------
        elif isinstance(el, Mosfet):
            for state_name, n_plus, n_minus, capacitance in _mosfet_charge_state_specs(el):
                v_prev = cap_voltages.get(state_name, 0.0)
                capacitance = _mosfet_charge_dynamic_capacitance(
                    el,
                    state_name,
                    capacitance,
                    v_prev,
                )
                if capacitance <= 0.0:
                    continue
                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_eq = g_eq * v_prev + cap_currents.get(state_name, 0.0)
                elif method == "gear2":
                    v_older = cap_voltages_older.get(state_name, v_prev)
                    g_eq = 3.0 * capacitance / (2.0 * h)
                    I_eq = capacitance * (4.0 * v_prev - v_older) / (2.0 * h)
                else:
                    g_eq = capacitance / h
                    I_eq = g_eq * v_prev

                aug.elements.append(Resistor(
                    name=f"{state_name}_R",
                    n_plus=n_plus,
                    n_minus=n_minus,
                    resistance=1.0 / g_eq,
                ))
                aug.elements.append(CurrentSource(
                    name=f"{state_name}_I",
                    n_plus=n_minus,
                    n_minus=n_plus,
                    current=I_eq,
                ))

        # ---- BJT model-card charge companions ------------------------------
        elif isinstance(el, BJT):
            reverse_junction_voltage = cap_voltages.get(
                _bjt_base_collector_charge_state_name(el),
                0.0,
            )
            for state_name, n_plus, n_minus, state_kind in _bjt_charge_state_specs(el):
                v_prev = cap_voltages.get(state_name, 0.0)
                capacitance = _bjt_charge_dynamic_capacitance(
                    el,
                    state_kind,
                    v_prev,
                    reverse_junction_voltage,
                )
                if capacitance <= 0.0:
                    continue
                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_eq = g_eq * v_prev + cap_currents.get(state_name, 0.0)
                elif method == "gear2":
                    v_older = cap_voltages_older.get(state_name, v_prev)
                    g_eq = 3.0 * capacitance / (2.0 * h)
                    I_eq = capacitance * (4.0 * v_prev - v_older) / (2.0 * h)
                else:
                    g_eq = capacitance / h
                    I_eq = g_eq * v_prev

                aug.elements.append(Resistor(
                    name=f"{state_name}_R",
                    n_plus=n_plus,
                    n_minus=n_minus,
                    resistance=1.0 / g_eq,
                ))
                aug.elements.append(CurrentSource(
                    name=f"{state_name}_I",
                    n_plus=n_minus,
                    n_minus=n_plus,
                    current=I_eq,
                ))

        # ---- Inductor companion ------------------------------------------
        elif isinstance(el, Inductor) and el.name not in coupled_names:
            I_prev = ind_currents.get(el.name, 0.0)
            if method == "trap":
                g_eq = h / (2.0 * el.inductance)
                V_prev = ind_voltages.get(el.name, 0.0)
                I_eq = I_prev + g_eq * V_prev
            elif method == "gear2":
                I_older = ind_currents_older.get(el.name, el.initial_current)
                g_eq = 2.0 * h / (3.0 * el.inductance)
                I_eq = (4.0 * I_prev - I_older) / 3.0
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
    cap_voltages_older: dict[str, float],
    cap_currents: dict[str, float],
    ind_currents: dict[str, float],
    ind_currents_older: dict[str, float],
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
    coupled_names = _coupled_inductor_names(circuit)
    inductors = _inductor_by_name(circuit)
    for el in circuit.elements:
        if isinstance(el, MutualInductor):
            primary, secondary, mutual_inductance = _validate_mutual_inductor(
                el,
                inductors,
            )
            determinant = primary.inductance * secondary.inductance - mutual_inductance**2
            if determinant <= 0.0 or not math.isfinite(determinant):
                raise ValueError(f"{el.name}: coupled inductance matrix is singular")

            v_primary = (
                _node_voltage(primary.n_plus, op.node_voltages)
                - _node_voltage(primary.n_minus, op.node_voltages)
            )
            v_secondary = (
                _node_voltage(secondary.n_plus, op.node_voltages)
                - _node_voltage(secondary.n_minus, op.node_voltages)
            )
            if method == "trap":
                scale = h / (2.0 * determinant)
                v_primary_prev = ind_voltages.get(primary.name, 0.0)
                v_secondary_prev = ind_voltages.get(secondary.name, 0.0)
            else:
                scale = h / determinant
                v_primary_prev = 0.0
                v_secondary_prev = 0.0
            g11 = secondary.inductance * scale
            g12 = -mutual_inductance * scale
            g22 = primary.inductance * scale
            i_primary_eq = (
                ind_currents.get(primary.name, primary.initial_current)
                + g11 * v_primary_prev
                + g12 * v_secondary_prev
            )
            i_secondary_eq = (
                ind_currents.get(secondary.name, secondary.initial_current)
                + g12 * v_primary_prev
                + g22 * v_secondary_prev
            )
            ind_currents[primary.name] = g11 * v_primary + g12 * v_secondary + i_primary_eq
            ind_currents[secondary.name] = g12 * v_primary + g22 * v_secondary + i_secondary_eq
            ind_voltages[primary.name] = v_primary
            ind_voltages[secondary.name] = v_secondary

    for el in circuit.elements:
        if isinstance(el, Capacitor):
            v_plus = _node_voltage(el.n_plus, op.node_voltages)
            v_minus = _node_voltage(el.n_minus, op.node_voltages)
            v_new = v_plus - v_minus
            v_prev = cap_voltages.get(el.name, el.initial_voltage)
            v_older = cap_voltages_older.get(el.name, el.initial_voltage)

            if method == "trap":
                g_eq = 2.0 * el.capacitance / h
                I_prev = cap_currents.get(el.name, 0.0)
                cap_currents[el.name] = g_eq * (v_new - v_prev) - I_prev
            elif method == "gear2":
                cap_currents[el.name] = (
                    el.capacitance * (3.0 * v_new - 4.0 * v_prev + v_older)
                    / (2.0 * h)
                )
            else:
                g_eq = el.capacitance / h
                cap_currents[el.name] = g_eq * (v_new - v_prev)

            cap_voltages_older[el.name] = v_prev
            cap_voltages[el.name] = v_new

        elif isinstance(el, Diode) and _diode_has_charge_storage(el):
            state_name = _diode_charge_state_name(el)
            v_new = _diode_charge_voltage(el, op.node_voltages)
            v_prev = cap_voltages.get(state_name, v_new)
            v_older = cap_voltages_older.get(state_name, v_prev)
            capacitance = _diode_dynamic_capacitance(el, v_prev)

            if capacitance > 0.0:
                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_prev = cap_currents.get(state_name, 0.0)
                    cap_currents[state_name] = g_eq * (v_new - v_prev) - I_prev
                elif method == "gear2":
                    cap_currents[state_name] = (
                        capacitance * (3.0 * v_new - 4.0 * v_prev + v_older)
                        / (2.0 * h)
                    )
                else:
                    g_eq = capacitance / h
                    cap_currents[state_name] = g_eq * (v_new - v_prev)

            cap_voltages_older[state_name] = v_prev
            cap_voltages[state_name] = v_new

        elif isinstance(el, JFET):
            for state_name, n_plus, n_minus, capacitance in _jfet_charge_state_specs(el):
                v_new = _jfet_charge_state_voltage(n_plus, n_minus, op.node_voltages)
                v_prev = cap_voltages.get(state_name, v_new)
                v_older = cap_voltages_older.get(state_name, v_prev)

                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_prev = cap_currents.get(state_name, 0.0)
                    cap_currents[state_name] = g_eq * (v_new - v_prev) - I_prev
                elif method == "gear2":
                    cap_currents[state_name] = (
                        capacitance * (3.0 * v_new - 4.0 * v_prev + v_older)
                        / (2.0 * h)
                    )
                else:
                    g_eq = capacitance / h
                    cap_currents[state_name] = g_eq * (v_new - v_prev)

                cap_voltages_older[state_name] = v_prev
                cap_voltages[state_name] = v_new

        elif isinstance(el, Mosfet):
            for state_name, n_plus, n_minus, capacitance in _mosfet_charge_state_specs(el):
                v_new = _mosfet_charge_state_voltage(n_plus, n_minus, op.node_voltages)
                v_prev = cap_voltages.get(state_name, v_new)
                v_older = cap_voltages_older.get(state_name, v_prev)
                capacitance = _mosfet_charge_dynamic_capacitance(
                    el,
                    state_name,
                    capacitance,
                    v_prev,
                )

                if capacitance <= 0.0:
                    cap_voltages_older[state_name] = v_prev
                    cap_voltages[state_name] = v_new
                    continue

                if method == "trap":
                    g_eq = 2.0 * capacitance / h
                    I_prev = cap_currents.get(state_name, 0.0)
                    cap_currents[state_name] = g_eq * (v_new - v_prev) - I_prev
                elif method == "gear2":
                    cap_currents[state_name] = (
                        capacitance * (3.0 * v_new - 4.0 * v_prev + v_older)
                        / (2.0 * h)
                    )
                else:
                    g_eq = capacitance / h
                    cap_currents[state_name] = g_eq * (v_new - v_prev)

                cap_voltages_older[state_name] = v_prev
                cap_voltages[state_name] = v_new

        elif isinstance(el, BJT):
            reverse_junction_voltage = cap_voltages.get(
                _bjt_base_collector_charge_state_name(el),
                0.0,
            )
            for state_name, n_plus, n_minus, state_kind in _bjt_charge_state_specs(el):
                v_new = _bjt_charge_state_voltage(n_plus, n_minus, op.node_voltages)
                v_prev = cap_voltages.get(state_name, v_new)
                v_older = cap_voltages_older.get(state_name, v_prev)
                capacitance = _bjt_charge_dynamic_capacitance(
                    el,
                    state_kind,
                    v_prev,
                    reverse_junction_voltage,
                )

                if capacitance > 0.0:
                    if method == "trap":
                        g_eq = 2.0 * capacitance / h
                        I_prev = cap_currents.get(state_name, 0.0)
                        cap_currents[state_name] = g_eq * (v_new - v_prev) - I_prev
                    elif method == "gear2":
                        cap_currents[state_name] = (
                            capacitance * (3.0 * v_new - 4.0 * v_prev + v_older)
                            / (2.0 * h)
                        )
                    else:
                        g_eq = capacitance / h
                        cap_currents[state_name] = g_eq * (v_new - v_prev)

                cap_voltages_older[state_name] = v_prev
                cap_voltages[state_name] = v_new

        elif isinstance(el, Inductor) and el.name not in coupled_names:
            v_plus = _node_voltage(el.n_plus, op.node_voltages)
            v_minus = _node_voltage(el.n_minus, op.node_voltages)
            v_new = v_plus - v_minus
            I_prev = ind_currents.get(el.name, 0.0)
            I_older = ind_currents_older.get(el.name, el.initial_current)
            V_prev = ind_voltages.get(el.name, 0.0)

            if method == "trap":
                g_eq = h / (2.0 * el.inductance)
                I_eq = I_prev + g_eq * V_prev
                ind_currents[el.name] = g_eq * v_new + I_eq
            elif method == "gear2":
                ind_currents[el.name] = (
                    2.0 * h * v_new / (3.0 * el.inductance)
                    + (4.0 * I_prev - I_older) / 3.0
                )
            else:
                g_eq = h / el.inductance
                ind_currents[el.name] = g_eq * v_new + I_prev

            ind_currents_older[el.name] = I_prev
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
        elif isinstance(el, Diode) and _diode_has_charge_storage(el):
            state_name = _diode_charge_state_name(el)
            v1 = cap_voltages_now.get(state_name, 0.0)
            v0 = cap_voltages_prev.get(state_name, 0.0)
            vm1 = cap_voltages_prev2.get(state_name, 0.0)
            lte_c = abs(v1 - 2.0 * v0 + vm1) / 2.0
            if lte_c > max_lte:
                max_lte = lte_c
        elif isinstance(el, JFET):
            for state_name, _, _, _ in _jfet_charge_state_specs(el):
                v1 = cap_voltages_now.get(state_name, 0.0)
                v0 = cap_voltages_prev.get(state_name, 0.0)
                vm1 = cap_voltages_prev2.get(state_name, 0.0)
                lte_c = abs(v1 - 2.0 * v0 + vm1) / 2.0
                if lte_c > max_lte:
                    max_lte = lte_c
        elif isinstance(el, Mosfet):
            for state_name, _, _, _ in _mosfet_charge_state_specs(el):
                v1 = cap_voltages_now.get(state_name, 0.0)
                v0 = cap_voltages_prev.get(state_name, 0.0)
                vm1 = cap_voltages_prev2.get(state_name, 0.0)
                lte_c = abs(v1 - 2.0 * v0 + vm1) / 2.0
                if lte_c > max_lte:
                    max_lte = lte_c
        elif isinstance(el, BJT):
            for state_name, _, _, _ in _bjt_charge_state_specs(el):
                v1 = cap_voltages_now.get(state_name, 0.0)
                v0 = cap_voltages_prev.get(state_name, 0.0)
                vm1 = cap_voltages_prev2.get(state_name, 0.0)
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
    """Transient (time-domain) analysis with trapezoidal, backward-Euler, or Gear-2 integration.

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
        ``"euler"`` (backward Euler — 1st-order, unconditionally stable), or
        ``"gear2"`` (BDF2 after one backward-Euler bootstrap step).
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
        if isinstance(e, (Capacitor, Inductor, TransmissionLine)):
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
            if el.initial_current != 0.0:
                init_circuit.add(CurrentSource(
                    name=f"_L_{el.name}_I0",
                    n_plus=el.n_plus,
                    n_minus=el.n_minus,
                    current=el.initial_current,
                ))
        elif isinstance(el, TransmissionLine):
            _add_transmission_line_companion(init_circuit, el, 0.0, 0.0)
    op = dc_op(init_circuit, max_iterations=max_iterations, tol=tol)
    if not op.converged:
        return TransientResult(points=[], converged=False, method=method)

    line_history: dict[str, list[TransmissionLineSample]] = {}
    line_branch_currents = _append_transmission_line_history(
        circuit,
        line_history,
        0.0,
        op.node_voltages,
    )
    initial_branch_currents = dict(op.branch_currents)
    initial_branch_currents.update(line_branch_currents)
    for el in circuit.elements:
        if isinstance(el, Inductor):
            initial_branch_currents[el.name] = el.initial_current
    points: list[TransientPoint] = [
        TransientPoint(
            time=0.0,
            node_voltages=dict(op.node_voltages),
            branch_currents=initial_branch_currents,
        )
    ]

    # ---- Reactive element history ------------------------------------------
    cap_voltages: dict[str, float] = {
        el.name: el.initial_voltage
        for el in circuit.elements if isinstance(el, Capacitor)
    }
    for el in circuit.elements:
        if isinstance(el, Diode) and _diode_has_charge_storage(el):
            cap_voltages[_diode_charge_state_name(el)] = _diode_charge_voltage(el, op.node_voltages)
        elif isinstance(el, JFET):
            for state_name, n_plus, n_minus, _ in _jfet_charge_state_specs(el):
                cap_voltages[state_name] = _jfet_charge_state_voltage(
                    n_plus,
                    n_minus,
                    op.node_voltages,
                )
        elif isinstance(el, Mosfet):
            for state_name, n_plus, n_minus, _ in _mosfet_charge_state_specs(el):
                cap_voltages[state_name] = _mosfet_charge_state_voltage(
                    n_plus,
                    n_minus,
                    op.node_voltages,
                )
        elif isinstance(el, BJT):
            for state_name, n_plus, n_minus, _ in _bjt_charge_state_specs(el):
                cap_voltages[state_name] = _bjt_charge_state_voltage(
                    n_plus,
                    n_minus,
                    op.node_voltages,
                )
    cap_currents: dict[str, float] = {
        el.name: 0.0
        for el in circuit.elements if isinstance(el, Capacitor)
    }
    for el in circuit.elements:
        if isinstance(el, Diode) and _diode_has_charge_storage(el):
            cap_currents[_diode_charge_state_name(el)] = 0.0
        elif isinstance(el, JFET):
            for state_name, _, _, _ in _jfet_charge_state_specs(el):
                cap_currents[state_name] = 0.0
        elif isinstance(el, Mosfet):
            for state_name, _, _, _ in _mosfet_charge_state_specs(el):
                cap_currents[state_name] = 0.0
        elif isinstance(el, BJT):
            for state_name, _, _, _ in _bjt_charge_state_specs(el):
                cap_currents[state_name] = 0.0
    cap_voltages_older: dict[str, float] = dict(cap_voltages)
    ind_currents: dict[str, float] = {
        el.name: el.initial_current
        for el in circuit.elements if isinstance(el, Inductor)
    }
    ind_currents_older: dict[str, float] = dict(ind_currents)
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
        step_method = "euler" if method == "gear2" and len(points) < 2 else method
        # Clamp last step to land exactly on t_stop.
        h = min(h, t_stop - (t - h) + 1e-12 * t_stop)
        if h < _min_step:
            h = _min_step  # floor; stop shrinking

        aug = _build_transient_companions(
            circuit, h, step_method,
            cap_voltages, cap_voltages_older, cap_currents,
            ind_currents, ind_currents_older, ind_voltages,
            line_history,
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
                elif isinstance(el, Diode) and _diode_has_charge_storage(el):
                    cap_voltages_new[_diode_charge_state_name(el)] = _diode_charge_voltage(
                        el,
                        op.node_voltages,
                    )
                elif isinstance(el, JFET):
                    for state_name, n_plus, n_minus, _ in _jfet_charge_state_specs(el):
                        cap_voltages_new[state_name] = _jfet_charge_state_voltage(
                            n_plus,
                            n_minus,
                            op.node_voltages,
                        )
                elif isinstance(el, Mosfet):
                    for state_name, n_plus, n_minus, _ in _mosfet_charge_state_specs(el):
                        cap_voltages_new[state_name] = _mosfet_charge_state_voltage(
                            n_plus,
                            n_minus,
                            op.node_voltages,
                        )
                elif isinstance(el, BJT):
                    for state_name, n_plus, n_minus, _ in _bjt_charge_state_specs(el):
                        cap_voltages_new[state_name] = _bjt_charge_state_voltage(
                            n_plus,
                            n_minus,
                            op.node_voltages,
                        )

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
                circuit, h, step_method, op,
                cap_voltages, cap_voltages_older, cap_currents,
                ind_currents, ind_currents_older, ind_voltages,
            )
            line_branch_currents = _append_transmission_line_history(
                circuit,
                line_history,
                t_actual,
                op.node_voltages,
            )
            branch_currents = dict(op.branch_currents)
            branch_currents.update(line_branch_currents)
            branch_currents.update(ind_currents)
            cap_voltages_prev = dict(cap_voltages)
            points.append(TransientPoint(
                time=t_actual,
                node_voltages=dict(op.node_voltages),
                branch_currents=branch_currents,
            ))

            if lte < tol_lte / 8.0:
                h = min(h * 2.0, _max_step)
        else:
            # Non-adaptive or backward Euler or not enough history yet.
            _update_reactive_state(
                circuit, h, step_method, op,
                cap_voltages, cap_voltages_older, cap_currents,
                ind_currents, ind_currents_older, ind_voltages,
            )
            line_branch_currents = _append_transmission_line_history(
                circuit,
                line_history,
                t,
                op.node_voltages,
            )
            branch_currents = dict(op.branch_currents)
            branch_currents.update(line_branch_currents)
            branch_currents.update(ind_currents)
            cap_voltages_prev = dict(cap_voltages)
            points.append(TransientPoint(
                time=t,
                node_voltages=dict(op.node_voltages),
                branch_currents=branch_currents,
            ))

        t += h

    return TransientResult(points=points, converged=True,
                           method=method, steps_rejected=steps_rejected)


def transient_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    t_stop: float,
    t_step: float,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerTransientResult:
    """Run fixed-step transient analysis at each named corner."""
    return CornerTransientResult(
        points=[
            CornerTransientPoint(
                corner_name=corner.name,
                points=transient(
                    _circuit_with_corner(circuit, corner),
                    t_stop=t_stop,
                    t_step=t_step,
                    method=method,
                    max_iterations=max_iterations,
                    tol=tol,
                ).points,
            )
            for corner in corners
        ]
    )


def transient_adaptive_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    t_stop: float,
    t_step: float,
    method: str = "trap",
    tol_lte: float = 1e-4,
    min_step: float | None = None,
    max_step: float | None = None,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerAdaptiveTransientResult:
    """Run LTE-adaptive transient analysis at each named corner."""
    return CornerAdaptiveTransientResult(
        points=[
            CornerAdaptiveTransientPoint(
                corner_name=corner.name,
                result=transient(
                    _circuit_with_corner(circuit, corner),
                    t_stop=t_stop,
                    t_step=t_step,
                    method=method,
                    adaptive=True,
                    tol_lte=tol_lte,
                    min_step=min_step,
                    max_step=max_step,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ]
    )


_DIGITAL_BRIDGE_TIME_EPSILON = 1.0e-18


def digital_events_to_pwl_waveform(
    events: list[DigitalEvent],
    levels: DigitalLogicLevels,
) -> PwlWaveform:
    """Convert hardware-VM digital events into a SPICE PWL waveform."""
    _validate_digital_logic_levels(levels, "digital_events")
    if not events:
        raise ValueError("digital_events: at least one digital event is required")

    previous_time = -math.inf
    for event in events:
        _validate_digital_event_time(event.time_seconds, previous_time, "digital_events")
        _normalize_digital_state(event.state)
        previous_time = event.time_seconds

    points: list[tuple[float, float]] = []
    current_state = _normalize_digital_state(events[0].state)
    points.append((events[0].time_seconds, levels.voltage_for(current_state)))

    for event in events[1:]:
        event_state = _normalize_digital_state(event.state)
        if event_state == current_state:
            continue
        start_time = event.time_seconds
        end_time = start_time + levels.transition_seconds
        last_time = points[-1][0]
        if start_time <= last_time:
            raise ValueError("digital_events: digital transition overlaps the previous transition")
        points.append((start_time, levels.voltage_for(current_state)))
        points.append((end_time, levels.voltage_for(event_state)))
        current_state = event_state

    if len(points) == 1:
        points.append((points[0][0] + levels.transition_seconds, levels.voltage_for(current_state)))

    return PwlWaveform(tuple(points))


def digital_events_to_voltage_source(
    name: str,
    positive: str,
    negative: str,
    events: list[DigitalEvent],
    levels: DigitalLogicLevels,
) -> VoltageSource:
    """Create a voltage source that drives a digital event stream into SPICE."""
    if not events:
        raise ValueError("digital_events: at least one digital event is required")
    initial_voltage = levels.voltage_for(events[0].state)
    return VoltageSource(
        name,
        positive,
        negative,
        initial_voltage,
        digital_events_to_pwl_waveform(events, levels),
    )


def digital_event_streams_to_voltage_sources(
    streams: list[DigitalEventStream],
    negative: str,
    levels: DigitalLogicLevels,
) -> list[VoltageSource]:
    """Convert named digital event streams into SPICE voltage sources."""
    negative_node = negative.strip()
    if not negative_node:
        raise ValueError("digital_event_streams: digital event stream negative node must not be empty")
    seen_signal_names: set[str] = set()
    sources: list[VoltageSource] = []
    for stream in streams:
        signal_name = _validate_digital_event_stream_name(stream, seen_signal_names)
        sources.append(
            digital_events_to_voltage_source(
                f"V{signal_name}",
                signal_name,
                negative_node,
                stream.events,
                levels,
            )
        )
    return sources


def digital_event_streams_to_bridge_schedule(
    streams: list[DigitalEventStream],
    levels: DigitalLogicLevels,
) -> DigitalBridgeSchedule:
    """Derive deterministic VM breakpoints from digital input streams."""
    _validate_digital_logic_levels(levels, "digital_bridge_schedule")
    seen_signal_names: set[str] = set()
    breakpoints: list[float] = []
    stop_time = 0.0
    for stream in streams:
        _validate_digital_event_stream_name(stream, seen_signal_names)
        digital_events_to_pwl_waveform(stream.events, levels)
        current_state = _normalize_digital_state(stream.events[0].state)
        for index, event in enumerate(stream.events):
            event_state = _normalize_digital_state(event.state)
            breakpoints.append(event.time_seconds)
            stop_time = max(stop_time, event.time_seconds)
            if index > 0 and event_state != current_state:
                transition_end = event.time_seconds + levels.transition_seconds
                breakpoints.append(transition_end)
                stop_time = max(stop_time, transition_end)
                current_state = event_state

    breakpoints.sort()
    deduped: list[float] = []
    for breakpoint in breakpoints:
        if not deduped or abs(breakpoint - deduped[-1]) > _DIGITAL_BRIDGE_TIME_EPSILON:
            deduped.append(breakpoint)
    return DigitalBridgeSchedule(stop_time=stop_time, breakpoints=deduped)


def transient_with_digital_event_streams(
    circuit: Circuit,
    input_streams: list[DigitalEventStream],
    negative: str,
    levels: DigitalLogicLevels,
    *,
    t_stop: float,
    t_step: float,
    output_probes: list[tuple[str, str]],
    thresholds: DigitalThresholds,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> DigitalTransientBridgeResult:
    """Run transient analysis with digital VM streams driving analog sources."""
    bridged = _circuit_with_extra_voltage_sources(
        circuit,
        digital_event_streams_to_voltage_sources(input_streams, negative, levels),
    )
    result = transient(
        bridged,
        t_stop=t_stop,
        t_step=t_step,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
    )
    return DigitalTransientBridgeResult(
        points=result.points,
        output_streams=sample_transient_probes_as_digital_event_streams(
            result.points,
            output_probes,
            thresholds,
        ),
    )


def transient_with_digital_event_streams_corners(
    circuit: Circuit,
    input_streams: list[DigitalEventStream],
    negative: str,
    levels: DigitalLogicLevels,
    corners: list[CornerSpec],
    *,
    t_stop: float,
    t_step: float,
    output_probes: list[tuple[str, str]],
    thresholds: DigitalThresholds,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerDigitalTransientBridgeResult:
    """Run the digital bridge across named transient corners."""
    return CornerDigitalTransientBridgeResult(
        points=[
            CornerDigitalTransientBridgePoint(
                corner_name=corner.name,
                result=transient_with_digital_event_streams(
                    _circuit_with_corner(circuit, corner),
                    input_streams,
                    negative,
                    levels,
                    t_stop=t_stop,
                    t_step=t_step,
                    output_probes=output_probes,
                    thresholds=thresholds,
                    method=method,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ]
    )


def transient_adaptive_with_digital_event_streams(
    circuit: Circuit,
    input_streams: list[DigitalEventStream],
    negative: str,
    levels: DigitalLogicLevels,
    *,
    t_stop: float,
    t_step: float,
    output_probes: list[tuple[str, str]],
    thresholds: DigitalThresholds,
    method: str = "trap",
    tol_lte: float = 1e-4,
    min_step: float | None = None,
    max_step: float | None = None,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> AdaptiveDigitalTransientBridgeResult:
    """Run adaptive transient analysis with digital VM streams driving SPICE."""
    bridged = _circuit_with_extra_voltage_sources(
        circuit,
        digital_event_streams_to_voltage_sources(input_streams, negative, levels),
    )
    result = transient(
        bridged,
        t_stop=t_stop,
        t_step=t_step,
        method=method,
        adaptive=True,
        tol_lte=tol_lte,
        min_step=min_step,
        max_step=max_step,
        max_iterations=max_iterations,
        tol=tol,
    )
    return AdaptiveDigitalTransientBridgeResult(
        result=result,
        output_streams=sample_transient_probes_as_digital_event_streams(
            result.points,
            output_probes,
            thresholds,
        ),
    )


def transient_adaptive_with_digital_event_streams_corners(
    circuit: Circuit,
    input_streams: list[DigitalEventStream],
    negative: str,
    levels: DigitalLogicLevels,
    corners: list[CornerSpec],
    *,
    t_stop: float,
    t_step: float,
    output_probes: list[tuple[str, str]],
    thresholds: DigitalThresholds,
    method: str = "trap",
    tol_lte: float = 1e-4,
    min_step: float | None = None,
    max_step: float | None = None,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerAdaptiveDigitalTransientBridgeResult:
    """Run the adaptive digital bridge across named transient corners."""
    return CornerAdaptiveDigitalTransientBridgeResult(
        points=[
            CornerAdaptiveDigitalTransientBridgePoint(
                corner_name=corner.name,
                result=transient_adaptive_with_digital_event_streams(
                    _circuit_with_corner(circuit, corner),
                    input_streams,
                    negative,
                    levels,
                    t_stop=t_stop,
                    t_step=t_step,
                    output_probes=output_probes,
                    thresholds=thresholds,
                    method=method,
                    tol_lte=tol_lte,
                    min_step=min_step,
                    max_step=max_step,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ]
    )


def sample_transient_probe_as_digital_events(
    points: list[TransientPoint],
    probe: str,
    thresholds: DigitalThresholds,
) -> list[DigitalEvent]:
    """Threshold a transient probe into a stable digital event stream."""
    _validate_digital_thresholds(thresholds)
    events: list[DigitalEvent] = []
    current_state: DigitalState | None = None
    for point in points:
        if point.time <= _DIGITAL_BRIDGE_TIME_EPSILON:
            continue
        voltage = _table_probe_value(
            point.node_voltages,
            point.branch_currents,
            probe,
            "sample_transient_probe_as_digital_events",
        )
        state = thresholds.classify(voltage)
        if state is None:
            continue
        if current_state != state:
            events.append(DigitalEvent(point.time, state))
            current_state = state
    return events


def sample_transient_probes_as_digital_event_streams(
    points: list[TransientPoint],
    output_probes: list[tuple[str, str]],
    thresholds: DigitalThresholds,
) -> list[DigitalEventStream]:
    """Threshold multiple transient probes into named digital streams."""
    seen_signal_names: set[str] = set()
    streams: list[DigitalEventStream] = []
    for signal_name, probe in output_probes:
        trimmed = signal_name.strip()
        if not trimmed:
            raise ValueError("digital_event_stream: digital event stream signal name must not be empty")
        if trimmed in seen_signal_names:
            raise ValueError(f"{trimmed}: digital event stream signal names must be unique")
        seen_signal_names.add(trimmed)
        streams.append(
            DigitalEventStream(
                trimmed,
                sample_transient_probe_as_digital_events(points, probe, thresholds),
            )
        )
    return streams


def format_digital_event_table(events: list[DigitalEvent]) -> str:
    """Format digital events as stable tab-separated text."""
    rows = ["Index\tTime\tState"]
    previous_time = -math.inf
    for index, event in enumerate(events):
        _validate_digital_event_time(event.time_seconds, previous_time, "digital_event")
        previous_time = event.time_seconds
        rows.append(
            "\t".join(
                [
                    str(index),
                    _format_table_number(event.time_seconds),
                    _normalize_digital_state(event.state),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_digital_event_stream_table(streams: list[DigitalEventStream]) -> str:
    """Format named digital streams as stable tab-separated text."""
    rows = ["Signal\tIndex\tTime\tState"]
    for stream in streams:
        if not stream.signal_name.strip():
            raise ValueError("digital_event_stream: digital event stream signal name must not be empty")
        previous_time = -math.inf
        for index, event in enumerate(stream.events):
            _validate_digital_event_time(event.time_seconds, previous_time, stream.signal_name)
            previous_time = event.time_seconds
            rows.append(
                "\t".join(
                    [
                        stream.signal_name,
                        str(index),
                        _format_table_number(event.time_seconds),
                        _normalize_digital_state(event.state),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_corner_digital_event_stream_table(
    result: CornerDigitalTransientBridgeResult,
) -> str:
    """Format named-corner digital bridge streams as stable text."""
    rows = ["Corner\tSignal\tIndex\tTime\tState"]
    for corner in result.points:
        for stream in corner.result.output_streams:
            if not stream.signal_name.strip():
                raise ValueError("digital_event_stream: digital event stream signal name must not be empty")
            previous_time = -math.inf
            for index, event in enumerate(stream.events):
                _validate_digital_event_time(event.time_seconds, previous_time, stream.signal_name)
                previous_time = event.time_seconds
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            stream.signal_name,
                            str(index),
                            _format_table_number(event.time_seconds),
                            _normalize_digital_state(event.state),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def format_adaptive_digital_event_stream_table(
    result: AdaptiveDigitalTransientBridgeResult,
) -> str:
    """Format adaptive digital bridge streams as stable text."""
    rows = ["Method\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState"]
    for stream in result.output_streams:
        if not stream.signal_name.strip():
            raise ValueError("digital_event_stream: digital event stream signal name must not be empty")
        previous_time = -math.inf
        for index, event in enumerate(stream.events):
            _validate_digital_event_time(event.time_seconds, previous_time, stream.signal_name)
            previous_time = event.time_seconds
            rows.append(
                "\t".join(
                    [
                        result.result.method,
                        str(result.result.steps_rejected),
                        str(result.result.converged).lower(),
                        stream.signal_name,
                        str(index),
                        _format_table_number(event.time_seconds),
                        _normalize_digital_state(event.state),
                    ]
                )
            )
    rows.append("")
    return "\n".join(rows)


def format_corner_adaptive_digital_event_stream_table(
    result: CornerAdaptiveDigitalTransientBridgeResult,
) -> str:
    """Format named-corner adaptive digital bridge streams as stable text."""
    rows = ["Corner\tMethod\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState"]
    for corner in result.points:
        for stream in corner.result.output_streams:
            if not stream.signal_name.strip():
                raise ValueError("digital_event_stream: digital event stream signal name must not be empty")
            previous_time = -math.inf
            for index, event in enumerate(stream.events):
                _validate_digital_event_time(event.time_seconds, previous_time, stream.signal_name)
                previous_time = event.time_seconds
                rows.append(
                    "\t".join(
                        [
                            corner.corner_name,
                            corner.result.result.method,
                            str(corner.result.result.steps_rejected),
                            str(corner.result.result.converged).lower(),
                            stream.signal_name,
                            str(index),
                            _format_table_number(event.time_seconds),
                            _normalize_digital_state(event.state),
                        ]
                    )
                )
    rows.append("")
    return "\n".join(rows)


def format_digital_bridge_schedule_table(schedule: DigitalBridgeSchedule) -> str:
    """Format a hardware-VM bridge schedule as stable text."""
    if not math.isfinite(schedule.stop_time) or schedule.stop_time < 0.0:
        raise ValueError("digital_bridge_schedule: digital bridge stop time must be finite and non-negative")
    rows = ["Index\tTime\tStopTime"]
    previous_time = -math.inf
    for index, time_seconds in enumerate(schedule.breakpoints):
        _validate_digital_event_time(time_seconds, previous_time, "digital_bridge_schedule")
        if time_seconds > schedule.stop_time:
            raise ValueError("digital_bridge_schedule: digital bridge breakpoint must not exceed stop time")
        previous_time = time_seconds
        rows.append(
            "\t".join(
                [
                    str(index),
                    _format_table_number(time_seconds),
                    _format_table_number(schedule.stop_time),
                ]
            )
        )
    rows.append("")
    return "\n".join(rows)


def format_digital_event_stream_vcd(
    streams: list[DigitalEventStream],
    *,
    module_name: str = "spice_bridge",
    timescale: str = "1ps",
) -> str:
    """Format digital streams as deterministic VCD for VM/probe correlation."""
    if timescale != "1ps":
        raise ValueError("digital_event_stream_vcd: only 1ps timescale is supported")
    if not module_name.strip():
        raise ValueError("digital_event_stream_vcd: module name must not be empty")

    seen_signal_names: set[str] = set()
    signal_ids: dict[str, str] = {}
    for index, stream in enumerate(streams):
        signal_name = _validate_digital_event_stream_name(stream, seen_signal_names)
        signal_ids[signal_name] = _vcd_identifier(index)
        previous_time = -math.inf
        for event in stream.events:
            _validate_digital_event_time(event.time_seconds, previous_time, signal_name)
            _normalize_digital_state(event.state)
            previous_time = event.time_seconds

    rows = [
        "$version coding-adventures spice-engine mixed-signal bridge $end",
        f"$timescale {timescale} $end",
        f"$scope module {module_name.strip()} $end",
    ]
    for stream in streams:
        signal_name = stream.signal_name.strip()
        rows.append(f"$var wire 1 {signal_ids[signal_name]} {signal_name} $end")
    rows.extend(["$upscope $end", "$enddefinitions $end", "$dumpvars"])
    for stream in streams:
        if stream.events:
            rows.append(f"{_vcd_state_value(stream.events[0].state)}{signal_ids[stream.signal_name.strip()]}")
    rows.append("$end")

    events_by_tick: dict[int, list[tuple[str, DigitalState]]] = {}
    for stream in streams:
        signal_name = stream.signal_name.strip()
        for event in stream.events:
            tick = _vcd_tick(event.time_seconds)
            events_by_tick.setdefault(tick, []).append((signal_ids[signal_name], event.state))
    for tick in sorted(events_by_tick):
        rows.append(f"#{tick}")
        for signal_id, state in events_by_tick[tick]:
            rows.append(f"{_vcd_state_value(state)}{signal_id}")
    rows.append("")
    return "\n".join(rows)


def _circuit_with_extra_voltage_sources(
    circuit: Circuit,
    sources: list[VoltageSource],
) -> Circuit:
    bridged = Circuit(
        elements=[*circuit.elements],
        subcircuits=dict(circuit.subcircuits),
    )
    for source in sources:
        bridged.add(source)
    return bridged


def _normalize_digital_state(state: DigitalState | str) -> DigitalState:
    text = str(state).strip().lower()
    if text == "low":
        return "low"
    if text == "high":
        return "high"
    raise ValueError(f"digital_event: unsupported digital state {state!r}")


def _validate_digital_logic_levels(levels: DigitalLogicLevels, context: str) -> None:
    if not (
        math.isfinite(levels.low_voltage)
        and math.isfinite(levels.high_voltage)
        and math.isfinite(levels.transition_seconds)
    ):
        raise ValueError(f"{context}: digital logic levels must be finite")
    if levels.high_voltage <= levels.low_voltage:
        raise ValueError(f"{context}: digital high voltage must be greater than low voltage")
    if levels.transition_seconds <= 0.0:
        raise ValueError(f"{context}: digital transition time must be finite and positive")


def _validate_digital_thresholds(thresholds: DigitalThresholds) -> None:
    if not (
        math.isfinite(thresholds.low_max_voltage)
        and math.isfinite(thresholds.high_min_voltage)
    ):
        raise ValueError("digital_thresholds: digital thresholds must be finite")
    if thresholds.high_min_voltage <= thresholds.low_max_voltage:
        raise ValueError("digital_thresholds: digital high threshold must be greater than low threshold")


def _validate_digital_event_stream_name(
    stream: DigitalEventStream,
    seen_signal_names: set[str],
) -> str:
    signal_name = stream.signal_name.strip()
    if not signal_name:
        raise ValueError("digital_event_stream: digital event stream signal name must not be empty")
    if signal_name in seen_signal_names:
        raise ValueError(f"{signal_name}: digital event stream signal names must be unique")
    seen_signal_names.add(signal_name)
    return signal_name


def _validate_digital_event_time(
    time_seconds: float,
    previous_time: float,
    context: str,
) -> None:
    if not math.isfinite(time_seconds) or time_seconds < 0.0:
        raise ValueError(f"{context}: digital event times must be finite and non-negative")
    if time_seconds <= previous_time:
        raise ValueError(f"{context}: digital event times must be strictly increasing")


def _vcd_identifier(index: int) -> str:
    return f"s{index}"


def _vcd_tick(time_seconds: float) -> int:
    if not math.isfinite(time_seconds) or time_seconds < 0.0:
        raise ValueError("digital_event_stream_vcd: event times must be finite and non-negative")
    return int(round(time_seconds / 1.0e-12))


def _vcd_state_value(state: DigitalState) -> str:
    return "0" if _normalize_digital_state(state) == "low" else "1"


def pss_residual(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
) -> PssResidualResult | None:
    """Run one estimated period and report the node-voltage closure residual.

    This is a small PSS foothold: it does not solve the shooting-Newton system,
    but it exposes the residual that such a solver must drive toward zero.
    """
    period = estimate_period(circuit)
    if period is None:
        return None
    if steps_per_period <= 0:
        raise ValueError("steps_per_period must be positive")
    if residual_tol < 0.0:
        raise ValueError("residual_tol must be non-negative")

    time_step = period / steps_per_period
    result = transient(
        circuit,
        t_stop=period,
        t_step=time_step,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
    )
    if len(result.points) < 2:
        return PssResidualResult(
            period=period,
            time_step=time_step,
            node_residuals={},
            branch_residuals={},
            residual_vector=[],
            max_abs_branch_residual=0.0,
            max_abs_residual=0.0,
            residual_l2_norm=0.0,
            residual_rms_norm=0.0,
            residual_tol=residual_tol,
            within_tolerance=False,
            converged=False,
        )

    start_nodes = result.points[0].node_voltages
    end_nodes = result.points[-1].node_voltages
    nodes = set(start_nodes) | set(end_nodes)
    node_residuals = {
        node: end_nodes.get(node, 0.0) - start_nodes.get(node, 0.0)
        for node in sorted(nodes)
    }
    start_branches = result.points[0].branch_currents
    end_branches = result.points[-1].branch_currents
    branches = set(start_branches) | set(end_branches)
    branch_residuals = {
        branch: end_branches.get(branch, 0.0) - start_branches.get(branch, 0.0)
        for branch in sorted(branches)
    }
    residual_vector = [
        PssResidualEntry(kind="node", name=node, value=node_residuals[node])
        for node in sorted(node_residuals)
    ] + [
        PssResidualEntry(
            kind="branch_current",
            name=branch,
            value=branch_residuals[branch],
        )
        for branch in sorted(branch_residuals)
    ]
    max_abs_node = max((abs(value) for value in node_residuals.values()), default=0.0)
    max_abs_branch = max((abs(value) for value in branch_residuals.values()), default=0.0)
    max_abs = max(max_abs_node, max_abs_branch)
    residual_l2_norm = math.sqrt(
        sum(entry.value * entry.value for entry in residual_vector)
    )
    residual_rms_norm = (
        residual_l2_norm / math.sqrt(len(residual_vector))
        if residual_vector
        else 0.0
    )
    return PssResidualResult(
        period=period,
        time_step=time_step,
        node_residuals=node_residuals,
        branch_residuals=branch_residuals,
        residual_vector=residual_vector,
        max_abs_branch_residual=max_abs_branch,
        max_abs_residual=max_abs,
        residual_l2_norm=residual_l2_norm,
        residual_rms_norm=residual_rms_norm,
        residual_tol=residual_tol,
        within_tolerance=result.converged and max_abs <= residual_tol,
        converged=result.converged,
    )


def _pss_state_vector(circuit: Circuit) -> list[PssStateEntry]:
    state_vector: list[PssStateEntry] = []
    for element in circuit.elements:
        if isinstance(element, Capacitor):
            state_vector.append(
                PssStateEntry(
                    kind="capacitor_voltage",
                    name=element.name,
                    value=element.initial_voltage,
                )
            )
        elif isinstance(element, Inductor):
            state_vector.append(
                PssStateEntry(
                    kind="inductor_current",
                    name=element.name,
                    value=element.initial_current,
                )
            )
    return state_vector


def _with_perturbed_pss_state(
    circuit: Circuit,
    target: PssStateEntry,
    perturbation: float,
) -> Circuit:
    elements: list[Element] = []
    for element in circuit.elements:
        if (
            target.kind == "capacitor_voltage"
            and isinstance(element, Capacitor)
            and element.name == target.name
        ):
            elements.append(
                replace(
                    element,
                    initial_voltage=element.initial_voltage + perturbation,
                )
            )
        elif (
            target.kind == "inductor_current"
            and isinstance(element, Inductor)
            and element.name == target.name
        ):
            elements.append(
                replace(
                    element,
                    initial_current=element.initial_current + perturbation,
                )
            )
        else:
            elements.append(element)
    return Circuit(elements=elements, subcircuits=dict(circuit.subcircuits))


def _with_pss_state_vector(
    circuit: Circuit,
    state_vector: list[PssStateEntry],
) -> Circuit:
    target_by_key = {(state.kind, state.name): state.value for state in state_vector}
    elements: list[Element] = []
    for element in circuit.elements:
        if isinstance(element, Capacitor):
            value = target_by_key.get(("capacitor_voltage", element.name))
            if value is not None:
                elements.append(replace(element, initial_voltage=value))
            else:
                elements.append(element)
        elif isinstance(element, Inductor):
            value = target_by_key.get(("inductor_current", element.name))
            if value is not None:
                elements.append(replace(element, initial_current=value))
            else:
                elements.append(element)
        else:
            elements.append(element)
    return Circuit(elements=elements, subcircuits=dict(circuit.subcircuits))


def pss_residual_jacobian(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
) -> PssResidualJacobianResult | None:
    """Estimate d(residual vector)/d(initial reactive state) for PSS shooting."""
    if not math.isfinite(perturbation) or perturbation <= 0.0:
        raise ValueError("perturbation must be finite and positive")

    residual = pss_residual(
        circuit,
        steps_per_period=steps_per_period,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
        residual_tol=residual_tol,
    )
    if residual is None:
        return None

    state_vector = _pss_state_vector(circuit)
    columns: list[PssResidualJacobianColumn] = []
    for state in state_vector:
        perturbed = pss_residual(
            _with_perturbed_pss_state(circuit, state, perturbation),
            steps_per_period=steps_per_period,
            method=method,
            max_iterations=max_iterations,
            tol=tol,
            residual_tol=residual_tol,
        )
        if perturbed is None:
            raise ValueError("perturbed circuit no longer has an estimated period")
        if len(perturbed.residual_vector) != len(residual.residual_vector):
            raise ValueError("perturbed residual vector changed shape")
        derivatives: list[PssResidualEntry] = []
        for base_entry, perturbed_entry in zip(
            residual.residual_vector,
            perturbed.residual_vector,
            strict=True,
        ):
            if (
                perturbed_entry.kind != base_entry.kind
                or perturbed_entry.name != base_entry.name
            ):
                raise ValueError("perturbed residual vector changed ordering")
            derivatives.append(
                PssResidualEntry(
                    kind=base_entry.kind,
                    name=base_entry.name,
                    value=(perturbed_entry.value - base_entry.value) / perturbation,
                )
            )
        columns.append(PssResidualJacobianColumn(state=state, residual_derivatives=derivatives))

    jacobian = [
        [column.residual_derivatives[row_index].value for column in columns]
        for row_index in range(len(residual.residual_vector))
    ]
    return PssResidualJacobianResult(
        residual=residual,
        state_vector=state_vector,
        perturbation=perturbation,
        columns=columns,
        jacobian=jacobian,
    )


def _pss_normal_equations_update(jacobian: PssResidualJacobianResult) -> list[float]:
    column_count = len(jacobian.state_vector)
    if column_count == 0:
        return []

    residual_values = [entry.value for entry in jacobian.residual.residual_vector]
    normal_matrix = [[0.0 for _ in range(column_count)] for _ in range(column_count)]
    normal_rhs = [0.0 for _ in range(column_count)]
    for row_index, row in enumerate(jacobian.jacobian):
        residual_value = residual_values[row_index]
        for col_index in range(column_count):
            normal_rhs[col_index] -= row[col_index] * residual_value
            for other_col in range(column_count):
                normal_matrix[col_index][other_col] += row[col_index] * row[other_col]
    return _solve(normal_matrix, normal_rhs)


def pss_newton_update(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
) -> PssNewtonUpdateResult | None:
    """Compute a least-squares Newton update for PSS reactive initial states."""
    jacobian = pss_residual_jacobian(
        circuit,
        steps_per_period=steps_per_period,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
        residual_tol=residual_tol,
        perturbation=perturbation,
    )
    if jacobian is None:
        return None

    update_values = _pss_normal_equations_update(jacobian)
    state_updates = [
        PssStateEntry(kind=state.kind, name=state.name, value=update)
        for state, update in zip(jacobian.state_vector, update_values, strict=True)
    ]
    next_state_vector = [
        PssStateEntry(
            kind=state.kind,
            name=state.name,
            value=state.value + update,
        )
        for state, update in zip(jacobian.state_vector, update_values, strict=True)
    ]
    update_l2_norm = math.sqrt(sum(update * update for update in update_values))
    return PssNewtonUpdateResult(
        jacobian=jacobian,
        state_updates=state_updates,
        next_state_vector=next_state_vector,
        update_l2_norm=update_l2_norm,
    )


def pss_newton_candidate(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
) -> PssNewtonCandidateResult | None:
    """Apply one PSS Newton update and report the candidate residual."""
    update = pss_newton_update(
        circuit,
        steps_per_period=steps_per_period,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
        residual_tol=residual_tol,
        perturbation=perturbation,
    )
    if update is None:
        return None

    candidate_circuit = _with_pss_state_vector(circuit, update.next_state_vector)
    candidate_residual = pss_residual(
        candidate_circuit,
        steps_per_period=steps_per_period,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
        residual_tol=residual_tol,
    )
    if candidate_residual is None:
        raise ValueError("candidate circuit no longer has an estimated period")

    return PssNewtonCandidateResult(
        update=update,
        candidate_circuit=candidate_circuit,
        candidate_state_vector=_pss_state_vector(candidate_circuit),
        candidate_residual=candidate_residual,
    )


def pss_newton_iteration(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
) -> PssNewtonIterationResult | None:
    """Run one PSS Newton iteration and keep only an improving candidate."""
    candidate = pss_newton_candidate(
        circuit,
        steps_per_period=steps_per_period,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
        residual_tol=residual_tol,
        perturbation=perturbation,
    )
    if candidate is None:
        return None

    base_residual = candidate.update.jacobian.residual
    candidate_residual = candidate.candidate_residual
    base_norm = base_residual.residual_l2_norm
    candidate_norm = candidate_residual.residual_l2_norm
    accepted = candidate_norm <= base_norm
    next_circuit = candidate.candidate_circuit if accepted else circuit
    next_state_vector = (
        candidate.candidate_state_vector if accepted else candidate.update.jacobian.state_vector
    )
    next_residual = candidate_residual if accepted else base_residual
    residual_l2_ratio = candidate_norm / base_norm if base_norm > 0.0 else 0.0

    return PssNewtonIterationResult(
        candidate=candidate,
        accepted=accepted,
        residual_l2_reduction=base_norm - candidate_norm,
        residual_l2_ratio=residual_l2_ratio,
        next_circuit=next_circuit,
        next_state_vector=next_state_vector,
        next_residual=next_residual,
        converged=next_residual.within_tolerance,
    )


def pss_newton_solve(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
    max_newton_iterations: int = 8,
) -> PssNewtonSolveResult | None:
    """Run accepted PSS Newton iterations until convergence or no improvement."""
    if max_newton_iterations <= 0:
        raise ValueError("max_newton_iterations must be positive")

    current_circuit = circuit
    iterations: list[PssNewtonIterationResult] = []
    for _ in range(max_newton_iterations):
        iteration = pss_newton_iteration(
            current_circuit,
            steps_per_period=steps_per_period,
            method=method,
            max_iterations=max_iterations,
            tol=tol,
            residual_tol=residual_tol,
            perturbation=perturbation,
        )
        if iteration is None:
            return None

        iterations.append(iteration)
        current_circuit = iteration.next_circuit
        if iteration.converged or not iteration.accepted:
            break

    final_iteration = iterations[-1]
    final_residual = final_iteration.next_residual
    return PssNewtonSolveResult(
        iterations=iterations,
        final_circuit=final_iteration.next_circuit,
        final_state_vector=final_iteration.next_state_vector,
        final_residual=final_residual,
        converged=final_residual.within_tolerance,
        iteration_count=len(iterations),
    )


def pss(
    circuit: Circuit,
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
    max_newton_iterations: int = 8,
) -> PssResult | None:
    """Solve PSS and return one steady-state period from the solved circuit."""
    solve = pss_newton_solve(
        circuit,
        steps_per_period=steps_per_period,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
        residual_tol=residual_tol,
        perturbation=perturbation,
        max_newton_iterations=max_newton_iterations,
    )
    if solve is None:
        return None

    steady_state = transient(
        solve.final_circuit,
        t_stop=solve.final_residual.period,
        t_step=solve.final_residual.time_step,
        method=method,
        max_iterations=max_iterations,
        tol=tol,
    )
    return PssResult(
        solve=solve,
        steady_state=steady_state,
        period=solve.final_residual.period,
        time_step=solve.final_residual.time_step,
        converged=solve.converged and steady_state.converged,
    )


def pss_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    steps_per_period: int = 64,
    method: str = "trap",
    max_iterations: int = 50,
    tol: float = 1e-6,
    residual_tol: float = 1e-6,
    perturbation: float = 1e-6,
    max_newton_iterations: int = 8,
) -> CornerPssResult | None:
    """Solve PSS at each named corner, returning ``None`` if any corner is non-periodic."""
    points: list[CornerPssPoint] = []
    for corner in corners:
        result = pss(
            _circuit_with_corner(circuit, corner),
            steps_per_period=steps_per_period,
            method=method,
            max_iterations=max_iterations,
            tol=tol,
            residual_tol=residual_tol,
            perturbation=perturbation,
            max_newton_iterations=max_newton_iterations,
        )
        if result is None:
            return None
        points.append(CornerPssPoint(corner_name=corner.name, result=result))
    return CornerPssResult(points=points)


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
    if len(A) >= _SPARSE_SOLVER_THRESHOLD:
        return _solve_complex_sparse(A, b)
    return _solve_complex_dense(A, b)


def _solve_complex_dense(A: list[list[complex]], b: list[complex]) -> list[complex]:
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


def _solve_complex_sparse(A: list[list[complex]], b: list[complex]) -> list[complex]:
    """Sparse-row complex Gaussian elimination with partial pivoting."""

    n = len(A)
    if n == 0:
        return []
    rows = [
        {col: value for col, value in enumerate(row) if value != 0j}
        for row in A
    ]
    rhs = list(b)

    for pivot_col in range(n):
        pivot_row = max(
            range(pivot_col, n),
            key=lambda row: abs(rows[row].get(pivot_col, 0j)),
        )
        pivot_abs = abs(rows[pivot_row].get(pivot_col, 0j))
        if pivot_abs < 1e-15:
            raise ZeroDivisionError(f"singular matrix at row {pivot_col}")

        rows[pivot_col], rows[pivot_row] = rows[pivot_row], rows[pivot_col]
        rhs[pivot_col], rhs[pivot_row] = rhs[pivot_row], rhs[pivot_col]

        pivot_value = rows[pivot_col][pivot_col]
        pivot_entries = [
            (col, value)
            for col, value in rows[pivot_col].items()
            if col > pivot_col
        ]
        for row_index in range(pivot_col + 1, n):
            value = rows[row_index].get(pivot_col, 0j)
            if value == 0j:
                continue
            factor = value / pivot_value
            rows[row_index].pop(pivot_col, None)
            for col, pivot_entry in pivot_entries:
                next_value = rows[row_index].get(col, 0j) - factor * pivot_entry
                if abs(next_value) < 1e-15:
                    rows[row_index].pop(col, None)
                else:
                    rows[row_index][col] = next_value
            rhs[row_index] -= factor * rhs[pivot_col]

    x: list[complex] = [0j] * n
    for row_index in range(n - 1, -1, -1):
        diag = rows[row_index].get(row_index, 0j)
        if abs(diag) < 1e-15:
            raise ZeroDivisionError(f"singular matrix at row {row_index}")
        total = rhs[row_index]
        for col, value in rows[row_index].items():
            if col > row_index:
                total -= value * x[col]
        x[row_index] = total / diag
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


def _stamp_vccs_c(
    G: list[list[complex]],
    node_to_idx: dict[str, int],
    n_plus: str,
    n_minus: str,
    ctrl_plus: str,
    ctrl_minus: str,
    gm: complex,
) -> None:
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


def _inductor_by_name(circuit: Circuit) -> dict[str, Inductor]:
    return {el.name: el for el in circuit.elements if isinstance(el, Inductor)}


def _coupled_inductor_names(circuit: Circuit) -> set[str]:
    names: set[str] = set()
    for el in circuit.elements:
        if isinstance(el, MutualInductor):
            names.add(el.primary)
            names.add(el.secondary)
    return names


def _validate_mutual_inductor(
    mutual: MutualInductor,
    inductors: dict[str, Inductor],
) -> tuple[Inductor, Inductor, float]:
    if not math.isfinite(mutual.coupling):
        raise ValueError(f"{mutual.name}: coupling must be finite")
    if abs(mutual.coupling) >= 1.0:
        raise ValueError(f"{mutual.name}: coupling magnitude must be less than one")
    if mutual.primary == mutual.secondary:
        raise ValueError(f"{mutual.name}: coupled inductors must be distinct")
    primary = inductors.get(mutual.primary)
    if primary is None:
        raise ValueError(f"{mutual.name}: referenced inductor {mutual.primary!r} was not found")
    secondary = inductors.get(mutual.secondary)
    if secondary is None:
        raise ValueError(f"{mutual.name}: referenced inductor {mutual.secondary!r} was not found")
    if primary.inductance <= 0.0 or not math.isfinite(primary.inductance):
        raise ValueError(f"{primary.name}: inductance must be finite and positive")
    if secondary.inductance <= 0.0 or not math.isfinite(secondary.inductance):
        raise ValueError(f"{secondary.name}: inductance must be finite and positive")
    mutual_inductance = mutual.coupling * math.sqrt(primary.inductance * secondary.inductance)
    return primary, secondary, mutual_inductance


def _stamp_ac_mutual_inductor(
    mutual: MutualInductor,
    inductors: dict[str, Inductor],
    G: list[list[complex]],
    omega: float,
    node_to_idx: dict[str, int],
) -> None:
    primary, secondary, mutual_inductance = _validate_mutual_inductor(mutual, inductors)
    if omega == 0.0:
        _stamp_g_c(G, node_to_idx, primary.n_plus, primary.n_minus, 1e12 + 0j)
        _stamp_g_c(G, node_to_idx, secondary.n_plus, secondary.n_minus, 1e12 + 0j)
        return

    determinant = primary.inductance * secondary.inductance - mutual_inductance**2
    if determinant <= 0.0 or not math.isfinite(determinant):
        raise ValueError(f"{mutual.name}: coupled inductance matrix is singular")

    scale = 1.0 / (1j * omega * determinant)
    y11 = secondary.inductance * scale
    y12 = -mutual_inductance * scale
    y22 = primary.inductance * scale
    _stamp_g_c(G, node_to_idx, primary.n_plus, primary.n_minus, y11)
    _stamp_g_c(G, node_to_idx, secondary.n_plus, secondary.n_minus, y22)
    _stamp_vccs_c(
        G,
        node_to_idx,
        primary.n_plus,
        primary.n_minus,
        secondary.n_plus,
        secondary.n_minus,
        y12,
    )
    _stamp_vccs_c(
        G,
        node_to_idx,
        secondary.n_plus,
        secondary.n_minus,
        primary.n_plus,
        primary.n_minus,
        y12,
    )


def _validate_transmission_line(line: TransmissionLine) -> None:
    if not math.isfinite(line.characteristic_impedance):
        raise ValueError(f"{line.name}: characteristic impedance must be finite")
    if line.characteristic_impedance <= 0.0:
        raise ValueError(f"{line.name}: characteristic impedance must be positive")
    if not math.isfinite(line.delay):
        raise ValueError(f"{line.name}: delay must be finite")
    if line.delay <= 0.0:
        raise ValueError(f"{line.name}: delay must be positive")


def _stamp_ac_transmission_line(
    line: TransmissionLine,
    G: list[list[complex]],
    omega: float,
    node_to_idx: dict[str, int],
) -> None:
    _validate_transmission_line(line)
    phase = omega * line.delay
    sin_phase = math.sin(phase)
    if abs(sin_phase) < 1.0e-12:
        raise ValueError(f"{line.name}: transmission line phase is singular at this frequency")
    cos_phase = math.cos(phase)
    y11 = complex(0.0, -cos_phase / (line.characteristic_impedance * sin_phase))
    y12 = complex(0.0, 1.0 / (line.characteristic_impedance * sin_phase))
    _stamp_g_c(G, node_to_idx, line.n1, line.n2, y11)
    _stamp_g_c(G, node_to_idx, line.n3, line.n4, y11)
    _stamp_vccs_c(G, node_to_idx, line.n1, line.n2, line.n3, line.n4, y12)
    _stamp_vccs_c(G, node_to_idx, line.n3, line.n4, line.n1, line.n2, y12)


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
    inductors: dict[str, Inductor],
    coupled_inductor_names: set[str],
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
        if el.name in coupled_inductor_names:
            return
        # Admittance Y_L = 1/(jωL).  At ω = 0, Y → ∞ (short circuit); model
        # as a very large conductance to keep the matrix non-singular.
        if omega == 0.0:
            y_l: complex = 1e12 + 0j
        else:
            y_l = 1.0 / (1j * omega * el.inductance)
        _stamp_g_c(G, node_to_idx, el.n_plus, el.n_minus, y_l)

    elif isinstance(el, MutualInductor):
        _stamp_ac_mutual_inductor(el, inductors, G, omega, node_to_idx)

    elif isinstance(el, TransmissionLine):
        _stamp_ac_transmission_line(el, G, omega, node_to_idx)

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

    elif isinstance(el, CustomModel):
        conductance = _custom_model_conductance(el, node_to_idx, dc_x)
        _stamp_g_c(G, node_to_idx, el.n_plus, el.n_minus, conductance + 0j)

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
        # Small-signal model: linearised conductance gd = (Is/(N*Vt))·exp(Vd/(N*Vt)).
        # The dynamic (differential) conductance is the derivative of
        # I = Is*(exp(Vd/(N*Vt)) − 1) with respect to Vd, evaluated at the OP.
        intrinsic_anode = _diode_intrinsic_anode_node(el)
        Va = 0.0 if _is_ground(intrinsic_anode) else dc_x[node_to_idx[intrinsic_anode]]
        Vk = 0.0 if _is_ground(el.cathode) else dc_x[node_to_idx[el.cathode]]
        Vd = Va - Vk
        _, gd = _diode_current_conductance(el, Vd)
        diffusion_capacitance = el.Tt * gd
        depletion_capacitance = _diode_depletion_capacitance(el, Vd)
        _stamp_g_c(
            G,
            node_to_idx,
            intrinsic_anode,
            el.cathode,
            gd + 1j * omega * (depletion_capacitance + diffusion_capacitance),
        )
        if el.Rs > 0.0:
            _stamp_g_c(G, node_to_idx, el.anode, intrinsic_anode, 1.0 / el.Rs)

    elif isinstance(el, JFET):
        Vd = 0.0 if _is_ground(el.drain) else dc_x[node_to_idx[el.drain]]
        Vg = 0.0 if _is_ground(el.gate) else dc_x[node_to_idx[el.gate]]
        Vs = 0.0 if _is_ground(el.source) else dc_x[node_to_idx[el.source]]
        _, gm_j, gds_j = _eval_jfet(el, Vg - Vs, Vd - Vs)
        _stamp_g_c(G, node_to_idx, el.drain, el.source, gds_j + 0j)
        if el.Cgs > 0.0:
            _stamp_g_c(G, node_to_idx, el.gate, el.source, 1j * omega * el.Cgs)
        if el.Cgd > 0.0:
            _stamp_g_c(G, node_to_idx, el.gate, el.drain, 1j * omega * el.Cgd)
        if not _is_ground(el.drain):
            d = node_to_idx[el.drain]
            if not _is_ground(el.gate):
                G[d][node_to_idx[el.gate]] += gm_j + 0j
            if not _is_ground(el.source):
                G[d][node_to_idx[el.source]] -= gm_j + 0j
        if not _is_ground(el.source):
            s = node_to_idx[el.source]
            if not _is_ground(el.gate):
                G[s][node_to_idx[el.gate]] -= gm_j + 0j
            if not _is_ground(el.source):
                G[s][node_to_idx[el.source]] += gm_j + 0j

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
        _stamp_g_c(G, node_to_idx, el.gate, el.source, 1j * omega * r.Cgs)
        _stamp_g_c(G, node_to_idx, el.gate, el.drain, 1j * omega * r.Cgd)
        _stamp_g_c(G, node_to_idx, el.gate, el.body, 1j * omega * r.Cgb)
        _stamp_g_c(G, node_to_idx, el.body, el.source, 1j * omega * r.Cbs)
        _stamp_g_c(G, node_to_idx, el.body, el.drain, 1j * omega * r.Cbd)
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
        _validate_bjt(el)
        external_base = el.base
        if el.Re > 0.0:
            intrinsic_emitter = _bjt_intrinsic_emitter_node(el)
            _stamp_g_c(
                G,
                node_to_idx,
                el.emitter,
                intrinsic_emitter,
                complex(1.0 / el.Re),
            )
            el = replace(el, emitter=intrinsic_emitter, Re=0.0)
        if el.Rc > 0.0:
            intrinsic_collector = _bjt_intrinsic_collector_node(el)
            _stamp_g_c(
                G,
                node_to_idx,
                el.collector,
                intrinsic_collector,
                complex(1.0 / el.Rc),
            )
            el = replace(el, collector=intrinsic_collector, Rc=0.0)
        if el.Rb > 0.0:
            intrinsic_base = _bjt_intrinsic_base_node(el)
            Vb_rb = (
                0.0
                if _is_ground(intrinsic_base)
                else dc_x[node_to_idx[intrinsic_base]]
            )
            Ve_rb = 0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
            Vc_rb = (
                0.0
                if _is_ground(el.collector)
                else dc_x[node_to_idx[el.collector]]
            )
            base_resistance = _bjt_effective_base_resistance(
                el, Vb_rb, Ve_rb, Vc_rb
            )
            _stamp_g_c(
                G,
                node_to_idx,
                el.base,
                intrinsic_base,
                complex(1.0 / base_resistance),
            )
            el = replace(el, base=intrinsic_base, Rb=0.0, Rbm=None, Irb=0.0)
        Vc_dc = 0.0 if _is_ground(el.collector) else dc_x[node_to_idx[el.collector]]
        Vb_dc = 0.0 if _is_ground(el.base) else dc_x[node_to_idx[el.base]]
        Ve_dc = 0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
        Vjunc = (
            min(Vb_dc - Ve_dc, 0.7) if el.polarity == "NPN"
            else min(Ve_dc - Vb_dc, 0.7)
        )
        Vreverse = (
            min(Vb_dc - Vc_dc, 0.7) if el.polarity == "NPN"
            else min(Vc_dc - Vb_dc, 0.7)
        )
        Vcollector_leakage = (
            Vb_dc - Vc_dc if el.polarity == "NPN"
            else Vc_dc - Vb_dc
        )
        forward_thermal_voltage = el.Vt * el.Nf
        reverse_thermal_voltage = el.Vt * el.Nr
        exp_t = math.exp(Vjunc / forward_thermal_voltage)
        exp_reverse = math.exp(Vreverse / reverse_thermal_voltage)
        base_collector_current = el.Is * (exp_t - 1.0)
        base_gm = (el.Is / forward_thermal_voltage) * exp_t
        output_voltage = Vc_dc - Ve_dc if el.polarity == "NPN" else Ve_dc - Vc_dc
        early_factor = _bjt_early_factor(el, Vjunc, output_voltage)
        _, gm_b, charge_factor = _bjt_forward_transport(
            el, base_collector_current, base_gm, early_factor
        )
        output_conductance = (
            0.0 if el.Vaf == 0.0 else base_collector_current / el.Vaf / charge_factor
        )
        gm_reverse: float = (el.Is / reverse_thermal_voltage) * exp_reverse
        _, leakage_conductance = _bjt_base_emitter_leakage(el, Vjunc)
        _, collector_leakage_conductance = _bjt_base_collector_leakage(
            el, Vcollector_leakage
        )
        _, reverse_base_conductance = _bjt_reverse_base_current(
            el, Vcollector_leakage
        )
        g_pi: float = base_gm / el.beta_f + leakage_conductance
        diffusion_capacitance = (
            el.Tf * _bjt_forward_transit_time_scale(el, Vjunc, Vreverse) * gm_b
        )
        excess_phase = omega * el.Tf * el.Ptf * math.pi / 180.0
        gm_ac = complex(
            gm_b * math.cos(excess_phase),
            -gm_b * math.sin(excess_phase),
        )
        reverse_diffusion_capacitance = el.Tr * gm_reverse
        y_be = g_pi + 1j * omega * (
            _bjt_base_emitter_depletion_capacitance(el, Vjunc) + diffusion_capacitance
        )
        base_collector_depletion = _bjt_base_collector_depletion_capacitance(
            el, Vreverse
        )
        y_bc = collector_leakage_conductance + reverse_base_conductance + 1j * omega * (
            el.Xcjc * base_collector_depletion + reverse_diffusion_capacitance
        )
        y_bx = 1j * omega * (1.0 - el.Xcjc) * base_collector_depletion
        _stamp_g_c(G, node_to_idx, el.collector, el.emitter, output_conductance + 0j)
        if y_bx != 0j:
            _stamp_g_c(G, node_to_idx, external_base, el.collector, y_bx)

        if el.polarity == "NPN":
            _stamp_g_c(G, node_to_idx, el.base, el.emitter, y_be)
            _stamp_g_c(G, node_to_idx, el.base, el.collector, y_bc)
            if not _is_ground(el.collector):
                c_i = node_to_idx[el.collector]
                if not _is_ground(el.base):
                    G[c_i][node_to_idx[el.base]] += gm_ac
                if not _is_ground(el.emitter):
                    G[c_i][node_to_idx[el.emitter]] -= gm_ac
            if not _is_ground(el.emitter):
                e_i = node_to_idx[el.emitter]
                if not _is_ground(el.base):
                    G[e_i][node_to_idx[el.base]] -= gm_ac
                if not _is_ground(el.emitter):
                    G[e_i][node_to_idx[el.emitter]] += gm_ac
        else:  # PNP
            _stamp_g_c(G, node_to_idx, el.emitter, el.base, y_be)
            _stamp_g_c(G, node_to_idx, el.base, el.collector, y_bc)
            if not _is_ground(el.emitter):
                e_i = node_to_idx[el.emitter]
                if not _is_ground(el.emitter):
                    G[e_i][node_to_idx[el.emitter]] += gm_ac
                if not _is_ground(el.base):
                    G[e_i][node_to_idx[el.base]] -= gm_ac
            if not _is_ground(el.collector):
                c_i = node_to_idx[el.collector]
                if not _is_ground(el.emitter):
                    G[c_i][node_to_idx[el.emitter]] -= gm_ac
                if not _is_ground(el.base):
                    G[c_i][node_to_idx[el.base]] += gm_ac


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
    inductors = _inductor_by_name(circuit)
    coupled_inductor_names = _coupled_inductor_names(circuit)

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
                inductors,
                coupled_inductor_names,
                explicit_ac_sources=explicit_ac_sources,
            )

        try:
            x_c = _solve_complex(G_c, b_c)
        except ZeroDivisionError:
            x_c = [0j] * size  # singular — return zeros for this frequency

        node_v = {name: x_c[idx] for name, idx in node_to_idx.items()}
        branch_i = {
            f"I({src.name})": x_c[n_nodes + i] for i, src in enumerate(branch_srcs)
        }
        ac_points.append(AcPoint(freq=freq, node_voltages=node_v, branch_currents=branch_i))

    return AcResult(points=ac_points)


def _circuit_with_sparameter_drive(
    circuit: Circuit,
    port_sources: tuple[str, str],
    driven_source: str,
) -> Circuit:
    elements: list[Element] = []
    seen_ports: set[str] = set()
    port_set = set(port_sources)

    for element in circuit.elements:
        if isinstance(element, VoltageSource) and element.name in port_set:
            seen_ports.add(element.name)
            magnitude = 1.0 if element.name == driven_source else 0.0
            elements.append(replace(element, ac=AcSource(magnitude=magnitude)))
        else:
            elements.append(element)

    missing = [source for source in port_sources if source not in seen_ports]
    if missing:
        raise ValueError(f"s_parameters: missing voltage-source port(s): {missing}")

    return Circuit(elements)


def _branch_current_into_network(point: AcPoint, source_name: str) -> complex:
    key = source_name if source_name.startswith("I(") else f"I({source_name})"
    if key not in point.branch_currents:
        raise ValueError(f"s_parameters: missing branch current for {source_name!r}")
    return -point.branch_currents[key]


def _y_to_s_2port(
    y11: complex,
    y21: complex,
    y12: complex,
    y22: complex,
    z0: float,
) -> tuple[complex, complex, complex, complex]:
    a11 = 1.0 - z0 * y11
    a12 = -z0 * y12
    a21 = -z0 * y21
    a22 = 1.0 - z0 * y22

    b11 = 1.0 + z0 * y11
    b12 = z0 * y12
    b21 = z0 * y21
    b22 = 1.0 + z0 * y22
    det = b11 * b22 - b12 * b21
    if abs(det) < 1.0e-18:
        raise ZeroDivisionError("s_parameters: singular Y-to-S conversion")

    inv_b11 = b22 / det
    inv_b12 = -b12 / det
    inv_b21 = -b21 / det
    inv_b22 = b11 / det

    return (
        a11 * inv_b11 + a12 * inv_b21,
        a21 * inv_b11 + a22 * inv_b21,
        a11 * inv_b12 + a12 * inv_b22,
        a21 * inv_b12 + a22 * inv_b22,
    )


def s_parameters(
    circuit: Circuit,
    *,
    port1_source: str,
    port2_source: str,
    frequencies: list[float],
    reference_impedance: float = 50.0,
) -> SParameterResult:
    """Extract two-port S-parameters from AC small-signal solves.

    The two ports are represented by named independent voltage sources.  For
    each frequency the engine drives one port source with a 1 V AC phasor,
    shorts the other port source with a 0 V AC phasor, measures the port
    currents, builds the 2x2 Y-parameter matrix, then converts Y to S for the
    requested reference impedance.
    """
    if reference_impedance <= 0.0 or not math.isfinite(reference_impedance):
        raise ValueError("s_parameters: reference_impedance must be finite and positive")
    if any(freq <= 0.0 or not math.isfinite(freq) for freq in frequencies):
        raise ValueError("s_parameters: frequencies must be finite and positive")

    ports = (port1_source, port2_source)
    points: list[SParameterPoint] = []
    for freq in frequencies:
        y_columns: list[tuple[complex, complex]] = []
        for driven in ports:
            driven_circuit = _circuit_with_sparameter_drive(circuit, ports, driven)
            ac_point = ac_sweep(
                driven_circuit,
                f_start=freq,
                f_stop=freq,
                n_points=1,
                sweep="lin",
            ).points[0]
            y_columns.append(
                (
                    _branch_current_into_network(ac_point, port1_source),
                    _branch_current_into_network(ac_point, port2_source),
                )
            )

        s11, s21, s12, s22 = _y_to_s_2port(
            y_columns[0][0],
            y_columns[0][1],
            y_columns[1][0],
            y_columns[1][1],
            reference_impedance,
        )
        points.append(SParameterPoint(freq=freq, s11=s11, s21=s21, s12=s12, s22=s22))

    return SParameterResult(
        port1_source=port1_source,
        port2_source=port2_source,
        reference_impedance=reference_impedance,
        points=points,
    )


def s_parameters_corners(
    circuit: Circuit,
    *,
    port1_source: str,
    port2_source: str,
    frequencies: list[float],
    corners: list[CornerSpec],
    reference_impedance: float = 50.0,
) -> CornerSParameterResult:
    """Extract two-port S-parameters at each named corner."""
    return CornerSParameterResult(
        points=[
            CornerSParameterPoint(
                corner_name=corner.name,
                result=s_parameters(
                    _circuit_with_corner(circuit, corner),
                    port1_source=port1_source,
                    port2_source=port2_source,
                    frequencies=frequencies,
                    reference_impedance=reference_impedance,
                ),
            )
            for corner in corners
        ],
        port1_source=port1_source,
        port2_source=port2_source,
        reference_impedance=reference_impedance,
    )


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

        elif isinstance(el, CustomModel):
            conductance = _custom_model_conductance(el, node_to_idx, dc_x)
            _stamp_g(G, node_to_idx, el.n_plus, el.n_minus, conductance)

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
            # Small-signal conductance: gd = dI/dVd = (Is/(N*Vt))·exp(Vd/(N*Vt)).
            intrinsic_anode = _diode_intrinsic_anode_node(el)
            Va = (
                0.0
                if _is_ground(intrinsic_anode)
                else dc_x[node_to_idx[intrinsic_anode]]
            )
            Vk = 0.0 if _is_ground(el.cathode) else dc_x[node_to_idx[el.cathode]]
            Vd = Va - Vk
            _, gd = _diode_current_conductance(el, Vd)
            _stamp_g(G, node_to_idx, intrinsic_anode, el.cathode, gd)
            if el.Rs > 0.0:
                _stamp_g(G, node_to_idx, el.anode, intrinsic_anode, 1.0 / el.Rs)

        elif isinstance(el, JFET):
            Vd = 0.0 if _is_ground(el.drain) else dc_x[node_to_idx[el.drain]]
            Vg = 0.0 if _is_ground(el.gate) else dc_x[node_to_idx[el.gate]]
            Vs = 0.0 if _is_ground(el.source) else dc_x[node_to_idx[el.source]]
            _, gm_j, gds_j = _eval_jfet(el, Vg - Vs, Vd - Vs)
            _stamp_g(G, node_to_idx, el.drain, el.source, gds_j)
            if not _is_ground(el.drain):
                d = node_to_idx[el.drain]
                if not _is_ground(el.gate):
                    G[d][node_to_idx[el.gate]] += gm_j
                if not _is_ground(el.source):
                    G[d][node_to_idx[el.source]] -= gm_j
            if not _is_ground(el.source):
                s = node_to_idx[el.source]
                if not _is_ground(el.gate):
                    G[s][node_to_idx[el.gate]] -= gm_j
                if not _is_ground(el.source):
                    G[s][node_to_idx[el.source]] += gm_j

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
            _validate_bjt(el)
            if el.Re > 0.0:
                intrinsic_emitter = _bjt_intrinsic_emitter_node(el)
                _stamp_g(
                    G,
                    node_to_idx,
                    el.emitter,
                    intrinsic_emitter,
                    1.0 / el.Re,
                )
                el = replace(el, emitter=intrinsic_emitter, Re=0.0)
            if el.Rc > 0.0:
                intrinsic_collector = _bjt_intrinsic_collector_node(el)
                _stamp_g(
                    G,
                    node_to_idx,
                    el.collector,
                    intrinsic_collector,
                    1.0 / el.Rc,
                )
                el = replace(el, collector=intrinsic_collector, Rc=0.0)
            if el.Rb > 0.0:
                intrinsic_base = _bjt_intrinsic_base_node(el)
                Vb_rb = (
                    0.0
                    if _is_ground(intrinsic_base)
                    else dc_x[node_to_idx[intrinsic_base]]
                )
                Ve_rb = (
                    0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
                )
                Vc_rb = (
                    0.0
                    if _is_ground(el.collector)
                    else dc_x[node_to_idx[el.collector]]
                )
                base_resistance = _bjt_effective_base_resistance(
                    el, Vb_rb, Ve_rb, Vc_rb
                )
                _stamp_g(
                    G,
                    node_to_idx,
                    el.base,
                    intrinsic_base,
                    1.0 / base_resistance,
                )
                el = replace(el, base=intrinsic_base, Rb=0.0, Rbm=None, Irb=0.0)
            Vc_dc = 0.0 if _is_ground(el.collector) else dc_x[node_to_idx[el.collector]]
            Vb_dc = 0.0 if _is_ground(el.base) else dc_x[node_to_idx[el.base]]
            Ve_dc = 0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
            Vjunc = (
                min(Vb_dc - Ve_dc, 0.7) if el.polarity == "NPN"
                else min(Ve_dc - Vb_dc, 0.7)
            )
            Vreverse = Vb_dc - Vc_dc if el.polarity == "NPN" else Vc_dc - Vb_dc
            forward_thermal_voltage = el.Vt * el.Nf
            exp_t = math.exp(Vjunc / forward_thermal_voltage)
            base_collector_current = el.Is * (exp_t - 1.0)
            base_gm = (el.Is / forward_thermal_voltage) * exp_t
            output_voltage = Vc_dc - Ve_dc if el.polarity == "NPN" else Ve_dc - Vc_dc
            early_factor = _bjt_early_factor(el, Vjunc, output_voltage)
            _, gm_b, charge_factor = _bjt_forward_transport(
                el, base_collector_current, base_gm, early_factor
            )
            output_conductance = (
                0.0 if el.Vaf == 0.0 else base_collector_current / el.Vaf / charge_factor
            )
            _, leakage_conductance = _bjt_base_emitter_leakage(el, Vjunc)
            _, collector_leakage_conductance = _bjt_base_collector_leakage(el, Vreverse)
            _, reverse_base_conductance = _bjt_reverse_base_current(el, Vreverse)
            g_pi: float = base_gm / el.beta_f + leakage_conductance
            _stamp_g(G, node_to_idx, el.collector, el.emitter, output_conductance)
            _stamp_g(
                G,
                node_to_idx,
                el.base,
                el.collector,
                collector_leakage_conductance + reverse_base_conductance,
            )
            _stamp_g(G, node_to_idx, el.base, el.collector, collector_leakage_conductance)

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


def dc_sweep_corners(
    circuit: Circuit,
    source_name: str,
    start: float,
    stop: float,
    step: float,
    corners: list[CornerSpec],
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerDcSweepResult:
    """Run a DC source sweep at each named corner.

    Each corner clones the circuit with explicit element-parameter overrides,
    then reuses :func:`dc_sweep` so every corner returns the same source-value
    sequence with its own operating-point snapshots.
    """
    return CornerDcSweepResult(
        points=[
            CornerDcSweepPoint(
                corner_name=corner.name,
                result=dc_sweep(
                    _circuit_with_corner(circuit, corner),
                    source_name,
                    start,
                    stop,
                    step,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ],
        source_name=source_name,
    )


def ac_sweep_corners(
    circuit: Circuit,
    corners: list[CornerSpec],
    *,
    f_start: float,
    f_stop: float,
    n_points: int = 50,
    sweep: str = "log",
) -> CornerAcSweepResult:
    """Run an AC frequency sweep at each named corner.

    Each corner clones the circuit with explicit element-parameter overrides,
    then reuses :func:`ac_sweep` so every corner returns the same frequency
    grid with its own complex phasor response.
    """
    return CornerAcSweepResult(
        points=[
            CornerAcSweepPoint(
                corner_name=corner.name,
                result=ac_sweep(
                    _circuit_with_corner(circuit, corner),
                    f_start=f_start,
                    f_stop=f_stop,
                    n_points=n_points,
                    sweep=sweep,
                ),
            )
            for corner in corners
        ]
    )


def tf_corners(
    circuit: Circuit,
    input_source: str,
    output_node: str,
    corners: list[CornerSpec],
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerTfResult:
    """Run DC small-signal transfer-function analysis at each named corner.

    Each corner clones the circuit with explicit element-parameter overrides,
    then reuses :func:`tf` so every corner reports the same transfer-function
    query with its own gain and impedance values.
    """
    return CornerTfResult(
        points=[
            CornerTfPoint(
                corner_name=corner.name,
                result=tf(
                    _circuit_with_corner(circuit, corner),
                    output_node=output_node,
                    input_source=input_source,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ],
        input_source=input_source,
        output_node=output_node,
    )


def sens_dc_corners(
    circuit: Circuit,
    output_node: str,
    corners: list[CornerSpec],
    *,
    max_iterations: int = 50,
    tol: float = 1e-6,
    perturbation: float = 1e-3,
    abs_floor: float = 1e-10,
) -> CornerSensResult:
    """Run DC sensitivity analysis at each named corner."""
    return CornerSensResult(
        points=[
            CornerSensPoint(
                corner_name=corner.name,
                result=sens_dc(
                    _circuit_with_corner(circuit, corner),
                    output_node,
                    max_iterations=max_iterations,
                    tol=tol,
                    perturbation=perturbation,
                    abs_floor=abs_floor,
                ),
            )
            for corner in corners
        ],
        output_node=output_node,
    )


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
                Diode(
                    el.name,
                    el.anode,
                    el.cathode,
                    el.Is + delta_is,
                    el.Vt,
                    el.N,
                    el.BV,
                    el.IBV,
                    el.Cjo,
                    el.Tt,
                    el.Vj,
                    el.M,
                    el.Fc,
                    el.Xti,
                    el.Eg,
                    el.Rs,
                    el.Kf,
                    el.Af,
                ),
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
                    Cje=el.Cje,
                    Cjc=el.Cjc,
                    Tf=el.Tf,
                    Tr=el.Tr,
                    Xti=el.Xti,
                    Eg=el.Eg,
                    Vaf=el.Vaf,
                    Var=el.Var,
                    Ikf=el.Ikf,
                    Ise=el.Ise,
                    Ne=el.Ne,
                    Isc=el.Isc,
                    Nc=el.Nc,
                    Nf=el.Nf,
                    Nr=el.Nr,
                    Vje=el.Vje,
                    Mje=el.Mje,
                    Vjc=el.Vjc,
                    Mjc=el.Mjc,
                    Fc=el.Fc,
                    Xtb=el.Xtb,
                    beta_r=el.beta_r,
                    Ikr=el.Ikr,
                    Re=el.Re,
                    Rc=el.Rc,
                    Rb=el.Rb,
                    Rbm=el.Rbm,
                    Irb=el.Irb,
                    Xcjc=el.Xcjc,
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
                    Cje=el.Cje,
                    Cjc=el.Cjc,
                    Tf=el.Tf,
                    Tr=el.Tr,
                    Xti=el.Xti,
                    Eg=el.Eg,
                    Vaf=el.Vaf,
                    Var=el.Var,
                    Ikf=el.Ikf,
                    Ise=el.Ise,
                    Ne=el.Ne,
                    Isc=el.Isc,
                    Nc=el.Nc,
                    Nf=el.Nf,
                    Nr=el.Nr,
                    Vje=el.Vje,
                    Mje=el.Mje,
                    Vjc=el.Vjc,
                    Mjc=el.Mjc,
                    Fc=el.Fc,
                    Xtb=el.Xtb,
                    beta_r=el.beta_r,
                    Ikr=el.Ikr,
                    Re=el.Re,
                    Rc=el.Rc,
                    Rb=el.Rb,
                    Rbm=el.Rbm,
                    Irb=el.Irb,
                    Xcjc=el.Xcjc,
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
        return Diode(
            el.name,
            el.anode,
            el.cathode,
            _draw(el.Is),
            el.Vt,
            el.N,
            el.BV,
            el.IBV,
            el.Cjo,
            el.Tt,
            el.Vj,
            el.M,
            el.Fc,
            el.Xti,
            el.Eg,
            el.Rs,
            el.Kf,
            el.Af,
        )

    if isinstance(el, BJT):
        return BJT(
            el.name, el.collector, el.base, el.emitter,
            polarity=el.polarity,
            Is=_draw(el.Is),
            beta_f=_draw(el.beta_f),
            Vt=el.Vt,
            Cje=el.Cje,
            Cjc=el.Cjc,
            Tf=el.Tf,
            Tr=el.Tr,
            Xti=el.Xti,
            Eg=el.Eg,
            Vaf=el.Vaf,
            Var=el.Var,
            Ikf=el.Ikf,
            Ise=el.Ise,
            Ne=el.Ne,
            Isc=el.Isc,
            Nc=el.Nc,
            Nf=el.Nf,
            Nr=el.Nr,
            Vje=el.Vje,
            Mje=el.Mje,
            Vjc=el.Vjc,
            Mjc=el.Mjc,
            Fc=el.Fc,
            Xtb=el.Xtb,
            beta_r=el.beta_r,
            Ikr=el.Ikr,
            Re=el.Re,
            Rc=el.Rc,
            Rb=el.Rb,
            Rbm=el.Rbm,
            Irb=el.Irb,
            Xcjc=el.Xcjc,
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


def mc_dc_corners(
    circuit: Circuit,
    output_node: str,
    n_trials: int,
    corners: list[CornerSpec],
    *,
    tolerance: float = 0.05,
    distribution: str = "gaussian",
    seed: int | None = None,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerMcResult:
    """Run Monte Carlo DC analysis at each named corner."""
    return CornerMcResult(
        points=[
            CornerMcPoint(
                corner_name=corner.name,
                result=mc_dc(
                    _circuit_with_corner(circuit, corner),
                    output_node,
                    n_trials=n_trials,
                    tolerance=tolerance,
                    distribution=distribution,
                    seed=seed,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ],
        output_node=output_node,
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
#   Mosfet      : S_i = 4kTγgm, channel thermal noise drain → source
#   All others (Capacitor, Inductor, VoltageSource, CurrentSource):
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
_MOSFET_CHANNEL_NOISE_GAMMA: float = 2.0 / 3.0


@dataclass(frozen=True)
class NoiseEntry:
    """Noise contribution from one element at one frequency.

    Attributes
    ----------
    element_name : str
        Name of the circuit element generating this noise.
    noise_type : str
        ``"thermal"`` (Johnson-Nyquist noise, for resistors and MOSFET
        channel noise) or
        ``"shot"`` (Poisson/shot noise, for diodes and BJTs), or
        ``"flicker"`` (BJT base-current flicker noise).
    source_psd : float
        Noise current power spectral density at the source itself, in A²/Hz.
        For resistors: ``4kT/R``.  For MOSFETs: ``4kTγgm``.  For
        diodes/BJTs: ``2q|I_DC|``.
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
) -> list[tuple[str, str, int | None, int | None, float, float]]:
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
    list of 6-tuples
        (element_name, noise_type, n_plus_idx, n_minus_idx, coefficient,
        frequency_exponent).
        ``n_plus_idx`` and ``n_minus_idx`` are integer matrix indices, or
        ``None`` when the terminal connects to ground.
        The current noise PSD is coefficient / frequency**frequency_exponent.
    """
    kT4 = 4.0 * _BOLTZMANN * temperature  # 4kT factor
    q2 = 2.0 * _ELECTRON_CHARGE           # 2q factor

    sources: list[tuple[str, str, int | None, int | None, float, float]] = []

    for el in circuit.elements:
        if isinstance(el, Resistor):
            # Johnson-Nyquist thermal noise: S_i = 4kT/R
            psd = kT4 / el.resistance
            n_p = node_to_idx.get(el.n_plus)   # None for ground
            n_m = node_to_idx.get(el.n_minus)  # None for ground
            sources.append((el.name, "thermal", n_p, n_m, psd, 0.0))

        elif isinstance(el, Diode):
            # Shot noise: S_i = 2q |I_D|
            # Use the actual converged DC voltage from dc_x — no clamp needed here
            # because we are evaluating at the operating point, not iterating Newton.
            # (The 0.7 V clamp in the Newton loop prevents divergence during
            # iterations; at convergence, Vd is the physically correct value.)
            intrinsic_anode = _diode_intrinsic_anode_node(el)
            Va = (
                0.0
                if _is_ground(intrinsic_anode)
                else dc_x[node_to_idx[intrinsic_anode]]
            )
            Vk = 0.0 if _is_ground(el.cathode) else dc_x[node_to_idx[el.cathode]]
            Vd = Va - Vk  # actual operating-point junction voltage
            I_D, _ = _diode_current_conductance(el, Vd, clamp_forward=False)
            psd = q2 * abs(I_D)
            n_a = (
                None
                if _is_ground(intrinsic_anode)
                else node_to_idx[intrinsic_anode]
            )
            n_k = None if _is_ground(el.cathode) else node_to_idx[el.cathode]
            sources.append((el.name, "shot", n_a, n_k, psd, 0.0))
            if el.Kf > 0.0:
                sources.append(
                    (el.name, "flicker", n_a, n_k, el.Kf * abs(I_D) ** el.Af, 1.0)
                )
            if el.Rs > 0.0:
                sources.append(
                    (
                        f"{el.name}:RS",
                        "thermal",
                        node_to_idx.get(el.anode),
                        node_to_idx.get(intrinsic_anode),
                        kT4 / el.Rs,
                        0.0,
                    )
                )

        elif isinstance(el, BJT):
            # Shot noise on the base-emitter junction: S_i = 2q |I_C|
            # I_C ≈ Is × exp(V_BE / Vt)  (dominates for forward-active BJT)
            # Use actual converged dc_x voltages — no clamp — same reasoning as Diode.
            if el.Re > 0.0:
                intrinsic_emitter = _bjt_intrinsic_emitter_node(el)
                sources.append((
                    f"{el.name}:RE",
                    "thermal",
                    node_to_idx.get(el.emitter),
                    node_to_idx.get(intrinsic_emitter),
                    kT4 / el.Re,
                    0.0,
                ))
                el = replace(el, emitter=intrinsic_emitter, Re=0.0)
            if el.Rc > 0.0:
                intrinsic_collector = _bjt_intrinsic_collector_node(el)
                sources.append((
                    f"{el.name}:RC",
                    "thermal",
                    node_to_idx.get(el.collector),
                    node_to_idx.get(intrinsic_collector),
                    kT4 / el.Rc,
                    0.0,
                ))
                el = replace(el, collector=intrinsic_collector, Rc=0.0)
            if el.Rb > 0.0:
                intrinsic_base = _bjt_intrinsic_base_node(el)
                Vb_rb = (
                    0.0
                    if _is_ground(intrinsic_base)
                    else dc_x[node_to_idx[intrinsic_base]]
                )
                Ve_rb = (
                    0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
                )
                Vc_rb = (
                    0.0
                    if _is_ground(el.collector)
                    else dc_x[node_to_idx[el.collector]]
                )
                base_resistance = _bjt_effective_base_resistance(
                    el, Vb_rb, Ve_rb, Vc_rb
                )
                sources.append((
                    f"{el.name}:RB",
                    "thermal",
                    node_to_idx.get(el.base),
                    node_to_idx.get(intrinsic_base),
                    kT4 / base_resistance,
                    0.0,
                ))
                el = replace(el, base=intrinsic_base, Rb=0.0, Rbm=None, Irb=0.0)
            Vb = 0.0 if _is_ground(el.base) else dc_x[node_to_idx[el.base]]
            Ve = 0.0 if _is_ground(el.emitter) else dc_x[node_to_idx[el.emitter]]
            Vc = 0.0 if _is_ground(el.collector) else dc_x[node_to_idx[el.collector]]
            Vjunc = (
                Vb - Ve if el.polarity == "NPN"
                else Ve - Vb
            )
            Vreverse = Vb - Vc if el.polarity == "NPN" else Vc - Vb
            output_voltage = Vc - Ve if el.polarity == "NPN" else Ve - Vc
            early_factor = _bjt_early_factor(el, Vjunc, output_voltage)
            exp_value = math.exp(Vjunc / (el.Vt * el.Nf))
            base_collector_current = el.Is * (exp_value - 1.0)
            base_gm = el.Is / (el.Vt * el.Nf) * exp_value
            I_C, _, _ = _bjt_forward_transport(
                el, base_collector_current, base_gm, early_factor
            )
            leakage_current, _ = _bjt_base_emitter_leakage(el, Vjunc)
            collector_leakage_current, _ = _bjt_base_collector_leakage(el, Vreverse)
            reverse_base_current, _ = _bjt_reverse_base_current(el, Vreverse)
            psd = q2 * (
                abs(I_C)
                + abs(leakage_current)
                + abs(collector_leakage_current)
                + abs(reverse_base_current)
            )
            n_b = None if _is_ground(el.base) else node_to_idx[el.base]
            n_e = None if _is_ground(el.emitter) else node_to_idx[el.emitter]
            sources.append((el.name, "shot", n_b, n_e, psd, 0.0))
            if el.Kf > 0.0:
                base_current = base_collector_current / el.beta_f + leakage_current
                sources.append((el.name, "flicker", n_b, n_e, el.Kf * abs(base_current) ** el.Af, 1.0))

        elif isinstance(el, Mosfet):
            # Long-channel MOSFET channel thermal noise: S_i = 4kTγgm.
            Vd = 0.0 if _is_ground(el.drain) else dc_x[node_to_idx[el.drain]]
            Vg = 0.0 if _is_ground(el.gate) else dc_x[node_to_idx[el.gate]]
            Vs = 0.0 if _is_ground(el.source) else dc_x[node_to_idx[el.source]]
            Vb = 0.0 if _is_ground(el.body) else dc_x[node_to_idx[el.body]]
            r = el.model.dc(Vg - Vs, Vd - Vs, Vb - Vs)  # type: ignore[attr-defined]
            gm = max(0.0, float(r.gm))
            if gm > 0.0:
                psd = kT4 * _MOSFET_CHANNEL_NOISE_GAMMA * gm
                n_d = None if _is_ground(el.drain) else node_to_idx[el.drain]
                n_s = None if _is_ground(el.source) else node_to_idx[el.source]
                sources.append((el.name, "thermal", n_d, n_s, psd, 0.0))

        elif isinstance(el, JFET):
            Vd = 0.0 if _is_ground(el.drain) else dc_x[node_to_idx[el.drain]]
            Vg = 0.0 if _is_ground(el.gate) else dc_x[node_to_idx[el.gate]]
            Vs = 0.0 if _is_ground(el.source) else dc_x[node_to_idx[el.source]]
            drain_current, gm, _ = _eval_jfet(el, Vg - Vs, Vd - Vs)
            gm = max(0.0, float(gm))
            if gm > 0.0:
                psd = kT4 * _MOSFET_CHANNEL_NOISE_GAMMA * gm
                n_d = None if _is_ground(el.drain) else node_to_idx[el.drain]
                n_s = None if _is_ground(el.source) else node_to_idx[el.source]
                sources.append((el.name, "thermal", n_d, n_s, psd, 0.0))
            if el.Kf > 0.0:
                n_d = None if _is_ground(el.drain) else node_to_idx[el.drain]
                n_s = None if _is_ground(el.source) else node_to_idx[el.source]
                sources.append(
                    (el.name, "flicker", n_d, n_s, el.Kf * abs(drain_current) ** el.Af, 1.0)
                )

        # Capacitors, Inductors, VoltageSources, CurrentSources: noiseless in
        # this first-order model.

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
    due to thermal noise (Johnson-Nyquist) in resistors, MOSFET channel
    thermal noise, and shot plus flicker noise in diodes and BJTs, at each frequency in
    ``freqs``.  Also reports the noise referred back to ``input_source`` so
    you can compare it directly to your signal level.

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
    - MOSFET channel noise uses the long-channel thermal approximation
      ``4kT γ gm`` with ``γ = 2/3``.
    - Noiseless elements: Capacitor, Inductor, VoltageSource, CurrentSource.
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
    inductors = _inductor_by_name(circuit)
    coupled_inductor_names = _coupled_inductor_names(circuit)

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
                inductors,
                coupled_inductor_names,
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
                for (name, ntype, _, _, coefficient, frequency_exponent) in noise_sources
                for psd in (coefficient / freq**frequency_exponent,)
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

        for (elem_name, noise_type, n_p, n_m, coefficient, frequency_exponent) in noise_sources:
            src_psd = coefficient / freq**frequency_exponent
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


def noise_ac_corners(
    circuit: Circuit,
    output_node: str,
    input_source: str,
    corners: list[CornerSpec],
    freqs: list[float] | None = None,
    *,
    temperature: float = 300.0,
    max_iterations: int = 50,
    tol: float = 1e-6,
) -> CornerNoiseResult:
    """Run AC noise analysis at each named corner."""
    return CornerNoiseResult(
        points=[
            CornerNoisePoint(
                corner_name=corner.name,
                result=noise_ac(
                    _circuit_with_corner(circuit, corner),
                    output_node,
                    input_source,
                    freqs=freqs,
                    temperature=temperature,
                    max_iterations=max_iterations,
                    tol=tol,
                ),
            )
            for corner in corners
        ],
        output_node=output_node,
        input_source=input_source,
    )
