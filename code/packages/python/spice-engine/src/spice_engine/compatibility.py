"""Compatibility corpus and release-readiness gates for SPICE deck parity."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from math import isfinite, pi


@dataclass(frozen=True, slots=True)
class CompatibilityOracle:
    """Documented oracle source for a compatibility deck."""

    reference: str
    version: str
    source: str


@dataclass(frozen=True, slots=True)
class CompatibilityGoldenValue:
    """A named oracle value and tolerance for a deck probe."""

    name: str
    value: float
    unit: str
    abs_tol: float
    rel_tol: float


@dataclass(frozen=True, slots=True)
class CompatibilityDeck:
    """A compact SPICE deck fixture with oracle metadata and known gaps."""

    id: str
    title: str
    analysis: str
    netlist: str
    oracle: CompatibilityOracle
    golden_values: tuple[CompatibilityGoldenValue, ...]
    known_incompatibilities: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeckControlDiagnostic:
    """A stable deck-control diagnostic emitted before analysis execution."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str


@dataclass(frozen=True, slots=True)
class DeckControlSummary:
    """Normalized active deck lines plus deck-control diagnostics."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    diagnostics: tuple[DeckControlDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckResolutionDiagnostic:
    """A stable diagnostic emitted while resolving deck source directives."""

    code: str
    directive: str
    source: str
    line_number: int
    message: str
    severity: str
    target: str | None = None


@dataclass(frozen=True, slots=True)
class DeckResolutionSummary:
    """Resolved active deck lines plus source-resolution metadata."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    diagnostics: tuple[DeckResolutionDiagnostic, ...]
    included_paths: tuple[str, ...]
    library_sections: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeckParameterValue:
    """A resolved scalar SPICE deck parameter."""

    name: str
    value: float


@dataclass(frozen=True, slots=True)
class DeckParameterDiagnostic:
    """A stable diagnostic emitted while resolving deck parameters."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    parameter: str | None = None
    expression: str | None = None


@dataclass(frozen=True, slots=True)
class DeckParameterSummary:
    """Resolved active deck lines plus scalar parameter values."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    parameters: tuple[DeckParameterValue, ...]
    diagnostics: tuple[DeckParameterDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckNodeCondition:
    """A resolved node-voltage initial-condition or nodeset hint."""

    directive: str
    node: str
    value: float
    line_number: int


@dataclass(frozen=True, slots=True)
class DeckInitialConditionDiagnostic:
    """A stable diagnostic emitted while resolving deck node conditions."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    token: str | None = None


@dataclass(frozen=True, slots=True)
class DeckInitialConditionSummary:
    """Resolved active deck lines plus `.ic` and `.nodeset` hints."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    initial_conditions: tuple[DeckNodeCondition, ...]
    nodesets: tuple[DeckNodeCondition, ...]
    diagnostics: tuple[DeckInitialConditionDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckFunctionDefinition:
    """A parsed scalar SPICE `.func` definition."""

    name: str
    arguments: tuple[str, ...]
    expression: str
    line_number: int


@dataclass(frozen=True, slots=True)
class DeckFunctionDiagnostic:
    """A stable diagnostic emitted while resolving deck functions."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    function_name: str | None = None
    expression: str | None = None


@dataclass(frozen=True, slots=True)
class DeckFunctionSummary:
    """Resolved active deck lines plus `.func` definitions."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    functions: tuple[DeckFunctionDefinition, ...]
    diagnostics: tuple[DeckFunctionDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckMeasurementCard:
    """A parsed scalar ``.measure`` / ``.meas`` probe card."""

    directive: str
    analysis: str
    name: str
    mode: str
    probe: str
    line_number: int
    from_value: float | None = None
    to_value: float | None = None
    at_value: float | None = None
    target_value: float | None = None
    crossing_kind: str | None = None
    crossing_count: int | None = None
    trigger_probe: str | None = None
    trigger_value: float | None = None
    trigger_crossing_kind: str | None = None
    trigger_crossing_count: int | None = None


@dataclass(frozen=True, slots=True)
class DeckMeasurementDiagnostic:
    """A stable diagnostic emitted while resolving deck measurements."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    token: str | None = None


@dataclass(frozen=True, slots=True)
class DeckMeasurementSummary:
    """Resolved active deck lines plus parsed measurement cards."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    measurements: tuple[DeckMeasurementCard, ...]
    diagnostics: tuple[DeckMeasurementDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckFourierCard:
    """A parsed transient ``.four`` Fourier-analysis card."""

    directive: str
    fundamental_frequency: float
    probes: tuple[str, ...]
    line_number: int
    harmonics: int | None = None
    from_value: float | None = None


@dataclass(frozen=True, slots=True)
class DeckFourierDiagnostic:
    """A stable diagnostic emitted while resolving deck Fourier cards."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    token: str | None = None


@dataclass(frozen=True, slots=True)
class DeckFourierSummary:
    """Resolved active deck lines plus parsed ``.four`` cards."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    fourier: tuple[DeckFourierCard, ...]
    diagnostics: tuple[DeckFourierDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckOutputSelection:
    """A parsed ``.save``, ``.probe``, ``.print``, or ``.plot`` output card."""

    directive: str
    analysis: str | None
    probes: tuple[str, ...]
    line_number: int


@dataclass(frozen=True, slots=True)
class DeckOutputDiagnostic:
    """A stable diagnostic emitted while resolving deck output selections."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    token: str | None = None


@dataclass(frozen=True, slots=True)
class DeckOutputSummary:
    """Resolved active deck lines plus parsed deck output selection cards."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    selections: tuple[DeckOutputSelection, ...]
    diagnostics: tuple[DeckOutputDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class DeckAnalysisPlan:
    """A parsed top-level SPICE analysis directive."""

    directive: str
    analysis: str
    line_number: int
    source_name: str | None = None
    start_value: float | None = None
    stop_value: float | None = None
    step_value: float | None = None
    sweep_kind: str | None = None
    point_count: int | None = None
    start_frequency: float | None = None
    stop_frequency: float | None = None
    step_time: float | None = None
    stop_time: float | None = None
    start_time: float | None = None
    max_step: float | None = None
    use_initial_conditions: bool = False


@dataclass(frozen=True, slots=True)
class DeckAnalysisDiagnostic:
    """A stable diagnostic emitted while resolving deck analysis cards."""

    code: str
    directive: str
    line_number: int
    message: str
    severity: str
    token: str | None = None


@dataclass(frozen=True, slots=True)
class DeckAnalysisSummary:
    """Resolved active deck lines plus parsed analysis-plan cards."""

    active_lines: tuple[str, ...]
    terminated: bool
    end_line_number: int | None
    analyses: tuple[DeckAnalysisPlan, ...]
    diagnostics: tuple[DeckAnalysisDiagnostic, ...]


@dataclass(frozen=True, slots=True)
class ReleaseReadinessIssue:
    """A release-readiness gate violation for a corpus deck."""

    deck_id: str
    field: str
    message: str


@dataclass(frozen=True, slots=True)
class ReleaseReadinessReport:
    """Summary of package release-readiness gates for the compatibility corpus."""

    passed: bool
    deck_count: int
    analyses: tuple[str, ...]
    issues: tuple[ReleaseReadinessIssue, ...]


_COMMON_KNOWN_INCOMPATIBILITIES = (
    "binary rawfile output is not part of this release gate",
    ".control blocks and vendor-specific directives are intentionally excluded",
    "golden values cover named probes, not byte-for-byte waveform dumps",
)

_COMPATIBILITY_CORPUS = (
    CompatibilityDeck(
        id="dc-op-resistive-divider",
        title="DC operating point resistive divider",
        analysis="op",
        netlist="""* dc-op-resistive-divider
V1 in 0 DC 10
R1 in out 10000
R2 out 0 10000
.op
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="divider-v1",
            source="V(out)=V1*R2/(R1+R2); I(V1)=-V1/(R1+R2)",
        ),
        golden_values=(
            CompatibilityGoldenValue("V(out)", 5.0, "V", 1.0e-9, 1.0e-9),
            CompatibilityGoldenValue("I(V1)", -5.0e-4, "A", 1.0e-12, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
    CompatibilityDeck(
        id="dc-sweep-resistive-divider",
        title="DC source sweep resistive divider",
        analysis="dc",
        netlist="""* dc-sweep-resistive-divider
V1 in 0 DC 0
R1 in out 10000
R2 out 0 10000
.dc V1 0 10 5
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="divider-sweep-v1",
            source="V(out)=V1*0.5 at each sweep point",
        ),
        golden_values=(
            CompatibilityGoldenValue("points", 3.0, "count", 0.0, 0.0),
            CompatibilityGoldenValue("V(out)@V1=10", 5.0, "V", 1.0e-9, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
    CompatibilityDeck(
        id="ac-rc-lowpass",
        title="AC RC low-pass cutoff",
        analysis="ac",
        netlist="""* ac-rc-lowpass
V1 in 0 DC 0 AC 1
R1 in out 1000
C1 out 0 1u
.ac dec 1 1 1k
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="rc-lowpass-v1",
            source="|V(out)|=1/sqrt(1+(2*pi*f*R*C)^2)",
        ),
        golden_values=(
            CompatibilityGoldenValue("f_c", 159.15494309189535, "Hz", 1.0e-9, 1.0e-9),
            CompatibilityGoldenValue("|V(out)|@f_c", 0.7071067811865475, "V", 1.0e-9, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
    CompatibilityDeck(
        id="tran-rc-step",
        title="Transient RC step response",
        analysis="tran",
        netlist="""* tran-rc-step
V1 in 0 PULSE(0 1 0 1n 1n 1m 2m)
R1 in out 1000
C1 out 0 1u
.tran 0.0001 0.001
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="rc-step-v1",
            source="V(out,t)=1-exp(-t/(R*C)) after an ideal 1 V step",
        ),
        golden_values=(
            CompatibilityGoldenValue("V(out)@1ms", 0.6321205588285577, "V", 1.0e-6, 1.0e-6),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES
        + ("finite-edge pulse decks compare at the idealized step oracle point",),
    ),
    CompatibilityDeck(
        id="tf-resistive-divider",
        title="Transfer-function resistive divider",
        analysis="tf",
        netlist="""* tf-resistive-divider
V1 in 0 DC 10
R1 in out 10000
R2 out 0 10000
.tf V(out) V1
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="divider-tf-v1",
            source="gain=R2/(R1+R2); input resistance=R1+R2",
        ),
        golden_values=(
            CompatibilityGoldenValue("gain", 0.5, "V/V", 1.0e-9, 1.0e-9),
            CompatibilityGoldenValue("input_resistance", 20000.0, "ohm", 1.0e-6, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
)

_SUPPORTED_ANALYSES = frozenset({"op", "dc", "ac", "tran", "tf"})
_REQUIRED_ANALYSES = frozenset({"op", "dc", "ac", "tran"})
_UNSUPPORTED_DECK_CONTROL_DIRECTIVES = frozenset({".include", ".lib", ".control"})
_UNSUPPORTED_RESOLVED_DIRECTIVES = frozenset({".control"})
_UNSUPPORTED_PARAMETER_DIRECTIVES = frozenset()
_SUPPORTED_CONTROL_BLOCK_COMMANDS = frozenset(
    {
        "op",
        ".op",
        "dc",
        ".dc",
        "ac",
        ".ac",
        "tran",
        ".tran",
        "save",
        ".save",
        "probe",
        ".probe",
        "measure",
        ".measure",
        "meas",
        ".meas",
        "four",
        ".four",
        "fourier",
        ".fourier",
        "print",
        ".print",
        "plot",
        ".plot",
    }
)
_SPICE_SUFFIX_FACTORS = {
    "t": 1.0e12,
    "g": 1.0e9,
    "meg": 1.0e6,
    "k": 1.0e3,
    "m": 1.0e-3,
    "mil": 25.4e-6,
    "u": 1.0e-6,
    "n": 1.0e-9,
    "p": 1.0e-12,
    "f": 1.0e-15,
}


def analyze_deck_controls(netlist: str) -> DeckControlSummary:
    """Return active pre-``.end`` lines and unsupported deck-control diagnostics."""

    active_lines: list[str] = []
    diagnostics: list[DeckControlDiagnostic] = []
    end_line_number: int | None = None
    in_control_block = False

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if in_control_block:
            if directive == ".endc":
                in_control_block = False
                continue
            control_line = _control_block_command_as_deck_line(stripped)
            if control_line is not None:
                active_lines.append(control_line)
                continue
            diagnostics.append(
                DeckControlDiagnostic(
                    code="SPICE_DECK_CONTROL_COMMAND",
                    directive=".control",
                    line_number=line_number,
                    message=(
                        f"{stripped!r} inside .control is not executed by "
                        "the deck execution foothold yet"
                    ),
                    severity="error",
                )
            )
            continue
        if directive == ".end":
            end_line_number = line_number
            break
        if directive in _UNSUPPORTED_DECK_CONTROL_DIRECTIVES:
            diagnostics.append(
                DeckControlDiagnostic(
                    code="SPICE_DECK_UNSUPPORTED_DIRECTIVE",
                    directive=directive,
                    line_number=line_number,
                    message=(
                        f"{directive} is not supported by the deck execution "
                        "foothold yet"
                    ),
                    severity="error",
                )
            )
            if directive == ".control":
                in_control_block = True
                continue
        active_lines.append(stripped)

    return DeckControlSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        diagnostics=tuple(diagnostics),
    )


def resolve_deck_sources(
    netlist: str,
    sources: Mapping[str, str],
) -> DeckResolutionSummary:
    """Expand ``.include`` and selected ``.lib`` sources from a content map."""

    state = _DeckResolutionState()
    active_lines, terminated, end_line_number = _resolve_deck_lines(
        netlist=netlist,
        source="<deck>",
        sources=sources,
        state=state,
        stack=(),
    )
    return DeckResolutionSummary(
        active_lines=tuple(active_lines),
        terminated=terminated,
        end_line_number=end_line_number,
        diagnostics=tuple(state.diagnostics),
        included_paths=tuple(state.included_paths),
        library_sections=tuple(state.library_sections),
    )


def resolve_deck_parameters(netlist: str) -> DeckParameterSummary:
    """Evaluate scalar ``.param`` / ``.func`` cards and rewrite deck expressions."""

    state = _DeckParameterState()
    _collect_parameter_functions(netlist, state)
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive == ".param":
            _resolve_param_line(stripped, line_number, state)
            continue
        if directive == ".func":
            continue
        if directive in _UNSUPPORTED_PARAMETER_DIRECTIVES:
            _add_parameter_diagnostic(
                state,
                code="SPICE_DECK_UNSUPPORTED_DIRECTIVE",
                directive=directive,
                line_number=line_number,
                message=f"{directive} is not supported by the parameter resolver yet",
            )
            active_lines.append(stripped)
            continue
        active_lines.append(_rewrite_parameter_expressions(stripped, line_number, state))

    return DeckParameterSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        parameters=tuple(state.parameter_values()),
        diagnostics=tuple(state.diagnostics),
    )


def resolve_deck_initial_conditions(netlist: str) -> DeckInitialConditionSummary:
    """Extract scalar ``.ic`` and ``.nodeset`` node-voltage hints."""

    state = _DeckInitialConditionState()
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive in {".ic", ".nodeset"}:
            _resolve_node_condition_line(stripped, line_number, directive, state)
            continue
        active_lines.append(stripped)

    return DeckInitialConditionSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        initial_conditions=tuple(state.initial_conditions),
        nodesets=tuple(state.nodesets),
        diagnostics=tuple(state.diagnostics),
    )


def resolve_deck_functions(netlist: str) -> DeckFunctionSummary:
    """Extract scalar ``.func`` definitions without executing them."""

    state = _DeckFunctionState()
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive == ".func":
            _resolve_function_line(stripped, line_number, state)
            continue
        active_lines.append(stripped)

    return DeckFunctionSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        functions=tuple(state.functions),
        diagnostics=tuple(state.diagnostics),
    )


def resolve_deck_measurements(netlist: str) -> DeckMeasurementSummary:
    """Extract the supported scalar ``.measure`` / ``.meas`` card subset."""

    state = _DeckMeasurementState()
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive in {".measure", ".meas"}:
            _resolve_measurement_line(stripped, line_number, directive, state)
            continue
        active_lines.append(stripped)

    return DeckMeasurementSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        measurements=tuple(state.measurements),
        diagnostics=tuple(state.diagnostics),
    )


def resolve_deck_fourier(netlist: str) -> DeckFourierSummary:
    """Extract supported transient ``.four`` cards before ``.end``."""

    state = _DeckFourierState()
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive == ".four":
            _resolve_fourier_line(stripped, line_number, state)
            continue
        active_lines.append(stripped)

    return DeckFourierSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        fourier=tuple(state.fourier),
        diagnostics=tuple(state.diagnostics),
    )


def resolve_deck_outputs(netlist: str) -> DeckOutputSummary:
    """Extract supported ``.save``, ``.probe``, ``.print``, and ``.plot`` cards."""

    state = _DeckOutputState()
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive in {".save", ".probe", ".print", ".plot"}:
            _resolve_output_line(stripped, line_number, directive, state)
            continue
        active_lines.append(stripped)

    return DeckOutputSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        selections=tuple(state.selections),
        diagnostics=tuple(state.diagnostics),
    )


def select_deck_output_probes(netlist: str, analysis: str) -> list[str]:
    """Return deduplicated output probes selected for an analysis."""

    summary = resolve_deck_outputs(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"select_deck_output_probes: line {diagnostic.line_number}: {diagnostic.message}"
        )
    selected: list[str] = []
    seen: set[str] = set()
    for selection in summary.selections:
        if selection.analysis is not None and not _deck_output_analysis_matches(
            selection.analysis,
            analysis,
        ):
            continue
        for probe in selection.probes:
            key = _deck_output_probe_key(probe)
            if key in seen:
                continue
            seen.add(key)
            selected.append(probe)
    return selected


def resolve_deck_analyses(netlist: str) -> DeckAnalysisSummary:
    """Extract supported top-level analysis directives before ``.end``."""

    state = _DeckAnalysisState()
    active_lines: list[str] = []
    end_line_number: int | None = None

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            end_line_number = line_number
            break
        if directive in {".op", ".dc", ".ac", ".tran"}:
            _resolve_analysis_line(stripped, line_number, directive, state)
            continue
        active_lines.append(stripped)

    return DeckAnalysisSummary(
        active_lines=tuple(active_lines),
        terminated=end_line_number is not None,
        end_line_number=end_line_number,
        analyses=tuple(state.analyses),
        diagnostics=tuple(state.diagnostics),
    )


def select_deck_analysis_plan(
    netlist: str,
    analysis: str | None = None,
) -> DeckAnalysisPlan:
    """Return one explicit or implicit deck analysis plan for execution."""

    summary = resolve_deck_analyses(netlist)
    if summary.diagnostics:
        diagnostic = summary.diagnostics[0]
        raise ValueError(
            f"select_deck_analysis_plan: line {diagnostic.line_number}: {diagnostic.message}"
        )

    requested_analysis = None
    if analysis is not None:
        requested_analysis = _normalize_deck_analysis_name(analysis)
        if requested_analysis is None:
            raise ValueError(f"select_deck_analysis_plan: unsupported analysis {analysis!r}")

    plans = list(summary.analyses)
    if requested_analysis is not None:
        plans = [plan for plan in plans if plan.analysis == requested_analysis]
        if not plans:
            raise ValueError(
                f"select_deck_analysis_plan: no .{requested_analysis} analysis card found"
            )
        if len(plans) > 1:
            raise ValueError(
                f"select_deck_analysis_plan: multiple .{requested_analysis} analysis cards found"
            )
        return plans[0]

    if not plans:
        return DeckAnalysisPlan(".op", "op", 0)
    if len(plans) > 1:
        raise ValueError(
            "select_deck_analysis_plan: multiple analysis cards found; "
            "pass analysis to select one"
        )
    return plans[0]


def compatibility_corpus() -> tuple[CompatibilityDeck, ...]:
    """Return the canonical first release-readiness compatibility corpus."""

    return _COMPATIBILITY_CORPUS


def release_readiness_gates(
    corpus: Sequence[CompatibilityDeck] | None = None,
) -> ReleaseReadinessReport:
    """Validate the compatibility corpus metadata used as package release gates."""

    decks = _COMPATIBILITY_CORPUS if corpus is None else tuple(corpus)
    issues: list[ReleaseReadinessIssue] = []
    seen_ids: set[str] = set()
    analyses: list[str] = []

    if not decks:
        issues.append(
            ReleaseReadinessIssue(
                deck_id="corpus",
                field="deck_count",
                message="compatibility corpus must contain at least one deck",
            )
        )

    for deck in decks:
        deck_id = deck.id or "<missing>"
        _validate_non_empty(deck_id, "id", deck.id, issues)
        _validate_non_empty(deck_id, "title", deck.title, issues)
        _validate_non_empty(deck_id, "netlist", deck.netlist, issues)
        _validate_non_empty(deck_id, "oracle.reference", deck.oracle.reference, issues)
        _validate_non_empty(deck_id, "oracle.version", deck.oracle.version, issues)
        _validate_non_empty(deck_id, "oracle.source", deck.oracle.source, issues)
        if deck.id in seen_ids:
            issues.append(
                ReleaseReadinessIssue(deck_id, "id", "deck ids must be unique")
            )
        seen_ids.add(deck.id)
        if deck.analysis not in _SUPPORTED_ANALYSES:
            issues.append(
                ReleaseReadinessIssue(
                    deck_id,
                    "analysis",
                    f"unsupported analysis {deck.analysis!r}",
                )
            )
        elif deck.analysis not in analyses:
            analyses.append(deck.analysis)
        if ".end" not in deck.netlist.lower():
            issues.append(
                ReleaseReadinessIssue(deck_id, "netlist", "deck must include .end")
            )
        if not deck.golden_values:
            issues.append(
                ReleaseReadinessIssue(
                    deck_id,
                    "golden_values",
                    "deck must include at least one golden value",
                )
            )
        for index, golden in enumerate(deck.golden_values):
            field_prefix = f"golden_values[{index}]"
            _validate_non_empty(deck_id, f"{field_prefix}.name", golden.name, issues)
            _validate_non_empty(deck_id, f"{field_prefix}.unit", golden.unit, issues)
            if not isfinite(golden.value):
                issues.append(
                    ReleaseReadinessIssue(
                        deck_id,
                        f"{field_prefix}.value",
                        "golden value must be finite",
                    )
                )
            if golden.abs_tol < 0.0 or golden.rel_tol < 0.0:
                issues.append(
                    ReleaseReadinessIssue(
                        deck_id,
                        f"{field_prefix}.tolerance",
                        "tolerances must be non-negative",
                    )
                )
            if golden.abs_tol == 0.0 and golden.rel_tol == 0.0 and golden.unit != "count":
                issues.append(
                    ReleaseReadinessIssue(
                        deck_id,
                        f"{field_prefix}.tolerance",
                        "non-count golden values need an absolute or relative tolerance",
                    )
                )
        if not deck.known_incompatibilities:
            issues.append(
                ReleaseReadinessIssue(
                    deck_id,
                    "known_incompatibilities",
                    "deck must document known incompatibility boundaries",
                )
            )

    missing = sorted(_REQUIRED_ANALYSES.difference(analyses))
    for analysis in missing:
        issues.append(
            ReleaseReadinessIssue(
                deck_id="corpus",
                field="analysis_coverage",
                message=f"missing required {analysis!r} compatibility deck",
            )
        )

    return ReleaseReadinessReport(
        passed=not issues,
        deck_count=len(decks),
        analyses=tuple(analyses),
        issues=tuple(issues),
    )


def format_compatibility_corpus_table(
    corpus: Sequence[CompatibilityDeck] | None = None,
) -> str:
    """Return a stable tab-separated compatibility corpus summary."""

    decks = _COMPATIBILITY_CORPUS if corpus is None else tuple(corpus)
    lines = ["id\tanalysis\toracle\tgolden_values\tknown_incompatibilities"]
    for deck in decks:
        golden = ",".join(
            f"{entry.name}={entry.value:.6e}{entry.unit}"
            for entry in deck.golden_values
        )
        lines.append(
            "\t".join(
                [
                    deck.id,
                    deck.analysis,
                    f"{deck.oracle.reference}@{deck.oracle.version}",
                    golden,
                    str(len(deck.known_incompatibilities)),
                ]
            )
        )
    return "\n".join(lines)


def format_release_readiness_report(report: ReleaseReadinessReport) -> str:
    """Return a stable tab-separated release-readiness report."""

    lines = [
        "passed\tdeck_count\tanalyses\tissue_count",
        f"{str(report.passed).lower()}\t{report.deck_count}\t{','.join(report.analyses)}\t{len(report.issues)}",
    ]
    if report.issues:
        lines.append("deck_id\tfield\tmessage")
        lines.extend(
            f"{issue.deck_id}\t{issue.field}\t{issue.message}" for issue in report.issues
        )
    return "\n".join(lines)


def _validate_non_empty(
    deck_id: str,
    field: str,
    value: str,
    issues: list[ReleaseReadinessIssue],
) -> None:
    if value.strip():
        return
    issues.append(
        ReleaseReadinessIssue(deck_id, field, "field must be documented and non-empty")
    )


@dataclass(slots=True)
class _DeckResolutionState:
    diagnostics: list[DeckResolutionDiagnostic]
    included_paths: list[str]
    library_sections: list[str]

    def __init__(self) -> None:
        self.diagnostics = []
        self.included_paths = []
        self.library_sections = []


@dataclass(slots=True)
class _DeckParameterState:
    diagnostics: list[DeckParameterDiagnostic]
    parameters: dict[str, DeckParameterValue]
    functions: dict[str, DeckFunctionDefinition]
    order: list[str]

    def __init__(self) -> None:
        self.diagnostics = []
        self.parameters = {}
        self.functions = {}
        self.order = []

    def set_parameter(self, name: str, value: float) -> None:
        key = name.lower()
        if key not in self.parameters:
            self.order.append(key)
        self.parameters[key] = DeckParameterValue(name=name, value=value)

    def parameter_values(self) -> list[DeckParameterValue]:
        return [self.parameters[key] for key in self.order]

    def set_function(self, definition: DeckFunctionDefinition) -> None:
        self.functions[definition.name.lower()] = definition

    def get_function(self, name: str) -> DeckFunctionDefinition | None:
        return self.functions.get(name.lower())


@dataclass(slots=True)
class _DeckInitialConditionState:
    diagnostics: list[DeckInitialConditionDiagnostic]
    initial_conditions: list[DeckNodeCondition]
    nodesets: list[DeckNodeCondition]

    def __init__(self) -> None:
        self.diagnostics = []
        self.initial_conditions = []
        self.nodesets = []


@dataclass(slots=True)
class _DeckFunctionState:
    diagnostics: list[DeckFunctionDiagnostic]
    functions: list[DeckFunctionDefinition]

    def __init__(self) -> None:
        self.diagnostics = []
        self.functions = []


@dataclass(slots=True)
class _DeckMeasurementState:
    diagnostics: list[DeckMeasurementDiagnostic]
    measurements: list[DeckMeasurementCard]

    def __init__(self) -> None:
        self.diagnostics = []
        self.measurements = []


class _DeckFourierState:
    diagnostics: list[DeckFourierDiagnostic]
    fourier: list[DeckFourierCard]

    def __init__(self) -> None:
        self.diagnostics = []
        self.fourier = []


class _DeckOutputState:
    diagnostics: list[DeckOutputDiagnostic]
    selections: list[DeckOutputSelection]

    def __init__(self) -> None:
        self.diagnostics = []
        self.selections = []


class _DeckAnalysisState:
    diagnostics: list[DeckAnalysisDiagnostic]
    analyses: list[DeckAnalysisPlan]

    def __init__(self) -> None:
        self.diagnostics = []
        self.analyses = []


def _resolve_node_condition_line(
    line: str,
    line_number: int,
    directive: str,
    state: _DeckInitialConditionState,
) -> None:
    tokens = _directive_tokens(line)
    if len(tokens) == 1:
        _add_initial_condition_diagnostic(
            state,
            code="SPICE_DECK_CONDITION_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=f"{directive} requires at least one V(node)=value assignment",
        )
        return

    empty_parameter_state = _DeckParameterState()
    for token in tokens[1:]:
        if "=" not in token:
            _add_initial_condition_diagnostic(
                state,
                code="SPICE_DECK_CONDITION_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"{directive} assignment {token!r} must use V(node)=value syntax",
                token=token,
            )
            continue
        target, expression = token.split("=", 1)
        node = _parse_node_condition_target(target.strip())
        if node is None:
            _add_initial_condition_diagnostic(
                state,
                code="SPICE_DECK_CONDITION_TARGET",
                directive=directive,
                line_number=line_number,
                message=f"{directive} target {target!r} must use V(node) syntax",
                token=token,
            )
            continue
        expression = _strip_expression_delimiters(expression.strip())
        try:
            value = _evaluate_parameter_expression(expression, empty_parameter_state)
        except ValueError as error:
            _add_initial_condition_diagnostic(
                state,
                code="SPICE_DECK_CONDITION_EXPRESSION",
                directive=directive,
                line_number=line_number,
                message=str(error),
                token=token,
            )
            continue
        condition = DeckNodeCondition(
            directive=directive,
            node=node,
            value=value,
            line_number=line_number,
        )
        if directive == ".ic":
            state.initial_conditions.append(condition)
        else:
            state.nodesets.append(condition)


def _resolve_function_line(line: str, line_number: int, state: _DeckFunctionState) -> None:
    parts = line.split(None, 1)
    if len(parts) == 1 or not parts[1].strip():
        _add_function_diagnostic(
            state,
            code="SPICE_DECK_FUNC_ARGUMENT",
            line_number=line_number,
            message=".func requires a name(args) expression definition",
        )
        return

    parsed = _parse_function_signature(parts[1].strip())
    if parsed is None:
        _add_function_diagnostic(
            state,
            code="SPICE_DECK_FUNC_SIGNATURE",
            line_number=line_number,
            message=".func definition must use name(args) expression syntax",
        )
        return
    name, arguments, expression = parsed
    if not _is_parameter_name(name):
        _add_function_diagnostic(
            state,
            code="SPICE_DECK_FUNC_SIGNATURE",
            line_number=line_number,
            message=f".func name {name!r} is not a valid identifier",
            function_name=name,
        )
        return
    invalid_argument = next((argument for argument in arguments if not _is_parameter_name(argument)), None)
    if invalid_argument is not None:
        _add_function_diagnostic(
            state,
            code="SPICE_DECK_FUNC_ARGUMENT",
            line_number=line_number,
            message=f".func argument {invalid_argument!r} is not a valid identifier",
            function_name=name,
        )
        return
    if len({argument.lower() for argument in arguments}) != len(arguments):
        _add_function_diagnostic(
            state,
            code="SPICE_DECK_FUNC_ARGUMENT",
            line_number=line_number,
            message=f".func {name!r} has duplicate argument names",
            function_name=name,
        )
        return
    expression = _strip_expression_delimiters(expression.strip())
    if not expression:
        _add_function_diagnostic(
            state,
            code="SPICE_DECK_FUNC_EXPRESSION",
            line_number=line_number,
            message=f".func {name!r} requires a non-empty expression",
            function_name=name,
        )
        return
    state.functions.append(
        DeckFunctionDefinition(
            name=name,
            arguments=tuple(arguments),
            expression=expression,
            line_number=line_number,
        )
    )


def _resolve_param_line(
    line: str,
    line_number: int,
    state: _DeckParameterState,
) -> None:
    tokens = _directive_tokens(line)
    if len(tokens) == 1:
        _add_parameter_diagnostic(
            state,
            code="SPICE_DECK_PARAM_ARGUMENT",
            directive=".param",
            line_number=line_number,
            message=".param requires at least one name=value assignment",
        )
        return

    for token in tokens[1:]:
        if "=" not in token:
            _add_parameter_diagnostic(
                state,
                code="SPICE_DECK_PARAM_ARGUMENT",
                directive=".param",
                line_number=line_number,
                message=f".param assignment {token!r} must use name=value syntax",
                parameter=token,
            )
            continue
        name, expression = token.split("=", 1)
        name = name.strip()
        expression = _strip_expression_delimiters(expression.strip())
        if not _is_parameter_name(name):
            _add_parameter_diagnostic(
                state,
                code="SPICE_DECK_PARAM_NAME",
                directive=".param",
                line_number=line_number,
                message=f".param name {name!r} is not a valid identifier",
                parameter=name,
                expression=expression,
            )
            continue
        try:
            value = _evaluate_parameter_expression(expression, state)
        except ValueError as error:
            _add_parameter_diagnostic(
                state,
                code="SPICE_DECK_PARAM_EXPRESSION",
                directive=".param",
                line_number=line_number,
                message=str(error),
                parameter=name,
                expression=expression,
            )
            continue
        state.set_parameter(name, value)


def _collect_parameter_functions(netlist: str, state: _DeckParameterState) -> None:
    function_state = _DeckFunctionState()
    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if directive == ".end":
            break
        if directive == ".func":
            _resolve_function_line(stripped, line_number, function_state)

    for definition in function_state.functions:
        state.set_function(definition)
    for diagnostic in function_state.diagnostics:
        _add_parameter_diagnostic(
            state,
            code=diagnostic.code,
            directive=diagnostic.directive,
            line_number=diagnostic.line_number,
            message=diagnostic.message,
            parameter=diagnostic.function_name,
            expression=diagnostic.expression,
        )


def _rewrite_parameter_expressions(
    line: str,
    line_number: int,
    state: _DeckParameterState,
) -> str:
    line = _replace_delimited_parameter_expressions(line, "{", "}", line_number, state)
    return _replace_delimited_parameter_expressions(line, "'", "'", line_number, state)


def _replace_delimited_parameter_expressions(
    line: str,
    open_token: str,
    close_token: str,
    line_number: int,
    state: _DeckParameterState,
) -> str:
    result: list[str] = []
    index = 0
    while index < len(line):
        if line[index] != open_token:
            result.append(line[index])
            index += 1
            continue
        close_index = line.find(close_token, index + 1)
        if close_index == -1:
            _add_parameter_diagnostic(
                state,
                code="SPICE_DECK_PARAM_UNTERMINATED",
                directive=".param",
                line_number=line_number,
                message=f"unterminated parameter expression starting at column {index + 1}",
            )
            result.append(line[index:])
            break
        expression = line[index + 1 : close_index].strip()
        try:
            value = _evaluate_parameter_expression(expression, state)
        except ValueError as error:
            _add_parameter_diagnostic(
                state,
                code="SPICE_DECK_PARAM_UNRESOLVED",
                directive=".param",
                line_number=line_number,
                message=str(error),
                expression=expression,
            )
            result.append(line[index : close_index + 1])
        else:
            result.append(_format_parameter_number(value))
        index = close_index + 1
    return "".join(result)


def _evaluate_parameter_expression(
    expression: str,
    state: _DeckParameterState,
) -> float:
    parser = _ParameterExpressionParser(expression, state.parameters, state.functions)
    value = parser.parse()
    if not isfinite(value):
        raise ValueError(f"parameter expression {expression!r} did not evaluate to a finite value")
    return value


class _ParameterExpressionParser:
    def __init__(
        self,
        expression: str,
        parameters: Mapping[str, DeckParameterValue],
        functions: Mapping[str, DeckFunctionDefinition],
        local_values: Mapping[str, float] | None = None,
        call_stack: tuple[str, ...] = (),
    ) -> None:
        self.expression = expression
        self.parameters = parameters
        self.functions = functions
        self.local_values = local_values or {}
        self.call_stack = call_stack
        self.index = 0

    def parse(self) -> float:
        if not self.expression:
            raise ValueError("parameter expression must not be empty")
        value = self._parse_expression()
        self._skip_whitespace()
        if self.index != len(self.expression):
            raise ValueError(
                f"unexpected token {self.expression[self.index]!r} in parameter expression"
            )
        return value

    def _parse_expression(self) -> float:
        value = self._parse_term()
        while True:
            self._skip_whitespace()
            if self._match("+"):
                value += self._parse_term()
            elif self._match("-"):
                value -= self._parse_term()
            else:
                return value

    def _parse_term(self) -> float:
        value = self._parse_power()
        while True:
            self._skip_whitespace()
            if self._match("*"):
                value *= self._parse_power()
            elif self._match("/"):
                denominator = self._parse_power()
                if denominator == 0.0:
                    raise ValueError("division by zero in parameter expression")
                value /= denominator
            else:
                return value

    def _parse_power(self) -> float:
        value = self._parse_unary()
        self._skip_whitespace()
        if self._match("^"):
            value **= self._parse_power()
        return value

    def _parse_unary(self) -> float:
        self._skip_whitespace()
        if self._match("+"):
            return self._parse_unary()
        if self._match("-"):
            return -self._parse_unary()
        return self._parse_primary()

    def _parse_primary(self) -> float:
        self._skip_whitespace()
        if self._match("("):
            value = self._parse_expression()
            self._skip_whitespace()
            if not self._match(")"):
                raise ValueError("missing ')' in parameter expression")
            return value
        if self.index >= len(self.expression):
            raise ValueError("unexpected end of parameter expression")
        char = self.expression[self.index]
        if char.isdigit() or char == ".":
            return self._parse_number()
        if char.isalpha() or char == "_":
            return self._parse_identifier()
        raise ValueError(f"unexpected token {char!r} in parameter expression")

    def _parse_number(self) -> float:
        start = self.index
        saw_digit = False
        while self.index < len(self.expression) and self.expression[self.index].isdigit():
            saw_digit = True
            self.index += 1
        if self.index < len(self.expression) and self.expression[self.index] == ".":
            self.index += 1
            while self.index < len(self.expression) and self.expression[self.index].isdigit():
                saw_digit = True
                self.index += 1
        if not saw_digit:
            raise ValueError("expected digit in numeric parameter expression")
        if self.index < len(self.expression) and self.expression[self.index] in {"e", "E"}:
            exponent_index = self.index
            self.index += 1
            if self.index < len(self.expression) and self.expression[self.index] in {"+", "-"}:
                self.index += 1
            exponent_start = self.index
            while self.index < len(self.expression) and self.expression[self.index].isdigit():
                self.index += 1
            if exponent_start == self.index:
                self.index = exponent_index
        numeric = float(self.expression[start : self.index])
        suffix_start = self.index
        while self.index < len(self.expression) and self.expression[self.index].isalpha():
            self.index += 1
        suffix = self.expression[suffix_start : self.index].lower()
        if not suffix:
            return numeric
        if suffix not in _SPICE_SUFFIX_FACTORS:
            raise ValueError(f"unsupported numeric suffix {suffix!r}")
        return numeric * _SPICE_SUFFIX_FACTORS[suffix]

    def _parse_identifier(self) -> float:
        start = self.index
        while self.index < len(self.expression) and (
            self.expression[self.index].isalnum() or self.expression[self.index] == "_"
        ):
            self.index += 1
        name = self.expression[start : self.index]
        self._skip_whitespace()
        if self.index < len(self.expression) and self.expression[self.index] == "(":
            return self._evaluate_function_call(name, self._parse_call_arguments())
        local = self.local_values.get(name.lower())
        if local is not None:
            return local
        if name.lower() == "pi":
            return pi
        parameter = self.parameters.get(name.lower())
        if parameter is None:
            raise ValueError(f"unknown parameter {name!r}")
        return parameter.value

    def _parse_call_arguments(self) -> list[float]:
        if not self._match("("):
            raise ValueError("expected '(' in function call")
        self._skip_whitespace()
        if self._match(")"):
            return []
        arguments: list[float] = []
        while True:
            arguments.append(self._parse_expression())
            self._skip_whitespace()
            if self._match(","):
                continue
            if self._match(")"):
                return arguments
            raise ValueError("missing ')' in function call")

    def _evaluate_function_call(self, name: str, values: Sequence[float]) -> float:
        definition = self.functions.get(name.lower())
        if definition is None:
            raise ValueError(f"unknown function {name!r}")
        if len(values) != len(definition.arguments):
            raise ValueError(
                f"function {name!r} expected {len(definition.arguments)} arguments but got {len(values)}"
            )
        key = definition.name.lower()
        if key in self.call_stack:
            raise ValueError(f"recursive function call {name!r}")
        local_values = dict(self.local_values)
        for argument, value in zip(definition.arguments, values, strict=True):
            local_values[argument.lower()] = value
        parser = _ParameterExpressionParser(
            definition.expression,
            self.parameters,
            self.functions,
            local_values,
            (*self.call_stack, key),
        )
        return parser.parse()

    def _skip_whitespace(self) -> None:
        while self.index < len(self.expression) and self.expression[self.index].isspace():
            self.index += 1

    def _match(self, token: str) -> bool:
        if self.expression.startswith(token, self.index):
            self.index += len(token)
            return True
        return False


def _add_parameter_diagnostic(
    state: _DeckParameterState,
    *,
    code: str,
    directive: str,
    line_number: int,
    message: str,
    parameter: str | None = None,
    expression: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckParameterDiagnostic(
            code=code,
            directive=directive,
            line_number=line_number,
            message=message,
            severity="error",
            parameter=parameter,
            expression=expression,
        )
    )


def _add_initial_condition_diagnostic(
    state: _DeckInitialConditionState,
    *,
    code: str,
    directive: str,
    line_number: int,
    message: str,
    token: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckInitialConditionDiagnostic(
            code=code,
            directive=directive,
            line_number=line_number,
            message=message,
            severity="error",
            token=token,
        )
    )


def _add_function_diagnostic(
    state: _DeckFunctionState,
    *,
    code: str,
    line_number: int,
    message: str,
    function_name: str | None = None,
    expression: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckFunctionDiagnostic(
            code=code,
            directive=".func",
            line_number=line_number,
            message=message,
            severity="error",
            function_name=function_name,
            expression=expression,
        )
    )


def _add_measurement_diagnostic(
    state: _DeckMeasurementState,
    *,
    code: str,
    directive: str,
    line_number: int,
    message: str,
    token: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckMeasurementDiagnostic(
            code=code,
            directive=directive,
            line_number=line_number,
            message=message,
            severity="error",
            token=token,
        )
    )


def _add_fourier_diagnostic(
    state: _DeckFourierState,
    *,
    code: str,
    line_number: int,
    message: str,
    token: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckFourierDiagnostic(
            code=code,
            directive=".four",
            line_number=line_number,
            message=message,
            severity="error",
            token=token,
        )
    )


def _add_output_diagnostic(
    state: _DeckOutputState,
    *,
    code: str,
    directive: str,
    line_number: int,
    message: str,
    token: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckOutputDiagnostic(
            code=code,
            directive=directive,
            line_number=line_number,
            message=message,
            severity="error",
            token=token,
        )
    )


def _add_analysis_diagnostic(
    state: _DeckAnalysisState,
    *,
    code: str,
    directive: str,
    line_number: int,
    message: str,
    token: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckAnalysisDiagnostic(
            code=code,
            directive=directive,
            line_number=line_number,
            message=message,
            severity="error",
            token=token,
        )
    )


def _parse_node_condition_target(target: str) -> str | None:
    if len(target) < 4 or not target.lower().startswith("v(") or not target.endswith(")"):
        return None
    node = target[2:-1].strip()
    return node or None


def _parse_function_signature(rest: str) -> tuple[str, list[str], str] | None:
    open_index = rest.find("(")
    if open_index < 0:
        return None
    close_index = rest.find(")", open_index + 1)
    if close_index < 0:
        return None
    name = rest[:open_index].strip()
    arguments_raw = rest[open_index + 1 : close_index].strip()
    expression = rest[close_index + 1 :].strip()
    arguments = [] if not arguments_raw else [item.strip() for item in arguments_raw.split(",")]
    return name, arguments, expression


def _is_parameter_name(name: str) -> bool:
    if not name or not (name[0].isalpha() or name[0] == "_"):
        return False
    return all(char.isalnum() or char == "_" for char in name)


def _resolve_measurement_line(
    line: str,
    line_number: int,
    directive: str,
    state: _DeckMeasurementState,
) -> None:
    tokens = _directive_tokens(line)
    if len(tokens) < 5:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=f"{directive} requires analysis, name, mode, and probe tokens",
        )
        return

    analysis = tokens[1].strip().lower()
    if analysis not in {"tran", "transient", "dc", "ac"}:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ANALYSIS",
            directive=directive,
            line_number=line_number,
            message=f"only transient, dc, and ac .measure cards are supported, got {tokens[1]!r}",
            token=tokens[1],
        )
        return

    name = tokens[2].strip()
    if not _is_parameter_name(name):
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_NAME",
            directive=directive,
            line_number=line_number,
            message=f"measurement name {name!r} is not a valid identifier",
            token=name,
        )
        return

    if tokens[3].strip().lower() == "trig":
        _resolve_measurement_delay_line(tokens, line_number, directive, state, analysis, name)
        return

    mode = _normalize_measurement_mode_token(tokens[3])
    if mode is None:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_MODE",
            directive=directive,
            line_number=line_number,
            message=f"unsupported measurement mode {tokens[3]!r}",
            token=tokens[3],
        )
        return

    empty_parameter_state = _DeckParameterState()
    target_value: float | None = None
    if mode == "when":
        if "=" not in tokens[4]:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message="WHEN measurements require probe=target syntax",
                token=tokens[4],
            )
            return
        probe_token, target_expression = tokens[4].split("=", 1)
        try:
            target_value = _evaluate_parameter_expression(
                _strip_expression_delimiters(target_expression.strip()),
                empty_parameter_state,
            )
        except ValueError as error:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_EXPRESSION",
                directive=directive,
                line_number=line_number,
                message=str(error),
                token=tokens[4],
            )
            return
        probe = _unquote_token(probe_token.strip())
    else:
        probe = _unquote_token(tokens[4].strip())
    if not probe:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_PROBE",
            directive=directive,
            line_number=line_number,
            message="measurement probe must not be empty",
            token=tokens[4],
        )
        return

    from_value: float | None = None
    to_value: float | None = None
    at_value: float | None = None
    crossing_kind: str | None = None
    crossing_count: int | None = None
    seen_window_tokens: set[str] = set()
    diagnostic_count = len(state.diagnostics)
    for token in tokens[5:]:
        if "=" not in token:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"measurement option {token!r} must use name=value syntax",
                token=token,
            )
            continue
        key, expression = token.split("=", 1)
        key = key.strip().lower()
        if key not in {"from", "to", "at", "rise", "fall", "cross"}:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"unsupported measurement option {key!r}",
                token=token,
            )
            continue
        if key in seen_window_tokens:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"duplicate measurement option {key!r}",
                token=token,
            )
            continue
        seen_window_tokens.add(key)
        try:
            value = _evaluate_parameter_expression(
                _strip_expression_delimiters(expression.strip()),
                empty_parameter_state,
            )
        except ValueError as error:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_EXPRESSION",
                directive=directive,
                line_number=line_number,
                message=str(error),
                token=token,
            )
            continue
        if key in {"rise", "fall", "cross"}:
            if mode != "when":
                _add_measurement_diagnostic(
                    state,
                    code="SPICE_DECK_MEASURE_ARGUMENT",
                    directive=directive,
                    line_number=line_number,
                    message="RISE, FALL, and CROSS options are only supported with WHEN mode",
                    token=token,
                )
                continue
            if crossing_kind is not None:
                _add_measurement_diagnostic(
                    state,
                    code="SPICE_DECK_MEASURE_ARGUMENT",
                    directive=directive,
                    line_number=line_number,
                    message="only one of RISE, FALL, or CROSS may be specified",
                    token=token,
                )
                continue
            if not isfinite(value) or value < 1.0 or not value.is_integer():
                _add_measurement_diagnostic(
                    state,
                    code="SPICE_DECK_MEASURE_ARGUMENT",
                    directive=directive,
                    line_number=line_number,
                    message="RISE, FALL, and CROSS counts must be positive integers",
                    token=token,
                )
                continue
            crossing_kind = key
            crossing_count = int(value)
            continue
        if key == "from":
            from_value = value
        elif key == "to":
            to_value = value
        else:
            at_value = value

    if mode == "find" and at_value is None:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message="FIND measurements require an AT value",
        )
    if mode == "when" and target_value is None:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message="WHEN measurements require a target value",
        )
    if mode != "find" and at_value is not None:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message="measurement AT value is only supported with FIND mode",
        )
    if at_value is not None and (from_value is not None or to_value is not None):
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message="measurement AT value cannot be combined with FROM or TO",
        )

    if from_value is not None and to_value is not None and from_value > to_value:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_WINDOW",
            directive=directive,
            line_number=line_number,
            message="measurement FROM value must be <= TO value",
        )

    if len(state.diagnostics) != diagnostic_count:
        return

    state.measurements.append(
        DeckMeasurementCard(
            directive=directive,
            analysis=analysis,
            name=name,
            mode=mode,
            probe=probe,
            line_number=line_number,
            from_value=from_value,
            to_value=to_value,
            at_value=at_value,
            target_value=target_value,
            crossing_kind=crossing_kind,
            crossing_count=crossing_count,
            trigger_probe=None,
            trigger_value=None,
            trigger_crossing_kind=None,
            trigger_crossing_count=None,
        )
    )


def _resolve_fourier_line(
    line: str,
    line_number: int,
    state: _DeckFourierState,
) -> None:
    tokens = _directive_tokens(line)
    if len(tokens) < 3:
        _add_fourier_diagnostic(
            state,
            code="SPICE_DECK_FOURIER_ARGUMENT",
            line_number=line_number,
            message=".four requires a fundamental frequency and at least one probe",
        )
        return

    empty_parameter_state = _DeckParameterState()
    try:
        fundamental_frequency = _evaluate_parameter_expression(
            _strip_expression_delimiters(tokens[1].strip()),
            empty_parameter_state,
        )
    except ValueError as error:
        _add_fourier_diagnostic(
            state,
            code="SPICE_DECK_FOURIER_EXPRESSION",
            line_number=line_number,
            message=str(error),
            token=tokens[1],
        )
        return
    if not isfinite(fundamental_frequency) or fundamental_frequency <= 0.0:
        _add_fourier_diagnostic(
            state,
            code="SPICE_DECK_FOURIER_FREQUENCY",
            line_number=line_number,
            message=".four fundamental frequency must be finite and positive",
            token=tokens[1],
        )
        return

    probes: list[str] = []
    harmonics: int | None = None
    from_value: float | None = None
    seen_options: set[str] = set()
    diagnostic_count = len(state.diagnostics)
    for token in tokens[2:]:
        if "=" in token:
            key, expression = token.split("=", 1)
            key = key.strip().lower()
            if key not in {"harmonics", "from"}:
                _add_fourier_diagnostic(
                    state,
                    code="SPICE_DECK_FOURIER_ARGUMENT",
                    line_number=line_number,
                    message=f"unsupported .four option {key!r}",
                    token=token,
                )
                continue
            if key in seen_options:
                _add_fourier_diagnostic(
                    state,
                    code="SPICE_DECK_FOURIER_ARGUMENT",
                    line_number=line_number,
                    message=f"duplicate .four option {key!r}",
                    token=token,
                )
                continue
            seen_options.add(key)
            try:
                value = _evaluate_parameter_expression(
                    _strip_expression_delimiters(expression.strip()),
                    empty_parameter_state,
                )
            except ValueError as error:
                _add_fourier_diagnostic(
                    state,
                    code="SPICE_DECK_FOURIER_EXPRESSION",
                    line_number=line_number,
                    message=str(error),
                    token=token,
                )
                continue
            if key == "harmonics":
                if not isfinite(value) or value < 1.0 or not value.is_integer():
                    _add_fourier_diagnostic(
                        state,
                        code="SPICE_DECK_FOURIER_ARGUMENT",
                        line_number=line_number,
                        message=".four HARMONICS value must be a positive integer",
                        token=token,
                    )
                    continue
                harmonics = int(value)
            else:
                from_value = value
            continue
        probe = _unquote_token(token.strip())
        if not probe:
            _add_fourier_diagnostic(
                state,
                code="SPICE_DECK_FOURIER_PROBE",
                line_number=line_number,
                message=".four probe must not be empty",
                token=token,
            )
            continue
        probes.append(probe)

    if not probes and len(state.diagnostics) == diagnostic_count:
        _add_fourier_diagnostic(
            state,
            code="SPICE_DECK_FOURIER_PROBE",
            line_number=line_number,
            message=".four requires at least one probe",
        )
    if from_value is not None and not isfinite(from_value):
        _add_fourier_diagnostic(
            state,
            code="SPICE_DECK_FOURIER_WINDOW",
            line_number=line_number,
            message=".four FROM value must be finite",
        )

    if len(state.diagnostics) != diagnostic_count:
        return

    state.fourier.append(
        DeckFourierCard(
            directive=".four",
            fundamental_frequency=fundamental_frequency,
            probes=tuple(probes),
            line_number=line_number,
            harmonics=harmonics,
            from_value=from_value,
        )
    )


def _resolve_output_line(
    line: str,
    line_number: int,
    directive: str,
    state: _DeckOutputState,
) -> None:
    tokens = _directive_tokens(line)
    if len(tokens) < 2:
        message = (
            f"{directive} requires an analysis token and at least one probe token"
            if directive in {".print", ".plot"}
            else f"{directive} requires at least one probe token"
        )
        _add_output_diagnostic(
            state,
            code="SPICE_DECK_OUTPUT_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=message,
        )
        return

    analysis: str | None = None
    probe_tokens = tokens[1:]
    if directive in {".print", ".plot"}:
        if len(tokens) < 3:
            _add_output_diagnostic(
                state,
                code="SPICE_DECK_OUTPUT_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"{directive} requires an analysis token and at least one probe token",
            )
            return
        analysis = _normalize_deck_output_analysis(tokens[1])
        if analysis is None:
            _add_output_diagnostic(
                state,
                code="SPICE_DECK_OUTPUT_ANALYSIS",
                directive=directive,
                line_number=line_number,
                message=f"{directive} analysis must be op, dc, ac, or tran, got {tokens[1]!r}",
                token=tokens[1],
            )
            return
        probe_tokens = tokens[2:]
    elif directive == ".probe" and _normalize_deck_output_analysis(tokens[1]) is not None:
        analysis = _normalize_deck_output_analysis(tokens[1])
        probe_tokens = tokens[2:]
    if not probe_tokens:
        _add_output_diagnostic(
            state,
            code="SPICE_DECK_OUTPUT_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=f"{directive} requires at least one probe token",
        )
        return

    probes: list[str] = []
    for token in probe_tokens:
        text = _unquote_token(token)
        probe = _normalize_deck_output_probe(text)
        if probe is None:
            _add_output_diagnostic(
                state,
                code="SPICE_DECK_OUTPUT_PROBE",
                directive=directive,
                line_number=line_number,
                message=f"{directive} probe must be V(node) or I(source), got {text!r}",
                token=text,
            )
            continue
        probes.append(probe)
    if not probes:
        return

    state.selections.append(
        DeckOutputSelection(
            directive=directive,
            analysis=analysis,
            probes=tuple(probes),
            line_number=line_number,
        )
    )


def _resolve_analysis_line(
    line: str,
    line_number: int,
    directive: str,
    state: _DeckAnalysisState,
) -> None:
    tokens = _directive_tokens(line)
    if directive == ".op":
        _resolve_op_analysis(tokens, line_number, state)
    elif directive == ".dc":
        _resolve_dc_analysis(tokens, line_number, state)
    elif directive == ".ac":
        _resolve_ac_analysis(tokens, line_number, state)
    elif directive == ".tran":
        _resolve_tran_analysis(tokens, line_number, state)


def _resolve_op_analysis(
    tokens: list[str],
    line_number: int,
    state: _DeckAnalysisState,
) -> None:
    if len(tokens) != 1:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=".op",
            line_number=line_number,
            message=".op does not accept analysis arguments",
            token=tokens[1],
        )
        return
    state.analyses.append(DeckAnalysisPlan(".op", "op", line_number))


def _resolve_dc_analysis(
    tokens: list[str],
    line_number: int,
    state: _DeckAnalysisState,
) -> None:
    if len(tokens) != 5:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=".dc",
            line_number=line_number,
            message=".dc requires source, start, stop, and step tokens",
        )
        return
    source_name = _unquote_token(tokens[1]).strip()
    if not source_name:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=".dc",
            line_number=line_number,
            message=".dc source name must not be empty",
            token=tokens[1],
        )
        return
    start_value = _parse_deck_analysis_value(tokens[2], ".dc", line_number, state)
    stop_value = _parse_deck_analysis_value(tokens[3], ".dc", line_number, state)
    step_value = _parse_deck_analysis_value(tokens[4], ".dc", line_number, state)
    if start_value is None or stop_value is None or step_value is None:
        return
    if step_value == 0.0:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_SWEEP",
            directive=".dc",
            line_number=line_number,
            message=".dc step value must be non-zero",
            token=tokens[4],
        )
        return
    if (start_value < stop_value and step_value < 0.0) or (
        start_value > stop_value and step_value > 0.0
    ):
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_SWEEP",
            directive=".dc",
            line_number=line_number,
            message=".dc step direction must move from start toward stop",
            token=tokens[4],
        )
        return
    state.analyses.append(
        DeckAnalysisPlan(
            directive=".dc",
            analysis="dc",
            line_number=line_number,
            source_name=source_name,
            start_value=start_value,
            stop_value=stop_value,
            step_value=step_value,
        )
    )


def _resolve_ac_analysis(
    tokens: list[str],
    line_number: int,
    state: _DeckAnalysisState,
) -> None:
    if len(tokens) != 5:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=".ac",
            line_number=line_number,
            message=".ac requires sweep kind, point count, start frequency, and stop frequency",
        )
        return
    sweep_kind = _normalize_ac_sweep_kind(tokens[1])
    if sweep_kind is None:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_MODE",
            directive=".ac",
            line_number=line_number,
            message=f".ac sweep kind must be LIN, DEC, or OCT, got {tokens[1]!r}",
            token=tokens[1],
        )
        return
    point_count = _parse_deck_analysis_integer(tokens[2], ".ac", line_number, state)
    start_frequency = _parse_deck_analysis_value(tokens[3], ".ac", line_number, state)
    stop_frequency = _parse_deck_analysis_value(tokens[4], ".ac", line_number, state)
    if point_count is None or start_frequency is None or stop_frequency is None:
        return
    if point_count < 1:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_SWEEP",
            directive=".ac",
            line_number=line_number,
            message=".ac point count must be a positive integer",
            token=tokens[2],
        )
        return
    if start_frequency <= 0.0 or stop_frequency <= 0.0 or stop_frequency < start_frequency:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_SWEEP",
            directive=".ac",
            line_number=line_number,
            message=".ac frequencies must be positive and stop must be >= start",
        )
        return
    state.analyses.append(
        DeckAnalysisPlan(
            directive=".ac",
            analysis="ac",
            line_number=line_number,
            sweep_kind=sweep_kind,
            point_count=point_count,
            start_frequency=start_frequency,
            stop_frequency=stop_frequency,
        )
    )


def _resolve_tran_analysis(
    tokens: list[str],
    line_number: int,
    state: _DeckAnalysisState,
) -> None:
    if len(tokens) < 3:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=".tran",
            line_number=line_number,
            message=".tran requires step time and stop time",
        )
        return
    use_initial_conditions = False
    numeric_tokens: list[str] = []
    for token in tokens[3:]:
        if token.strip().lower() == "uic":
            use_initial_conditions = True
            continue
        numeric_tokens.append(token)
    if len(numeric_tokens) > 2:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=".tran",
            line_number=line_number,
            message=".tran supports optional start time, max step, and UIC only",
            token=numeric_tokens[2],
        )
        return
    step_time = _parse_deck_analysis_value(tokens[1], ".tran", line_number, state)
    stop_time = _parse_deck_analysis_value(tokens[2], ".tran", line_number, state)
    start_time = (
        _parse_deck_analysis_value(numeric_tokens[0], ".tran", line_number, state)
        if len(numeric_tokens) >= 1
        else None
    )
    max_step = (
        _parse_deck_analysis_value(numeric_tokens[1], ".tran", line_number, state)
        if len(numeric_tokens) >= 2
        else None
    )
    if step_time is None or stop_time is None:
        return
    if (len(numeric_tokens) >= 1 and start_time is None) or (
        len(numeric_tokens) >= 2 and max_step is None
    ):
        return
    if step_time <= 0.0 or stop_time <= 0.0:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_INTERVAL",
            directive=".tran",
            line_number=line_number,
            message=".tran step time and stop time must be positive",
        )
        return
    if start_time is not None and (start_time < 0.0 or start_time > stop_time):
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_INTERVAL",
            directive=".tran",
            line_number=line_number,
            message=".tran start time must be non-negative and <= stop time",
        )
        return
    if max_step is not None and max_step <= 0.0:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_INTERVAL",
            directive=".tran",
            line_number=line_number,
            message=".tran max step must be positive",
        )
        return
    state.analyses.append(
        DeckAnalysisPlan(
            directive=".tran",
            analysis="tran",
            line_number=line_number,
            step_time=step_time,
            stop_time=stop_time,
            start_time=start_time,
            max_step=max_step,
            use_initial_conditions=use_initial_conditions,
        )
    )


def _parse_deck_analysis_value(
    token: str,
    directive: str,
    line_number: int,
    state: _DeckAnalysisState,
) -> float | None:
    try:
        return _evaluate_parameter_expression(
            _strip_expression_delimiters(_unquote_token(token).strip()),
            _DeckParameterState(),
        )
    except ValueError as error:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_EXPRESSION",
            directive=directive,
            line_number=line_number,
            message=str(error),
            token=token,
        )
        return None


def _parse_deck_analysis_integer(
    token: str,
    directive: str,
    line_number: int,
    state: _DeckAnalysisState,
) -> int | None:
    value = _parse_deck_analysis_value(token, directive, line_number, state)
    if value is None:
        return None
    if value < 0.0 or value % 1.0 != 0.0:
        _add_analysis_diagnostic(
            state,
            code="SPICE_DECK_ANALYSIS_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=f"{directive} point count must be an integer",
            token=token,
        )
        return None
    return int(value)


def _normalize_ac_sweep_kind(token: str) -> str | None:
    normalized = token.strip().lower()
    if normalized in {"lin", "dec", "oct"}:
        return normalized
    return None


@dataclass(frozen=True, slots=True)
class _ParsedMeasurementEdge:
    probe: str
    value: float
    crossing_kind: str | None
    crossing_count: int | None


def _resolve_measurement_delay_line(
    tokens: list[str],
    line_number: int,
    directive: str,
    state: _DeckMeasurementState,
    analysis: str,
    name: str,
) -> None:
    if analysis not in {"tran", "transient"}:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message="TRIG/TARG measurements are only supported for transient analysis",
            token=tokens[3],
        )
        return
    try:
        target_index = next(
            index
            for index, token in enumerate(tokens[4:], start=4)
            if token.strip().lower() == "targ"
        )
    except StopIteration:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message="TRIG measurements require a TARG section",
        )
        return

    empty_parameter_state = _DeckParameterState()
    trigger = _parse_measurement_delay_edge(
        tokens[4:target_index],
        "TRIG",
        directive,
        line_number,
        state,
        empty_parameter_state,
    )
    if trigger is None:
        return
    target_result = _parse_measurement_delay_target_section(
        tokens[target_index + 1 :],
        directive,
        line_number,
        state,
        empty_parameter_state,
    )
    if target_result is None:
        return
    target, from_value, to_value = target_result
    if from_value is not None and to_value is not None and from_value > to_value:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_WINDOW",
            directive=directive,
            line_number=line_number,
            message="measurement FROM value must be <= TO value",
        )
        return

    state.measurements.append(
        DeckMeasurementCard(
            directive=directive,
            analysis=analysis,
            name=name,
            mode="delay",
            probe=target.probe,
            line_number=line_number,
            from_value=from_value,
            to_value=to_value,
            at_value=None,
            target_value=target.value,
            crossing_kind=target.crossing_kind,
            crossing_count=target.crossing_count,
            trigger_probe=trigger.probe,
            trigger_value=trigger.value,
            trigger_crossing_kind=trigger.crossing_kind,
            trigger_crossing_count=trigger.crossing_count,
        )
    )


def _parse_measurement_delay_target_section(
    tokens: list[str],
    directive: str,
    line_number: int,
    state: _DeckMeasurementState,
    parameter_state: _DeckParameterState,
) -> tuple[_ParsedMeasurementEdge, float | None, float | None] | None:
    edge_tokens: list[str] = []
    from_value: float | None = None
    to_value: float | None = None
    seen_window_tokens: set[str] = set()
    for token in tokens:
        if "=" not in token:
            edge_tokens.append(token)
            continue
        key, expression = token.split("=", 1)
        key = key.strip().lower()
        if key not in {"from", "to"}:
            edge_tokens.append(token)
            continue
        if key in seen_window_tokens:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"duplicate measurement option {key!r}",
                token=token,
            )
            return None
        seen_window_tokens.add(key)
        try:
            value = _evaluate_parameter_expression(
                _strip_expression_delimiters(expression.strip()),
                parameter_state,
            )
        except ValueError as error:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_EXPRESSION",
                directive=directive,
                line_number=line_number,
                message=str(error),
                token=token,
            )
            return None
        if key == "from":
            from_value = value
        else:
            to_value = value
    edge = _parse_measurement_delay_edge(
        edge_tokens,
        "TARG",
        directive,
        line_number,
        state,
        parameter_state,
    )
    if edge is None:
        return None
    return edge, from_value, to_value


def _parse_measurement_delay_edge(
    tokens: list[str],
    section: str,
    directive: str,
    line_number: int,
    state: _DeckMeasurementState,
    parameter_state: _DeckParameterState,
) -> _ParsedMeasurementEdge | None:
    if not tokens:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=f"{section} measurements require a probe target",
        )
        return None
    value: float | None = None
    first = tokens[0]
    if "=" in first:
        probe_token, expression = first.split("=", 1)
        try:
            value = _evaluate_parameter_expression(
                _strip_expression_delimiters(expression.strip()),
                parameter_state,
            )
        except ValueError as error:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_EXPRESSION",
                directive=directive,
                line_number=line_number,
                message=str(error),
                token=first,
            )
            return None
        probe = _unquote_token(probe_token.strip())
    else:
        probe = _unquote_token(first.strip())
    if not probe:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_PROBE",
            directive=directive,
            line_number=line_number,
            message=f"{section} measurement probe must not be empty",
            token=first,
        )
        return None

    crossing_kind: str | None = None
    crossing_count: int | None = None
    seen_tokens: set[str] = set()
    for token in tokens[1:]:
        if "=" not in token:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"{section} measurement option {token!r} must use name=value syntax",
                token=token,
            )
            return None
        key, expression = token.split("=", 1)
        key = key.strip().lower()
        if key not in {"val", "rise", "fall", "cross"}:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"unsupported {section} measurement option {key!r}",
                token=token,
            )
            return None
        if key in seen_tokens:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_ARGUMENT",
                directive=directive,
                line_number=line_number,
                message=f"duplicate {section} measurement option {key!r}",
                token=token,
            )
            return None
        seen_tokens.add(key)
        try:
            parsed = _evaluate_parameter_expression(
                _strip_expression_delimiters(expression.strip()),
                parameter_state,
            )
        except ValueError as error:
            _add_measurement_diagnostic(
                state,
                code="SPICE_DECK_MEASURE_EXPRESSION",
                directive=directive,
                line_number=line_number,
                message=str(error),
                token=token,
            )
            return None
        if key == "val":
            value = parsed
        else:
            if crossing_kind is not None:
                _add_measurement_diagnostic(
                    state,
                    code="SPICE_DECK_MEASURE_ARGUMENT",
                    directive=directive,
                    line_number=line_number,
                    message=f"only one {section} RISE, FALL, or CROSS option may be specified",
                    token=token,
                )
                return None
            if not isfinite(parsed) or parsed < 1.0 or not parsed.is_integer():
                _add_measurement_diagnostic(
                    state,
                    code="SPICE_DECK_MEASURE_ARGUMENT",
                    directive=directive,
                    line_number=line_number,
                    message=f"{section} RISE, FALL, and CROSS counts must be positive integers",
                    token=token,
                )
                return None
            crossing_kind = key
            crossing_count = int(parsed)
    if value is None:
        _add_measurement_diagnostic(
            state,
            code="SPICE_DECK_MEASURE_ARGUMENT",
            directive=directive,
            line_number=line_number,
            message=f"{section} measurements require a VAL value or probe=value target",
        )
        return None
    return _ParsedMeasurementEdge(probe, value, crossing_kind, crossing_count)


def _normalize_measurement_mode_token(mode: str) -> str | None:
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
        "find": "find",
        "when": "when",
    }
    normalized = aliases.get(normalized, normalized)
    return (
        normalized
        if normalized in {"max", "min", "avg", "rms", "pp", "last", "find", "when"}
        else None
    )


def _normalize_deck_output_analysis(analysis: str) -> str | None:
    normalized = analysis.strip().lower()
    if normalized in {"op", "dcop"}:
        return "op"
    if normalized == "dc":
        return "dc"
    if normalized == "ac":
        return "ac"
    if normalized in {"tran", "transient"}:
        return "tran"
    return None


def _normalize_deck_analysis_name(analysis: str) -> str | None:
    normalized = analysis.strip().lower().removeprefix(".").replace("_", "-")
    if normalized in {"op", "dcop", "operating-point", "operatingpoint"}:
        return "op"
    if normalized in {"dc", "dc-sweep", "dcsweep"}:
        return "dc"
    if normalized in {"ac", "ac-sweep", "acsweep"}:
        return "ac"
    if normalized in {"tran", "transient"}:
        return "tran"
    return None


def _deck_output_analysis_matches(requested: str, analysis: str) -> bool:
    return _normalize_deck_output_analysis(requested) == _normalize_deck_output_analysis(analysis)


def _normalize_deck_output_probe(token: str) -> str | None:
    text = token.strip()
    if not text.endswith(")"):
        return None
    lower = text.lower()
    if lower.startswith("v("):
        prefix = "V"
    elif lower.startswith("i("):
        prefix = "I"
    else:
        return None
    target = text[2:-1].strip()
    if (
        not target
        or "(" in target
        or ")" in target
        or "," in target
        or any(character.isspace() for character in target)
    ):
        return None
    return f"{prefix}({target})"


def _deck_output_probe_key(probe: str) -> str:
    return probe.lower()


def _strip_expression_delimiters(expression: str) -> str:
    if len(expression) >= 2 and (
        (expression[0] == "{" and expression[-1] == "}")
        or (expression[0] == "'" and expression[-1] == "'")
    ):
        return expression[1:-1].strip()
    return expression


def _format_parameter_number(value: float) -> str:
    if value == 0.0:
        return "0"
    abs_value = abs(value)
    if 1.0e-12 <= abs_value < 1.0e12:
        formatted = f"{value:.12f}".rstrip("0").rstrip(".")
        return "0" if formatted == "-0" else formatted
    mantissa, exponent = f"{value:.12e}".split("e")
    mantissa = mantissa.rstrip("0").rstrip(".")
    exponent_value = int(exponent)
    return f"{mantissa}e{exponent_value:+d}"


def _resolve_deck_lines(
    *,
    netlist: str,
    source: str,
    sources: Mapping[str, str],
    state: _DeckResolutionState,
    stack: tuple[str, ...],
) -> tuple[list[str], bool, int | None]:
    active_lines: list[str] = []
    end_line_number: int | None = None
    in_control_block = False

    for line_number, raw_line in enumerate(netlist.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            continue
        directive = _deck_directive(stripped)
        if in_control_block:
            if directive == ".endc":
                in_control_block = False
                continue
            control_line = _control_block_command_as_deck_line(stripped)
            if control_line is not None:
                active_lines.append(control_line)
                continue
            state.diagnostics.append(
                DeckResolutionDiagnostic(
                    code="SPICE_DECK_CONTROL_COMMAND",
                    directive=".control",
                    source=source,
                    line_number=line_number,
                    message=(
                        f"{stripped!r} inside .control is not executed by "
                        "the deck source resolver yet"
                    ),
                    severity="error",
                )
            )
            continue
        if directive == ".end":
            end_line_number = line_number
            break
        if directive == ".include":
            active_lines.extend(
                _resolve_include(
                    line=stripped,
                    source=source,
                    line_number=line_number,
                    sources=sources,
                    state=state,
                    stack=stack,
                )
            )
            continue
        if directive == ".lib":
            active_lines.extend(
                _resolve_library_section(
                    line=stripped,
                    source=source,
                    line_number=line_number,
                    sources=sources,
                    state=state,
                    stack=stack,
                )
            )
            continue
        if directive in _UNSUPPORTED_RESOLVED_DIRECTIVES:
            state.diagnostics.append(
                DeckResolutionDiagnostic(
                    code="SPICE_DECK_UNSUPPORTED_DIRECTIVE",
                    directive=directive,
                    source=source,
                    line_number=line_number,
                    message=(
                        f"{directive} is not supported by the deck source "
                        "resolver yet"
                    ),
                    severity="error",
                )
            )
            if directive == ".control":
                in_control_block = True
                continue
        active_lines.append(stripped)

    return active_lines, end_line_number is not None, end_line_number


def _resolve_include(
    *,
    line: str,
    source: str,
    line_number: int,
    sources: Mapping[str, str],
    state: _DeckResolutionState,
    stack: tuple[str, ...],
) -> list[str]:
    tokens = _directive_tokens(line)
    target = _unquote_token(tokens[1]) if len(tokens) >= 2 else None
    if not target:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_INCLUDE_ARGUMENT",
            directive=".include",
            source=source,
            line_number=line_number,
            message=".include requires a source path",
        )
        return []
    if target in stack:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_INCLUDE_CYCLE",
            directive=".include",
            source=source,
            line_number=line_number,
            message=f".include cycle detected for {target}",
            target=target,
        )
        return []
    content = sources.get(target)
    if content is None:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_INCLUDE_NOT_FOUND",
            directive=".include",
            source=source,
            line_number=line_number,
            message=f".include source {target!r} was not provided",
            target=target,
        )
        return []

    state.included_paths.append(target)
    resolved, _, _ = _resolve_deck_lines(
        netlist=content,
        source=target,
        sources=sources,
        state=state,
        stack=(*stack, target),
    )
    return resolved


def _resolve_library_section(
    *,
    line: str,
    source: str,
    line_number: int,
    sources: Mapping[str, str],
    state: _DeckResolutionState,
    stack: tuple[str, ...],
) -> list[str]:
    tokens = _directive_tokens(line)
    path = _unquote_token(tokens[1]) if len(tokens) >= 2 else None
    section = _unquote_token(tokens[2]) if len(tokens) >= 3 else None
    if not path or not section:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_LIB_ARGUMENT",
            directive=".lib",
            source=source,
            line_number=line_number,
            message=".lib requires a source path and section name",
            target=path,
        )
        return []
    content = sources.get(path)
    target = f"{path}:{section}"
    if content is None:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_LIB_NOT_FOUND",
            directive=".lib",
            source=source,
            line_number=line_number,
            message=f".lib source {path!r} was not provided",
            target=target,
        )
        return []
    if target in stack:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_LIB_CYCLE",
            directive=".lib",
            source=source,
            line_number=line_number,
            message=f".lib cycle detected for {target}",
            target=target,
        )
        return []

    section_lines = _extract_library_section(
        content=content,
        path=path,
        section=section,
        call_source=source,
        call_line_number=line_number,
        state=state,
    )
    if section_lines is None:
        return []

    state.library_sections.append(target)
    resolved, _, _ = _resolve_deck_lines(
        netlist="\n".join(section_lines),
        source=target,
        sources=sources,
        state=state,
        stack=(*stack, target),
    )
    return resolved


def _extract_library_section(
    *,
    content: str,
    path: str,
    section: str,
    call_source: str,
    call_line_number: int,
    state: _DeckResolutionState,
) -> list[str] | None:
    in_section = False
    section_start_line: int | None = None
    section_lines: list[str] = []
    wanted = section.lower()
    target = f"{path}:{section}"

    for line_number, raw_line in enumerate(content.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith(("*", ";")):
            if in_section:
                section_lines.append(raw_line)
            continue
        directive = _deck_directive(stripped)
        tokens = _directive_tokens(stripped)
        if not in_section:
            if (
                directive == ".lib"
                and len(tokens) >= 2
                and _unquote_token(tokens[1]).lower() == wanted
            ):
                in_section = True
                section_start_line = line_number
            continue
        if directive in {".endl", ".endlib"}:
            return section_lines
        section_lines.append(raw_line)

    if not in_section:
        _add_resolution_diagnostic(
            state,
            code="SPICE_DECK_LIB_SECTION_NOT_FOUND",
            directive=".lib",
            source=call_source,
            line_number=call_line_number,
            message=f".lib section {section!r} was not found in {path!r}",
            target=target,
        )
        return None

    _add_resolution_diagnostic(
        state,
        code="SPICE_DECK_LIB_SECTION_UNTERMINATED",
        directive=".lib",
        source=path,
        line_number=section_start_line or 1,
        message=f".lib section {section!r} in {path!r} is missing .endl",
        target=target,
    )
    return None


def _add_resolution_diagnostic(
    state: _DeckResolutionState,
    *,
    code: str,
    directive: str,
    source: str,
    line_number: int,
    message: str,
    target: str | None = None,
) -> None:
    state.diagnostics.append(
        DeckResolutionDiagnostic(
            code=code,
            directive=directive,
            source=source,
            line_number=line_number,
            message=message,
            severity="error",
            target=target,
        )
    )


def _directive_tokens(line: str) -> list[str]:
    return line.split()


def _unquote_token(token: str) -> str:
    if len(token) >= 2 and token[0] == token[-1] and token[0] in {"'", '"'}:
        return token[1:-1]
    return token


def _deck_directive(line: str) -> str | None:
    if not line.startswith("."):
        return None
    return line.split(None, 1)[0].lower()


def _control_block_command_as_deck_line(line: str) -> str | None:
    parts = line.split(maxsplit=1)
    if not parts:
        return None
    command = parts[0].lower()
    if command not in _SUPPORTED_CONTROL_BLOCK_COMMANDS:
        return None
    if command in {"four", ".four", "fourier", ".fourier"}:
        directive = ".four"
    else:
        directive = command if command.startswith(".") else f".{command}"
    if len(parts) == 1:
        return directive
    return f"{directive} {parts[1]}"
