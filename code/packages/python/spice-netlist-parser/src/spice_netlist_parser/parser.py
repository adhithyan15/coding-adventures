"""Parser for a practical first slice of SPICE3 netlists."""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from spice_engine import (
    CCCS,
    VCCS,
    VCVS,
    Capacitor,
    Circuit,
    CurrentSource,
    ExpWaveform,
    Inductor,
    PulseWaveform,
    PwlWaveform,
    Resistor,
    SinWaveform,
    VoltageSource,
    Waveform,
)


class NetlistParseError(ValueError):
    """Raised when a SPICE netlist line is syntactically unsupported."""


@dataclass(frozen=True, slots=True)
class OpAnalysis:
    """A `.op` operating-point analysis card."""


@dataclass(frozen=True, slots=True)
class TranAnalysis:
    """A `.tran tstep tstop` transient analysis card."""

    t_step: float
    t_stop: float


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


type Analysis = OpAnalysis | TranAnalysis | DcAnalysis | AcAnalysis


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
    title: str | None = None

    def op_cards(self) -> list[OpAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, OpAnalysis)]

    def tran_cards(self) -> list[TranAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, TranAnalysis)]

    def dc_cards(self) -> list[DcAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, DcAnalysis)]

    def ac_cards(self) -> list[AcAnalysis]:
        return [analysis for analysis in self.analyses if isinstance(analysis, AcAnalysis)]


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
        try:
            if statement.fields[0].startswith("."):
                parsed.analyses.append(_parse_directive(statement.fields))
            elif statement.fields[0].upper().startswith("X"):
                for element in _expand_subckt_instance(statement.fields, subckts, []):
                    parsed.circuit.add(element)
            else:
                parsed.circuit.add(_parse_element(statement.fields))
        except NetlistParseError as exc:
            raise NetlistParseError(f"line {statement.line_number}: {exc}") from exc
    return parsed


def parse_value(token: str) -> float:
    """Parse a SPICE numeric token with an engineering suffix."""

    match = _VALUE_RE.match(token)
    if match is None:
        raise NetlistParseError(f"expected numeric value, got {token!r}")
    suffix = match.group(2).lower()
    if suffix not in _SUFFIXES:
        raise NetlistParseError(f"unsupported numeric suffix {match.group(2)!r}")
    return float(match.group(1)) * _SUFFIXES[suffix]


def _parse_element(fields: list[str]) -> object:
    name = fields[0]
    prefix = _element_prefix(name)
    if prefix == "R":
        _require_fields(fields, 4, "resistor")
        return Resistor(name, fields[1], fields[2], parse_value(fields[3]))
    if prefix == "C":
        _require_fields(fields, 4, "capacitor")
        return Capacitor(name, fields[1], fields[2], parse_value(fields[3]))
    if prefix == "L":
        _require_fields(fields, 4, "inductor")
        return Inductor(name, fields[1], fields[2], parse_value(fields[3]))
    if prefix == "V":
        _require_min_fields(fields, 4, "voltage source")
        voltage, waveform = _parse_source_value(fields[3:])
        return VoltageSource(name, fields[1], fields[2], voltage, waveform)
    if prefix == "I":
        _require_min_fields(fields, 4, "current source")
        current, waveform = _parse_source_value(fields[3:])
        return CurrentSource(name, fields[1], fields[2], current, waveform)
    if prefix == "G":
        _require_fields(fields, 6, "VCCS")
        return VCCS(name, fields[1], fields[2], fields[3], fields[4], parse_value(fields[5]))
    if prefix == "E":
        _require_fields(fields, 6, "VCVS")
        return VCVS(name, fields[1], fields[2], fields[3], fields[4], parse_value(fields[5]))
    if prefix == "F":
        _require_fields(fields, 5, "CCCS")
        return CCCS(name, fields[1], fields[2], fields[3], parse_value(fields[4]))
    raise NetlistParseError(f"unsupported element {name!r}")


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
            elements.extend(_expand_subckt_instance(local_fields, subckts, next_stack))
        else:
            elements.append(_parse_element(local_fields))
    return elements


def _map_subckt_fields(
    fields: list[str], instance_name: str, node_map: dict[str, str]
) -> list[str]:
    name = f"{instance_name}.{fields[0]}"
    prefix = fields[0][0].upper()
    mapped = [name, *fields[1:]]
    if prefix in {"R", "C", "L", "V", "I"}:
        _require_min_fields(fields, 3, "subcircuit element")
        mapped[1] = _map_subckt_node(fields[1], instance_name, node_map)
        mapped[2] = _map_subckt_node(fields[2], instance_name, node_map)
    elif prefix in {"E", "G"}:
        _require_min_fields(fields, 5, "subcircuit controlled source")
        for index in range(1, 5):
            mapped[index] = _map_subckt_node(fields[index], instance_name, node_map)
    elif prefix == "F":
        _require_min_fields(fields, 4, "subcircuit current-controlled source")
        mapped[1] = _map_subckt_node(fields[1], instance_name, node_map)
        mapped[2] = _map_subckt_node(fields[2], instance_name, node_map)
        mapped[3] = _map_subckt_source_ref(fields[3], instance_name)
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


def _parse_source_value(fields: list[str]) -> tuple[float, Waveform | None]:
    if not fields:
        raise NetlistParseError("source is missing a value")
    if fields[0].upper() == "DC":
        if len(fields) < 2:
            raise NetlistParseError("DC source form requires a value")
        return parse_value(fields[1]), None
    if len(fields) == 1 and "(" in fields[0]:
        waveform = _parse_waveform(fields[0])
        return waveform(0.0), waveform
    if fields[0].upper().startswith(("PWL(", "SIN(", "PULSE(", "EXP(")):
        joined = " ".join(fields)
        waveform = _parse_waveform(joined)
        return waveform(0.0), waveform
    return parse_value(fields[0]), None


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
        _require_fields(fields, 3, ".tran")
        return TranAnalysis(t_step=parse_value(fields[1]), t_stop=parse_value(fields[2]))
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
    raise NetlistParseError(f"unsupported directive {fields[0]!r}")


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


def _pad(values: list[float], count: int, default: float) -> list[float]:
    return values + [default] * max(0, count - len(values))
