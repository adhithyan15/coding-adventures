"""Parse tiny text scores and render them through the note-to-sound stack.

This module is deliberately written as a teaching layer. The parser handles a
small human-friendly score format, then the renderer hands every note to
``musical-instruments`` and unpitched hits to ``percussion-kit`` so the lower
abstractions stay visible:

```text
note text -> frequency -> oscillator -> sampler -> PCM -> optional sink
drum hit  -> excitation -> modes -> envelope -> PCM -> optional sink
```

All regular expressions have been eliminated (CWE-1333) in favor of deterministic,
linear-time string scanning.
"""

from __future__ import annotations

import json
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field, replace
from importlib import import_module
from math import isfinite
from numbers import Integral, Real
from pathlib import Path
from typing import Literal

from musical_instruments import (
    MAX_NOTE_DURATION_SECONDS,
    MAX_PARTIALS,
    ADSREnvelope,
    HarmonicPartial,
    InstrumentNoteRender,
    InstrumentProfile,
    get_instrument,
    instrument_for_gm_program,
    render_instrument_note,
)
from note_audio import DEFAULT_MAX_SAMPLE_COUNT
from note_frequency import parse_note
from oscillator import sample_count_for_duration
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat
from percussion_kit import (
    PercussionHitEvent,
    PercussionHitRender,
    all_drum_kits,
    all_standard_hit_ids,
    render_percussion_sequence,
)

# Canonical resource limits (CWE-400 / CWE-190 / CWE-20)
MAX_SCORE_BYTES: int = 1_000_000
MAX_SCORE_LENGTH: int = 1_000_000
MAX_LINE_LENGTH: int = 10_000
MAX_EVENTS: int = 10_000
MAX_INSTRUMENTS: int = 128
MAX_TRACKS: int = 128
MAX_DRUM_HITS: int = 10_000
MIN_SAMPLE_RATE_HZ: float = 1000.0
MAX_SAMPLE_RATE_HZ: float = 192_000.0
MAX_SAMPLE_COUNT: int = 16_777_216
MAX_DURATION_SECONDS: float = 600.0
MAX_PPQ: int = 100_000

DEFAULT_TEMPO_BPM: float = 120.0
DEFAULT_METER: str = "4/4"
DEFAULT_AMPLITUDE: float = 0.18
DEFAULT_INSTRUMENT_ID: str = "sine"
DEFAULT_PPQ: int = 480
DEFAULT_MAX_SCORE_LENGTH: int = 1_000_000
DEFAULT_MAX_LINE_LENGTH: int = 10_000
DEFAULT_MAX_EVENT_COUNT: int = 10_000
BEAT_TICK_TOLERANCE: float = 1e-9
SEMITONES_PER_OCTAVE: int = 12

DEFAULT_ADDITIVE_ENVELOPE = ADSREnvelope(
    attack_seconds=0.005,
    decay_seconds=0.010,
    sustain_level=0.90,
    release_seconds=0.030,
)

CANONICAL_SHARP_SPELLINGS: tuple[str, ...] = (
    "C",
    "C#",
    "D",
    "D#",
    "E",
    "F",
    "F#",
    "G",
    "G#",
    "A",
    "A#",
    "B",
)

DURATION_BEATS: dict[str, float] = {
    "w": 4.0,
    "h": 2.0,
    "q": 1.0,
    "e": 0.5,
    "s": 0.25,
}

DIRECTIVE_NAMES: frozenset[str] = frozenset(
    {
        "title",
        "tempo",
        "meter",
        "amplitude",
        "sample_rate",
        "instrument",
        "program",
        "drumkit",
    }
)

PORTABLE_FORMAT_VERSION: str = "music-machine-score/v2"

HAPPY_BIRTHDAY_TEXT: str = """title: Happy Birthday
tempo: 120
meter: 3/4
amplitude: 0.18
instrument: sine
sample_rate: 44100

G4/e G4/e | A4/q G4/q C5/q | B4/h R/q |
G4/e G4/e | A4/q G4/q D5/q | C5/h R/q |
G4/e G4/e | G5/q E5/q C5/q | B4/q A4/q R/q |
F5/e F5/e | E5/q C5/q D5/q | C5/h
"""

MINI_ORCHESTRA_TEXT: str = """format: music-machine-score/v2
title: Mini Orchestra
ppq: 120
sample_rate: 16000
tempo 0 96
meter 0 4/4

instrument flute profile=flute_naive gain=0.20
instrument violin profile=violin_naive gain=0.17
instrument piano profile=piano_naive gain=0.14
instrument bass program=33 gain=0.15

track melody instrument=flute
track harmony instrument=violin
track comp instrument=piano
track low instrument=bass

event melody 0 120 note C5 velocity=0.72
event melody 120 120 note D5 velocity=0.72
event melody 240 120 note E5 velocity=0.72
event melody 360 120 note G5 velocity=0.72
event harmony 0 240 note E4 velocity=0.48
event harmony 240 240 note G4 velocity=0.42
event comp 0 240 note C4,E4,G4 velocity=0.42
event comp 240 240 note C4,E4,G4 velocity=0.36
event low 0 480 note C2 velocity=0.56

event melody 480 120 note E5 velocity=0.72
event melody 600 120 note D5 velocity=0.72
event melody 720 120 note C5 velocity=0.72
event melody 840 120 note G4 velocity=0.72
event harmony 480 240 note A4 velocity=0.48
event harmony 720 240 note C5 velocity=0.42
event comp 480 240 note F4,A4,C5 velocity=0.42
event comp 720 240 note F4,A4,C5 velocity=0.36
event low 480 480 note F2 velocity=0.56

event melody 960 120 note A4 velocity=0.72
event melody 1080 120 note C5 velocity=0.72
event melody 1200 120 note D5 velocity=0.72
event melody 1320 120 note E5 velocity=0.72
event harmony 960 240 note B4 velocity=0.48
event harmony 1200 240 note D5 velocity=0.42
event comp 960 240 note G3,B3,D4 velocity=0.42
event comp 1200 240 note G3,B3,D4 velocity=0.36
event low 960 480 note G2 velocity=0.56

event melody 1440 120 note G5 velocity=0.72
event melody 1560 120 note E5 velocity=0.72
event melody 1680 120 note D5 velocity=0.72
event melody 1800 120 note C5 velocity=0.72
event harmony 1440 240 note E4 velocity=0.48
event harmony 1680 240 note B4 velocity=0.42
event comp 1440 240 note C4,E4,G4 velocity=0.42
event comp 1680 240 note C4,E4,G4 velocity=0.36
event low 1440 480 note C2 velocity=0.56
"""

PITCHED_PERCUSSION_MIX_TEXT: str = """format: music-machine-score/v2
title: Pitched Percussion Mix
ppq: 120
sample_rate: 44100
tempo 0 100
meter 0 4/4

instrument flute profile=flute_naive gain=0.12
instrument piano profile=piano_naive gain=0.10
instrument glock profile=glockenspiel_naive gain=0.08
instrument vibes profile=vibraphone_naive gain=0.08
instrument timp profile=timpani_naive gain=0.10

track melody instrument=flute
track comp instrument=piano
track sparkle instrument=glock
track sustain instrument=vibes
track drum instrument=timp

event melody 0 120 note C5 velocity=0.55
event melody 120 120 note D5 velocity=0.55
event melody 240 120 note E5 velocity=0.55
event melody 360 120 note G5 velocity=0.55
event melody 480 120 note E5 velocity=0.55
event melody 600 120 note D5 velocity=0.55
event melody 720 120 note C5 velocity=0.55
event melody 840 240 note G4 velocity=0.55
event melody 1080 120 note A4 velocity=0.55
event melody 1200 120 note C5 velocity=0.55
event melody 1320 120 note D5 velocity=0.55
event melody 1440 120 note E5 velocity=0.55
event melody 1560 120 note G5 velocity=0.55
event melody 1680 120 note E5 velocity=0.55
event melody 1800 120 note D5 velocity=0.55
event melody 1920 240 note C5 velocity=0.55

event comp 0 240 note C4,E4,G4 velocity=0.32
event comp 240 240 note C4,E4,G4 velocity=0.28
event comp 480 240 note F4,A4,C5 velocity=0.32
event comp 720 240 note F4,A4,C5 velocity=0.28
event comp 960 240 note G3,B3,D4 velocity=0.32
event comp 1200 240 note G3,B3,D4 velocity=0.28
event comp 1440 240 note C4,E4,G4 velocity=0.32
event comp 1680 240 note C4,E4,G4 velocity=0.28

event sparkle 0 60 note C6 velocity=0.30
event sparkle 240 60 note G5 velocity=0.24
event sparkle 480 60 note A5 velocity=0.30
event sparkle 720 60 note C6 velocity=0.24
event sparkle 960 60 note D6 velocity=0.30
event sparkle 1200 60 note B5 velocity=0.24
event sparkle 1440 60 note C6 velocity=0.30
event sparkle 1680 60 note G5 velocity=0.24

event sustain 0 480 note G4 velocity=0.18
event sustain 480 480 note A4 velocity=0.18
event sustain 960 480 note B4 velocity=0.18
event sustain 1440 480 note G4 velocity=0.18

event drum 0 120 note C3 velocity=0.34
event drum 480 120 note F2 velocity=0.34
event drum 960 120 note G2 velocity=0.34
event drum 1440 120 note C3 velocity=0.34
"""

ScoreEventKind = Literal["note", "rest", "hit"]
PortableEventKind = Literal["note", "rest", "hit"]


# =============================================================================
# Defensive Numeric & String Validation (VER15 / CWE-1333 / CWE-400 / CWE-20)
# =============================================================================


def _finite_float(name: str, value: Real) -> float:
    """Convert a finite real value to ``float`` and reject bool/string traps."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")
    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")
    return converted


def _positive_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted <= 0.0:
        raise ValueError(f"{name} must be > 0.0, got {converted}")
    return converted


def _non_negative_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted < 0.0:
        raise ValueError(f"{name} must be >= 0.0, got {converted}")
    return converted


def _unit_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted < 0.0 or converted > 1.0:
        raise ValueError(f"{name} must be in [0.0, 1.0], got {converted}")
    return converted


def _positive_integer(name: str, value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer > 0, got {value!r}")
    converted = int(value)
    if converted <= 0:
        raise ValueError(f"{name} must be > 0, got {converted}")
    return converted


def _non_negative_integer(name: str, value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer >= 0, got {value!r}")
    converted = int(value)
    if converted < 0:
        raise ValueError(f"{name} must be >= 0, got {converted}")
    return converted


def _non_negative_tick(name: str, value: int) -> int:
    return _non_negative_integer(name, value)


def _signed_integer(name: str, value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer, got {value!r}")
    return int(value)


def _integer_valued_sample_rate(text: str) -> int:
    try:
        value = float(text)
    except (ValueError, TypeError) as exc:
        raise ValueError(f"sample_rate must be an integer, got {text!r}") from exc

    if not isfinite(value) or value != round(value):
        raise ValueError(f"sample_rate must be an integer, got {text!r}")
    rate = int(round(value))
    if rate <= 0:
        raise ValueError(f"sample_rate must be > 0, got {rate}")
    if rate > MAX_SAMPLE_RATE_HZ:
        raise ValueError(
            f"sample_rate {rate} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
        )
    return rate


def _integer_directive(name: str, text: str) -> int:
    try:
        value = float(text)
    except (ValueError, TypeError) as exc:
        raise ValueError(f"{name} must be an integer, got {text!r}") from exc

    if not isfinite(value) or value != round(value):
        raise ValueError(f"{name} must be an integer, got {text!r}")
    return int(round(value))


def _parse_float_directive(name: str, text: str) -> float:
    try:
        value = float(text)
    except (ValueError, TypeError) as exc:
        raise ValueError(f"{name} must be a number, got {text!r}") from exc

    if not isfinite(value):
        raise ValueError(f"{name} must be a number, got {text!r}")
    return value


# CWE-1333 Linear-time scanners (replacing re.compile)


def _parse_directive_line(line: str) -> tuple[str, str] | None:
    """Parse a directive line matching ^([A-Za-z_][A-Za-z0-9_]*):\\s*(.*)$ linearly."""
    colon_idx = line.find(":")
    if colon_idx == -1:
        return None
    name = line[:colon_idx]
    if not name:
        return None
    first = name[0]
    if not (("a" <= first <= "z") or ("A" <= first <= "Z") or first == "_"):
        return None
    for ch in name[1:]:
        if not (
            ("a" <= ch <= "z") or ("A" <= ch <= "Z") or ("0" <= ch <= "9") or ch == "_"
        ):
            return None
    value = line[colon_idx + 1 :].lstrip(" \t")
    return name, value


def _is_identifier(text: str) -> bool:
    """Check text matches ^[A-Za-z_][A-Za-z0-9_-]*$ linearly."""
    if not text:
        return False
    first = text[0]
    if not (("a" <= first <= "z") or ("A" <= first <= "Z") or first == "_"):
        return False
    for ch in text[1:]:
        if not (
            ("a" <= ch <= "z")
            or ("A" <= ch <= "Z")
            or ("0" <= ch <= "9")
            or ch in ("_", "-")
        ):
            return False
    return True


def _validate_identifier(name: str, value: str) -> str:
    converted = str(value)
    if not _is_identifier(converted):
        raise ValueError(f"{name} must be an identifier, got {value!r}")
    return converted


def _is_valid_meter(text: str) -> bool:
    """Check meter matches ^([1-9][0-9]*)/([1-9][0-9]*)$ linearly."""
    if text.count("/") != 1:
        return False
    num_str, den_str = text.split("/", 1)
    if not num_str or not den_str:
        return False
    if not num_str.isdigit() or num_str.startswith("0"):
        return False
    return den_str.isdigit() and not den_str.startswith("0")


def _validate_meter(text: str) -> str:
    if not _is_valid_meter(text):
        raise ValueError(f"meter must look like positive beats/beat-unit, got {text!r}")
    return text


def _parse_properties(tokens: list[str]) -> dict[str, str]:
    properties: dict[str, str] = {}
    for token in tokens:
        if token.count("=") != 1:
            raise ValueError(f"property {token!r} must be key=value")
        key, value = token.split("=", maxsplit=1)
        if key == "" or value == "":
            raise ValueError(f"property {token!r} must be key=value")
        if key in properties:
            raise ValueError(f"duplicate property {key!r}")
        properties[key] = value
    return properties


def _parse_tick(text: str, name: str) -> int:
    try:
        value = int(text)
    except (ValueError, TypeError) as exc:
        raise ValueError(f"{name} must be an integer, got {text!r}") from exc
    return _non_negative_tick(name, value)


def _safe_round_sample(seconds: Real, rate: Real) -> int:
    """Guard float-to-int sample rounding against overflow (CWE-190)."""
    val = float(seconds) * float(rate)
    if not isfinite(val) or val < 0.0:
        return 0
    if val > MAX_SAMPLE_COUNT:
        return MAX_SAMPLE_COUNT
    return int(round(val))


def _validate_file_path(path: str | Path) -> Path:
    """Validate path against null bytes, symlinks, and existence (CWE-22)."""
    raw = str(path)
    if "\0" in raw:
        raise ValueError(f"file path contains null byte: {raw!r}")
    p = Path(path)
    if p.is_symlink():
        raise ValueError(f"symlinks are not permitted: {raw!r}")
    if not p.is_file():
        raise FileNotFoundError(f"score file not found: {raw!r}")
    return p


# =============================================================================
# Additive Harmonics & Envelope Parsers (MUS05 §9)
# =============================================================================


def _parse_harmonics(text: str) -> tuple[HarmonicPartial, ...]:
    """Parse comma-separated harmonic partials e.g. '1:1.0,2:0.4,3:0.2'."""
    partials: list[HarmonicPartial] = []
    items = [item.strip() for item in text.split(",") if item.strip()]
    if not items:
        raise ValueError("harmonics must contain at least one partial")
    if len(items) > MAX_PARTIALS:
        raise ValueError(
            f"harmonics count {len(items)} exceeds MAX_PARTIALS={MAX_PARTIALS}"
        )

    for item in items:
        parts = item.split(":")
        if len(parts) == 2:
            mult_s, amp_s = parts
            phase_s = "0.0"
        elif len(parts) == 3:
            mult_s, amp_s, phase_s = parts
        else:
            raise ValueError(
                f"harmonic partial {item!r} must be multiplier:amplitude[:phase]"
            )

        mult = _positive_float(
            "frequency_multiplier", _parse_float_directive("multiplier", mult_s)
        )
        amp = _non_negative_float(
            "amplitude", _parse_float_directive("amplitude", amp_s)
        )
        phase = _finite_float(
            "phase_cycles", _parse_float_directive("phase_cycles", phase_s)
        )
        partials.append(
            HarmonicPartial(
                frequency_multiplier=mult,
                amplitude=amp,
                phase_cycles=phase,
            )
        )
    return tuple(partials)


def _parse_envelope(text: str) -> ADSREnvelope:
    """Parse comma-separated ADSR envelope 'attack,decay,sustain,release'."""
    parts = [part.strip() for part in text.split(",")]
    if len(parts) != 4:
        raise ValueError(f"envelope must be attack,decay,sustain,release, got {text!r}")
    atk = _non_negative_float(
        "attack_seconds", _parse_float_directive("attack", parts[0])
    )
    dec = _non_negative_float(
        "decay_seconds", _parse_float_directive("decay", parts[1])
    )
    sus = _unit_float("sustain_level", _parse_float_directive("sustain", parts[2]))
    rel = _non_negative_float(
        "release_seconds", _parse_float_directive("release", parts[3])
    )

    if (
        atk > MAX_NOTE_DURATION_SECONDS
        or dec > MAX_NOTE_DURATION_SECONDS
        or rel > MAX_NOTE_DURATION_SECONDS
    ):
        raise ValueError(f"envelope times must be <= {MAX_NOTE_DURATION_SECONDS}")

    return ADSREnvelope(
        attack_seconds=atk,
        decay_seconds=dec,
        sustain_level=sus,
        release_seconds=rel,
    )


# =============================================================================
# Core Domain Models
# =============================================================================


def resolve_instrument_id(
    *,
    instrument_id: str | None = None,
    gm_program: int | None = None,
) -> str:
    """Resolve score instrument settings to a concrete profile id."""
    if instrument_id is not None and gm_program is not None:
        raise ValueError("score may specify instrument or program, not both")
    if gm_program is not None:
        return instrument_for_gm_program(gm_program).id
    if instrument_id is None:
        return DEFAULT_INSTRUMENT_ID
    if (
        instrument_id in {"drumkit", "standard_kit", "standard"}
        or instrument_id in all_drum_kits()
    ):
        return instrument_id
    return get_instrument(instrument_id).id


def beats_for_duration_symbol(symbol: str) -> float:
    """Return how many quarter-note beats a duration symbol represents."""
    if not isinstance(symbol, str) or symbol == "":
        raise ValueError("duration symbol must be non-empty text")
    if symbol.count(".") > 1:
        raise ValueError(f"duration symbol {symbol!r} may have at most one dot")

    dotted = symbol.endswith(".")
    base_symbol = symbol[:-1] if dotted else symbol
    try:
        base_beats = DURATION_BEATS[base_symbol]
    except KeyError as exc:
        raise ValueError(f"unknown duration symbol {symbol!r}") from exc

    return base_beats * 1.5 if dotted else base_beats


@dataclass(frozen=True)
class ScoreEvent:
    """One musical instruction: play a note, rest, or strike a percussion hit."""

    kind: ScoreEventKind
    note: str | None
    duration_symbol: str
    beat_count: float
    duration_seconds: float
    source_token: str
    hit_id: str | None = None

    def __post_init__(self) -> None:
        if self.kind not in {"note", "rest", "hit"}:
            raise ValueError(
                f"kind must be 'note', 'rest', or 'hit', got {self.kind!r}"
            )
        if self.kind == "note":
            if self.note is None:
                raise ValueError("note events must include a note")
            object.__setattr__(self, "note", str(parse_note(self.note)))
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "rest":
            if self.note is not None:
                raise ValueError("rest events cannot include a note")
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "hit":
            hit = self.hit_id or self.note
            if hit is None:
                raise ValueError("hit events must include a hit_id")
            hit_str = str(hit).lower()
            if hit_str not in all_standard_hit_ids():
                raise ValueError(f"unknown hit_id {hit_str!r}")
            object.__setattr__(self, "hit_id", hit_str)
            object.__setattr__(self, "note", None)

        object.__setattr__(
            self,
            "beat_count",
            _positive_float("beat_count", self.beat_count),
        )
        object.__setattr__(
            self,
            "duration_seconds",
            _positive_float("duration_seconds", self.duration_seconds),
        )
        object.__setattr__(self, "duration_symbol", str(self.duration_symbol))
        object.__setattr__(self, "source_token", str(self.source_token))


@dataclass(frozen=True)
class TextScore:
    """A parsed score plus the global settings needed to render it."""

    title: str
    tempo_bpm: float
    meter: str
    amplitude: float
    sample_rate_hz: int
    instrument_id: str
    gm_program: int | None
    events: tuple[ScoreEvent, ...]
    drumkit: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(self, "title", str(self.title))
        object.__setattr__(
            self,
            "tempo_bpm",
            _positive_float("tempo_bpm", self.tempo_bpm),
        )
        object.__setattr__(self, "meter", _validate_meter(self.meter))
        object.__setattr__(self, "amplitude", _unit_float("amplitude", self.amplitude))
        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_integer("sample_rate_hz", self.sample_rate_hz),
        )
        if self.sample_rate_hz > MAX_SAMPLE_RATE_HZ:
            raise ValueError(
                f"sample_rate_hz {self.sample_rate_hz} exceeds "
                f"MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
            )
        gm_program = None
        if self.gm_program is not None:
            gm_program = _positive_integer("gm_program", self.gm_program)
            if gm_program > 128:
                raise ValueError("gm_program must be in [1, 128]")
        object.__setattr__(self, "gm_program", gm_program)

        if (
            self.drumkit is not None
            or self.instrument_id in {"drumkit", "standard_kit", "standard"}
            or self.instrument_id in all_drum_kits()
        ):
            kit_name = self.drumkit or (
                "standard_kit"
                if self.instrument_id in {"drumkit", "standard_kit", "standard"}
                else self.instrument_id
            )
            if kit_name in {"standard", "standard_kit", "drumkit"}:
                kit_name = "standard_kit"
            object.__setattr__(self, "drumkit", kit_name)
            object.__setattr__(self, "instrument_id", "drumkit")
        else:
            if gm_program is None:
                resolved_instrument_id = resolve_instrument_id(
                    instrument_id=str(self.instrument_id),
                )
            else:
                resolved_instrument_id = resolve_instrument_id(gm_program=gm_program)
            object.__setattr__(self, "instrument_id", resolved_instrument_id)

        object.__setattr__(self, "events", tuple(self.events))
        if not self.events:
            raise ValueError("score must contain at least one event")
        if len(self.events) > MAX_EVENTS:
            raise ValueError(f"score event count exceeds MAX_EVENTS={MAX_EVENTS}")

    def seconds_per_beat(self) -> float:
        return 60.0 / self.tempo_bpm

    def total_duration_seconds(self) -> float:
        return sum(event.duration_seconds for event in self.events)


@dataclass(frozen=True)
class RenderedScore:
    """The joined PCM output plus inspectable per-note and per-hit renderings."""

    score: TextScore
    pcm_buffer: PCMBuffer
    rendered_notes: tuple[InstrumentNoteRender, ...]
    rendered_hits: tuple[PercussionHitRender, ...] = ()

    def __post_init__(self) -> None:
        if not isinstance(self.score, TextScore):
            raise ValueError("score must be a TextScore")
        if not isinstance(self.pcm_buffer, PCMBuffer):
            raise ValueError("pcm_buffer must be a PCMBuffer")
        object.__setattr__(self, "rendered_notes", tuple(self.rendered_notes))
        for index, rendered_note in enumerate(self.rendered_notes):
            if not isinstance(rendered_note, InstrumentNoteRender):
                raise ValueError(
                    f"rendered_notes[{index}] must be an InstrumentNoteRender"
                )
        object.__setattr__(self, "rendered_hits", tuple(self.rendered_hits))
        for index, hit in enumerate(self.rendered_hits):
            if not isinstance(hit, PercussionHitRender):
                raise ValueError(
                    f"rendered_hits[{index}] must be a PercussionHitRender"
                )


@dataclass(frozen=True)
class TempoEvent:
    """A tempo change in the portable multi-track format."""

    start_tick: int
    bpm: float

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "start_tick",
            _non_negative_tick("start_tick", self.start_tick),
        )
        object.__setattr__(self, "bpm", _positive_float("bpm", self.bpm))

    def to_dict(self) -> dict[str, object]:
        return {"bpm": self.bpm, "start_tick": self.start_tick}

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> TempoEvent:
        if not isinstance(data, Mapping):
            raise ValueError("TempoEvent must be a mapping")
        if "start_tick" not in data or "bpm" not in data:
            raise ValueError("TempoEvent missing required keys 'start_tick' and 'bpm'")
        return cls(
            start_tick=_non_negative_tick("start_tick", data["start_tick"]),  # type: ignore
            bpm=_positive_float("bpm", data["bpm"]),  # type: ignore
        )


@dataclass(frozen=True)
class MeterEvent:
    """A meter change in the portable multi-track format."""

    start_tick: int
    meter: str

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "start_tick",
            _non_negative_tick("start_tick", self.start_tick),
        )
        object.__setattr__(self, "meter", _validate_meter(self.meter))

    def to_dict(self) -> dict[str, object]:
        return {"meter": self.meter, "start_tick": self.start_tick}

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> MeterEvent:
        if not isinstance(data, Mapping):
            raise ValueError("MeterEvent must be a mapping")
        if "start_tick" not in data or "meter" not in data:
            raise ValueError(
                "MeterEvent missing required keys 'start_tick' and 'meter'"
            )
        if not isinstance(data["meter"], str):
            raise ValueError("MeterEvent 'meter' must be a string")
        return cls(
            start_tick=_non_negative_tick("start_tick", data["start_tick"]),  # type: ignore
            meter=_validate_meter(data["meter"]),
        )


@dataclass(frozen=True)
class InstrumentDeclaration:
    """A named instrument available to portable-score tracks."""

    id: str
    profile_id: str
    gain: float = 1.0
    kind: str = "preset"
    kit: str | None = None
    harmonics: tuple[HarmonicPartial, ...] | None = None
    envelope: ADSREnvelope | None = None
    custom_profile: InstrumentProfile | None = None

    def __post_init__(self) -> None:
        object.__setattr__(self, "id", _validate_identifier("instrument id", self.id))
        object.__setattr__(self, "gain", _unit_float("gain", self.gain))

        kind = self.kind
        if (
            kind == "drumkit"
            or self.profile_id.startswith("drumkit")
            or self.kit is not None
        ):
            object.__setattr__(self, "kind", "drumkit")
            kit_name = self.kit or "standard_kit"
            if kit_name == "standard":
                kit_name = "standard_kit"
            valid_kit_ids = {k.id for k in all_drum_kits()} | {"standard_kit"}
            if kit_name not in valid_kit_ids:
                raise ValueError(f"unknown drum kit {kit_name!r}")
            object.__setattr__(self, "kit", kit_name)
            object.__setattr__(self, "profile_id", "drumkit")
        elif kind == "additive" or self.custom_profile is not None:
            object.__setattr__(self, "kind", "additive")
            if self.harmonics is None and self.custom_profile is None:
                raise ValueError("kind=additive must provide harmonics")
            if self.custom_profile is None:
                harmonics = tuple(self.harmonics) if self.harmonics else ()
                envelope = self.envelope or DEFAULT_ADDITIVE_ENVELOPE
                profile = InstrumentProfile(
                    id=self.id,
                    display_name=self.id,
                    synthesis_kind="additive",
                    gain=1.0,
                    harmonic_profile=harmonics,
                    envelope_profile=envelope,
                )
                object.__setattr__(self, "custom_profile", profile)
                object.__setattr__(self, "harmonics", harmonics)
                object.__setattr__(self, "envelope", envelope)
            else:
                object.__setattr__(
                    self, "harmonics", self.custom_profile.harmonic_profile
                )
                object.__setattr__(
                    self, "envelope", self.custom_profile.envelope_profile
                )
            object.__setattr__(self, "profile_id", self.id)
        else:
            resolved = resolve_instrument_id(instrument_id=self.profile_id)
            object.__setattr__(self, "profile_id", resolved)
            object.__setattr__(self, "kind", "preset")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "id": self.id,
            "profile_id": self.profile_id,
            "gain": self.gain,
            "kind": self.kind,
        }
        if self.kit is not None:
            d["kit"] = self.kit
        if self.harmonics is not None:
            d["harmonics"] = [
                [p.frequency_multiplier, p.amplitude, p.phase_cycles]
                for p in self.harmonics
            ]
        if self.envelope is not None:
            d["envelope"] = [
                self.envelope.attack_seconds,
                self.envelope.decay_seconds,
                self.envelope.sustain_level,
                self.envelope.release_seconds,
            ]
        return d

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> InstrumentDeclaration:
        if not isinstance(data, Mapping):
            raise ValueError("InstrumentDeclaration must be a mapping")
        if "id" not in data:
            raise ValueError("InstrumentDeclaration missing 'id'")
        if not isinstance(data["id"], str):
            raise ValueError("InstrumentDeclaration 'id' must be a string")
        inst_id = data["id"]
        gain = _unit_float("gain", data.get("gain", 1.0))  # type: ignore

        raw_kind = data.get("kind", "preset")
        if not isinstance(raw_kind, str):
            raise ValueError("InstrumentDeclaration 'kind' must be a string")
        kind = raw_kind

        raw_kit = data.get("kit")
        if raw_kit is not None and not isinstance(raw_kit, str):
            raise ValueError("InstrumentDeclaration 'kit' must be a string")
        kit = raw_kit

        harmonics = None
        if "harmonics" in data and data["harmonics"] is not None:
            raw_harmonics = data["harmonics"]
            if not isinstance(raw_harmonics, (list, tuple)):
                raise ValueError("harmonics must be a list of partials")
            if len(raw_harmonics) > MAX_PARTIALS:
                raise ValueError(
                    f"harmonic partial count {len(raw_harmonics)} exceeds "
                    f"MAX_PARTIALS={MAX_PARTIALS}"
                )
            h_list: list[HarmonicPartial] = []
            for item in raw_harmonics:
                if isinstance(item, (list, tuple)) and len(item) in (2, 3):
                    m = _positive_float("multiplier", item[0])
                    a = _non_negative_float("amplitude", item[1])
                    p = _finite_float("phase", item[2] if len(item) == 3 else 0.0)
                    h_list.append(HarmonicPartial(m, a, p))
                else:
                    raise ValueError(f"malformed partial {item!r}")
            harmonics = tuple(h_list)

        envelope = None
        if "envelope" in data and data["envelope"] is not None:
            raw_env = data["envelope"]
            if not isinstance(raw_env, (list, tuple)) or len(raw_env) != 4:
                raise ValueError("envelope must be [atk, dec, sus, rel]")
            atk = _non_negative_float("attack", raw_env[0])
            dec = _non_negative_float("decay", raw_env[1])
            sus = _unit_float("sustain", raw_env[2])
            rel = _non_negative_float("release", raw_env[3])
            if (
                atk > MAX_NOTE_DURATION_SECONDS
                or dec > MAX_NOTE_DURATION_SECONDS
                or rel > MAX_NOTE_DURATION_SECONDS
            ):
                raise ValueError(
                    f"envelope times must be <= {MAX_NOTE_DURATION_SECONDS}"
                )
            envelope = ADSREnvelope(
                attack_seconds=atk,
                decay_seconds=dec,
                sustain_level=sus,
                release_seconds=rel,
            )

        raw_profile = data.get("profile_id", inst_id)
        if not isinstance(raw_profile, str):
            raise ValueError("InstrumentDeclaration 'profile_id' must be a string")
        profile_id = raw_profile
        return cls(
            id=inst_id,
            profile_id=profile_id,
            gain=gain,
            kind=kind,
            kit=kit,
            harmonics=harmonics,
            envelope=envelope,
        )


@dataclass(frozen=True)
class TrackDeclaration:
    """A playable track in the portable multi-track format."""

    id: str
    instrument_id: str

    def __post_init__(self) -> None:
        object.__setattr__(self, "id", _validate_identifier("track id", self.id))
        object.__setattr__(
            self,
            "instrument_id",
            _validate_identifier("instrument_id", self.instrument_id),
        )

    def to_dict(self) -> dict[str, object]:
        return {"id": self.id, "instrument_id": self.instrument_id}

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> TrackDeclaration:
        if not isinstance(data, Mapping):
            raise ValueError("TrackDeclaration must be a mapping")
        if "id" not in data or "instrument_id" not in data:
            raise ValueError(
                "TrackDeclaration missing required keys 'id' and 'instrument_id'"
            )
        if not isinstance(data["id"], str):
            raise ValueError("TrackDeclaration 'id' must be a string")
        if not isinstance(data["instrument_id"], str):
            raise ValueError("TrackDeclaration 'instrument_id' must be a string")
        return cls(
            id=data["id"],
            instrument_id=data["instrument_id"],
        )


@dataclass(frozen=True)
class PortableScoreEvent:
    """An explicit timed note, chord, rest, or percussion hit event."""

    track_id: str
    start_tick: int
    duration_tick: int
    kind: PortableEventKind
    notes: tuple[str, ...] = ()
    velocity: float = 1.0
    source_order: int = 0
    hit_id: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "track_id",
            _validate_identifier("track_id", self.track_id),
        )
        object.__setattr__(
            self,
            "start_tick",
            _non_negative_tick("start_tick", self.start_tick),
        )
        object.__setattr__(
            self,
            "duration_tick",
            _positive_integer("duration_tick", self.duration_tick),
        )
        if self.kind not in {"note", "rest", "hit"}:
            raise ValueError(
                f"kind must be 'note', 'rest', or 'hit', got {self.kind!r}"
            )

        if self.kind == "note":
            notes = tuple(str(parse_note(note)) for note in self.notes)
            if not notes:
                raise ValueError("note events must include at least one note")
            object.__setattr__(self, "notes", notes)
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "rest":
            if self.notes:
                raise ValueError("rest events cannot include notes")
            object.__setattr__(self, "notes", ())
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "hit":
            if self.notes:
                raise ValueError("hit events cannot include notes")
            hit = self.hit_id
            if hit is None:
                raise ValueError("hit events must include a hit_id")
            hit_str = str(hit).lower()
            if hit_str not in all_standard_hit_ids():
                raise ValueError(f"unknown hit_id {hit_str!r}")
            object.__setattr__(self, "hit_id", hit_str)
            object.__setattr__(self, "notes", ())

        object.__setattr__(self, "velocity", _unit_float("velocity", self.velocity))
        object.__setattr__(
            self,
            "source_order",
            _non_negative_tick("source_order", self.source_order),
        )

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "duration_tick": self.duration_tick,
            "kind": self.kind,
            "notes": list(self.notes),
            "source_order": self.source_order,
            "start_tick": self.start_tick,
            "track_id": self.track_id,
            "velocity": self.velocity,
        }
        if self.hit_id is not None:
            d["hit_id"] = self.hit_id
        return d

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> PortableScoreEvent:
        if not isinstance(data, Mapping):
            raise ValueError("PortableScoreEvent must be a mapping")
        for key in ("track_id", "start_tick", "duration_tick", "kind"):
            if key not in data:
                raise ValueError(f"PortableScoreEvent missing required key {key!r}")
        if not isinstance(data["track_id"], str):
            raise ValueError("PortableScoreEvent 'track_id' must be a string")
        if not isinstance(data["kind"], str):
            raise ValueError("PortableScoreEvent 'kind' must be a string")

        raw_notes = data.get("notes", ())
        if not isinstance(raw_notes, (list, tuple)):
            raise ValueError("PortableScoreEvent 'notes' must be a list or tuple")
        notes = tuple(str(n) for n in raw_notes)

        raw_hit_id = data.get("hit_id")
        if raw_hit_id is not None and not isinstance(raw_hit_id, str):
            raise ValueError("PortableScoreEvent 'hit_id' must be a string")
        hit_id = raw_hit_id

        return cls(
            track_id=data["track_id"],
            start_tick=_non_negative_tick("start_tick", data["start_tick"]),  # type: ignore
            duration_tick=_positive_integer("duration_tick", data["duration_tick"]),  # type: ignore
            kind=data["kind"],  # type: ignore
            notes=notes,
            hit_id=hit_id,
            velocity=_unit_float("velocity", data.get("velocity", 1.0)),  # type: ignore
            source_order=_non_negative_tick(
                "source_order", data.get("source_order", 0)
            ),  # type: ignore
        )


@dataclass(frozen=True)
class PortableScore:
    """A parsed `music-machine-score/v2` multi-track score."""

    format_version: str
    title: str
    ppq: int
    sample_rate_hz: int
    tempo_events: tuple[TempoEvent, ...]
    meter_events: tuple[MeterEvent, ...]
    instruments: tuple[InstrumentDeclaration, ...]
    tracks: tuple[TrackDeclaration, ...]
    events: tuple[PortableScoreEvent, ...]

    def __post_init__(self) -> None:
        if self.format_version != PORTABLE_FORMAT_VERSION:
            raise ValueError(
                f"format_version must be {PORTABLE_FORMAT_VERSION!r}, "
                f"got {self.format_version!r}"
            )
        object.__setattr__(self, "title", str(self.title))
        object.__setattr__(self, "ppq", _positive_integer("ppq", self.ppq))
        if self.ppq > MAX_PPQ:
            raise ValueError(f"ppq {self.ppq} exceeds MAX_PPQ={MAX_PPQ}")

        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_integer("sample_rate_hz", self.sample_rate_hz),
        )
        if self.sample_rate_hz > MAX_SAMPLE_RATE_HZ:
            raise ValueError(
                f"sample_rate_hz {self.sample_rate_hz} exceeds "
                f"MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
            )

        tempos = tuple(sorted(self.tempo_events, key=lambda event: event.start_tick))
        if not tempos or tempos[0].start_tick != 0:
            tempos = (TempoEvent(0, DEFAULT_TEMPO_BPM),) + tempos
        meters = tuple(sorted(self.meter_events, key=lambda event: event.start_tick))
        if not meters:
            meters = (MeterEvent(0, DEFAULT_METER),)
        object.__setattr__(self, "tempo_events", tempos)
        object.__setattr__(self, "meter_events", meters)
        object.__setattr__(self, "instruments", tuple(self.instruments))
        object.__setattr__(self, "tracks", tuple(self.tracks))
        object.__setattr__(
            self,
            "events",
            tuple(
                sorted(
                    self.events,
                    key=lambda event: (event.start_tick, event.source_order),
                )
            ),
        )
        self._validate_references()

    def _validate_references(self) -> None:
        if len(self.tempo_events) > MAX_EVENTS:
            raise ValueError(
                f"tempo_events count {len(self.tempo_events)} exceeds "
                f"MAX_EVENTS={MAX_EVENTS}"
            )
        if len(self.meter_events) > MAX_EVENTS:
            raise ValueError(
                f"meter_events count {len(self.meter_events)} exceeds "
                f"MAX_EVENTS={MAX_EVENTS}"
            )
        if len(self.instruments) > MAX_INSTRUMENTS:
            raise ValueError(
                f"instrument count {len(self.instruments)} exceeds "
                f"MAX_INSTRUMENTS={MAX_INSTRUMENTS}"
            )
        if len(self.tracks) > MAX_TRACKS:
            raise ValueError(
                f"track count {len(self.tracks)} exceeds MAX_TRACKS={MAX_TRACKS}"
            )
        if len(self.events) > MAX_EVENTS:
            raise ValueError(
                f"event count {len(self.events)} exceeds MAX_EVENTS={MAX_EVENTS}"
            )

        instrument_ids = {instrument.id for instrument in self.instruments}
        if len(instrument_ids) != len(self.instruments):
            raise ValueError("instrument ids must be unique")
        track_ids = {track.id for track in self.tracks}
        if len(track_ids) != len(self.tracks):
            raise ValueError("track ids must be unique")
        for track in self.tracks:
            if track.instrument_id not in instrument_ids:
                raise ValueError(f"track {track.id!r} references unknown instrument")
        for event in self.events:
            if event.track_id not in track_ids:
                raise ValueError(f"event references unknown track {event.track_id!r}")

    def instrument_map(self) -> dict[str, InstrumentDeclaration]:
        return {instrument.id: instrument for instrument in self.instruments}

    def track_map(self) -> dict[str, TrackDeclaration]:
        return {track.id: track for track in self.tracks}

    def to_dict(self) -> dict[str, object]:
        """Convert this score to a canonical JSON-serializable dictionary."""
        return {
            "events": [event.to_dict() for event in self.events],
            "format_version": self.format_version,
            "instruments": [inst.to_dict() for inst in self.instruments],
            "meter_events": [meter.to_dict() for meter in self.meter_events],
            "ppq": self.ppq,
            "sample_rate_hz": self.sample_rate_hz,
            "tempo_events": [tempo.to_dict() for tempo in self.tempo_events],
            "title": self.title,
            "tracks": [track.to_dict() for track in self.tracks],
        }

    def to_json(self, *, indent: int | None = 2) -> str:
        """Deterministic canonical JSON serialization of this score."""
        return json.dumps(
            self.to_dict(),
            indent=indent,
            sort_keys=True,
            allow_nan=False,
        )

    @classmethod
    def from_dict(cls, data: Mapping[str, object]) -> PortableScore:
        """Construct a PortableScore from a dictionary with strict validation."""
        if not isinstance(data, Mapping):
            raise ValueError("Score data must be a dictionary")

        expected_keys = {
            "format_version",
            "title",
            "ppq",
            "sample_rate_hz",
            "tempo_events",
            "meter_events",
            "instruments",
            "tracks",
            "events",
        }
        unknown = sorted(set(data) - expected_keys)
        if unknown:
            raise ValueError(f"unknown score keys: {', '.join(unknown)}")

        format_version = str(data.get("format_version", ""))
        title = str(data.get("title", "Untitled"))
        ppq = _positive_integer("ppq", data.get("ppq", DEFAULT_PPQ))  # type: ignore
        sample_rate_hz = _positive_integer(
            "sample_rate_hz",
            data.get("sample_rate_hz", int(DEFAULT_SAMPLE_RATE_HZ)),  # type: ignore
        )

        tempo_raw = data.get("tempo_events", ())
        if not isinstance(tempo_raw, (list, tuple)):
            raise ValueError("tempo_events must be a list")
        if len(tempo_raw) > MAX_EVENTS:
            raise ValueError(
                f"tempo_events count {len(tempo_raw)} exceeds MAX_EVENTS={MAX_EVENTS}"
            )

        meter_raw = data.get("meter_events", ())
        if not isinstance(meter_raw, (list, tuple)):
            raise ValueError("meter_events must be a list")
        if len(meter_raw) > MAX_EVENTS:
            raise ValueError(
                f"meter_events count {len(meter_raw)} exceeds MAX_EVENTS={MAX_EVENTS}"
            )

        inst_raw = data.get("instruments", ())
        if not isinstance(inst_raw, (list, tuple)):
            raise ValueError("instruments must be a list")
        if len(inst_raw) > MAX_INSTRUMENTS:
            raise ValueError(
                f"instruments count {len(inst_raw)} exceeds "
                f"MAX_INSTRUMENTS={MAX_INSTRUMENTS}"
            )

        track_raw = data.get("tracks", ())
        if not isinstance(track_raw, (list, tuple)):
            raise ValueError("tracks must be a list")
        if len(track_raw) > MAX_TRACKS:
            raise ValueError(
                f"tracks count {len(track_raw)} exceeds MAX_TRACKS={MAX_TRACKS}"
            )

        event_raw = data.get("events", ())
        if not isinstance(event_raw, (list, tuple)):
            raise ValueError("events must be a list")
        if len(event_raw) > MAX_EVENTS:
            raise ValueError(
                f"events count {len(event_raw)} exceeds MAX_EVENTS={MAX_EVENTS}"
            )

        tempo_events = tuple(TempoEvent.from_dict(item) for item in tempo_raw)
        meter_events = tuple(MeterEvent.from_dict(item) for item in meter_raw)
        instruments = tuple(InstrumentDeclaration.from_dict(item) for item in inst_raw)
        tracks = tuple(TrackDeclaration.from_dict(item) for item in track_raw)
        events = tuple(PortableScoreEvent.from_dict(item) for item in event_raw)

        return cls(
            format_version=format_version,
            title=title,
            ppq=ppq,
            sample_rate_hz=sample_rate_hz,
            tempo_events=tempo_events,
            meter_events=meter_events,
            instruments=instruments,
            tracks=tracks,
            events=events,
        )

    @classmethod
    def from_json(
        cls,
        json_text: str,
        *,
        max_bytes: int = MAX_SCORE_BYTES,
    ) -> PortableScore:
        """Parse canonical JSON into a PortableScore.

        Rejects NaN/Inf and oversized text.
        """
        if not isinstance(json_text, str):
            raise ValueError("json_text must be a string")
        encoded = json_text.encode("utf-8")
        if len(encoded) > max_bytes:
            raise ValueError(
                f"JSON text length {len(encoded)} exceeds max_bytes={max_bytes}"
            )

        def _reject_constant(val: str) -> None:
            raise ValueError(f"non-finite JSON constants are not allowed: {val!r}")

        try:
            data = json.loads(json_text, parse_constant=_reject_constant)
        except (json.JSONDecodeError, RecursionError) as exc:
            raise ValueError(f"malformed JSON: {exc}") from exc

        if not isinstance(data, dict):
            raise ValueError("JSON score root must be an object")

        return cls.from_dict(data)

    def to_text(self) -> str:
        """Serialize this PortableScore to canonical text representation."""
        lines = [
            f"format: {self.format_version}",
            f"title: {self.title}",
            f"ppq: {self.ppq}",
            f"sample_rate: {self.sample_rate_hz}",
        ]

        lines.extend(
            f"tempo {event.start_tick} {_format_portable_number(event.bpm)}"
            for event in self.tempo_events
        )
        lines.extend(
            f"meter {event.start_tick} {event.meter}" for event in self.meter_events
        )

        if self.instruments:
            lines.append("")
            lines.extend(_portable_instrument_line(item) for item in self.instruments)

        if self.tracks:
            lines.append("")
            lines.extend(
                f"track {track.id} instrument={track.instrument_id}"
                for track in self.tracks
            )

        if self.events:
            lines.append("")
            lines.extend(_portable_event_line(event) for event in self.events)

        return "\n".join(lines)


# =============================================================================
# Builders and Arranger Abstractions
# =============================================================================


@dataclass
class PortableScoreBuilder:
    """Programmatic builder for `music-machine-score/v2` portable scores."""

    title: str = "Untitled"
    ppq: int = DEFAULT_PPQ
    sample_rate_hz: int = int(DEFAULT_SAMPLE_RATE_HZ)
    format_version: str = PORTABLE_FORMAT_VERSION
    _tempo_events: list[TempoEvent] = field(
        default_factory=list,
        init=False,
        repr=False,
    )
    _meter_events: list[MeterEvent] = field(
        default_factory=list,
        init=False,
        repr=False,
    )
    _instruments: list[InstrumentDeclaration] = field(
        default_factory=list,
        init=False,
        repr=False,
    )
    _tracks: list[TrackDeclaration] = field(
        default_factory=list,
        init=False,
        repr=False,
    )
    _events: list[PortableScoreEvent] = field(
        default_factory=list,
        init=False,
        repr=False,
    )
    _source_order: int = field(default=0, init=False, repr=False)

    def __post_init__(self) -> None:
        self.title = str(self.title)
        self.ppq = _positive_integer("ppq", self.ppq)
        self.sample_rate_hz = _positive_integer("sample_rate_hz", self.sample_rate_hz)
        if self.sample_rate_hz > MAX_SAMPLE_RATE_HZ:
            raise ValueError(
                f"sample_rate_hz {self.sample_rate_hz} exceeds "
                f"MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
            )
        if self.format_version != PORTABLE_FORMAT_VERSION:
            raise ValueError(
                f"format_version must be {PORTABLE_FORMAT_VERSION!r}, "
                f"got {self.format_version!r}"
            )

    def add_tempo(self, start_tick: int, bpm: float) -> PortableScoreBuilder:
        self._tempo_events.append(TempoEvent(start_tick=start_tick, bpm=bpm))
        return self

    def add_meter(self, start_tick: int, meter: str) -> PortableScoreBuilder:
        self._meter_events.append(MeterEvent(start_tick=start_tick, meter=meter))
        return self

    def add_instrument(
        self,
        instrument_id: str,
        *,
        profile: str | None = None,
        program: int | None = None,
        kind: str | None = None,
        kit: str | None = None,
        harmonics: tuple[HarmonicPartial, ...] | None = None,
        envelope: ADSREnvelope | None = None,
        gain: float = 1.0,
    ) -> PortableScoreBuilder:
        selector_count = sum(value is not None for value in (profile, program, kind))
        if selector_count != 1:
            raise ValueError(
                "instrument must specify exactly one of profile, program, or kind"
            )

        if program is not None:
            profile_id = instrument_for_gm_program(program).id
            decl = InstrumentDeclaration(
                id=instrument_id,
                profile_id=profile_id,
                gain=gain,
                kind="preset",
            )
        elif profile is not None:
            profile_id = resolve_instrument_id(instrument_id=profile)
            decl = InstrumentDeclaration(
                id=instrument_id,
                profile_id=profile_id,
                gain=gain,
                kind="preset",
            )
        else:
            if kind == "drumkit":
                decl = InstrumentDeclaration(
                    id=instrument_id,
                    profile_id="drumkit",
                    gain=gain,
                    kind="drumkit",
                    kit=kit or "standard",
                )
            elif kind == "additive":
                decl = InstrumentDeclaration(
                    id=instrument_id,
                    profile_id=instrument_id,
                    gain=gain,
                    kind="additive",
                    harmonics=harmonics,
                    envelope=envelope,
                )
            elif kind in {"sine", "silence"}:
                decl = InstrumentDeclaration(
                    id=instrument_id,
                    profile_id=kind,
                    gain=gain,
                    kind=kind,
                )
            else:
                raise ValueError(
                    f"kind must be sine, silence, drumkit, or additive, got {kind!r}"
                )

        self._instruments.append(decl)
        return self

    def add_track(self, track_id: str, *, instrument_id: str) -> PortableScoreBuilder:
        self._tracks.append(TrackDeclaration(track_id, instrument_id))
        return self

    def add_note(
        self,
        track_id: str,
        start_tick: int,
        duration_tick: int,
        note: str,
        *,
        velocity: float = 1.0,
    ) -> PortableScoreBuilder:
        self._events.append(
            PortableScoreEvent(
                track_id=track_id,
                start_tick=start_tick,
                duration_tick=duration_tick,
                kind="note",
                notes=(note,),
                velocity=velocity,
                source_order=self._next_source_order(),
            )
        )
        return self

    def add_chord(
        self,
        track_id: str,
        start_tick: int,
        duration_tick: int,
        notes: Iterable[str],
        *,
        velocity: float = 1.0,
    ) -> PortableScoreBuilder:
        if isinstance(notes, str):
            raise ValueError("notes must be an iterable of note strings")

        self._events.append(
            PortableScoreEvent(
                track_id=track_id,
                start_tick=start_tick,
                duration_tick=duration_tick,
                kind="note",
                notes=tuple(notes),
                velocity=velocity,
                source_order=self._next_source_order(),
            )
        )
        return self

    def add_hit(
        self,
        track_id: str,
        start_tick: int,
        duration_tick: int,
        hit_id: str,
        *,
        velocity: float = 1.0,
    ) -> PortableScoreBuilder:
        self._events.append(
            PortableScoreEvent(
                track_id=track_id,
                start_tick=start_tick,
                duration_tick=duration_tick,
                kind="hit",
                notes=(),
                hit_id=hit_id,
                velocity=velocity,
                source_order=self._next_source_order(),
            )
        )
        return self

    def add_rest(
        self,
        track_id: str,
        start_tick: int,
        duration_tick: int,
    ) -> PortableScoreBuilder:
        self._events.append(
            PortableScoreEvent(
                track_id=track_id,
                start_tick=start_tick,
                duration_tick=duration_tick,
                kind="rest",
                notes=(),
                source_order=self._next_source_order(),
            )
        )
        return self

    def beats_to_ticks(self, beat_count: Real) -> int:
        beats = _finite_float("beat_count", beat_count)
        if beats < 0.0:
            raise ValueError(f"beat_count must be >= 0.0, got {beats}")
        raw_ticks = beats * self.ppq
        rounded = round(raw_ticks)
        if abs(raw_ticks - rounded) > BEAT_TICK_TOLERANCE:
            raise ValueError(
                f"beat_count {beats} is not representable at ppq={self.ppq}"
            )
        return int(rounded)

    def measure_ticks(self, meter: str | None = None) -> int:
        active_meter = _builder_meter(self, meter)
        numerator, denominator = _meter_parts(active_meter)
        beats = numerator * (4.0 / denominator)
        return self.beats_to_ticks(beats)

    def measure_start_tick(
        self,
        measure_number: int,
        *,
        meter: str | None = None,
    ) -> int:
        if isinstance(measure_number, bool) or not isinstance(measure_number, Integral):
            raise ValueError("measure_number must be an integer >= 1")
        converted = int(measure_number)
        if converted < 1:
            raise ValueError("measure_number must be >= 1")
        return (converted - 1) * self.measure_ticks(meter)

    def tick_in_measure(
        self,
        measure_number: int,
        beat_offset: Real = 0.0,
        *,
        meter: str | None = None,
    ) -> int:
        start_tick = self.measure_start_tick(measure_number, meter=meter)
        return start_tick + self.beats_to_ticks(beat_offset)

    def phrase(
        self,
        track_id: str,
        *,
        measure_number: int = 1,
        beat_offset: Real = 0.0,
        meter: str | None = None,
    ) -> PhraseBuilder:
        start_tick = self.tick_in_measure(
            measure_number,
            beat_offset,
            meter=meter,
        )
        return PhraseBuilder(self, track_id, start_tick=start_tick)

    def capture_motif(
        self,
        track_id: str,
        *,
        start_tick: int,
        end_tick: int,
    ) -> PhraseMotif:
        checked_track = _validate_identifier("track id", track_id)
        start = _non_negative_tick("start_tick", start_tick)
        end = _non_negative_tick("end_tick", end_tick)
        if end < start:
            raise ValueError("end_tick must be >= start_tick")

        motif_events = tuple(
            PhraseMotifEvent(
                offset_tick=event.start_tick - start,
                duration_tick=event.duration_tick,
                kind=event.kind,
                notes=event.notes,
                hit_id=event.hit_id,
                velocity=event.velocity,
            )
            for event in self._events
            if event.track_id == checked_track and start <= event.start_tick < end
        )
        return PhraseMotif(duration_tick=end - start, events=motif_events)

    def apply_motif(
        self,
        motif: PhraseMotif,
        track_id: str,
        start_tick: int,
        *,
        transpose_semitones: int = 0,
        velocity_scale: Real = 1.0,
        repeat_count: int = 1,
        repeat_spacing_tick: int | None = None,
    ) -> PortableScoreBuilder:
        if not isinstance(motif, PhraseMotif):
            raise ValueError("motif must be a PhraseMotif")
        checked_track = _validate_identifier("track id", track_id)
        origin_tick = _non_negative_tick("start_tick", start_tick)
        repeats = _positive_integer("repeat_count", repeat_count)
        spacing = (
            motif.duration_tick
            if repeat_spacing_tick is None
            else _positive_integer("repeat_spacing_tick", repeat_spacing_tick)
        )
        scale = _unit_float("velocity_scale", velocity_scale)
        transpose = _signed_integer("transpose_semitones", transpose_semitones)

        for repetition in range(repeats):
            base_tick = origin_tick + repetition * spacing
            for event in motif.events:
                event_start = base_tick + event.offset_tick
                event_velocity = min(1.0, event.velocity * scale)
                if event.kind == "note":
                    transposed_notes = tuple(
                        _transpose_note_text(note, transpose) for note in event.notes
                    )
                    self._events.append(
                        PortableScoreEvent(
                            track_id=checked_track,
                            start_tick=event_start,
                            duration_tick=event.duration_tick,
                            kind="note",
                            notes=transposed_notes,
                            velocity=event_velocity,
                            source_order=self._next_source_order(),
                        )
                    )
                elif event.kind == "hit":
                    self._events.append(
                        PortableScoreEvent(
                            track_id=checked_track,
                            start_tick=event_start,
                            duration_tick=event.duration_tick,
                            kind="hit",
                            notes=(),
                            hit_id=event.hit_id,
                            velocity=event_velocity,
                            source_order=self._next_source_order(),
                        )
                    )
                else:
                    self._events.append(
                        PortableScoreEvent(
                            track_id=checked_track,
                            start_tick=event_start,
                            duration_tick=event.duration_tick,
                            kind="rest",
                            notes=(),
                            source_order=self._next_source_order(),
                        )
                    )
        return self

    def capture_section(
        self,
        *,
        start_tick: int,
        end_tick: int,
        track_ids: Iterable[str] | None = None,
    ) -> ArrangementSection:
        start = _non_negative_tick("start_tick", start_tick)
        end = _non_negative_tick("end_tick", end_tick)
        if end < start:
            raise ValueError("end_tick must be >= start_tick")

        active_tracks: set[str] | None = None
        if track_ids is not None:
            if isinstance(track_ids, str):
                raise ValueError("track_ids must be an iterable of track ids")
            active_tracks = {
                _validate_identifier("track id", track_id) for track_id in track_ids
            }

        section_duration = end - start
        section_events: list[ArrangementSectionEvent] = []
        for event in self._events:
            if active_tracks is not None and event.track_id not in active_tracks:
                continue
            if not start <= event.start_tick < end:
                continue
            offset_tick = event.start_tick - start
            if offset_tick + event.duration_tick > section_duration:
                raise ValueError("captured section event extends beyond end_tick")
            section_events.append(
                ArrangementSectionEvent(
                    track_id=event.track_id,
                    offset_tick=offset_tick,
                    duration_tick=event.duration_tick,
                    kind=event.kind,
                    notes=event.notes,
                    hit_id=event.hit_id,
                    velocity=event.velocity,
                )
            )
        return ArrangementSection(
            duration_tick=section_duration,
            events=tuple(section_events),
        )

    def apply_section(
        self,
        section: ArrangementSection,
        start_tick: int,
        *,
        track_map: Mapping[str, str] | None = None,
        transpose_semitones: int | Mapping[str, int] = 0,
        velocity_scale: Real | Mapping[str, Real] = 1.0,
        repeat_count: int = 1,
        repeat_spacing_tick: int | None = None,
    ) -> PortableScoreBuilder:
        if not isinstance(section, ArrangementSection):
            raise ValueError("section must be an ArrangementSection")
        origin_tick = _non_negative_tick("start_tick", start_tick)
        repeats = _positive_integer("repeat_count", repeat_count)
        spacing = (
            section.duration_tick
            if repeat_spacing_tick is None
            else _positive_integer("repeat_spacing_tick", repeat_spacing_tick)
        )
        validated_track_map = _validated_track_map(track_map)
        transpose_by_track = _validated_track_integer_map(
            "transpose_semitones",
            transpose_semitones,
        )
        scale_by_track = _validated_track_float_map(
            "velocity_scale",
            velocity_scale,
        )
        default_transpose = (
            _signed_integer("transpose_semitones", transpose_semitones)
            if not isinstance(transpose_semitones, Mapping)
            else 0
        )
        default_scale = (
            _unit_float("velocity_scale", velocity_scale)
            if not isinstance(velocity_scale, Mapping)
            else 1.0
        )

        for repetition in range(repeats):
            base_tick = origin_tick + repetition * spacing
            for event in section.events:
                target_track = validated_track_map.get(
                    event.track_id,
                    event.track_id,
                )
                transpose = transpose_by_track.get(
                    event.track_id,
                    default_transpose,
                )
                scale = scale_by_track.get(event.track_id, default_scale)
                event_start = base_tick + event.offset_tick
                event_velocity = min(1.0, event.velocity * scale)

                if event.kind == "note":
                    transposed_notes = tuple(
                        _transpose_note_text(note, transpose) for note in event.notes
                    )
                    self._events.append(
                        PortableScoreEvent(
                            track_id=target_track,
                            start_tick=event_start,
                            duration_tick=event.duration_tick,
                            kind="note",
                            notes=transposed_notes,
                            velocity=event_velocity,
                            source_order=self._next_source_order(),
                        )
                    )
                elif event.kind == "hit":
                    self._events.append(
                        PortableScoreEvent(
                            track_id=target_track,
                            start_tick=event_start,
                            duration_tick=event.duration_tick,
                            kind="hit",
                            notes=(),
                            hit_id=event.hit_id,
                            velocity=event_velocity,
                            source_order=self._next_source_order(),
                        )
                    )
                else:
                    self._events.append(
                        PortableScoreEvent(
                            track_id=target_track,
                            start_tick=event_start,
                            duration_tick=event.duration_tick,
                            kind="rest",
                            notes=(),
                            source_order=self._next_source_order(),
                        )
                    )
        return self

    def build(self) -> PortableScore:
        return PortableScore(
            format_version=self.format_version,
            title=self.title,
            ppq=self.ppq,
            sample_rate_hz=self.sample_rate_hz,
            tempo_events=tuple(self._tempo_events),
            meter_events=tuple(self._meter_events),
            instruments=tuple(self._instruments),
            tracks=tuple(self._tracks),
            events=tuple(self._events),
        )

    def to_text(self) -> str:
        return self.build().to_text()

    def _next_source_order(self) -> int:
        source_order = self._source_order
        self._source_order += 1
        return source_order


@dataclass
class PhraseBuilder:
    """Mutable cursor for sequencing a phrase on one track."""

    score_builder: PortableScoreBuilder
    track_id: str
    start_tick: int = 0
    origin_tick: int | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.score_builder, PortableScoreBuilder):
            raise ValueError("score_builder must be a PortableScoreBuilder")
        self.track_id = _validate_identifier("track id", self.track_id)
        self.start_tick = _non_negative_tick("start_tick", self.start_tick)
        if self.origin_tick is None:
            self.origin_tick = self.start_tick
        else:
            self.origin_tick = _non_negative_tick("origin_tick", self.origin_tick)

    @property
    def current_tick(self) -> int:
        return self.start_tick

    def note(
        self,
        note: str,
        duration_beats: Real,
        *,
        velocity: float = 1.0,
    ) -> PhraseBuilder:
        duration_tick = self.score_builder.beats_to_ticks(duration_beats)
        self.score_builder.add_note(
            self.track_id,
            self.start_tick,
            duration_tick,
            note,
            velocity=velocity,
        )
        self.start_tick += duration_tick
        return self

    def chord(
        self,
        notes: Iterable[str],
        duration_beats: Real,
        *,
        velocity: float = 1.0,
    ) -> PhraseBuilder:
        duration_tick = self.score_builder.beats_to_ticks(duration_beats)
        self.score_builder.add_chord(
            self.track_id,
            self.start_tick,
            duration_tick,
            notes,
            velocity=velocity,
        )
        self.start_tick += duration_tick
        return self

    def hit(
        self,
        hit_id: str,
        duration_beats: Real,
        *,
        velocity: float = 1.0,
    ) -> PhraseBuilder:
        duration_tick = self.score_builder.beats_to_ticks(duration_beats)
        self.score_builder.add_hit(
            self.track_id,
            self.start_tick,
            duration_tick,
            hit_id,
            velocity=velocity,
        )
        self.start_tick += duration_tick
        return self

    def rest(self, duration_beats: Real) -> PhraseBuilder:
        duration_tick = self.score_builder.beats_to_ticks(duration_beats)
        self.score_builder.add_rest(
            self.track_id,
            self.start_tick,
            duration_tick,
        )
        self.start_tick += duration_tick
        return self

    def advance_beats(self, beat_count: Real) -> PhraseBuilder:
        self.start_tick += self.score_builder.beats_to_ticks(beat_count)
        return self

    def jump_to_measure(
        self,
        measure_number: int,
        beat_offset: Real = 0.0,
        *,
        meter: str | None = None,
    ) -> PhraseBuilder:
        self.start_tick = self.score_builder.tick_in_measure(
            measure_number,
            beat_offset,
            meter=meter,
        )
        return self

    def motif(self, *, start_tick: int | None = None) -> PhraseMotif:
        start = self.origin_tick if start_tick is None else start_tick
        return self.score_builder.capture_motif(
            self.track_id,
            start_tick=start,
            end_tick=self.current_tick,
        )

    def apply_motif(
        self,
        motif: PhraseMotif,
        *,
        transpose_semitones: int = 0,
        velocity_scale: Real = 1.0,
        repeat_count: int = 1,
        spacing_beats: Real | None = None,
        advance: bool = True,
    ) -> PhraseBuilder:
        spacing_tick = (
            motif.duration_tick
            if spacing_beats is None
            else self.score_builder.beats_to_ticks(spacing_beats)
        )
        self.score_builder.apply_motif(
            motif,
            self.track_id,
            self.start_tick,
            transpose_semitones=transpose_semitones,
            velocity_scale=velocity_scale,
            repeat_count=repeat_count,
            repeat_spacing_tick=spacing_tick,
        )
        if advance:
            self.start_tick += spacing_tick * repeat_count
        return self


@dataclass(frozen=True)
class PhraseMotifEvent:
    """A track-agnostic portable event stored relative to motif start."""

    offset_tick: int
    duration_tick: int
    kind: PortableEventKind
    notes: tuple[str, ...] = ()
    velocity: float = 1.0
    hit_id: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "offset_tick",
            _non_negative_tick("offset_tick", self.offset_tick),
        )
        object.__setattr__(
            self,
            "duration_tick",
            _positive_integer("duration_tick", self.duration_tick),
        )
        if self.kind not in {"note", "rest", "hit"}:
            raise ValueError(
                f"kind must be 'note', 'rest', or 'hit', got {self.kind!r}"
            )
        if self.kind == "note":
            notes = tuple(str(parse_note(note)) for note in self.notes)
            if not notes:
                raise ValueError("note motif events must include at least one note")
            object.__setattr__(self, "notes", notes)
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "rest":
            if self.notes:
                raise ValueError("rest motif events cannot include notes")
            object.__setattr__(self, "notes", ())
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "hit":
            if self.notes:
                raise ValueError("hit motif events cannot include notes")
            hit = self.hit_id
            if hit is None:
                raise ValueError("hit motif events must include a hit_id")
            hit_str = str(hit).lower()
            if hit_str not in all_standard_hit_ids():
                raise ValueError(f"unknown hit_id {hit_str!r}")
            object.__setattr__(self, "hit_id", hit_str)
            object.__setattr__(self, "notes", ())
        object.__setattr__(self, "velocity", _unit_float("velocity", self.velocity))


@dataclass(frozen=True)
class PhraseMotif:
    """Reusable single-track phrase fragment that can be repeated or transposed."""

    duration_tick: int
    events: tuple[PhraseMotifEvent, ...]

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "duration_tick",
            _non_negative_tick("duration_tick", self.duration_tick),
        )
        ordered = tuple(sorted(self.events, key=lambda event: event.offset_tick))
        for event in ordered:
            if event.offset_tick + event.duration_tick > self.duration_tick:
                raise ValueError("motif event extends beyond motif duration")
        object.__setattr__(self, "events", ordered)


def _validated_track_map(
    value: Mapping[str, str] | None,
) -> dict[str, str]:
    if value is None:
        return {}
    if not isinstance(value, Mapping):
        raise ValueError(
            "track_map must be a mapping of source track ids to target ids"
        )
    return {
        _validate_identifier("track id", source_track): _validate_identifier(
            "track id",
            target_track,
        )
        for source_track, target_track in value.items()
    }


def _validated_track_integer_map(
    name: str,
    value: int | Mapping[str, int],
) -> dict[str, int]:
    if isinstance(value, Mapping):
        return {
            _validate_identifier("track id", track_id): _signed_integer(name, item)
            for track_id, item in value.items()
        }
    return {}


def _validated_track_float_map(
    name: str,
    value: Real | Mapping[str, Real],
) -> dict[str, float]:
    if isinstance(value, Mapping):
        return {
            _validate_identifier("track id", track_id): _positive_float(name, item)
            for track_id, item in value.items()
        }
    return {}


@dataclass(frozen=True)
class ArrangementSectionEvent:
    """A multi-track reusable event stored relative to section start."""

    track_id: str
    offset_tick: int
    duration_tick: int
    kind: PortableEventKind
    notes: tuple[str, ...] = ()
    velocity: float = 1.0
    hit_id: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "track_id",
            _validate_identifier("track_id", self.track_id),
        )
        object.__setattr__(
            self,
            "offset_tick",
            _non_negative_tick("offset_tick", self.offset_tick),
        )
        object.__setattr__(
            self,
            "duration_tick",
            _positive_integer("duration_tick", self.duration_tick),
        )
        if self.kind not in {"note", "rest", "hit"}:
            raise ValueError(
                f"kind must be 'note', 'rest', or 'hit', got {self.kind!r}"
            )
        if self.kind == "note":
            notes = tuple(str(parse_note(note)) for note in self.notes)
            if not notes:
                raise ValueError("note section events must include at least one note")
            object.__setattr__(self, "notes", notes)
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "rest":
            if self.notes:
                raise ValueError("rest section events cannot include notes")
            object.__setattr__(self, "notes", ())
            object.__setattr__(self, "hit_id", None)
        elif self.kind == "hit":
            if self.notes:
                raise ValueError("hit section events cannot include notes")
            hit = self.hit_id
            if hit is None:
                raise ValueError("hit section events must include a hit_id")
            hit_str = str(hit).lower()
            if hit_str not in all_standard_hit_ids():
                raise ValueError(f"unknown hit_id {hit_str!r}")
            object.__setattr__(self, "hit_id", hit_str)
            object.__setattr__(self, "notes", ())
        object.__setattr__(self, "velocity", _unit_float("velocity", self.velocity))


@dataclass(frozen=True)
class ArrangementSection:
    """Reusable multi-track arrangement fragment such as a verse or chorus."""

    duration_tick: int
    events: tuple[ArrangementSectionEvent, ...]

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "duration_tick",
            _non_negative_tick("duration_tick", self.duration_tick),
        )
        ordered = tuple(
            sorted(
                self.events,
                key=lambda event: (event.offset_tick, event.track_id),
            )
        )
        for event in ordered:
            if event.offset_tick + event.duration_tick > self.duration_tick:
                raise ValueError("section event extends beyond section duration")
        object.__setattr__(self, "events", ordered)


@dataclass(frozen=True)
class RenderedPortableScore:
    """Rendered output for a portable multi-track score."""

    score: PortableScore
    pcm_buffer: PCMBuffer
    rendered_notes: tuple[InstrumentNoteRender, ...]
    rendered_hits: tuple[PercussionHitRender | PercussionHitEvent, ...] = ()

    def __post_init__(self) -> None:
        if not isinstance(self.score, PortableScore):
            raise ValueError("score must be a PortableScore")
        if not isinstance(self.pcm_buffer, PCMBuffer):
            raise ValueError("pcm_buffer must be a PCMBuffer")
        object.__setattr__(self, "rendered_notes", tuple(self.rendered_notes))
        for index, rendered_note in enumerate(self.rendered_notes):
            if not isinstance(rendered_note, InstrumentNoteRender):
                raise ValueError(
                    f"rendered_notes[{index}] must be an InstrumentNoteRender"
                )
        object.__setattr__(self, "rendered_hits", tuple(self.rendered_hits))
        for index, hit in enumerate(self.rendered_hits):
            if not isinstance(hit, (PercussionHitRender, PercussionHitEvent)):
                raise ValueError(
                    f"rendered_hits[{index}] must be a "
                    "PercussionHitRender or PercussionHitEvent"
                )


def _duration_seconds(duration_symbol: str, tempo_bpm: float) -> tuple[float, float]:
    beat_count = beats_for_duration_symbol(duration_symbol)
    return beat_count, beat_count * (60.0 / tempo_bpm)


def _score_event_from_token(token: str, tempo_bpm: float) -> ScoreEvent:
    if token.count("/") != 1:
        raise ValueError(f"music token {token!r} must look like pitch/duration")

    pitch_text, duration_symbol = token.split("/", maxsplit=1)
    if pitch_text == "" or duration_symbol == "":
        raise ValueError(f"music token {token!r} must look like pitch/duration")

    beat_count, duration_seconds = _duration_seconds(duration_symbol, tempo_bpm)
    lower_pitch = pitch_text.lower()
    if lower_pitch in {"r", "rest"}:
        return ScoreEvent(
            kind="rest",
            note=None,
            duration_symbol=duration_symbol,
            beat_count=beat_count,
            duration_seconds=duration_seconds,
            source_token=token,
        )

    if lower_pitch in all_standard_hit_ids():
        return ScoreEvent(
            kind="hit",
            note=None,
            hit_id=lower_pitch,
            duration_symbol=duration_symbol,
            beat_count=beat_count,
            duration_seconds=duration_seconds,
            source_token=token,
        )

    note = parse_note(pitch_text)
    return ScoreEvent(
        kind="note",
        note=str(note),
        duration_symbol=duration_symbol,
        beat_count=beat_count,
        duration_seconds=duration_seconds,
        source_token=token,
    )


# =============================================================================
# Parsers: TextScore & PortableScore
# =============================================================================


def parse_score(
    text: str,
    *,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_score_bytes: int | None = None,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
) -> TextScore:
    """Parse beginner-friendly text sheet music into a ``TextScore``."""
    if not isinstance(text, str):
        raise ValueError("score text must be a string")

    actual_max_len = (
        max_score_bytes if max_score_bytes is not None else max_score_length
    )
    score_limit = _non_negative_integer("max_score_length", actual_max_len)
    line_limit = _non_negative_integer("max_line_length", max_line_length)
    event_limit = _non_negative_integer("max_event_count", max_event_count)
    if len(text.encode("utf-8")) > score_limit:
        raise ValueError(
            f"score text length {len(text)} exceeds max_score_length={score_limit}"
        )

    title = "Untitled"
    tempo_bpm = DEFAULT_TEMPO_BPM
    meter = DEFAULT_METER
    amplitude = DEFAULT_AMPLITUDE
    sample_rate_hz = int(DEFAULT_SAMPLE_RATE_HZ)
    instrument_id = DEFAULT_INSTRUMENT_ID
    instrument_directive_seen = False
    gm_program: int | None = None
    drumkit: str | None = None
    events: list[ScoreEvent] = []

    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        if len(raw_line) > line_limit:
            raise ValueError(
                f"line {line_number}: line length {len(raw_line)} exceeds "
                f"max_line_length={line_limit}"
            )

        line = raw_line.strip()
        if line == "" or line.startswith("#"):
            continue

        directive_match = _parse_directive_line(line)
        if directive_match is not None:
            name, value = directive_match
            if events:
                raise ValueError(
                    f"line {line_number}: directive {name!r} appears after music"
                )
            if name not in DIRECTIVE_NAMES:
                raise ValueError(f"line {line_number}: unknown directive {name!r}")
            if value == "":
                raise ValueError(f"line {line_number}: directive {name!r} is empty")

            if name == "title":
                title = value
            elif name == "tempo":
                tempo_bpm = _positive_float(
                    "tempo",
                    _parse_float_directive("tempo", value),
                )
            elif name == "meter":
                meter = _validate_meter(value)
            elif name == "amplitude":
                amplitude = _unit_float(
                    "amplitude",
                    _parse_float_directive("amplitude", value),
                )
            elif name == "sample_rate":
                sample_rate_hz = _integer_valued_sample_rate(value)
            elif name == "drumkit":
                if value not in all_drum_kits():
                    raise ValueError(f"line {line_number}: unknown drum kit {value!r}")
                drumkit = value
                instrument_id = "drumkit"
            elif name == "instrument":
                if gm_program is not None:
                    raise ValueError(
                        f"line {line_number}: score may specify instrument or "
                        "program, not both"
                    )
                if value in {"drumkit", "standard_kit"} or value in all_drum_kits():
                    drumkit = value if value in all_drum_kits() else "standard"
                    instrument_id = "drumkit"
                else:
                    try:
                        instrument_id = get_instrument(value).id
                    except ValueError as exc:
                        raise ValueError(f"line {line_number}: {exc}") from exc
                instrument_directive_seen = True
            elif name == "program":
                if instrument_directive_seen:
                    raise ValueError(
                        f"line {line_number}: score may specify instrument or "
                        "program, not both"
                    )
                gm_program = _integer_directive("program", value)
                try:
                    instrument_id = instrument_for_gm_program(gm_program).id
                except ValueError as exc:
                    raise ValueError(f"line {line_number}: {exc}") from exc
            continue

        for token in line.split():
            if token == "|":
                continue
            if len(events) >= event_limit:
                raise ValueError(
                    f"line {line_number}: event count exceeds "
                    f"max_event_count={event_limit}"
                )
            try:
                events.append(_score_event_from_token(token, tempo_bpm))
            except ValueError as exc:
                raise ValueError(f"line {line_number}: {exc}") from exc

    return TextScore(
        title=title,
        tempo_bpm=tempo_bpm,
        meter=meter,
        amplitude=amplitude,
        sample_rate_hz=sample_rate_hz,
        instrument_id=instrument_id,
        gm_program=gm_program,
        events=tuple(events),
        drumkit=drumkit,
    )


def parse_portable_score(
    text: str,
    *,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_score_bytes: int | None = None,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
) -> PortableScore:
    """Parse a canonical multi-track score in the portable format."""
    if not isinstance(text, str):
        raise ValueError("score text must be a string")

    actual_max_len = (
        max_score_bytes if max_score_bytes is not None else max_score_length
    )
    score_limit = _non_negative_integer("max_score_length", actual_max_len)
    line_limit = _non_negative_integer("max_line_length", max_line_length)
    event_limit = _non_negative_integer("max_event_count", max_event_count)
    if len(text.encode("utf-8")) > score_limit:
        raise ValueError(
            f"score text length {len(text)} exceeds max_score_length={score_limit}"
        )

    title = "Untitled"
    ppq = DEFAULT_PPQ
    sample_rate_hz = int(DEFAULT_SAMPLE_RATE_HZ)
    tempo_events: list[TempoEvent] = []
    meter_events: list[MeterEvent] = []
    instruments: list[InstrumentDeclaration] = []
    tracks: list[TrackDeclaration] = []
    events: list[PortableScoreEvent] = []
    saw_format = False

    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        if len(raw_line) > line_limit:
            raise ValueError(
                f"line {line_number}: line length {len(raw_line)} exceeds "
                f"max_line_length={line_limit}"
            )

        line = raw_line.strip()
        if line == "" or line.startswith("#"):
            continue

        directive_match = _parse_directive_line(line)
        if directive_match is not None:
            name, value = directive_match
            if name == "title":
                title = value
            elif name == "ppq":
                ppq = _positive_integer("ppq", _integer_directive("ppq", value))
            elif name == "sample_rate":
                sample_rate_hz = _integer_valued_sample_rate(value)
            elif name == "format":
                if saw_format:
                    raise ValueError(
                        f"line {line_number}: format directive must only appear once"
                    )
                if value != PORTABLE_FORMAT_VERSION:
                    raise ValueError(
                        f"line {line_number}: unsupported score format {value!r}"
                    )
                saw_format = True
            else:
                raise ValueError(f"line {line_number}: unknown directive {name!r}")
            continue

        tokens = line.split()
        command = tokens[0]
        try:
            if command == "tempo":
                if len(tokens) != 3:
                    raise ValueError("tempo lines must be: tempo <start_tick> <bpm>")
                tempo_events.append(
                    TempoEvent(
                        _parse_tick(tokens[1], "tempo start_tick"),
                        _positive_float(
                            "tempo bpm",
                            _parse_float_directive("tempo bpm", tokens[2]),
                        ),
                    )
                )
            elif command == "meter":
                if len(tokens) != 3:
                    raise ValueError("meter lines must be: meter <start_tick> <meter>")
                meter_events.append(
                    MeterEvent(
                        _parse_tick(tokens[1], "meter start_tick"),
                        tokens[2],
                    )
                )
            elif command == "instrument":
                instruments.append(_parse_instrument_line(tokens))
            elif command == "track":
                tracks.append(_parse_track_line(tokens))
            elif command == "event":
                if len(events) >= event_limit:
                    raise ValueError(
                        f"event count exceeds max_event_count={event_limit}"
                    )
                events.append(_parse_portable_event_line(tokens, len(events)))
            else:
                raise ValueError(f"unknown command {command!r}")
        except ValueError as exc:
            raise ValueError(f"line {line_number}: {exc}") from exc

    if not saw_format:
        raise ValueError(f"first directive must be 'format: {PORTABLE_FORMAT_VERSION}'")

    return PortableScore(
        format_version=PORTABLE_FORMAT_VERSION,
        title=title,
        ppq=ppq,
        sample_rate_hz=sample_rate_hz,
        tempo_events=tuple(tempo_events),
        meter_events=tuple(meter_events),
        instruments=tuple(instruments),
        tracks=tuple(tracks),
        events=tuple(events),
    )


def _parse_instrument_line(tokens: list[str]) -> InstrumentDeclaration:
    if len(tokens) < 3:
        raise ValueError("instrument lines must include id and properties")
    instrument_id = _validate_identifier("instrument id", tokens[1])
    properties = _parse_properties(tokens[2:])
    gain = _unit_float(
        "gain",
        _parse_float_directive("gain", properties.get("gain", "1.0")),
    )

    allowed = {"gain", "program", "profile", "kind", "kit", "harmonics", "envelope"}
    unknown = sorted(set(properties) - allowed)
    if unknown:
        raise ValueError(f"unknown instrument properties: {', '.join(unknown)}")

    if "program" in properties:
        if "profile" in properties or "kind" in properties:
            raise ValueError("instrument may specify only one of program/profile/kind")
        profile_id = instrument_for_gm_program(
            _integer_directive("program", properties["program"])
        ).id
        return InstrumentDeclaration(
            id=instrument_id, profile_id=profile_id, gain=gain, kind="preset"
        )

    if "profile" in properties:
        if "kind" in properties:
            raise ValueError("instrument may specify only one of program/profile/kind")
        profile_id = resolve_instrument_id(instrument_id=properties["profile"])
        return InstrumentDeclaration(
            id=instrument_id, profile_id=profile_id, gain=gain, kind="preset"
        )

    if "kind" in properties:
        kind = properties["kind"]
        if kind in {"sine", "silence"}:
            return InstrumentDeclaration(
                id=instrument_id, profile_id=kind, gain=gain, kind=kind
            )
        elif kind == "drumkit":
            raw_kit = properties.get("kit", "standard_kit")
            kit = "standard_kit" if raw_kit in {"standard", "standard_kit"} else raw_kit
            valid_kit_ids = {k.id for k in all_drum_kits()} | {"standard_kit"}
            if kit not in valid_kit_ids:
                raise ValueError(f"unknown drum kit {raw_kit!r}")
            return InstrumentDeclaration(
                id=instrument_id,
                profile_id="drumkit",
                gain=gain,
                kind="drumkit",
                kit=kit,
            )
        elif kind == "additive":
            if "harmonics" not in properties:
                raise ValueError("kind=additive must provide harmonics")
            harmonics = _parse_harmonics(properties["harmonics"])
            envelope = (
                _parse_envelope(properties["envelope"])
                if "envelope" in properties
                else DEFAULT_ADDITIVE_ENVELOPE
            )
            return InstrumentDeclaration(
                id=instrument_id,
                profile_id=instrument_id,
                gain=gain,
                kind="additive",
                harmonics=harmonics,
                envelope=envelope,
            )
        else:
            raise ValueError(
                f"kind must be sine, silence, drumkit, or additive, got {kind!r}"
            )

    raise ValueError("instrument must include program, profile, or kind")


def _parse_track_line(tokens: list[str]) -> TrackDeclaration:
    if len(tokens) < 3:
        raise ValueError("track lines must include id and instrument=<id>")
    track_id = _validate_identifier("track id", tokens[1])
    properties = _parse_properties(tokens[2:])
    unknown = sorted(set(properties) - {"instrument"})
    if unknown:
        raise ValueError(f"unknown track properties: {', '.join(unknown)}")
    if "instrument" not in properties:
        raise ValueError("track must include instrument=<id>")
    return TrackDeclaration(track_id, properties["instrument"])


def _parse_portable_event_line(
    tokens: list[str],
    source_order: int,
) -> PortableScoreEvent:
    if len(tokens) < 5:
        raise ValueError(
            "event lines must be: event <track> <start> <duration> note|rest|hit ..."
        )
    track_id = _validate_identifier("track id", tokens[1])
    start_tick = _parse_tick(tokens[2], "event start_tick")
    duration_tick = _positive_integer(
        "event duration_tick",
        _parse_tick(tokens[3], "event duration_tick"),
    )
    kind = tokens[4]
    if kind == "rest":
        if len(tokens) != 5:
            raise ValueError("rest events must not include notes or properties")
        return PortableScoreEvent(
            track_id=track_id,
            start_tick=start_tick,
            duration_tick=duration_tick,
            kind="rest",
            notes=(),
            source_order=source_order,
        )

    if kind == "hit":
        if len(tokens) < 6:
            raise ValueError("hit events must include a hit_id")
        hit_id = tokens[5].lower()
        properties = _parse_properties(tokens[6:])
        unknown = sorted(set(properties) - {"velocity"})
        if unknown:
            raise ValueError(f"unknown event properties: {', '.join(unknown)}")
        velocity = _unit_float(
            "velocity",
            _parse_float_directive("velocity", properties.get("velocity", "1.0")),
        )
        return PortableScoreEvent(
            track_id=track_id,
            start_tick=start_tick,
            duration_tick=duration_tick,
            kind="hit",
            notes=(),
            hit_id=hit_id,
            velocity=velocity,
            source_order=source_order,
        )

    if kind != "note":
        raise ValueError(f"event kind must be note, rest, or hit, got {kind!r}")

    if len(tokens) < 6:
        raise ValueError("note events must include a pitch list")
    pitch_list = tokens[5]
    notes = tuple(note for note in pitch_list.split(",") if note)
    properties = _parse_properties(tokens[6:])
    unknown = sorted(set(properties) - {"velocity"})
    if unknown:
        raise ValueError(f"unknown event properties: {', '.join(unknown)}")
    velocity = _unit_float(
        "velocity",
        _parse_float_directive("velocity", properties.get("velocity", "1.0")),
    )
    return PortableScoreEvent(
        track_id=track_id,
        start_tick=start_tick,
        duration_tick=duration_tick,
        kind="note",
        notes=notes,
        velocity=velocity,
        source_order=source_order,
    )


def _format_portable_number(value: float) -> str:
    return f"{value:g}"


def _portable_instrument_line(instrument: InstrumentDeclaration) -> str:
    if instrument.kind == "drumkit" or instrument.profile_id.startswith("drumkit"):
        kit_str = (
            f" kit={instrument.kit}"
            if instrument.kit and instrument.kit != "standard"
            else " kit=standard"
        )
        return (
            f"instrument {instrument.id} kind=drumkit{kit_str} "
            f"gain={_format_portable_number(instrument.gain)}"
        )
    if instrument.kind == "additive" or (
        instrument.custom_profile
        and instrument.custom_profile.synthesis_kind == "additive"
    ):
        harm_str = ""
        if instrument.harmonics:
            harm_str = " harmonics=" + ",".join(
                f"{_format_portable_number(p.frequency_multiplier)}:{_format_portable_number(p.amplitude)}"
                + (
                    f":{_format_portable_number(p.phase_cycles)}"
                    if p.phase_cycles != 0.0
                    else ""
                )
                for p in instrument.harmonics
            )
        env_str = ""
        if instrument.envelope:
            env = instrument.envelope
            env_str = (
                f" envelope={_format_portable_number(env.attack_seconds)},"
                f"{_format_portable_number(env.decay_seconds)},"
                f"{_format_portable_number(env.sustain_level)},"
                f"{_format_portable_number(env.release_seconds)}"
            )
        return (
            f"instrument {instrument.id} kind=additive "
            f"gain={_format_portable_number(instrument.gain)}"
            f"{harm_str}{env_str}"
        )
    selector = (
        f"kind={instrument.profile_id}"
        if instrument.profile_id in {"sine", "silence"}
        else f"profile={instrument.profile_id}"
    )
    return (
        f"instrument {instrument.id} {selector} "
        f"gain={_format_portable_number(instrument.gain)}"
    )


def _portable_event_line(event: PortableScoreEvent) -> str:
    if event.kind == "rest":
        return f"event {event.track_id} {event.start_tick} {event.duration_tick} rest"
    if event.kind == "hit":
        return (
            f"event {event.track_id} {event.start_tick} {event.duration_tick} "
            f"hit {event.hit_id} velocity={_format_portable_number(event.velocity)}"
        )
    notes = ",".join(event.notes)
    return (
        f"event {event.track_id} {event.start_tick} {event.duration_tick} "
        f"note {notes} velocity={_format_portable_number(event.velocity)}"
    )


def _builder_meter(builder: PortableScoreBuilder, meter: str | None) -> str:
    if meter is not None:
        return _validate_meter(meter)
    if not builder._meter_events:
        return DEFAULT_METER
    return sorted(builder._meter_events, key=lambda event: event.start_tick)[0].meter


def _meter_parts(meter: str) -> tuple[int, int]:
    validated = _validate_meter(meter)
    numerator_text, denominator_text = validated.split("/", 1)
    return int(numerator_text), int(denominator_text)


def _transpose_note_text(text: str, semitone_offset: int) -> str:
    if semitone_offset == 0:
        return text
    parsed = parse_note(text)
    absolute_index = (parsed.octave + 1) * SEMITONES_PER_OCTAVE + parsed.chromatic_index
    transposed_index = absolute_index + semitone_offset
    octave, chromatic_index = divmod(transposed_index, SEMITONES_PER_OCTAVE)
    return f"{CANONICAL_SHARP_SPELLINGS[chromatic_index]}{octave - 1}"


# =============================================================================
# Path Loaders (CWE-22)
# =============================================================================


def parse_score_file(
    path: str | Path,
    *,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
) -> TextScore:
    """Read and parse a text score from disk with strict path validation (CWE-22)."""
    limit = min(max_score_length, MAX_SCORE_BYTES)
    checked_path = _validate_file_path(path)
    if checked_path.stat().st_size > limit:
        raise ValueError(
            f"score file size {checked_path.stat().st_size} "
            f"exceeds limit of {limit} bytes"
        )
    text = checked_path.read_text(encoding="utf-8")
    return parse_score(
        text,
        max_score_length=limit,
        max_line_length=max_line_length,
        max_event_count=max_event_count,
    )


def parse_portable_score_file(
    path: str | Path,
    *,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
) -> PortableScore:
    """Read and parse a portable score from disk with path validation (CWE-22)."""
    limit = min(max_score_length, MAX_SCORE_BYTES)
    checked_path = _validate_file_path(path)
    if checked_path.stat().st_size > limit:
        raise ValueError(
            f"score file size {checked_path.stat().st_size} "
            f"exceeds limit of {limit} bytes"
        )
    text = checked_path.read_text(encoding="utf-8")
    return parse_portable_score(
        text,
        max_score_length=limit,
        max_line_length=max_line_length,
        max_event_count=max_event_count,
    )


# =============================================================================
# Pre-Flight Resource Budgets & Audio Rendering
# =============================================================================


def _validate_total_sample_budget(
    events: tuple[ScoreEvent, ...],
    sample_rate_hz: int,
    instrument_id: str,
    max_sample_count: int,
) -> None:
    sample_limit = _positive_integer("max_sample_count", max_sample_count)
    if sample_limit > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"max_sample_count {sample_limit} exceeds "
            f"MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )

    rate = _positive_integer("sample_rate_hz", sample_rate_hz)
    if rate > MAX_SAMPLE_RATE_HZ:
        raise ValueError(
            f"sample_rate_hz {rate} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
        )

    profile = (
        get_instrument(instrument_id)
        if instrument_id not in {"drumkit", "standard_kit"}
        else None
    )
    release_seconds = (
        profile.envelope_profile.release_seconds if profile is not None else 0.0
    )

    total_samples = 0
    for event in events:
        event_duration = event.duration_seconds
        if event.kind == "note":
            event_duration += release_seconds
        total_samples += sample_count_for_duration(event_duration, rate)
        if total_samples > sample_limit:
            raise ValueError(
                f"score render would require {total_samples} samples, "
                f"above max_sample_count={sample_limit}"
            )


def render_score_to_pcm(
    score: TextScore,
    *,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> RenderedScore:
    """Render a parsed text score into one signed 16-bit mono PCM buffer."""
    if not isinstance(score, TextScore):
        raise ValueError("score must be a TextScore")

    sample_limit = _positive_integer("max_sample_count", max_sample_count)
    if sample_limit > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"max_sample_count exceeds MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )

    total_dur = score.total_duration_seconds()
    if total_dur > MAX_DURATION_SECONDS:
        raise ValueError(
            f"total score duration {total_dur}s exceeds "
            f"MAX_DURATION_SECONDS={MAX_DURATION_SECONDS}"
        )

    _validate_total_sample_budget(
        score.events,
        score.sample_rate_hz,
        score.instrument_id,
        max_sample_count,
    )

    pcm_samples: list[int] = []
    rendered_notes: list[InstrumentNoteRender] = []
    rendered_hits: list[PercussionHitRender] = []
    clipped_sample_count = 0
    elapsed_seconds = 0.0
    cursor_sample_index = 0

    # Group sequential drum hits to apply hi-hat choke attenuation (MUS07 §7)
    has_hits = any(event.kind == "hit" for event in score.events)
    if has_hits and all(event.kind in {"hit", "rest"} for event in score.events):
        kit_events: list[PercussionHitEvent] = []
        cur_t = 0.0
        for event in score.events:
            if event.kind == "hit" and event.hit_id:
                kit_events.append(
                    PercussionHitEvent(
                        hit_id=event.hit_id,
                        start_seconds=cur_t,
                        duration_seconds=event.duration_seconds,
                        velocity=score.amplitude,
                    )
                )
            cur_t += event.duration_seconds

        active_kit = score.drumkit or "standard_kit"
        if active_kit in {"standard", "standard_kit", "drumkit"}:
            active_kit = "standard_kit"
        percussion_render = render_percussion_sequence(
            kit_events,
            sample_rate_hz=score.sample_rate_hz,
            kit=active_kit,
            max_sample_count=sample_limit,
        )
        total_samples = list(percussion_render.pcm_buffer.samples)
        min_required = sample_count_for_duration(total_dur, score.sample_rate_hz)
        if len(total_samples) < min_required:
            total_samples.extend([0] * (min_required - len(total_samples)))

        return RenderedScore(
            score=score,
            pcm_buffer=PCMBuffer(
                samples=tuple(total_samples),
                pcm_format=PCMFormat(sample_rate_hz=score.sample_rate_hz),
                clipped_sample_count=percussion_render.pcm_buffer.clipped_sample_count,
            ),
            rendered_notes=(),
            rendered_hits=tuple(rendered_hits),
        )

    for event in score.events:
        if event.kind == "rest":
            silence_count = sample_count_for_duration(
                event.duration_seconds,
                score.sample_rate_hz,
            )
            cursor_sample_index += silence_count
            if len(pcm_samples) < cursor_sample_index:
                pcm_samples.extend([0] * (cursor_sample_index - len(pcm_samples)))
        elif event.kind == "hit":
            kit_events = [
                PercussionHitEvent(
                    hit_id=event.hit_id or "kick",
                    start_seconds=0.0,
                    duration_seconds=event.duration_seconds,
                    velocity=score.amplitude,
                )
            ]
            active_kit = score.drumkit or "standard_kit"
            if active_kit in {"standard", "standard_kit", "drumkit"}:
                active_kit = "standard_kit"
            seq_render = render_percussion_sequence(
                kit_events,
                sample_rate_hz=score.sample_rate_hz,
                kit=active_kit,
                max_sample_count=sample_limit,
            )
            clipped_sample_count += _mix_pcm_samples(
                pcm_samples,
                cursor_sample_index,
                seq_render.pcm_buffer.samples,
            )
            cursor_sample_index += sample_count_for_duration(
                event.duration_seconds,
                score.sample_rate_hz,
            )
        else:
            rendered_note = render_instrument_note(
                event.note,
                event.duration_seconds,
                instrument=score.instrument_id,
                sample_rate_hz=score.sample_rate_hz,
                amplitude=score.amplitude,
                start_time_seconds=elapsed_seconds,
                max_sample_count=max_sample_count,
            )
            rendered_notes.append(rendered_note)
            clipped_sample_count += rendered_note.pcm_buffer.clipped_sample_count
            clipped_sample_count += _mix_pcm_samples(
                pcm_samples,
                cursor_sample_index,
                rendered_note.pcm_buffer.samples,
            )
            cursor_sample_index += sample_count_for_duration(
                event.duration_seconds,
                score.sample_rate_hz,
            )
        elapsed_seconds += event.duration_seconds

    return RenderedScore(
        score=score,
        pcm_buffer=PCMBuffer(
            samples=tuple(pcm_samples),
            pcm_format=PCMFormat(sample_rate_hz=score.sample_rate_hz),
            clipped_sample_count=clipped_sample_count,
        ),
        rendered_notes=tuple(rendered_notes),
        rendered_hits=tuple(rendered_hits),
    )


class _TempoMap:
    """Pre-computed cumulative seconds table with O(log T) binary search queries."""

    __slots__ = ("ppq", "start_ticks", "bpms", "cumulative_seconds")

    def __init__(
        self,
        tempo_events: tuple[TempoEvent, ...] | list[TempoEvent],
        ppq: int,
    ) -> None:
        self.ppq = ppq
        tempos = sorted(tempo_events, key=lambda e: e.start_tick)
        if not tempos or tempos[0].start_tick != 0:
            tempos = [TempoEvent(0, DEFAULT_TEMPO_BPM)] + tempos

        self.start_ticks: list[int] = []
        self.bpms: list[float] = []
        self.cumulative_seconds: list[float] = []
        cur_sec = 0.0

        for i, t in enumerate(tempos):
            if i > 0:
                prev_tick = self.start_ticks[-1]
                prev_bpm = self.bpms[-1]
                cur_sec += (t.start_tick - prev_tick) * 60.0 / (prev_bpm * ppq)
            self.start_ticks.append(t.start_tick)
            self.bpms.append(t.bpm)
            self.cumulative_seconds.append(cur_sec)

    def seconds_at_tick(self, tick: int) -> float:
        checked_tick = _non_negative_tick("tick", tick)
        low = 0
        high = len(self.start_ticks)
        while low < high:
            mid = (low + high) // 2
            if checked_tick < self.start_ticks[mid]:
                high = mid
            else:
                low = mid + 1
        idx = max(0, low - 1)
        base_tick = self.start_ticks[idx]
        base_bpm = self.bpms[idx]
        base_sec = self.cumulative_seconds[idx]
        return base_sec + (checked_tick - base_tick) * 60.0 / (base_bpm * self.ppq)


def _seconds_at_tick(
    tick: int,
    ppq: int,
    tempo_events: tuple[TempoEvent, ...],
) -> float:
    return _TempoMap(tempo_events, ppq).seconds_at_tick(tick)


def _mix_pcm_samples(
    target: list[int],
    start_sample: int,
    source: tuple[int, ...] | list[int],
) -> int:
    required_count = start_sample + len(source)
    if len(target) < required_count:
        target.extend([0] * (required_count - len(target)))

    clipped_count = 0
    for offset, sample in enumerate(source):
        target_index = start_sample + offset
        mixed = target[target_index] + sample
        if mixed > 32_767:
            target[target_index] = 32_767
            clipped_count += 1
        elif mixed < -32_768:
            target[target_index] = -32_768
            clipped_count += 1
        else:
            target[target_index] = mixed
    return clipped_count


def render_portable_score_to_pcm(
    score: PortableScore,
    *,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> RenderedPortableScore:
    """Render a parsed portable multi-track score into one PCM buffer."""
    if not isinstance(score, PortableScore):
        raise ValueError("score must be a PortableScore")

    sample_limit = _positive_integer("max_sample_count", max_sample_count)
    if sample_limit > MAX_SAMPLE_COUNT:
        raise ValueError(
            f"max_sample_count {sample_limit} exceeds "
            f"MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )

    instrument_by_id = score.instrument_map()
    track_by_id = score.track_map()
    tempo_map = _TempoMap(score.tempo_events, score.ppq)

    # Pre-flight duration and track/instrument capability checks
    last_tick = max((e.start_tick + e.duration_tick for e in score.events), default=0)
    total_dur = tempo_map.seconds_at_tick(last_tick)
    if total_dur > MAX_DURATION_SECONDS:
        raise ValueError(
            f"score duration {total_dur}s exceeds "
            f"MAX_DURATION_SECONDS={MAX_DURATION_SECONDS}"
        )

    pre_samples = _safe_round_sample(total_dur, score.sample_rate_hz)
    if pre_samples > sample_limit:
        raise ValueError(
            f"score render would require {pre_samples} samples, "
            f"above max_sample_count={sample_limit}"
        )

    for event in score.events:
        track = track_by_id[event.track_id]
        inst = instrument_by_id[track.instrument_id]
        if event.kind == "note" and inst.kind == "drumkit":
            raise ValueError(
                f"track {track.id!r} uses drumkit instrument {inst.id!r} "
                "and cannot play note events"
            )
        if event.kind == "hit" and inst.kind != "drumkit":
            raise ValueError(
                f"track {track.id!r} uses pitched instrument {inst.id!r} "
                "and cannot play hit events"
            )

    pcm_samples: list[int] = []
    rendered_notes: list[InstrumentNoteRender] = []
    rendered_hits: list[PercussionHitRender] = []
    clipped_sample_count = 0

    # 1. Render Pitched Notes
    for event in score.events:
        if event.kind != "note":
            continue

        start_seconds = tempo_map.seconds_at_tick(event.start_tick)
        end_seconds = tempo_map.seconds_at_tick(event.start_tick + event.duration_tick)
        duration_seconds = end_seconds - start_seconds
        start_sample = sample_count_for_duration(start_seconds, score.sample_rate_hz)

        track = track_by_id[event.track_id]
        instrument = instrument_by_id[track.instrument_id]
        note_amplitude = event.velocity * instrument.gain / max(1, len(event.notes))
        target_inst: str | InstrumentProfile = (
            instrument.custom_profile
            if instrument.custom_profile is not None
            else instrument.profile_id
        )

        for note in event.notes:
            rendered_note = render_instrument_note(
                note,
                duration_seconds,
                instrument=target_inst,
                sample_rate_hz=score.sample_rate_hz,
                amplitude=note_amplitude,
                max_sample_count=max_sample_count,
            )
            rendered_note = replace(
                rendered_note,
                floating_samples=replace(
                    rendered_note.floating_samples,
                    start_time_seconds=start_seconds,
                ),
                pcm_buffer=replace(
                    rendered_note.pcm_buffer,
                    start_time_seconds=start_seconds,
                ),
            )
            rendered_notes.append(rendered_note)
            clipped_sample_count += rendered_note.pcm_buffer.clipped_sample_count
            required_count = start_sample + rendered_note.pcm_buffer.sample_count()
            if required_count > sample_limit:
                raise ValueError(
                    f"render would create {required_count} samples, "
                    f"above max_sample_count={sample_limit}"
                )
            clipped_sample_count += _mix_pcm_samples(
                pcm_samples,
                start_sample,
                rendered_note.pcm_buffer.samples,
            )
            if len(pcm_samples) > sample_limit:
                raise ValueError(
                    f"render would create {len(pcm_samples)} samples, "
                    f"above max_sample_count={sample_limit}"
                )

    # 2. Render Drum Hits with Hi-Hat Choking (MUS07 §7 & §9)
    # Group hit events by drumkit kit
    drum_kits_present = {
        inst.kit or "standard_kit"
        for inst in score.instruments
        if inst.kind == "drumkit"
    }

    for kit_name in drum_kits_present:
        kit_events: list[PercussionHitEvent] = []
        for event in score.events:
            if event.kind != "hit":
                continue
            track = track_by_id[event.track_id]
            inst = instrument_by_id[track.instrument_id]
            inst_kit = inst.kit or "standard_kit"
            if inst_kit != kit_name:
                continue

            start_seconds = tempo_map.seconds_at_tick(event.start_tick)
            end_seconds = tempo_map.seconds_at_tick(
                event.start_tick + event.duration_tick
            )
            duration_seconds = end_seconds - start_seconds

            kit_events.append(
                PercussionHitEvent(
                    hit_id=event.hit_id or "kick",
                    start_seconds=start_seconds,
                    duration_seconds=duration_seconds,
                    velocity=event.velocity * inst.gain,
                )
            )

        if kit_events:
            percussion_render = render_percussion_sequence(
                kit_events,
                sample_rate_hz=score.sample_rate_hz,
                kit=kit_name,
                max_sample_count=sample_limit,
            )
            clipped_sample_count += percussion_render.pcm_buffer.clipped_sample_count
            rendered_hits.extend(kit_events)
            clipped_sample_count += _mix_pcm_samples(
                pcm_samples,
                0,
                percussion_render.pcm_buffer.samples,
            )

    # 3. Handle Rest Bounds
    for event in score.events:
        if event.kind == "rest":
            end_seconds = tempo_map.seconds_at_tick(
                event.start_tick + event.duration_tick
            )
            rest_end = sample_count_for_duration(end_seconds, score.sample_rate_hz)
            if rest_end > sample_limit:
                raise ValueError(
                    f"render would create {rest_end} samples, "
                    f"above max_sample_count={sample_limit}"
                )
            if len(pcm_samples) < rest_end:
                pcm_samples.extend([0] * (rest_end - len(pcm_samples)))

    if len(pcm_samples) > sample_limit:
        raise ValueError(
            f"render would create {len(pcm_samples)} samples, "
            f"above max_sample_count={sample_limit}"
        )

    return RenderedPortableScore(
        score=score,
        pcm_buffer=PCMBuffer(
            samples=tuple(pcm_samples),
            pcm_format=PCMFormat(sample_rate_hz=score.sample_rate_hz),
            clipped_sample_count=clipped_sample_count,
        ),
        rendered_notes=tuple(rendered_notes),
        rendered_hits=tuple(rendered_hits),
    )


# =============================================================================
# Playback Helpers (delegating to audio-device-sink lazily)
# =============================================================================

PlaybackSink = Callable[[PCMBuffer], object]
SinkPlayer = PlaybackSink


def _default_play_pcm_buffer(buffer: PCMBuffer) -> object:
    """Import the real audio sink only at the exact moment playback is requested."""
    sink = import_module("audio_device_sink")
    play_pcm_buffer = getattr(sink, "play_pcm_buffer", None)
    if not callable(play_pcm_buffer):
        raise TypeError("audio_device_sink.play_pcm_buffer must be callable")
    return play_pcm_buffer(buffer)


def _load_default_sink_player() -> SinkPlayer:
    return _default_play_pcm_buffer


def play_score(
    score: TextScore,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    player: SinkPlayer | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Render a text score and play it through an audio sink."""
    rendered_score = render_score_to_pcm(
        score,
        max_sample_count=max_sample_count,
    )
    active_player = play_pcm_buffer if play_pcm_buffer is not None else player
    if active_player is None:
        active_player = _default_play_pcm_buffer
    return active_player(rendered_score.pcm_buffer)


def play_score_text(
    text: str,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    player: SinkPlayer | None = None,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Parse a text score, render it, and play it through an audio sink."""
    score = parse_score(
        text,
        max_score_length=max_score_length,
        max_line_length=max_line_length,
        max_event_count=max_event_count,
    )
    return play_score(
        score,
        play_pcm_buffer=play_pcm_buffer,
        player=player,
        max_sample_count=max_sample_count,
    )


def play_portable_score(
    score: PortableScore,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    player: SinkPlayer | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Render a portable multi-track score and play it through an audio sink."""
    rendered_score = render_portable_score_to_pcm(
        score,
        max_sample_count=max_sample_count,
    )
    active_player = play_pcm_buffer if play_pcm_buffer is not None else player
    if active_player is None:
        active_player = _default_play_pcm_buffer
    return active_player(rendered_score.pcm_buffer)


def play_portable_score_text(
    text: str,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    player: SinkPlayer | None = None,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Parse a portable score text, render it, and play it through an audio sink."""
    score = parse_portable_score(
        text,
        max_score_length=max_score_length,
        max_line_length=max_line_length,
        max_event_count=max_event_count,
    )
    return play_portable_score(
        score,
        play_pcm_buffer=play_pcm_buffer,
        player=player,
        max_sample_count=max_sample_count,
    )
    return play_portable_score(
        score,
        player=player,
        max_sample_count=max_sample_count,
    )
