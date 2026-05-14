"""Parser for a practical first slice of SPICE3 netlists."""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from spice_engine import (
    VCCS,
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
        if head.lower() == ".end":
            break
        try:
            if head.startswith("."):
                parsed.analyses.append(_parse_directive(fields))
            else:
                parsed.circuit.add(_parse_element(fields))
        except NetlistParseError as exc:
            raise NetlistParseError(f"line {line_number}: {exc}") from exc
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
    prefix = name[0].upper()
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
    raise NetlistParseError(f"unsupported element {name!r}")


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
