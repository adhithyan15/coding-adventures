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
from collections.abc import Callable
from dataclasses import dataclass
from importlib import import_module
from math import isfinite
from numbers import Integral, Real
from typing import Literal

from note_audio import (
    DEFAULT_MAX_SAMPLE_COUNT,
    RenderedNote,
    render_note_to_sound_chain,
)
from note_frequency import parse_note
from oscillator import sample_count_for_duration
from pcm_audio import DEFAULT_SAMPLE_RATE_HZ, PCMBuffer, PCMFormat

DEFAULT_TEMPO_BPM = 120.0
DEFAULT_METER = "4/4"
DEFAULT_AMPLITUDE = 0.18
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
    }
)

DIRECTIVE_PATTERN = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$")
METER_PATTERN = re.compile(r"^([1-9][0-9]*)/([1-9][0-9]*)$")

HAPPY_BIRTHDAY_TEXT = """title: Happy Birthday
tempo: 120
meter: 3/4
amplitude: 0.18
sample_rate: 44100

G4/e G4/e | A4/q G4/q C5/q | B4/h R/q |
G4/e G4/e | A4/q G4/q D5/q | C5/h R/q |
G4/e G4/e | G5/q E5/q C5/q | B4/q A4/q R/q |
F5/e F5/e | E5/q C5/q D5/q | C5/h
"""

ScoreEventKind = Literal["note", "rest"]


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


def _integer_valued_sample_rate(text: str) -> int:
    try:
        value = float(text)
    except ValueError as exc:
        raise ValueError(f"sample_rate must be an integer, got {text!r}") from exc

    if not isfinite(value) or value != round(value):
        raise ValueError(f"sample_rate must be an integer, got {text!r}")
    return _positive_integer("sample_rate", int(round(value)))


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
    rendered_notes: tuple[RenderedNote, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.score, TextScore):
            raise ValueError("score must be a TextScore")
        if not isinstance(self.pcm_buffer, PCMBuffer):
            raise ValueError("pcm_buffer must be a PCMBuffer")
        object.__setattr__(self, "rendered_notes", tuple(self.rendered_notes))
        for index, rendered_note in enumerate(self.rendered_notes):
            if not isinstance(rendered_note, RenderedNote):
                raise ValueError(
                    f"rendered_notes[{index}] must be a note_audio.RenderedNote"
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
        events=tuple(events),
    )


def _validate_total_sample_budget(
    events: tuple[ScoreEvent, ...],
    sample_rate_hz: int,
    max_sample_count: int,
) -> None:
    limit = _positive_integer("max_sample_count", max_sample_count)
    total = sum(
        sample_count_for_duration(event.duration_seconds, sample_rate_hz)
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

    _validate_total_sample_budget(score.events, score.sample_rate_hz, max_sample_count)

    pcm_samples: list[int] = []
    rendered_notes: list[RenderedNote] = []
    clipped_sample_count = 0
    elapsed_seconds = 0.0

    for event in score.events:
        if event.kind == "rest":
            silence_count = sample_count_for_duration(
                event.duration_seconds,
                score.sample_rate_hz,
            )
            pcm_samples.extend(0 for _ in range(silence_count))
        else:
            rendered_note = render_note_to_sound_chain(
                event.note,
                event.duration_seconds,
                sample_rate_hz=score.sample_rate_hz,
                amplitude=score.amplitude,
                start_time_seconds=elapsed_seconds,
                max_sample_count=max_sample_count,
            )
            rendered_notes.append(rendered_note)
            pcm_samples.extend(rendered_note.pcm_buffer.samples)
            clipped_sample_count += rendered_note.pcm_buffer.clipped_sample_count
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
