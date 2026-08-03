"""Parser for a practical first slice of SPICE3 netlists."""

from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from dataclasses import fields as dataclass_fields
from typing import Literal

from mosfet_models import MOSFET, Level1Model, Level1Params, MosfetType
from spice_engine import (
    BJT,
    CCCS,
    CCVS,
    JFET,
    VCCS,
    VCVS,
    AcResult,
    AcSource,
    Capacitor,
    Circuit,
    CurrentSource,
    DcResult,
    DcSweepResult,
    Diode,
    ExpWaveform,
    Inductor,
    Mosfet,
    MutualInductor,
    PulseWaveform,
    PwlWaveform,
    Resistor,
    SinWaveform,
    TransientResult,
    TransmissionLine,
    VoltageSource,
    Waveform,
    ac_sweep,
    dc_op,
    dc_sweep,
    mosfet_from_model_card,
    normalize_model_card,
    transient,
)


class NetlistParseError(ValueError):
    """Raised when a SPICE netlist line is syntactically unsupported."""


@dataclass(frozen=True, slots=True)
class OpAnalysis:
    """A `.op` operating-point analysis card."""


@dataclass(frozen=True, slots=True)
class TranAnalysis:
    """A `.tran tstep tstop [method=<euler|trap|gear2>]` transient card."""

    t_step: float
    t_stop: float
    method: TransientMethod | None = None


@dataclass(frozen=True, slots=True)
class DcAnalysis:
    """A `.dc source start stop step` analysis card."""

    source_name: str
    start: float
    stop: float
    step: float


@dataclass(frozen=True, slots=True)
class AcAnalysis:
    """A `.ac mode points start stop` analysis card."""

    mode: str
    points: int
    start_hz: float
    stop_hz: float


@dataclass(frozen=True, slots=True)
class TfAnalysis:
    """A `.tf V(output_node) input_source` transfer-function analysis card."""

    output_node: str
    input_source: str


@dataclass(frozen=True, slots=True)
class SensAnalysis:
    """A `.sens V(output_node)` DC sensitivity analysis card."""

    output_node: str


@dataclass(frozen=True, slots=True)
class McAnalysis:
    """A `.mc V(output_node) n_trials [tolerance] [distribution] [seed]` card."""

    output_node: str
    n_trials: int
    tolerance: float = 0.05
    distribution: str = "gaussian"
    seed: int | None = None


@dataclass(frozen=True, slots=True)
class NoiseAnalysis:
    """A `.noise V(output_node) input_source [freq ...] [temp=<kelvin>]` card."""

    output_node: str
    input_source: str
    freqs: tuple[float, ...] = ()
    temperature: float = 300.0
    temperature_is_explicit: bool = False


@dataclass(frozen=True, slots=True)
class TempAnalysis:
    """A `.temp <celsius> [celsius ...]` operating-temperature card."""

    temperatures_celsius: tuple[float, ...]


@dataclass(frozen=True, slots=True)
class OutputProbe:
    """A voltage-node or branch-current output probe."""

    kind: Literal["voltage", "current"]
    target: str


@dataclass(frozen=True, slots=True)
class PrintAnalysis:
    """A `.print <analysis> <V(node)|I(source)>...` output card."""

    analysis: str
    probes: tuple[OutputProbe, ...]


@dataclass(frozen=True, slots=True)
class PlotAnalysis:
    """A `.plot <analysis> <V(node)|I(source)>...` output card."""

    analysis: str
    probes: tuple[OutputProbe, ...]


@dataclass(frozen=True, slots=True)
class SaveAnalysis:
    """A `.save <V(node)|I(source)>...` persistent output-selection card."""

    probes: tuple[OutputProbe, ...]


@dataclass(frozen=True, slots=True)
class ProbeAnalysis:
    """A `.probe [analysis] <V(node)|I(source)>...` output-selection card."""

    analysis: str | None
    probes: tuple[OutputProbe, ...]


type MeasureOperation = Literal["find", "max", "min", "avg", "rms"]


@dataclass(frozen=True, slots=True)
class MeasureAnalysis:
    """A `.measure <analysis> <name> <operation> <probe> ...` card."""

    analysis: str
    name: str
    operation: MeasureOperation
    probe: OutputProbe
    at: float | None = None
    start: float | None = None
    stop: float | None = None


@dataclass(frozen=True, slots=True)
class FourAnalysis:
    """A `.four <frequency> <V(node)|I(source)>...` Fourier-analysis card."""

    frequency_hz: float
    probes: tuple[OutputProbe, ...]


@dataclass(frozen=True, slots=True)
class DistortionAnalysis:
    """A `.disto mode points start stop <V(node)|I(source)>...` card."""

    mode: str
    points: int
    start_hz: float
    stop_hz: float
    probes: tuple[OutputProbe, ...]


@dataclass(frozen=True, slots=True)
class PoleZeroAnalysis:
    """A `.pz V(output_node) input_source [pole|zero|pz]` card."""

    output_node: str
    input_source: str
    kind: Literal["pole", "zero", "pz"] = "pz"


type OptionValue = float | str | bool
type TransientMethod = Literal["euler", "trap", "gear2"]


@dataclass(frozen=True, slots=True)
class OptionsAnalysis:
    """A `.options key=value ...` simulator-options card."""

    values: dict[str, OptionValue] = field(default_factory=dict)


type Analysis = (
    OpAnalysis
    | TranAnalysis
    | DcAnalysis
    | AcAnalysis
    | TfAnalysis
    | SensAnalysis
    | McAnalysis
    | NoiseAnalysis
    | TempAnalysis
    | PrintAnalysis
    | PlotAnalysis
    | SaveAnalysis
    | ProbeAnalysis
    | MeasureAnalysis
    | FourAnalysis
    | DistortionAnalysis
    | PoleZeroAnalysis
    | OptionsAnalysis
)
type RunnableAnalysis = OpAnalysis | TranAnalysis | DcAnalysis | AcAnalysis
type AnalysisKind = Literal["op", "tran", "dc", "ac"]
type AnalysisResult = DcResult | DcSweepResult | AcResult | TransientResult
type SelectedOutputValue = float | complex


@dataclass(frozen=True, slots=True)
class AnalysisPlanStep:
    """One executable `.op`, `.dc`, `.ac`, or `.tran` card in deck order."""

    index: int
    kind: AnalysisKind
    analysis: RunnableAnalysis


@dataclass(frozen=True, slots=True)
class AnalysisExecutionResult:
    """Result from executing one analysis-plan step."""

    index: int
    kind: AnalysisKind
    analysis: RunnableAnalysis
    result: AnalysisResult


@dataclass(frozen=True, slots=True)
class SelectedOutputRow:
    """One row from a selected `.print`, `.plot`, `.save`, or `.probe` output."""

    index: int
    axis_name: str | None
    axis_value: float | None
    values: dict[str, SelectedOutputValue]


@dataclass(frozen=True, slots=True)
class SelectedAnalysisOutput:
    """Selected output rows for one executed analysis result."""

    index: int
    kind: AnalysisKind
    probes: tuple[OutputProbe, ...]
    rows: tuple[SelectedOutputRow, ...]


@dataclass(frozen=True, slots=True)
class MeasureResult:
    """Computed result from one `.measure` card."""

    analysis_index: int
    analysis: str
    name: str
    operation: MeasureOperation
    probe: OutputProbe
    value: float


@dataclass(frozen=True, slots=True)
class ModelCard:
    """A parsed SPICE `.model` card."""

    name: str
    kind: str
    params: dict[str, float] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class _SourceSpec:
    dc_value: float
    waveform: Waveform | None = None
    ac: AcSource | None = None


@dataclass(frozen=True, slots=True)
class _Statement:
    line_number: int
    fields: list[str]


@dataclass(slots=True)
class _SubcktDefinition:
    name: str
    pins: list[str]
    body: list[_Statement] = field(default_factory=list)
    line_number: int = 0


@dataclass(slots=True)
class ParsedNetlist:
    """Parsed SPICE3 netlist with an executable SPICE engine circuit."""

    circuit: Circuit = field(default_factory=Circuit)
    analyses: list[Analysis] = field(default_factory=list)
    models: dict[str, ModelCard] = field(default_factory=dict)
    title: str | None = None

    def op_cards(self) -> list[OpAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, OpAnalysis)]

    def tran_cards(self) -> list[TranAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, TranAnalysis)]

    def dc_cards(self) -> list[DcAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, DcAnalysis)]

    def ac_cards(self) -> list[AcAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, AcAnalysis)]

    def tf_cards(self) -> list[TfAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, TfAnalysis)]

    def sens_cards(self) -> list[SensAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, SensAnalysis)]

    def mc_cards(self) -> list[McAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, McAnalysis)]

    def noise_cards(self) -> list[NoiseAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, NoiseAnalysis)]

    def temp_cards(self) -> list[TempAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, TempAnalysis)]

    def print_cards(self) -> list[PrintAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, PrintAnalysis)]

    def plot_cards(self) -> list[PlotAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, PlotAnalysis)]

    def save_cards(self) -> list[SaveAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, SaveAnalysis)]

    def probe_cards(self) -> list[ProbeAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, ProbeAnalysis)]

    def measure_cards(self) -> list[MeasureAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, MeasureAnalysis)]

    def four_cards(self) -> list[FourAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, FourAnalysis)]

    def distortion_cards(self) -> list[DistortionAnalysis]:
        return [
            analysis
            for analysis in self.analyses
            if isinstance(analysis, DistortionAnalysis)
        ]

    def pole_zero_cards(self) -> list[PoleZeroAnalysis]:
        return [
            analysis
            for analysis in self.analyses
            if isinstance(analysis, PoleZeroAnalysis)
        ]

    def options_cards(self) -> list[OptionsAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, OptionsAnalysis)]

    def transient_method(self, tran: TranAnalysis | None = None) -> TransientMethod | None:
        """Return the explicit `.tran` method or fallback `.options method` value."""

        if tran is not None and tran.method is not None:
            return tran.method
        for options in self.options_cards():
            value = options.values.get("method")
            if isinstance(value, str):
                return _parse_transient_method(value, ".options method")
        return None

    def dc_op_kwargs(self) -> dict[str, object]:
        """Return selected `.options` values as :func:`spice_engine.dc_op` kwargs."""

        values = _merged_options(self.options_cards())
        kwargs: dict[str, object] = {}
        tol = _option_number(values, ("reltol", "tol"))
        if tol is not None:
            kwargs["tol"] = tol
        max_iterations = _option_int(values, ("itl1", "maxiter", "maxiters", "max_iterations"))
        if max_iterations is not None:
            kwargs["max_iterations"] = max_iterations
        gmin = _option_number(values, ("gmin",))
        if gmin is not None:
            kwargs["pseudo_transient_shunt_conductance"] = gmin
        pseudo_steps = _option_int(values, ("srcsteps", "pseudo_transient_steps"))
        if pseudo_steps is not None:
            kwargs["pseudo_transient_steps"] = pseudo_steps
        pseudo_iterations = _option_int(values, ("itl6", "pseudo_transient_max_iterations"))
        if pseudo_iterations is not None:
            kwargs["pseudo_transient_max_iterations"] = pseudo_iterations
        return kwargs

    def transient_kwargs(
        self,
        tran: TranAnalysis | None = None,
        *,
        adaptive: bool = False,
    ) -> dict[str, object]:
        """Return selected `.options` values as :func:`spice_engine.transient` kwargs."""

        values = _merged_options(self.options_cards())
        kwargs: dict[str, object] = {}
        method = self.transient_method(tran)
        if method is not None:
            kwargs["method"] = method
        tol = _option_number(values, ("reltol", "tol"))
        if tol is not None:
            kwargs["tol"] = tol
        tol_lte = _option_number(values, ("trtol", "lte", "tol_lte"))
        if tol_lte is not None:
            kwargs["tol_lte"] = tol_lte
        min_step = _option_number(values, ("minstep", "tmin", "min_step"))
        if min_step is not None:
            kwargs["min_step"] = min_step
        max_step = _option_number(values, ("maxstep", "tmax", "max_step"))
        if max_step is not None:
            kwargs["max_step"] = max_step
        max_iterations = _option_int(values, ("itl4", "maxiter", "maxiters", "max_iterations"))
        if max_iterations is not None:
            kwargs["max_iterations"] = max_iterations
        if adaptive:
            kwargs["adaptive"] = True
        return kwargs

    def operating_temperature_kelvin(
        self,
        temperature_index: int = 0,
        *,
        default: float = 300.0,
    ) -> float:
        """Return the selected `.temp` operating temperature in Kelvin."""

        if temperature_index < 0:
            raise NetlistParseError("temperature index must be non-negative")
        temperatures = [
            celsius
            for card in self.temp_cards()
            for celsius in card.temperatures_celsius
        ]
        if not temperatures:
            return default
        try:
            return temperatures[temperature_index] + 273.15
        except IndexError as exc:
            raise NetlistParseError(
                f"temperature index {temperature_index} exceeds .temp entries"
            ) from exc

    def noise_temperature_kelvin(
        self,
        noise: NoiseAnalysis | None = None,
        *,
        temperature_index: int = 0,
        default: float = 300.0,
    ) -> float:
        """Return the temperature to pass to `spice_engine.noise_ac`."""

        if noise is not None and noise.temperature_is_explicit:
            return noise.temperature
        return self.operating_temperature_kelvin(
            temperature_index=temperature_index,
            default=default,
        )

    def analysis_plan(self) -> list[AnalysisPlanStep]:
        """Return runnable `.op`, `.dc`, `.ac`, and `.tran` cards in deck order."""

        return build_analysis_plan(self)

    def run_analysis_plan(
        self,
        plan: list[AnalysisPlanStep] | None = None,
    ) -> list[AnalysisExecutionResult]:
        """Execute runnable analysis cards against this parsed circuit."""

        return run_analysis_plan(self, plan)

    def select_outputs(
        self,
        results: list[AnalysisExecutionResult] | None = None,
    ) -> list[SelectedAnalysisOutput]:
        """Apply `.print`, `.plot`, `.save`, and `.probe` cards to results."""

        return select_outputs(self, results)

    def measure_results(
        self,
        results: list[AnalysisExecutionResult] | None = None,
    ) -> list[MeasureResult]:
        """Evaluate supported `.measure` cards against executed results."""

        return measure_results(self, results)


_VALUE_RE = re.compile(
    r"^\s*([+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)([a-zA-Zµ]*)\s*$"
)
_SUFFIXES = {
    "t": 1.0e12,
    "g": 1.0e9,
    "meg": 1.0e6,
    "k": 1.0e3,
    "": 1.0,
    "m": 1.0e-3,
    "u": 1.0e-6,
    "µ": 1.0e-6,
    "n": 1.0e-9,
    "p": 1.0e-12,
    "f": 1.0e-15,
}


def _merged_options(options_cards: list[OptionsAnalysis]) -> dict[str, OptionValue]:
    values: dict[str, OptionValue] = {}
    for options in options_cards:
        values.update(options.values)
    return values


def _option_number(
    values: dict[str, OptionValue],
    keys: tuple[str, ...],
) -> float | None:
    for key in keys:
        value = values.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, (float, int)):
            return float(value)
    return None


def _option_int(
    values: dict[str, OptionValue],
    keys: tuple[str, ...],
) -> int | None:
    value = _option_number(values, keys)
    return None if value is None else int(value)


def parse_netlist(text: str) -> ParsedNetlist:
    """Parse SPICE3 netlist text into a :class:`ParsedNetlist`."""

    parsed = ParsedNetlist()
    statements: list[_Statement] = []
    subckts: dict[str, _SubcktDefinition] = {}
    current_subckt: _SubcktDefinition | None = None
    saw_content = False
    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped:
            continue
        if stripped.startswith("*"):
            if not saw_content and parsed.title is None:
                parsed.title = stripped[1:].strip() or None
            continue
        saw_content = True

        fields = _split_fields(_strip_inline_comment(raw_line))
        if not fields:
            continue
        head = fields[0]
        head_lower = head.lower()

        try:
            if current_subckt is not None:
                if head_lower == ".ends":
                    _finish_subckt(current_subckt, fields)
                    subckts[current_subckt.name.lower()] = current_subckt
                    current_subckt = None
                elif head_lower == ".subckt":
                    raise NetlistParseError("nested .subckt definitions are not supported")
                else:
                    current_subckt.body.append(_Statement(line_number, fields))
                continue
            if head_lower == ".subckt":
                current_subckt = _start_subckt(fields, line_number, subckts)
                continue
            if head_lower == ".ends":
                raise NetlistParseError(".ends without matching .subckt")
        except NetlistParseError as exc:
            raise NetlistParseError(f"line {line_number}: {exc}") from exc

        if head_lower == ".end":
            break
        statements.append(_Statement(line_number, fields))

    if current_subckt is not None:
        raise NetlistParseError(
            f"line {current_subckt.line_number}: .subckt {current_subckt.name!r} is missing .ends"
        )

    for statement in statements:
        if statement.fields[0].lower() != ".model":
            continue
        try:
            model = _parse_model_card(statement.fields)
            key = model.name.lower()
            if key in parsed.models:
                raise NetlistParseError(f"duplicate .model definition {model.name!r}")
            parsed.models[key] = model
        except NetlistParseError as exc:
            raise NetlistParseError(f"line {statement.line_number}: {exc}") from exc

    for statement in statements:
        try:
            if statement.fields[0].lower() == ".model":
                continue
            if statement.fields[0].startswith("."):
                parsed.analyses.append(_parse_directive(statement.fields))
            elif statement.fields[0].upper().startswith("X"):
                for element in _expand_subckt_instance(
                    statement.fields, subckts, [], parsed.models
                ):
                    parsed.circuit.add(element)
            else:
                parsed.circuit.add(_parse_element(statement.fields, parsed.models))
        except NetlistParseError as exc:
            raise NetlistParseError(f"line {statement.line_number}: {exc}") from exc
    _validate_mutual_inductors(parsed.circuit)
    _validate_transmission_lines(parsed.circuit)
    return parsed


def build_analysis_plan(parsed: ParsedNetlist) -> list[AnalysisPlanStep]:
    """Build the executable `.op`, `.dc`, `.ac`, and `.tran` plan for a deck."""

    plan: list[AnalysisPlanStep] = []
    for index, analysis in enumerate(parsed.analyses):
        step = _analysis_plan_step(index, analysis)
        if step is not None:
            plan.append(step)
    return plan


def run_analysis_plan(
    parsed: ParsedNetlist,
    plan: list[AnalysisPlanStep] | None = None,
) -> list[AnalysisExecutionResult]:
    """Execute the runnable plan for a parsed netlist."""

    return [
        AnalysisExecutionResult(
            index=step.index,
            kind=step.kind,
            analysis=step.analysis,
            result=_execute_analysis_step(parsed, step),
        )
        for step in (build_analysis_plan(parsed) if plan is None else plan)
    ]


def run_netlist(text: str) -> list[AnalysisExecutionResult]:
    """Parse a deck and execute its runnable `.op`, `.dc`, `.ac`, and `.tran` cards."""

    return run_analysis_plan(parse_netlist(text))


def select_outputs(
    parsed: ParsedNetlist,
    results: list[AnalysisExecutionResult] | None = None,
) -> list[SelectedAnalysisOutput]:
    """Apply deck output-selection cards to executed analysis results."""

    selected: list[SelectedAnalysisOutput] = []
    execution_results = run_analysis_plan(parsed) if results is None else results
    for result in execution_results:
        probes = _selected_output_probes(parsed, result.kind)
        if not probes:
            continue
        selected.append(
            SelectedAnalysisOutput(
                index=result.index,
                kind=result.kind,
                probes=tuple(probes),
                rows=tuple(_selected_output_rows(result, probes)),
            )
        )
    return selected


def measure_results(
    parsed: ParsedNetlist,
    results: list[AnalysisExecutionResult] | None = None,
) -> list[MeasureResult]:
    """Evaluate supported `.measure` cards against executed analysis results."""

    execution_results = run_analysis_plan(parsed) if results is None else results
    measured: list[MeasureResult] = []
    for card in parsed.measure_cards():
        execution_result = _find_measure_execution_result(card, execution_results)
        value = _evaluate_measure(card, execution_result)
        measured.append(
            MeasureResult(
                analysis_index=execution_result.index,
                analysis=card.analysis,
                name=card.name,
                operation=card.operation,
                probe=card.probe,
                value=value,
            )
        )
    return measured


def parse_value(token: str) -> float:
    """Parse a SPICE numeric token with an engineering suffix."""

    match = _VALUE_RE.match(token)
    if match is None:
        raise NetlistParseError(f"expected numeric value, got {token!r}")
    suffix = match.group(2).lower()
    if suffix not in _SUFFIXES:
        raise NetlistParseError(f"unsupported numeric suffix {match.group(2)!r}")
    return float(match.group(1)) * _SUFFIXES[suffix]


def _analysis_plan_step(index: int, analysis: Analysis) -> AnalysisPlanStep | None:
    if isinstance(analysis, OpAnalysis):
        return AnalysisPlanStep(index=index, kind="op", analysis=analysis)
    if isinstance(analysis, TranAnalysis):
        return AnalysisPlanStep(index=index, kind="tran", analysis=analysis)
    if isinstance(analysis, DcAnalysis):
        return AnalysisPlanStep(index=index, kind="dc", analysis=analysis)
    if isinstance(analysis, AcAnalysis):
        return AnalysisPlanStep(index=index, kind="ac", analysis=analysis)
    return None


def _execute_analysis_step(parsed: ParsedNetlist, step: AnalysisPlanStep) -> AnalysisResult:
    analysis = step.analysis
    if isinstance(analysis, OpAnalysis):
        return dc_op(parsed.circuit, **parsed.dc_op_kwargs())
    if isinstance(analysis, DcAnalysis):
        dc_kwargs = parsed.dc_op_kwargs()
        sweep_kwargs = {
            key: dc_kwargs[key] for key in ("max_iterations", "tol") if key in dc_kwargs
        }
        return dc_sweep(
            parsed.circuit,
            analysis.source_name,
            analysis.start,
            analysis.stop,
            analysis.step,
            **sweep_kwargs,
        )
    if isinstance(analysis, AcAnalysis):
        return ac_sweep(
            parsed.circuit,
            f_start=analysis.start_hz,
            f_stop=analysis.stop_hz,
            n_points=analysis.points,
            sweep=_ac_sweep_mode(analysis),
        )
    if isinstance(analysis, TranAnalysis):
        transient_kwargs = parsed.transient_kwargs(analysis)
        transient_kwargs.setdefault("method", "euler")
        return transient(
            parsed.circuit,
            t_step=analysis.t_step,
            t_stop=analysis.t_stop,
            **transient_kwargs,
        )
    raise NetlistParseError(f"analysis card at index {step.index} is not executable")


def _ac_sweep_mode(analysis: AcAnalysis) -> Literal["log", "lin"]:
    if analysis.mode in ("dec", "log"):
        return "log"
    raise NetlistParseError(
        f".ac mode {analysis.mode!r} is not executable; supported modes are 'dec' and 'log'"
    )


def _selected_output_probes(parsed: ParsedNetlist, kind: AnalysisKind) -> list[OutputProbe]:
    probes: list[OutputProbe] = []
    seen: set[tuple[str, str]] = set()

    def add(new_probes: tuple[OutputProbe, ...]) -> None:
        for probe in new_probes:
            key = (probe.kind, probe.target.lower())
            if key not in seen:
                probes.append(probe)
                seen.add(key)

    for card in parsed.analyses:
        if isinstance(card, SaveAnalysis):
            add(card.probes)
        elif isinstance(card, ProbeAnalysis):
            if card.analysis is None or _analysis_name_matches(card.analysis, kind):
                add(card.probes)
        elif isinstance(card, (PrintAnalysis, PlotAnalysis)) and _analysis_name_matches(
            card.analysis,
            kind,
        ):
            add(card.probes)
    return probes


def _analysis_name_matches(requested: str, kind: AnalysisKind) -> bool:
    aliases = {
        "op": "op",
        "dcop": "op",
        "dc": "dc",
        "ac": "ac",
        "tran": "tran",
        "transient": "tran",
    }
    return aliases.get(requested.lower(), requested.lower()) == kind


def _selected_output_rows(
    execution: AnalysisExecutionResult,
    probes: list[OutputProbe],
) -> list[SelectedOutputRow]:
    result = execution.result
    if isinstance(result, DcResult):
        return [
            SelectedOutputRow(
                index=0,
                axis_name=None,
                axis_value=None,
                values=_selected_output_values(
                    result.node_voltages,
                    result.branch_currents,
                    probes,
                    ".op output selection",
                ),
            )
        ]
    if isinstance(result, DcSweepResult):
        return [
            SelectedOutputRow(
                index=index,
                axis_name="source",
                axis_value=point.source_value,
                values=_selected_output_values(
                    point.node_voltages,
                    point.branch_currents,
                    probes,
                    ".dc output selection",
                ),
            )
            for index, point in enumerate(result.points)
        ]
    if isinstance(result, AcResult):
        return [
            SelectedOutputRow(
                index=index,
                axis_name="frequency",
                axis_value=point.freq,
                values=_selected_output_values(
                    point.node_voltages,
                    point.branch_currents,
                    probes,
                    ".ac output selection",
                ),
            )
            for index, point in enumerate(result.points)
        ]
    if isinstance(result, TransientResult):
        return [
            SelectedOutputRow(
                index=index,
                axis_name="time",
                axis_value=point.time,
                values=_selected_output_values(
                    point.node_voltages,
                    point.branch_currents,
                    probes,
                    ".tran output selection",
                ),
            )
            for index, point in enumerate(result.points)
        ]
    raise NetlistParseError(f"analysis result at index {execution.index} is not selectable")


def _selected_output_values(
    node_voltages: dict[str, SelectedOutputValue],
    branch_currents: dict[str, SelectedOutputValue],
    probes: list[OutputProbe],
    context: str,
) -> dict[str, SelectedOutputValue]:
    return {
        _probe_label(probe): _probe_value(probe, node_voltages, branch_currents, context)
        for probe in probes
    }


def _find_measure_execution_result(
    card: MeasureAnalysis,
    results: list[AnalysisExecutionResult],
) -> AnalysisExecutionResult:
    for result in results:
        if _analysis_name_matches(card.analysis, result.kind):
            return result
    raise NetlistParseError(f".measure {card.name!r} references missing {card.analysis} analysis")


def _evaluate_measure(card: MeasureAnalysis, result: AnalysisExecutionResult) -> float:
    samples = _measure_samples(card, result)
    if not samples:
        raise NetlistParseError(f".measure {card.name!r} has no samples")
    if card.operation == "find":
        if result.kind == "op" and card.at is None:
            return _measure_numeric_value(samples[0][1])
        if card.at is None:
            raise NetlistParseError(f".measure {card.name!r} FIND requires AT=<value>")
        return _measure_numeric_value(_interpolate_measure_value(samples, card.at, card))

    ranged = _range_measure_samples(samples, card)
    values = [_measure_numeric_value(value) for _, value in ranged]
    if not values:
        raise NetlistParseError(f".measure {card.name!r} range has no samples")
    if card.operation == "max":
        return max(values)
    if card.operation == "min":
        return min(values)
    if card.operation == "avg":
        return _average_measure_value(ranged)
    if card.operation == "rms":
        return _rms_measure_value(ranged)
    raise NetlistParseError(f"unsupported .measure operation {card.operation!r}")


def _measure_samples(
    card: MeasureAnalysis,
    execution: AnalysisExecutionResult,
) -> list[tuple[float | None, SelectedOutputValue]]:
    result = execution.result
    if isinstance(result, DcResult):
        return [
            (
                None,
                _probe_value(
                    card.probe,
                    result.node_voltages,
                    result.branch_currents,
                    f".measure {card.name}",
                ),
            )
        ]
    if isinstance(result, DcSweepResult):
        return [
            (
                point.source_value,
                _probe_value(
                    card.probe,
                    point.node_voltages,
                    point.branch_currents,
                    f".measure {card.name}",
                ),
            )
            for point in result.points
        ]
    if isinstance(result, AcResult):
        return [
            (
                point.freq,
                _probe_value(
                    card.probe,
                    point.node_voltages,
                    point.branch_currents,
                    f".measure {card.name}",
                ),
            )
            for point in result.points
        ]
    if isinstance(result, TransientResult):
        return [
            (
                point.time,
                _probe_value(
                    card.probe,
                    point.node_voltages,
                    point.branch_currents,
                    f".measure {card.name}",
                ),
            )
            for point in result.points
        ]
    return []


def _range_measure_samples(
    samples: list[tuple[float | None, SelectedOutputValue]],
    card: MeasureAnalysis,
) -> list[tuple[float | None, SelectedOutputValue]]:
    if any(axis is None for axis, _ in samples):
        if card.start is not None or card.stop is not None:
            raise NetlistParseError(f".measure {card.name!r} range requires swept samples")
        return samples
    axis_samples = sorted((axis, value) for axis, value in samples if axis is not None)
    lower = axis_samples[0][0] if card.start is None else card.start
    upper = axis_samples[-1][0] if card.stop is None else card.stop
    if lower > upper:
        raise NetlistParseError(f".measure {card.name!r} FROM must be <= TO")
    ranged: list[tuple[float | None, SelectedOutputValue]] = []
    if card.start is not None:
        ranged.append((lower, _interpolate_measure_value(samples, lower, card)))
    ranged.extend(
        (axis, value)
        for axis, value in axis_samples
        if lower <= axis <= upper and not _axis_already_present(ranged, axis)
    )
    if card.stop is not None and not _axis_already_present(ranged, upper):
        ranged.append((upper, _interpolate_measure_value(samples, upper, card)))
    return sorted(ranged, key=lambda sample: -math.inf if sample[0] is None else sample[0])


def _axis_already_present(
    samples: list[tuple[float | None, SelectedOutputValue]],
    axis: float,
) -> bool:
    return any(
        existing_axis is not None and math.isclose(existing_axis, axis)
        for existing_axis, _ in samples
    )


def _interpolate_measure_value(
    samples: list[tuple[float | None, SelectedOutputValue]],
    target: float,
    card: MeasureAnalysis,
) -> SelectedOutputValue:
    axis_samples = sorted((axis, value) for axis, value in samples if axis is not None)
    if not axis_samples:
        raise NetlistParseError(f".measure {card.name!r} AT requires swept samples")
    if target < axis_samples[0][0] or target > axis_samples[-1][0]:
        raise NetlistParseError(f".measure {card.name!r} AT is outside the analysis range")
    for axis, value in axis_samples:
        if math.isclose(axis, target):
            return value
    for (left_axis, left_value), (right_axis, right_value) in zip(
        axis_samples,
        axis_samples[1:],
        strict=False,
    ):
        if left_axis <= target <= right_axis:
            fraction = (target - left_axis) / (right_axis - left_axis)
            return _interpolate_output_values(left_value, right_value, fraction)
    return axis_samples[-1][1]


def _interpolate_output_values(
    left: SelectedOutputValue,
    right: SelectedOutputValue,
    fraction: float,
) -> SelectedOutputValue:
    if isinstance(left, complex) or isinstance(right, complex):
        left_complex = complex(left)
        right_complex = complex(right)
        return left_complex + (right_complex - left_complex) * fraction
    return float(left) + (float(right) - float(left)) * fraction


def _average_measure_value(samples: list[tuple[float | None, SelectedOutputValue]]) -> float:
    numeric = [(axis, _measure_numeric_value(value)) for axis, value in samples]
    if len(numeric) < 2 or any(axis is None for axis, _ in numeric):
        return sum(value for _, value in numeric) / len(numeric)
    span = float(numeric[-1][0]) - float(numeric[0][0])
    if span <= 0.0:
        return sum(value for _, value in numeric) / len(numeric)
    area = sum(
        0.5 * (left_value + right_value) * (float(right_axis) - float(left_axis))
        for (left_axis, left_value), (right_axis, right_value) in zip(
            numeric,
            numeric[1:],
            strict=False,
        )
    )
    return area / span


def _rms_measure_value(samples: list[tuple[float | None, SelectedOutputValue]]) -> float:
    numeric = [(axis, _measure_numeric_value(value)) for axis, value in samples]
    if len(numeric) < 2 or any(axis is None for axis, _ in numeric):
        return math.sqrt(sum(value * value for _, value in numeric) / len(numeric))
    span = float(numeric[-1][0]) - float(numeric[0][0])
    if span <= 0.0:
        return math.sqrt(sum(value * value for _, value in numeric) / len(numeric))
    area = sum(
        0.5
        * (left_value * left_value + right_value * right_value)
        * (float(right_axis) - float(left_axis))
        for (left_axis, left_value), (right_axis, right_value) in zip(
            numeric,
            numeric[1:],
            strict=False,
        )
    )
    return math.sqrt(area / span)


def _measure_numeric_value(value: SelectedOutputValue) -> float:
    return abs(value) if isinstance(value, complex) else float(value)


def _probe_value(
    probe: OutputProbe,
    node_voltages: dict[str, SelectedOutputValue],
    branch_currents: dict[str, SelectedOutputValue],
    context: str,
) -> SelectedOutputValue:
    if probe.kind == "voltage":
        if probe.target.lower() in ("0", "gnd"):
            return 0j if _contains_complex_values(node_voltages) else 0.0
        value = _case_insensitive_get(node_voltages, probe.target)
        if value is None:
            raise NetlistParseError(f"{context}: missing voltage probe V({probe.target})")
        return value
    key = probe.target if probe.target.lower().startswith("i(") else f"I({probe.target})"
    value = _case_insensitive_get(branch_currents, key)
    if value is None:
        raise NetlistParseError(f"{context}: missing branch current probe I({probe.target})")
    return value


def _contains_complex_values(values: dict[str, SelectedOutputValue]) -> bool:
    return any(isinstance(value, complex) for value in values.values())


def _case_insensitive_get(
    values: dict[str, SelectedOutputValue],
    key: str,
) -> SelectedOutputValue | None:
    if key in values:
        return values[key]
    lower_key = key.lower()
    for candidate, value in values.items():
        if candidate.lower() == lower_key:
            return value
    return None


def _probe_label(probe: OutputProbe) -> str:
    return f"V({probe.target})" if probe.kind == "voltage" else f"I({probe.target})"


def _parse_element(fields: list[str], models: dict[str, ModelCard]) -> object:
    name = fields[0]
    prefix = _element_prefix(name)
    if prefix == "R":
        _require_fields(fields, 4, "resistor")
        return Resistor(name, fields[1], fields[2], parse_value(fields[3]))
    if prefix == "C":
        _require_min_fields(fields, 4, "capacitor")
        params = _parse_element_params(fields[4:], "capacitor")
        unsupported = set(params) - {"IC"}
        if unsupported:
            raise NetlistParseError(
                f"unsupported capacitor parameter {sorted(unsupported)[0]!r}"
            )
        return Capacitor(
            name,
            fields[1],
            fields[2],
            parse_value(fields[3]),
            initial_voltage=params.get("IC", 0.0),
        )
    if prefix == "L":
        _require_min_fields(fields, 4, "inductor")
        params = _parse_element_params(fields[4:], "inductor")
        unsupported = set(params) - {"IC"}
        if unsupported:
            raise NetlistParseError(
                f"unsupported inductor parameter {sorted(unsupported)[0]!r}"
            )
        return Inductor(
            name,
            fields[1],
            fields[2],
            parse_value(fields[3]),
            initial_current=params.get("IC", 0.0),
        )
    if prefix == "K":
        _require_fields(fields, 4, "mutual inductor")
        return MutualInductor(name, fields[1], fields[2], parse_value(fields[3]))
    if prefix == "T":
        _require_min_fields(fields, 6, "transmission line")
        params = _parse_element_params(fields[5:], "transmission line")
        unsupported = set(params) - {"Z0", "TD"}
        if unsupported:
            raise NetlistParseError(
                f"unsupported transmission line parameter {sorted(unsupported)[0]!r}"
            )
        if "Z0" not in params:
            raise NetlistParseError(f"{name}: transmission line requires Z0")
        if "TD" not in params:
            raise NetlistParseError(f"{name}: transmission line requires TD")
        return TransmissionLine(
            name,
            fields[1],
            fields[2],
            fields[3],
            fields[4],
            params["Z0"],
            params["TD"],
        )
    if prefix == "V":
        _require_min_fields(fields, 4, "voltage source")
        source = _parse_source_value(fields[3:])
        return VoltageSource(
            name, fields[1], fields[2], source.dc_value, source.waveform, source.ac
        )
    if prefix == "I":
        _require_min_fields(fields, 4, "current source")
        source = _parse_source_value(fields[3:])
        return CurrentSource(
            name, fields[1], fields[2], source.dc_value, source.waveform, source.ac
        )
    if prefix == "D":
        _require_fields(fields, 4, "diode")
        model = models.get(fields[3].lower())
        if model is None:
            raise NetlistParseError(f"unknown model {fields[3]!r} for diode {name!r}")
        if model.kind != "D":
            raise NetlistParseError(
                f"model {model.name!r} has kind {model.kind!r}, expected 'D'"
            )
        return Diode(
            name,
            fields[1],
            fields[2],
            Is=model.params.get("IS", model.params.get("JS", 1e-15)),
            Vt=model.params.get("VT", model.params.get("V_T", 0.02585)),
            N=model.params.get("N", 1.0),
            BV=model.params.get("BV"),
            IBV=model.params.get("IBV", 1e-3),
            Cjo=model.params.get("CJO", model.params.get("CJ0", 0.0)),
            Tt=model.params.get("TT", 0.0),
        )
    if prefix == "Q":
        _require_fields(fields, 5, "BJT")
        model = models.get(fields[4].lower())
        if model is None:
            raise NetlistParseError(f"unknown model {fields[4]!r} for BJT {name!r}")
        if model.kind not in {"NPN", "PNP"}:
            raise NetlistParseError(
                f"model {model.name!r} has kind {model.kind!r}, expected 'NPN' or 'PNP'"
            )
        return BJT(
            name,
            fields[1],
            fields[2],
            fields[3],
            polarity=model.kind,
            Is=model.params.get("IS", 1e-14),
            beta_f=model.params.get("BETA_F", model.params.get("BF", 100.0)),
            Vt=model.params.get("VT", 0.02585),
            Cje=model.params.get("CJE", model.params.get("CBE", 0.0)),
            Cjc=model.params.get("CJC", model.params.get("CBC", 0.0)),
            Tf=model.params.get("TF", 0.0),
            Tr=model.params.get("TR", 0.0),
        )
    if prefix == "J":
        _require_fields(fields, 5, "JFET")
        model = models.get(fields[4].lower())
        if model is None:
            raise NetlistParseError(f"unknown model {fields[4]!r} for JFET {name!r}")
        if model.kind not in {"NJF", "PJF"}:
            raise NetlistParseError(
                f"model {model.name!r} has kind {model.kind!r}, expected 'NJF' or 'PJF'"
            )
        return JFET(
            name,
            fields[1],
            fields[2],
            fields[3],
            polarity=model.kind,
            beta=model.params.get("BETA", model.params.get("B", 1.0e-4)),
            vto=model.params.get("VTO", -2.0 if model.kind == "NJF" else 2.0),
            lambda_=model.params.get("LAMBDA", 0.0),
            Cgs=model.params.get("CGS", model.params.get("CGS0", 0.0)),
            Cgd=model.params.get("CGD", model.params.get("CGD0", 0.0)),
        )
    if prefix == "M":
        _require_min_fields(fields, 6, "MOSFET")
        model = models.get(fields[5].lower())
        if model is None:
            raise NetlistParseError(f"unknown model {fields[5]!r} for MOSFET {name!r}")
        if model.kind not in {"NMOS", "PMOS"}:
            raise NetlistParseError(
                f"model {model.name!r} has kind {model.kind!r}, expected 'NMOS' or 'PMOS'"
            )
        instance_params = _parse_element_params(fields[6:], "MOSFET")
        if "NRD" in instance_params and (
            not math.isfinite(instance_params["NRD"]) or instance_params["NRD"] < 0.0
        ):
            raise NetlistParseError("MOSFET NRD must be finite and non-negative")
        if "NRS" in instance_params and (
            not math.isfinite(instance_params["NRS"]) or instance_params["NRS"] < 0.0
        ):
            raise NetlistParseError("MOSFET NRS must be finite and non-negative")
        if "AD" in instance_params and (
            not math.isfinite(instance_params["AD"]) or instance_params["AD"] < 0.0
        ):
            raise NetlistParseError("MOSFET AD must be finite and non-negative")
        if "AS" in instance_params and (
            not math.isfinite(instance_params["AS"]) or instance_params["AS"] < 0.0
        ):
            raise NetlistParseError("MOSFET AS must be finite and non-negative")
        if "PD" in instance_params and (
            not math.isfinite(instance_params["PD"]) or instance_params["PD"] < 0.0
        ):
            raise NetlistParseError("MOSFET PD must be finite and non-negative")
        if "PS" in instance_params and (
            not math.isfinite(instance_params["PS"]) or instance_params["PS"] < 0.0
        ):
            raise NetlistParseError("MOSFET PS must be finite and non-negative")
        return Mosfet(
            name,
            fields[1],
            fields[2],
            fields[3],
            fields[4],
            _build_mosfet_model(model, instance_params),
        )
    if prefix == "G":
        _require_fields(fields, 6, "VCCS")
        return VCCS(name, fields[1], fields[2], fields[3], fields[4], parse_value(fields[5]))
    if prefix == "E":
        _require_fields(fields, 6, "VCVS")
        return VCVS(name, fields[1], fields[2], fields[3], fields[4], parse_value(fields[5]))
    if prefix == "F":
        _require_fields(fields, 5, "CCCS")
        return CCCS(name, fields[1], fields[2], fields[3], parse_value(fields[4]))
    if prefix == "H":
        _require_fields(fields, 5, "CCVS")
        return CCVS(name, fields[1], fields[2], fields[3], parse_value(fields[4]))
    raise NetlistParseError(f"unsupported element {name!r}")


def _validate_mutual_inductors(circuit: Circuit) -> None:
    inductors = {
        element.name: element for element in circuit.elements if isinstance(element, Inductor)
    }
    for element in circuit.elements:
        if not isinstance(element, MutualInductor):
            continue
        if not math.isfinite(element.coupling):
            raise NetlistParseError(f"{element.name}: coupling must be finite")
        if abs(element.coupling) >= 1.0:
            raise NetlistParseError(
                f"{element.name}: coupling magnitude must be less than one"
            )
        if element.primary == element.secondary:
            raise NetlistParseError(f"{element.name}: coupled inductors must be distinct")
        if element.primary not in inductors:
            raise NetlistParseError(
                f"{element.name}: referenced inductor {element.primary!r} was not found"
            )
        if element.secondary not in inductors:
            raise NetlistParseError(
                f"{element.name}: referenced inductor {element.secondary!r} was not found"
            )


def _validate_transmission_lines(circuit: Circuit) -> None:
    for element in circuit.elements:
        if not isinstance(element, TransmissionLine):
            continue
        if not math.isfinite(element.characteristic_impedance):
            raise NetlistParseError(f"{element.name}: characteristic impedance must be finite")
        if element.characteristic_impedance <= 0.0:
            raise NetlistParseError(f"{element.name}: characteristic impedance must be positive")
        if not math.isfinite(element.delay):
            raise NetlistParseError(f"{element.name}: delay must be finite")
        if element.delay <= 0.0:
            raise NetlistParseError(f"{element.name}: delay must be positive")


def _start_subckt(
    fields: list[str], line_number: int, subckts: dict[str, _SubcktDefinition]
) -> _SubcktDefinition:
    _require_min_fields(fields, 3, ".subckt")
    name = fields[1]
    key = name.lower()
    if key in subckts:
        raise NetlistParseError(f"duplicate .subckt definition {name!r}")
    return _SubcktDefinition(name=name, pins=fields[2:], line_number=line_number)


def _finish_subckt(definition: _SubcktDefinition, fields: list[str]) -> None:
    if len(fields) > 2:
        raise NetlistParseError(".ends expects at most a subcircuit name")
    if len(fields) == 2 and fields[1].lower() != definition.name.lower():
        raise NetlistParseError(
            f".ends {fields[1]!r} does not match .subckt {definition.name!r}"
        )


def _expand_subckt_instance(
    fields: list[str],
    subckts: dict[str, _SubcktDefinition],
    stack: list[str],
    models: dict[str, ModelCard],
) -> list[object]:
    _require_min_fields(fields, 3, "subcircuit instance")
    instance_name = fields[0]
    subckt_name = fields[-1]
    definition = subckts.get(subckt_name.lower())
    if definition is None:
        raise NetlistParseError(f"unknown subcircuit {subckt_name!r}")
    if definition.name.lower() in stack:
        cycle = " -> ".join([*stack, definition.name.lower()])
        raise NetlistParseError(f"recursive subcircuit expansion is not supported: {cycle}")

    actual_nodes = fields[1:-1]
    if len(actual_nodes) != len(definition.pins):
        raise NetlistParseError(
            f"subcircuit {definition.name!r} expects {len(definition.pins)} pins, "
            f"got {len(actual_nodes)}"
        )

    node_map = {pin: actual for pin, actual in zip(definition.pins, actual_nodes, strict=True)}
    node_map.update(
        {pin.lower(): actual for pin, actual in zip(definition.pins, actual_nodes, strict=True)}
    )
    elements: list[object] = []
    next_stack = [*stack, definition.name.lower()]
    for statement in definition.body:
        if statement.fields[0].startswith("."):
            raise NetlistParseError(
                f"line {statement.line_number}: directives inside .subckt are not supported"
            )
        local_fields = _map_subckt_fields(statement.fields, instance_name, node_map)
        if _element_prefix(statement.fields[0]) == "X":
            elements.extend(_expand_subckt_instance(local_fields, subckts, next_stack, models))
        else:
            elements.append(_parse_element(local_fields, models))
    return elements


def _map_subckt_fields(
    fields: list[str], instance_name: str, node_map: dict[str, str]
) -> list[str]:
    name = f"{instance_name}.{fields[0]}"
    prefix = fields[0][0].upper()
    mapped = [name, *fields[1:]]
    if prefix in {"R", "C", "L", "V", "I", "D"}:
        _require_min_fields(fields, 3, "subcircuit element")
        mapped[1] = _map_subckt_node(fields[1], instance_name, node_map)
        mapped[2] = _map_subckt_node(fields[2], instance_name, node_map)
    elif prefix in {"Q", "J"}:
        _require_min_fields(fields, 4, "subcircuit BJT" if prefix == "Q" else "subcircuit JFET")
        mapped[1] = _map_subckt_node(fields[1], instance_name, node_map)
        mapped[2] = _map_subckt_node(fields[2], instance_name, node_map)
        mapped[3] = _map_subckt_node(fields[3], instance_name, node_map)
    elif prefix == "M":
        _require_min_fields(fields, 5, "subcircuit MOSFET")
        for index in range(1, 5):
            mapped[index] = _map_subckt_node(fields[index], instance_name, node_map)
    elif prefix in {"E", "G"}:
        _require_min_fields(fields, 5, "subcircuit controlled source")
        for index in range(1, 5):
            mapped[index] = _map_subckt_node(fields[index], instance_name, node_map)
    elif prefix in {"F", "H"}:
        _require_min_fields(fields, 4, "subcircuit current-controlled source")
        mapped[1] = _map_subckt_node(fields[1], instance_name, node_map)
        mapped[2] = _map_subckt_node(fields[2], instance_name, node_map)
        mapped[3] = _map_subckt_source_ref(fields[3], instance_name)
    elif prefix == "K":
        _require_fields(fields, 4, "subcircuit mutual inductor")
        mapped[1] = _map_subckt_source_ref(fields[1], instance_name)
        mapped[2] = _map_subckt_source_ref(fields[2], instance_name)
    elif prefix == "T":
        _require_min_fields(fields, 6, "subcircuit transmission line")
        for index in range(1, 5):
            mapped[index] = _map_subckt_node(fields[index], instance_name, node_map)
    elif prefix == "X":
        mapped[1:-1] = [
            _map_subckt_node(node, instance_name, node_map) for node in fields[1:-1]
        ]
    return mapped


def _map_subckt_source_ref(source: str, instance_name: str) -> str:
    if "." in source:
        return source
    return f"{instance_name}.{source}"


def _map_subckt_node(node: str, instance_name: str, node_map: dict[str, str]) -> str:
    if node.lower() in {"0", "gnd"}:
        return node
    if node in node_map:
        return node_map[node]
    if node.lower() in node_map:
        return node_map[node.lower()]
    return f"{instance_name}.{node}"


def _element_prefix(name: str) -> str:
    local_name = name.rsplit(".", 1)[-1]
    return local_name[0].upper()


def _parse_source_value(fields: list[str]) -> _SourceSpec:
    if not fields:
        raise NetlistParseError("source is missing a value")
    if fields[0].upper() == "DC":
        if len(fields) < 2:
            raise NetlistParseError("DC source form requires a value")
        return _SourceSpec(parse_value(fields[1]), ac=_parse_ac_suffix(fields[2:]))
    if fields[0].upper() == "AC":
        return _SourceSpec(0.0, ac=_parse_ac_suffix(fields))
    if len(fields) == 1 and "(" in fields[0]:
        waveform = _parse_waveform(fields[0])
        return _SourceSpec(waveform(0.0), waveform)
    if fields[0].upper().startswith(("PWL(", "SIN(", "PULSE(", "EXP(")):
        joined = " ".join(fields)
        waveform = _parse_waveform(joined)
        return _SourceSpec(waveform(0.0), waveform)
    return _SourceSpec(parse_value(fields[0]), ac=_parse_ac_suffix(fields[1:]))


def _parse_ac_suffix(fields: list[str]) -> AcSource | None:
    if not fields:
        return None
    if fields[0].upper() != "AC":
        raise NetlistParseError(f"unsupported source suffix {fields[0]!r}")
    if len(fields) not in {2, 3}:
        raise NetlistParseError("AC source form requires magnitude and optional phase")
    magnitude = parse_value(fields[1])
    phase = parse_value(fields[2]) if len(fields) == 3 else 0.0
    return AcSource(magnitude=magnitude, phase_degrees=phase)


def _parse_waveform(token: str) -> Waveform:
    match = re.match(r"^\s*([A-Za-z]+)\((.*)\)\s*$", token)
    if match is None:
        raise NetlistParseError(f"invalid source waveform {token!r}")
    kind = match.group(1).upper()
    values = [parse_value(part) for part in re.split(r"[\s,]+", match.group(2).strip()) if part]
    if kind == "PWL":
        if len(values) < 4 or len(values) % 2 != 0:
            raise NetlistParseError("PWL requires time/value pairs")
        return PwlWaveform(tuple(zip(values[0::2], values[1::2], strict=True)))
    if kind == "SIN":
        padded = _pad(values, 5, 0.0)
        return SinWaveform(
            offset=padded[0],
            amplitude=padded[1] if len(values) >= 2 else 1.0,
            frequency=padded[2] if len(values) >= 3 else 1.0,
            delay=padded[3],
            damping=padded[4],
        )
    if kind == "PULSE":
        padded = _pad(values, 7, 0.0)
        return PulseWaveform(
            v_initial=padded[0],
            v_pulsed=padded[1] if len(values) >= 2 else 1.0,
            delay=padded[2],
            rise_time=padded[3],
            fall_time=padded[4],
            pulse_width=padded[5] if len(values) >= 6 else 0.5,
            period=padded[6] if len(values) >= 7 else 1.0,
        )
    if kind == "EXP":
        padded = _pad(values, 6, 0.0)
        return ExpWaveform(
            v_initial=padded[0],
            v_pulsed=padded[1] if len(values) >= 2 else 1.0,
            rise_delay=padded[2],
            rise_tc=padded[3] if len(values) >= 4 else 1.0,
            fall_delay=padded[4] if len(values) >= 5 else 1.0,
            fall_tc=padded[5] if len(values) >= 6 else 1.0,
        )
    raise NetlistParseError(f"unsupported source waveform {kind!r}")


def _parse_directive(fields: list[str]) -> Analysis:
    directive = fields[0].lower()
    if directive == ".op":
        _require_fields(fields, 1, ".op")
        return OpAnalysis()
    if directive == ".tran":
        _require_min_fields(fields, 3, ".tran")
        return TranAnalysis(
            t_step=parse_value(fields[1]),
            t_stop=parse_value(fields[2]),
            method=_parse_tran_method_options(fields[3:]),
        )
    if directive == ".dc":
        _require_fields(fields, 5, ".dc")
        return DcAnalysis(
            source_name=fields[1],
            start=parse_value(fields[2]),
            stop=parse_value(fields[3]),
            step=parse_value(fields[4]),
        )
    if directive == ".ac":
        _require_fields(fields, 5, ".ac")
        return AcAnalysis(
            mode=fields[1].lower(),
            points=int(parse_value(fields[2])),
            start_hz=parse_value(fields[3]),
            stop_hz=parse_value(fields[4]),
        )
    if directive == ".tf":
        _require_fields(fields, 3, ".tf")
        return TfAnalysis(
            output_node=_parse_voltage_probe(fields[1], ".tf"),
            input_source=fields[2],
        )
    if directive == ".sens":
        _require_fields(fields, 2, ".sens")
        return SensAnalysis(output_node=_parse_voltage_probe(fields[1], ".sens"))
    if directive == ".mc":
        _require_min_fields(fields, 3, ".mc")
        _require_max_fields(fields, 6, ".mc")
        distribution = fields[4].lower() if len(fields) >= 5 else "gaussian"
        if distribution not in ("gaussian", "uniform"):
            raise NetlistParseError(
                f".mc distribution must be 'gaussian' or 'uniform', got {fields[4]!r}"
            )
        return McAnalysis(
            output_node=_parse_voltage_probe(fields[1], ".mc"),
            n_trials=int(parse_value(fields[2])),
            tolerance=parse_value(fields[3]) if len(fields) >= 4 else 0.05,
            distribution=distribution,
            seed=int(parse_value(fields[5])) if len(fields) >= 6 else None,
        )
    if directive == ".noise":
        _require_min_fields(fields, 3, ".noise")
        freqs: list[float] = []
        temperature = 300.0
        temperature_is_explicit = False
        tail_index = 3
        while tail_index < len(fields):
            token = fields[tail_index]
            lower_token = token.lower()
            if lower_token == "temp":
                if tail_index + 1 >= len(fields):
                    raise NetlistParseError(".noise temp requires a temperature value")
                temperature = parse_value(fields[tail_index + 1])
                temperature_is_explicit = True
                tail_index += 2
                continue
            if lower_token.startswith("temp="):
                temperature = parse_value(token.split("=", 1)[1])
                temperature_is_explicit = True
                tail_index += 1
                continue
            freqs.append(parse_value(token))
            tail_index += 1
        return NoiseAnalysis(
            output_node=_parse_voltage_probe(fields[1], ".noise"),
            input_source=fields[2],
            freqs=tuple(freqs),
            temperature=temperature,
            temperature_is_explicit=temperature_is_explicit,
        )
    if directive == ".temp":
        _require_min_fields(fields, 2, ".temp")
        return TempAnalysis(
            temperatures_celsius=tuple(parse_value(token) for token in fields[1:])
        )
    if directive == ".print":
        _require_min_fields(fields, 3, ".print")
        return PrintAnalysis(
            analysis=fields[1].lower(),
            probes=tuple(_parse_output_probe(token, ".print") for token in fields[2:]),
        )
    if directive == ".plot":
        _require_min_fields(fields, 3, ".plot")
        return PlotAnalysis(
            analysis=fields[1].lower(),
            probes=tuple(_parse_output_probe(token, ".plot") for token in fields[2:]),
        )
    if directive == ".save":
        _require_min_fields(fields, 2, ".save")
        return SaveAnalysis(
            probes=tuple(_parse_output_probe(token, ".save") for token in fields[1:])
        )
    if directive == ".probe":
        return _parse_probe_card(fields)
    if directive in (".measure", ".meas"):
        return _parse_measure_card(fields)
    if directive == ".four":
        _require_min_fields(fields, 3, ".four")
        return FourAnalysis(
            frequency_hz=parse_value(fields[1]),
            probes=tuple(_parse_output_probe(token, ".four") for token in fields[2:]),
        )
    if directive == ".disto":
        _require_min_fields(fields, 6, ".disto")
        return DistortionAnalysis(
            mode=fields[1].lower(),
            points=int(parse_value(fields[2])),
            start_hz=parse_value(fields[3]),
            stop_hz=parse_value(fields[4]),
            probes=tuple(_parse_output_probe(token, ".disto") for token in fields[5:]),
        )
    if directive == ".pz":
        _require_min_fields(fields, 3, ".pz")
        _require_max_fields(fields, 4, ".pz")
        kind = fields[3].lower() if len(fields) >= 4 else "pz"
        if kind not in ("pole", "zero", "pz"):
            raise NetlistParseError(
                f".pz kind must be 'pole', 'zero', or 'pz', got {fields[3]!r}"
            )
        return PoleZeroAnalysis(
            output_node=_parse_voltage_probe(fields[1], ".pz"),
            input_source=fields[2],
            kind=kind,
        )
    if directive == ".options":
        _require_min_fields(fields, 2, ".options")
        return OptionsAnalysis(values=_parse_options(fields[1:]))
    raise NetlistParseError(f"unsupported directive {fields[0]!r}")


def _parse_probe_card(fields: list[str]) -> ProbeAnalysis:
    _require_min_fields(fields, 2, ".probe")
    analysis: str | None = None
    probe_tokens = fields[1:]
    if len(fields) >= 3 and _is_analysis_selector(fields[1]):
        analysis = fields[1].lower()
        probe_tokens = fields[2:]
    return ProbeAnalysis(
        analysis=analysis,
        probes=tuple(_parse_output_probe(token, ".probe") for token in probe_tokens),
    )


def _parse_measure_card(fields: list[str]) -> MeasureAnalysis:
    directive = fields[0].lower()
    _require_min_fields(fields, 5, directive)
    operation = _parse_measure_operation(fields[3], directive)
    options = _parse_measure_options(fields[5:], directive)
    if operation == "find" and "at" not in options and fields[1].lower() not in ("op", "dcop"):
        raise NetlistParseError(f"{directive} FIND requires AT=<value>")
    if operation != "find" and "at" in options:
        raise NetlistParseError(f"{directive} {operation.upper()} does not support AT=<value>")
    return MeasureAnalysis(
        analysis=fields[1].lower(),
        name=fields[2],
        operation=operation,
        probe=_parse_output_probe(fields[4], directive),
        at=options.get("at"),
        start=options.get("from"),
        stop=options.get("to"),
    )


def _parse_measure_operation(token: str, directive: str) -> MeasureOperation:
    operation = token.lower()
    if operation not in ("find", "max", "min", "avg", "rms"):
        raise NetlistParseError(
            f"{directive} operation must be FIND, MAX, MIN, AVG, or RMS, got {token!r}"
        )
    return operation


def _parse_measure_options(tokens: list[str], directive: str) -> dict[str, float]:
    options: dict[str, float] = {}
    for token in tokens:
        if "=" not in token:
            raise NetlistParseError(f"{directive} option must be KEY=value, got {token!r}")
        key, raw_value = token.split("=", 1)
        key = key.strip().lower()
        if key not in ("at", "from", "to"):
            raise NetlistParseError(f"{directive} unsupported option {key!r}")
        if key in options:
            raise NetlistParseError(f"{directive} duplicate option {key!r}")
        if raw_value == "":
            raise NetlistParseError(f"{directive} option {key!r} requires a value")
        options[key] = parse_value(raw_value)
    return options


def _is_analysis_selector(token: str) -> bool:
    return token.lower() in {"op", "dcop", "dc", "ac", "tran", "transient"}


def _parse_options(tokens: list[str]) -> dict[str, OptionValue]:
    values: dict[str, OptionValue] = {}
    for token in tokens:
        if "=" in token:
            key, raw_value = token.split("=", 1)
            key = key.strip().lower()
            if not key:
                raise NetlistParseError(f".options contains empty option name in {token!r}")
            if raw_value == "":
                raise NetlistParseError(f".options {key!r} requires a value")
            values[key] = (
                _parse_transient_method(raw_value, ".options method")
                if key == "method"
                else _parse_option_value(raw_value)
            )
        else:
            key = token.strip().lower()
            if not key:
                raise NetlistParseError(".options contains an empty flag")
            values[key] = True
    return values


def _parse_tran_method_options(tokens: list[str]) -> TransientMethod | None:
    method: TransientMethod | None = None
    for token in tokens:
        if "=" not in token:
            raise NetlistParseError(
                f".tran unsupported trailing option {token!r}; use method=<euler|trap|gear2>"
            )
        key, raw_value = token.split("=", 1)
        key = key.strip().lower()
        if key != "method":
            raise NetlistParseError(f".tran unsupported option {key!r}")
        if raw_value == "":
            raise NetlistParseError(".tran method requires a value")
        method = _parse_transient_method(raw_value, ".tran method")
    return method


def _parse_transient_method(raw_value: str, context: str) -> TransientMethod:
    method = raw_value.strip().lower()
    if method in ("euler", "trap", "gear2"):
        return method
    raise NetlistParseError(f"{context} must be euler, trap, or gear2, got {raw_value!r}")


def _parse_option_value(raw_value: str) -> OptionValue:
    try:
        return parse_value(raw_value)
    except NetlistParseError:
        return raw_value


def _parse_voltage_probe(token: str, directive: str) -> str:
    match = re.fullmatch(r"(?i)v\(([^()\s]+)\)", token)
    if match is None:
        raise NetlistParseError(
            f"{directive} output must be a voltage probe V(node), got {token!r}"
        )
    return match.group(1)


def _parse_output_probe(token: str, directive: str) -> OutputProbe:
    match = re.fullmatch(r"(?i)([vi])\(([^()\s]+)\)", token)
    if match is None:
        raise NetlistParseError(
            f"{directive} probe must be V(node) or I(source), got {token!r}"
        )
    kind = "voltage" if match.group(1).lower() == "v" else "current"
    return OutputProbe(kind=kind, target=match.group(2))


def _parse_model_card(fields: list[str]) -> ModelCard:
    _require_min_fields(fields, 3, ".model")
    name = fields[1]
    kind: str
    params_text: str
    joined_tail = " ".join(fields[2:]).strip()
    inline = re.match(r"^([A-Za-z][A-Za-z0-9_]*)\s*(?:\((.*)\))?$", joined_tail)
    if len(fields) == 3 and inline is not None:
        kind = inline.group(1).upper()
        params_text = inline.group(2) or ""
    else:
        kind = fields[2].upper()
        params_text = " ".join(fields[3:]).strip()
        if params_text.startswith("(") and params_text.endswith(")"):
            params_text = params_text[1:-1]
    params = _parse_model_params(params_text)
    if kind == "D":
        saturation_current = params.get("IS", params.get("JS"))
        if saturation_current is not None and (
            not math.isfinite(saturation_current) or saturation_current <= 0.0
        ):
            raise NetlistParseError("diode IS must be finite and positive")
        thermal_voltage = params.get("VT", params.get("V_T"))
        if thermal_voltage is not None and (
            not math.isfinite(thermal_voltage) or thermal_voltage <= 0.0
        ):
            raise NetlistParseError("diode VT must be finite and positive")
    if kind in {"NJF", "PJF"}:
        gate_source_capacitance = params.get("CGS", params.get("CGS0"))
        if gate_source_capacitance is not None and (
            not math.isfinite(gate_source_capacitance)
            or gate_source_capacitance < 0.0
        ):
            raise NetlistParseError("JFET CGS must be finite and non-negative")
        gate_drain_capacitance = params.get("CGD", params.get("CGD0"))
        if gate_drain_capacitance is not None and (
            not math.isfinite(gate_drain_capacitance)
            or gate_drain_capacitance < 0.0
        ):
            raise NetlistParseError("JFET CGD must be finite and non-negative")
    if kind in {"NMOS", "PMOS"}:
        if "LEVEL" in params:
            level = params["LEVEL"]
            if not math.isfinite(level) or abs(level - 1.0) > 1.0e-12:
                raise NetlistParseError("only MOS LEVEL=1 model cards are supported")
        if "TOX" in params and (not math.isfinite(params["TOX"]) or params["TOX"] <= 0.0):
            raise NetlistParseError("MOSFET TOX must be finite and positive")
        substrate_doping = next(
            (params[name] for name in ("N_SUB", "NSUB", "N") if name in params),
            None,
        )
        if substrate_doping is not None and (
            not math.isfinite(substrate_doping) or substrate_doping <= 0.0
        ):
            raise NetlistParseError("MOSFET NSUB must be finite and positive")
        if "NSS" in params and (
            not math.isfinite(params["NSS"]) or params["NSS"] < 0.0
        ):
            raise NetlistParseError("MOSFET NSS must be finite and non-negative")
        if "TPG" in params and params["TPG"] not in {-1.0, 0.0, 1.0}:
            raise NetlistParseError("MOSFET TPG must be -1, 0, or 1")
        mobility = params.get("U0", params.get("UO"))
        if mobility is not None and (not math.isfinite(mobility) or mobility < 0.0):
            raise NetlistParseError("MOSFET U0 must be finite and non-negative")
        if "KP" in params and (not math.isfinite(params["KP"]) or params["KP"] <= 0.0):
            raise NetlistParseError("MOSFET KP must be finite and positive")
        threshold_voltage = next(
            (params[name] for name in ("VT0", "VTO", "VTH") if name in params),
            None,
        )
        if threshold_voltage is not None and not math.isfinite(threshold_voltage):
            raise NetlistParseError("MOSFET VT0 must be finite")
        channel_modulation = params.get("LAMBDA", params.get("LAM"))
        if channel_modulation is not None and not math.isfinite(channel_modulation):
            raise NetlistParseError("MOSFET LAMBDA must be finite")
        if "GAMMA" in params and (
            not math.isfinite(params["GAMMA"]) or params["GAMMA"] < 0.0
        ):
            raise NetlistParseError("MOSFET GAMMA must be finite and non-negative")
        if "PHI" in params and (
            not math.isfinite(params["PHI"]) or params["PHI"] <= 0.0
        ):
            raise NetlistParseError("MOSFET PHI must be finite and positive")
        if "W" in params and (
            not math.isfinite(params["W"]) or params["W"] <= 0.0
        ):
            raise NetlistParseError("MOSFET W must be finite and positive")
        if "L" in params and (
            not math.isfinite(params["L"]) or params["L"] <= 0.0
        ):
            raise NetlistParseError("MOSFET L must be finite and positive")
        if "LD" in params:
            length = params.get("L", Level1Params().L)
            lateral_diffusion = params["LD"]
            if (
                not math.isfinite(lateral_diffusion)
                or lateral_diffusion < 0.0
                or length - 2.0 * lateral_diffusion <= 0.0
            ):
                raise NetlistParseError(
                    "MOSFET LD must be finite and non-negative with L - 2*LD > 0"
                )
        if "IS" in params and (
            not math.isfinite(params["IS"]) or params["IS"] <= 0.0
        ):
            raise NetlistParseError("MOSFET IS must be finite and positive")
        if "RD" in params and (
            not math.isfinite(params["RD"]) or params["RD"] < 0.0
        ):
            raise NetlistParseError("MOSFET RD must be finite and non-negative")
        if "RS" in params and (
            not math.isfinite(params["RS"]) or params["RS"] < 0.0
        ):
            raise NetlistParseError("MOSFET RS must be finite and non-negative")
        if "RSH" in params and (
            not math.isfinite(params["RSH"]) or params["RSH"] < 0.0
        ):
            raise NetlistParseError("MOSFET RSH must be finite and non-negative")
        if "CJ" in params and (
            not math.isfinite(params["CJ"]) or params["CJ"] < 0.0
        ):
            raise NetlistParseError("MOSFET CJ must be finite and non-negative")
        if "CJSW" in params and (
            not math.isfinite(params["CJSW"]) or params["CJSW"] < 0.0
        ):
            raise NetlistParseError("MOSFET CJSW must be finite and non-negative")
        for parameter_name, canonical in (
            ("CBS", "CBS"),
            ("CJS", "CBS"),
            ("CBD", "CBD"),
            ("CJD", "CBD"),
        ):
            if parameter_name in params and (
                not math.isfinite(params[parameter_name])
                or params[parameter_name] < 0.0
            ):
                raise NetlistParseError(
                    f"MOSFET {canonical} must be finite and non-negative"
                )
        if "JS" in params and (
            not math.isfinite(params["JS"]) or params["JS"] < 0.0
        ):
            raise NetlistParseError("MOSFET JS must be finite and non-negative")
        if "PB" in params and (
            not math.isfinite(params["PB"]) or params["PB"] <= 0.0
        ):
            raise NetlistParseError("MOSFET PB must be finite and positive")
        if "MJ" in params and (
            not math.isfinite(params["MJ"]) or params["MJ"] < 0.0
        ):
            raise NetlistParseError("MOSFET MJ must be finite and non-negative")
        if "MJSW" in params and (
            not math.isfinite(params["MJSW"]) or params["MJSW"] < 0.0
        ):
            raise NetlistParseError("MOSFET MJSW must be finite and non-negative")
        if "FC" in params and (
            not math.isfinite(params["FC"]) or not 0.0 <= params["FC"] < 1.0
        ):
            raise NetlistParseError("MOSFET FC must be finite and in [0, 1)")
        if "KF" in params and (
            not math.isfinite(params["KF"]) or params["KF"] < 0.0
        ):
            raise NetlistParseError("MOSFET KF must be finite and non-negative")
        if "AF" in params and (
            not math.isfinite(params["AF"]) or params["AF"] < 0.0
        ):
            raise NetlistParseError("MOSFET AF must be finite and non-negative")
        nominal_temperature = params.get("T_NOM", params.get("TNOM"))
        if nominal_temperature is not None and (
            not math.isfinite(nominal_temperature) or nominal_temperature <= 0.0
        ):
            raise NetlistParseError("MOSFET TNOM must be finite and positive")
    return ModelCard(name=name, kind=kind, params=params)


def _parse_model_params(params_text: str) -> dict[str, float]:
    if not params_text.strip():
        return {}
    params: dict[str, float] = {}
    spans: list[tuple[int, int]] = []
    for match in re.finditer(r"([A-Za-z][A-Za-z0-9_]*)\s*=\s*([^,\s)]+)", params_text):
        params[match.group(1).upper()] = parse_value(match.group(2))
        spans.append(match.span())
    cursor = 0
    for start, end in spans:
        if params_text[cursor:start].strip(" \t,"):
            raise NetlistParseError(f"invalid .model parameter syntax {params_text!r}")
        cursor = end
    if params_text[cursor:].strip(" \t,"):
        raise NetlistParseError(f"invalid .model parameter syntax {params_text!r}")
    return params


def _parse_element_params(tokens: list[str], label: str) -> dict[str, float]:
    params: dict[str, float] = {}
    for token in tokens:
        if "=" not in token:
            raise NetlistParseError(f"invalid {label} parameter syntax {token!r}")
        name, value = token.split("=", 1)
        if not name or not value:
            raise NetlistParseError(f"invalid {label} parameter syntax {token!r}")
        params[name.upper()] = parse_value(value)
    return params


_LEVEL1_PARAM_ALIASES = {
    "NSUB": "N_SUB",
    "N": "N_SUB",
    "TNOM": "T_NOM",
    "UO": "U0",
    "VTO": "VT0",
    "VTH": "VT0",
    "LAM": "LAMBDA",
    "CJS": "CBS",
    "CJD": "CBD",
}


def _build_mosfet_model(model: ModelCard, instance_params: dict[str, float]) -> MOSFET:
    params = {**model.params, **instance_params}
    defaults = Level1Params()
    values = {
        field.name: getattr(defaults, field.name)
        for field in dataclass_fields(Level1Params)
        if field.init
    }
    for name, value in params.items():
        param_name = _LEVEL1_PARAM_ALIASES.get(name, name)
        if param_name in values and not isinstance(values[param_name], bool):
            values[param_name] = value
    canonical_params = {_LEVEL1_PARAM_ALIASES.get(name, name) for name in params}
    if "TOX" in model.params:
        derivation_params = dict(model.params)
        derivation_params["U0"] = values["U0"]
        normalized = normalize_model_card(model.name, model.kind, derivation_params)
        derived = mosfet_from_model_card("M", "d", "g", "s", "b", normalized)
        for field_name in ("KP", "VT0", "GAMMA", "PHI"):
            if field_name not in canonical_params:
                values[field_name] = getattr(derived.model.model.params, field_name)
    return MOSFET(
        type=MosfetType[model.kind],
        model=Level1Model(Level1Params(**values)),
    )


def _split_fields(line: str) -> list[str]:
    fields: list[str] = []
    current: list[str] = []
    depth = 0
    for char in line:
        if char.isspace() and depth == 0:
            if current:
                fields.append("".join(current))
                current = []
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth < 0:
                raise NetlistParseError("unmatched closing parenthesis")
        current.append(char)
    if depth != 0:
        raise NetlistParseError("unclosed parenthesis")
    if current:
        fields.append("".join(current))
    return fields


def _strip_inline_comment(line: str) -> str:
    return line.split(";", 1)[0]


def _require_fields(fields: list[str], count: int, label: str) -> None:
    if len(fields) != count:
        raise NetlistParseError(f"{label} expects {count} fields, got {len(fields)}")


def _require_min_fields(fields: list[str], count: int, label: str) -> None:
    if len(fields) < count:
        raise NetlistParseError(f"{label} expects at least {count} fields, got {len(fields)}")


def _require_max_fields(fields: list[str], count: int, label: str) -> None:
    if len(fields) > count:
        raise NetlistParseError(f"{label} expects at most {count} fields, got {len(fields)}")


def _pad(values: list[float], count: int, default: float) -> list[float]:
    return values + [default] * max(0, count - len(values))
