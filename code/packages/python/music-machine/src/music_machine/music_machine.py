"""Parse tiny text scores and render them through the note-to-sound stack.

This module is deliberately written as a teaching layer. The parser handles a
small human-friendly score format, then the renderer hands every note to
``note-audio`` so the lower abstractions stay visible:

```text
note text -> frequency -> oscillator -> sampler -> PCM -> optional sink
```

Rests are the one special case. A rest has no frequency or oscillator, so the
music machine appends zero-valued PCM samples for the requested amount of time.
"""

from __future__ import annotations

import re
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field, replace
from importlib import import_module
from math import isfinite
from numbers import Integral, Real
from typing import Literal

from musical_instruments import (
    InstrumentNoteRender,
    get_instrument,
    instrument_for_gm_program,
    render_instrument_note,
)
from note_audio import DEFAULT_MAX_SAMPLE_COUNT
from note_frequency import parse_note
from oscillator import sample_count_for_duration
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat

DEFAULT_TEMPO_BPM = 120.0
DEFAULT_METER = "4/4"
DEFAULT_AMPLITUDE = 0.18
DEFAULT_INSTRUMENT_ID = "sine"
DEFAULT_PPQ = 480
DEFAULT_MAX_SCORE_LENGTH = 1_000_000
DEFAULT_MAX_LINE_LENGTH = 10_000
DEFAULT_MAX_EVENT_COUNT = 10_000

DURATION_BEATS = {
    "w": 4.0,
    "h": 2.0,
    "q": 1.0,
    "e": 0.5,
    "s": 0.25,
}

DIRECTIVE_NAMES = frozenset(
    {
        "title",
        "tempo",
        "meter",
        "amplitude",
        "sample_rate",
        "instrument",
        "program",
    }
)

DIRECTIVE_PATTERN = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$")
IDENTIFIER_PATTERN = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
METER_PATTERN = re.compile(r"^([1-9][0-9]*)/([1-9][0-9]*)$")
PORTABLE_FORMAT_VERSION = "music-machine-score/v2"

HAPPY_BIRTHDAY_TEXT = """title: Happy Birthday
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

MINI_ORCHESTRA_TEXT = """format: music-machine-score/v2
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
event harmony 1680 240 note G4 velocity=0.42
event comp 1440 240 note C4,E4,G4 velocity=0.42
event comp 1680 240 note C4,E4,G4 velocity=0.36
event low 1440 480 note C2 velocity=0.56
"""

ScoreEventKind = Literal["note", "rest"]
PortableEventKind = Literal["note", "rest"]


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
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer >= 0, got {value!r}")
    converted = int(value)
    if converted < 0:
        raise ValueError(f"{name} must be >= 0, got {converted}")
    return converted


def _integer_valued_sample_rate(text: str) -> int:
    try:
        value = float(text)
    except ValueError as exc:
        raise ValueError(f"sample_rate must be an integer, got {text!r}") from exc

    if not isfinite(value) or value != round(value):
        raise ValueError(f"sample_rate must be an integer, got {text!r}")
    return _positive_integer("sample_rate", int(round(value)))


def _integer_directive(name: str, text: str) -> int:
    try:
        value = float(text)
    except ValueError as exc:
        raise ValueError(f"{name} must be an integer, got {text!r}") from exc

    if not isfinite(value) or value != round(value):
        raise ValueError(f"{name} must be an integer, got {text!r}")
    return _positive_integer(name, int(round(value)))


def _parse_float_directive(name: str, text: str) -> float:
    try:
        return float(text)
    except ValueError as exc:
        raise ValueError(f"{name} must be a number, got {text!r}") from exc


def _validate_meter(text: str) -> str:
    match = METER_PATTERN.fullmatch(text)
    if match is None:
        raise ValueError(f"meter must look like positive beats/beat-unit, got {text!r}")
    return text


def _validate_identifier(name: str, value: str) -> str:
    converted = str(value)
    if IDENTIFIER_PATTERN.fullmatch(converted) is None:
        raise ValueError(f"{name} must be an identifier, got {value!r}")
    return converted


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
    except ValueError as exc:
        raise ValueError(f"{name} must be an integer, got {text!r}") from exc
    return _non_negative_tick(name, value)


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
    return get_instrument(instrument_id).id


def beats_for_duration_symbol(symbol: str) -> float:
    """Return how many quarter-note beats a duration symbol represents.

    The base duration table is intentionally tiny. One optional dot adds half
    the base value, which mirrors the common musical rule that a dotted note is
    one-and-a-half times as long as the undotted note.
    """

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
    """One musical instruction: play a note or be silent for a duration."""

    kind: ScoreEventKind
    note: str | None
    duration_symbol: str
    beat_count: float
    duration_seconds: float
    source_token: str

    def __post_init__(self) -> None:
        if self.kind not in {"note", "rest"}:
            raise ValueError(f"kind must be 'note' or 'rest', got {self.kind!r}")
        if self.kind == "note":
            if self.note is None:
                raise ValueError("note events must include a note")
            object.__setattr__(self, "note", str(parse_note(self.note)))
        elif self.note is not None:
            raise ValueError("rest events cannot include a note")

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
        gm_program = None
        if self.gm_program is not None:
            gm_program = _positive_integer("gm_program", self.gm_program)
            if gm_program > 128:
                raise ValueError("gm_program must be in [1, 128]")
        object.__setattr__(self, "gm_program", gm_program)
        if gm_program is None:
            resolved_instrument_id = resolve_instrument_id(
                instrument_id=str(self.instrument_id),
            )
        else:
            resolved_instrument_id = resolve_instrument_id(gm_program=gm_program)
        object.__setattr__(
            self,
            "instrument_id",
            resolved_instrument_id,
        )
        object.__setattr__(self, "events", tuple(self.events))
        if not self.events:
            raise ValueError("score must contain at least one event")

    def seconds_per_beat(self) -> float:
        """Return the duration of a quarter-note beat at this score tempo."""

        return 60.0 / self.tempo_bpm

    def total_duration_seconds(self) -> float:
        """Return the sum of every note and rest duration."""

        return sum(event.duration_seconds for event in self.events)


@dataclass(frozen=True)
class RenderedScore:
    """The joined PCM output plus inspectable per-note renderings."""

    score: TextScore
    pcm_buffer: PCMBuffer
    rendered_notes: tuple[InstrumentNoteRender, ...]

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


@dataclass(frozen=True)
class InstrumentDeclaration:
    """A named instrument available to portable-score tracks."""

    id: str
    profile_id: str
    gain: float = 1.0

    def __post_init__(self) -> None:
        object.__setattr__(self, "id", _validate_identifier("instrument id", self.id))
        object.__setattr__(
            self,
            "profile_id",
            resolve_instrument_id(instrument_id=self.profile_id),
        )
        object.__setattr__(self, "gain", _unit_float("gain", self.gain))


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


@dataclass(frozen=True)
class PortableScoreEvent:
    """An explicit timed note, chord, or rest event."""

    track_id: str
    start_tick: int
    duration_tick: int
    kind: PortableEventKind
    notes: tuple[str, ...]
    velocity: float = 1.0
    source_order: int = 0

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
        if self.kind not in {"note", "rest"}:
            raise ValueError(f"kind must be 'note' or 'rest', got {self.kind!r}")
        notes = tuple(str(parse_note(note)) for note in self.notes)
        if self.kind == "note" and not notes:
            raise ValueError("note events must include at least one note")
        if self.kind == "rest" and notes:
            raise ValueError("rest events cannot include notes")
        object.__setattr__(self, "notes", notes)
        object.__setattr__(self, "velocity", _unit_float("velocity", self.velocity))
        object.__setattr__(
            self,
            "source_order",
            _non_negative_tick("source_order", self.source_order),
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
        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_integer("sample_rate_hz", self.sample_rate_hz),
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
        gain: float = 1.0,
    ) -> PortableScoreBuilder:
        selector_count = sum(
            value is not None for value in (profile, program, kind)
        )
        if selector_count != 1:
            raise ValueError(
                "instrument must specify exactly one of profile, program, or kind"
            )

        if program is not None:
            profile_id = instrument_for_gm_program(program).id
        elif profile is not None:
            profile_id = resolve_instrument_id(instrument_id=profile)
        else:
            if kind not in {"sine", "silence"}:
                raise ValueError("kind must be sine or silence in V2")
            profile_id = kind

        self._instruments.append(
            InstrumentDeclaration(
                id=instrument_id,
                profile_id=profile_id,
                gain=gain,
            )
        )
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
        score = self.build()
        lines = [
            f"format: {score.format_version}",
            f"title: {score.title}",
            f"ppq: {score.ppq}",
            f"sample_rate: {score.sample_rate_hz}",
        ]

        lines.extend(
            f"tempo {event.start_tick} {_format_portable_number(event.bpm)}"
            for event in score.tempo_events
        )
        lines.extend(
            f"meter {event.start_tick} {event.meter}" for event in score.meter_events
        )

        if score.instruments:
            lines.append("")
            lines.extend(_portable_instrument_line(item) for item in score.instruments)

        if score.tracks:
            lines.append("")
            lines.extend(
                f"track {track.id} instrument={track.instrument_id}"
                for track in score.tracks
            )

        if score.events:
            lines.append("")
            lines.extend(_portable_event_line(event) for event in score.events)

        return "\n".join(lines)

    def _next_source_order(self) -> int:
        source_order = self._source_order
        self._source_order += 1
        return source_order


@dataclass(frozen=True)
class RenderedPortableScore:
    """Rendered output for a portable multi-track score."""

    score: PortableScore
    pcm_buffer: PCMBuffer
    rendered_notes: tuple[InstrumentNoteRender, ...]

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
    if pitch_text.lower() in {"r", "rest"}:
        return ScoreEvent(
            kind="rest",
            note=None,
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


def parse_score(
    text: str,
    *,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
) -> TextScore:
    """Parse beginner-friendly text sheet music into a ``TextScore``.

    Directives apply from the top of the file to the whole score. Because event
    durations depend on tempo, V1 requires tempo changes to be global rather
    than interleaved with music tokens.
    """

    if not isinstance(text, str):
        raise ValueError("score text must be a string")

    score_limit = _non_negative_integer("max_score_length", max_score_length)
    line_limit = _non_negative_integer("max_line_length", max_line_length)
    event_limit = _non_negative_integer("max_event_count", max_event_count)
    if len(text) > score_limit:
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

        directive_match = DIRECTIVE_PATTERN.fullmatch(line)
        if directive_match is not None:
            name, value = directive_match.groups()
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
            elif name == "instrument":
                if gm_program is not None:
                    raise ValueError(
                        f"line {line_number}: score may specify instrument or "
                        "program, not both"
                    )
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
    )


def parse_portable_score(
    text: str,
    *,
    max_score_length: int = DEFAULT_MAX_SCORE_LENGTH,
    max_line_length: int = DEFAULT_MAX_LINE_LENGTH,
    max_event_count: int = DEFAULT_MAX_EVENT_COUNT,
) -> PortableScore:
    """Parse canonical `music-machine-score/v2` multi-track score text."""

    if not isinstance(text, str):
        raise ValueError("score text must be a string")

    score_limit = _non_negative_integer("max_score_length", max_score_length)
    line_limit = _non_negative_integer("max_line_length", max_line_length)
    event_limit = _non_negative_integer("max_event_count", max_event_count)
    if len(text) > score_limit:
        raise ValueError(
            f"score text length {len(text)} exceeds max_score_length={score_limit}"
        )

    saw_format = False
    title = "Untitled"
    ppq = DEFAULT_PPQ
    sample_rate_hz = int(DEFAULT_SAMPLE_RATE_HZ)
    tempo_events: list[TempoEvent] = []
    meter_events: list[MeterEvent] = []
    instruments: list[InstrumentDeclaration] = []
    tracks: list[TrackDeclaration] = []
    events: list[PortableScoreEvent] = []

    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        if len(raw_line) > line_limit:
            raise ValueError(
                f"line {line_number}: line length {len(raw_line)} exceeds "
                f"max_line_length={line_limit}"
            )
        line = raw_line.strip()
        if line == "" or line.startswith("#"):
            continue

        if not saw_format:
            if line != f"format: {PORTABLE_FORMAT_VERSION}":
                raise ValueError(
                    f"line {line_number}: first directive must be "
                    f"'format: {PORTABLE_FORMAT_VERSION}'"
                )
            saw_format = True
            continue

        directive_match = DIRECTIVE_PATTERN.fullmatch(line)
        if directive_match is not None:
            name, value = directive_match.groups()
            if name == "title":
                title = value
            elif name == "ppq":
                ppq = _integer_directive("ppq", value)
            elif name == "sample_rate":
                sample_rate_hz = _integer_valued_sample_rate(value)
            else:
                raise ValueError(f"line {line_number}: unknown directive {name!r}")
            continue

        tokens = line.split()
        if not tokens:
            continue
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
                events.append(
                    _parse_portable_event_line(tokens, len(events))
                )
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
    profile_id: str | None = None
    if "program" in properties:
        if "profile" in properties or "kind" in properties:
            raise ValueError("instrument may specify only one of program/profile/kind")
        profile_id = instrument_for_gm_program(
            _integer_directive("program", properties["program"])
        ).id
    elif "profile" in properties:
        if "kind" in properties:
            raise ValueError("instrument may specify only one of program/profile/kind")
        profile_id = resolve_instrument_id(instrument_id=properties["profile"])
    elif "kind" in properties:
        kind = properties["kind"]
        if kind not in {"sine", "silence"}:
            raise ValueError("kind must be sine or silence in V2")
        profile_id = kind
    else:
        raise ValueError("instrument must include program, profile, or kind")

    allowed = {"gain", "program", "profile", "kind"}
    unknown = sorted(set(properties) - allowed)
    if unknown:
        raise ValueError(f"unknown instrument properties: {', '.join(unknown)}")

    return InstrumentDeclaration(instrument_id, profile_id, gain)


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
            "event lines must be: event <track> <start> <duration> note|rest ..."
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
    if kind != "note":
        raise ValueError("event kind must be note or rest")
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

    notes = ",".join(event.notes)
    return (
        f"event {event.track_id} {event.start_tick} {event.duration_tick} "
        f"note {notes} velocity={_format_portable_number(event.velocity)}"
    )


def _validate_total_sample_budget(
    events: tuple[ScoreEvent, ...],
    sample_rate_hz: int,
    instrument_id: str,
    max_sample_count: int,
) -> None:
    limit = _positive_integer("max_sample_count", max_sample_count)
    instrument = get_instrument(instrument_id)
    total = sum(
        sample_count_for_duration(
            event.duration_seconds
            + (
                instrument.envelope_profile.release_seconds
                if event.kind == "note"
                else 0.0
            ),
            sample_rate_hz,
        )
        for event in events
    )
    if total > limit:
        raise ValueError(
            f"render would create {total} samples, above max_sample_count={limit}"
        )


def render_score_to_pcm(
    score: TextScore,
    *,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> RenderedScore:
    """Render a parsed score into one signed 16-bit mono PCM buffer."""

    if not isinstance(score, TextScore):
        raise ValueError("score must be a TextScore")

    _validate_total_sample_budget(
        score.events,
        score.sample_rate_hz,
        score.instrument_id,
        max_sample_count,
    )

    pcm_samples: list[int] = []
    rendered_notes: list[InstrumentNoteRender] = []
    clipped_sample_count = 0
    elapsed_seconds = 0.0
    cursor_sample_index = 0

    for event in score.events:
        if event.kind == "rest":
            silence_count = sample_count_for_duration(
                event.duration_seconds,
                score.sample_rate_hz,
            )
            cursor_sample_index += silence_count
            if len(pcm_samples) < cursor_sample_index:
                pcm_samples.extend(
                    0 for _ in range(cursor_sample_index - len(pcm_samples))
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
            required_count = (
                cursor_sample_index + rendered_note.pcm_buffer.sample_count()
            )
            if len(pcm_samples) < required_count:
                pcm_samples.extend(0 for _ in range(required_count - len(pcm_samples)))
            for offset, sample in enumerate(rendered_note.pcm_buffer.samples):
                sample_index = cursor_sample_index + offset
                mixed = pcm_samples[sample_index] + sample
                pcm_samples[sample_index] = max(-32_768, min(32_767, mixed))
            clipped_sample_count += rendered_note.pcm_buffer.clipped_sample_count
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
    )


def _seconds_at_tick(
    tick: int,
    ppq: int,
    tempo_events: tuple[TempoEvent, ...],
) -> float:
    checked_tick = _non_negative_tick("tick", tick)
    sorted_tempos = tuple(sorted(tempo_events, key=lambda event: event.start_tick))
    seconds = 0.0
    active_tick = sorted_tempos[0].start_tick
    active_bpm = sorted_tempos[0].bpm

    for tempo in sorted_tempos[1:]:
        if tempo.start_tick >= checked_tick:
            break
        tick_delta = tempo.start_tick - active_tick
        seconds += tick_delta * 60.0 / (active_bpm * ppq)
        active_tick = tempo.start_tick
        active_bpm = tempo.bpm

    seconds += (checked_tick - active_tick) * 60.0 / (active_bpm * ppq)
    return seconds


def _mix_pcm_samples(
    target: list[int],
    start_sample: int,
    source: tuple[int, ...],
) -> int:
    required_count = start_sample + len(source)
    if len(target) < required_count:
        target.extend(0 for _ in range(required_count - len(target)))

    clipped_count = 0
    for offset, sample in enumerate(source):
        target_index = start_sample + offset
        mixed = target[target_index] + sample
        clipped = max(-32_768, min(32_767, mixed))
        if clipped != mixed:
            clipped_count += 1
        target[target_index] = clipped
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
    instrument_by_id = score.instrument_map()
    track_by_id = score.track_map()
    pcm_samples: list[int] = []
    rendered_notes: list[InstrumentNoteRender] = []
    clipped_sample_count = 0

    for event in score.events:
        start_seconds = _seconds_at_tick(
            event.start_tick,
            score.ppq,
            score.tempo_events,
        )
        end_seconds = _seconds_at_tick(
            event.start_tick + event.duration_tick,
            score.ppq,
            score.tempo_events,
        )
        duration_seconds = end_seconds - start_seconds
        start_sample = sample_count_for_duration(start_seconds, score.sample_rate_hz)

        if event.kind == "rest":
            rest_end = sample_count_for_duration(end_seconds, score.sample_rate_hz)
            if rest_end > sample_limit:
                raise ValueError(
                    f"render would create {rest_end} samples, "
                    f"above max_sample_count={sample_limit}"
                )
            if len(pcm_samples) < rest_end:
                pcm_samples.extend(0 for _ in range(rest_end - len(pcm_samples)))
            continue

        track = track_by_id[event.track_id]
        instrument = instrument_by_id[track.instrument_id]
        note_amplitude = event.velocity * instrument.gain / max(1, len(event.notes))
        for note in event.notes:
            rendered_note = render_instrument_note(
                note,
                duration_seconds,
                instrument=instrument.profile_id,
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
    )


PlaybackSink = Callable[[PCMBuffer], object]


def _default_play_pcm_buffer(buffer: PCMBuffer) -> object:
    """Import the real audio sink only at the exact moment playback is requested."""

    sink = import_module("audio_device_sink")
    play_pcm_buffer = sink.play_pcm_buffer
    if not callable(play_pcm_buffer):
        raise TypeError("audio_device_sink.play_pcm_buffer must be callable")
    return play_pcm_buffer(buffer)


def play_score(
    score: TextScore,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Render a score and delegate its PCM buffer to an audio-device sink."""

    rendered_score = render_score_to_pcm(score, max_sample_count=max_sample_count)
    player = (
        play_pcm_buffer
        if play_pcm_buffer is not None
        else _default_play_pcm_buffer
    )
    return player(rendered_score.pcm_buffer)


def play_portable_score(
    score: PortableScore,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Render a portable multi-track score and delegate its PCM to a sink."""

    rendered_score = render_portable_score_to_pcm(
        score,
        max_sample_count=max_sample_count,
    )
    player = (
        play_pcm_buffer
        if play_pcm_buffer is not None
        else _default_play_pcm_buffer
    )
    return player(rendered_score.pcm_buffer)


def play_score_text(
    text: str,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Parse, render, and play a text score through a lazy playback sink."""

    return play_score(
        parse_score(text),
        play_pcm_buffer=play_pcm_buffer,
        max_sample_count=max_sample_count,
    )


def play_portable_score_text(
    text: str,
    *,
    play_pcm_buffer: PlaybackSink | None = None,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> object:
    """Parse, render, and play portable multi-track score text."""

    return play_portable_score(
        parse_portable_score(text),
        play_pcm_buffer=play_pcm_buffer,
        max_sample_count=max_sample_count,
    )
