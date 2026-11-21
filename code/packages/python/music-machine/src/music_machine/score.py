"""Text score parsing and rendering for the music machine (MUS01).

This module implements the tiny monophonic text score parser and PCM renderer.
All regular expressions have been eliminated (CWE-1333) in favor of deterministic
linear-time string scanning.
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Mapping, Sized
from dataclasses import dataclass
from numbers import Integral, Real

from note_frequency import Note, parse_note

MAX_SCORE_BYTES: int = 1_000_000
MAX_LINE_COUNT: int = 100_000
MAX_EVENTS: int = 10_000
MAX_SAMPLE_RATE_HZ: float = 192_000.0
MAX_SAMPLE_COUNT: int = 16_777_216


def _safe_repr(value: object) -> str:
    """Bounded O(1) representation for error messages (CWE-190 / CWE-400)."""
    if isinstance(value, bool):
        return repr(value)
    if isinstance(value, int):
        if value < -1_000_000 or value > 1_000_000:
            return "<out-of-range int>"
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        if len(value) > 64:
            return f"<str len={len(value)}>"
        return repr(value)
    return f"<{type(value).__name__}>"


def _finite_real(name: str, value: object) -> float:
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ScoreParseError(
            f"{name} must be a finite real number, got {_safe_repr(value)}"
        )
    try:
        converted = float(value)
    except OverflowError as exc:
        raise ScoreParseError(
            f"{name} overflows float range: {_safe_repr(value)}"
        ) from exc
    if not math.isfinite(converted):
        raise ScoreParseError(f"{name} must be finite, got {_safe_repr(value)}")
    return converted


class ScoreParseError(ValueError):
    """Raised when a score file cannot be parsed or rendered."""


@dataclass(frozen=True)
class MusicalEvent:
    """One event in a monophonic score."""

    note: Note | None
    duration_beats: float
    velocity: float = 1.0

    def __post_init__(self) -> None:
        if self.note is not None and not isinstance(self.note, Note):
            raise ScoreParseError("note must be a Note or None")
        beats = _finite_real("duration_beats", self.duration_beats)
        if beats <= 0.0:
            raise ScoreParseError("duration_beats must be a positive finite number")
        object.__setattr__(self, "duration_beats", beats)
        vel = _finite_real("velocity", self.velocity)
        if vel < 0.0 or vel > 1.0:
            raise ScoreParseError("velocity must be within [0.0, 1.0]")
        object.__setattr__(self, "velocity", vel)

    def duration_seconds(self, tempo_bpm: float) -> float:
        """Convert this event duration to seconds for the given tempo."""
        if isinstance(tempo_bpm, bool) or not isinstance(tempo_bpm, Real):
            raise ScoreParseError("tempo_bpm must be a positive finite number")
        try:
            tempo = float(tempo_bpm)
        except OverflowError as exc:
            raise ScoreParseError("tempo_bpm overflows float range") from exc
        if not math.isfinite(tempo) or tempo <= 0.0:
            raise ScoreParseError("tempo_bpm must be a positive finite number")
        return 60.0 * self.duration_beats / tempo


@dataclass(frozen=True)
class Score:
    """A parsed score with optional tempo and title."""

    events: tuple[MusicalEvent, ...]
    tempo_bpm: float = 120.0
    title: str | None = None

    def __post_init__(self) -> None:
        if self.title is not None and not isinstance(self.title, str):
            raise ScoreParseError("title must be a string or None")
        if isinstance(self.tempo_bpm, bool) or not isinstance(self.tempo_bpm, Real):
            raise ScoreParseError(
                "tempo_bpm must be a finite positive number, "
                f"got {_safe_repr(self.tempo_bpm)}"
            )
        try:
            tempo = float(self.tempo_bpm)
        except OverflowError as exc:
            raise ScoreParseError(
                f"tempo_bpm overflows float range: {_safe_repr(self.tempo_bpm)}"
            ) from exc
        if not math.isfinite(tempo) or tempo <= 0.0:
            raise ScoreParseError(
                f"tempo_bpm must be a positive finite number, got {tempo}"
            )
        object.__setattr__(self, "tempo_bpm", tempo)

        if isinstance(self.events, (str, bytes, bytearray, Mapping)) or not isinstance(
            self.events, Iterable
        ):
            raise ScoreParseError("events must be an iterable of MusicalEvent")
        if isinstance(self.events, Sized):
            try:
                declared_len = len(self.events)
            except OverflowError as exc:
                raise ScoreParseError(
                    f"event count exceeds MAX_EVENTS={MAX_EVENTS}"
                ) from exc
            if declared_len > MAX_EVENTS:
                raise ScoreParseError(
                    f"event count {declared_len} exceeds MAX_EVENTS={MAX_EVENTS}"
                )
        ev_list: list[MusicalEvent] = []
        for idx, ev in enumerate(self.events):
            if idx >= MAX_EVENTS:
                raise ScoreParseError(f"event count exceeds MAX_EVENTS={MAX_EVENTS}")
            if not isinstance(ev, MusicalEvent):
                raise ScoreParseError(f"events[{idx}] must be a MusicalEvent")
            ev_list.append(ev)
        object.__setattr__(self, "events", tuple(ev_list))


def _parse_duration(raw: str, *, line_no: int) -> float:
    try:
        value = float(raw)
    except (ValueError, TypeError) as exc:
        raise ScoreParseError(
            f"line {line_no}: invalid duration {_safe_repr(raw)}"
        ) from exc

    if not math.isfinite(value) or value <= 0.0:
        raise ScoreParseError(
            f"line {line_no}: duration must be a positive finite number"
        )
    return value


def _parse_velocity(raw: str | None, *, line_no: int) -> float:
    if raw is None:
        return 1.0

    try:
        value = float(raw)
    except (ValueError, TypeError) as exc:
        raise ScoreParseError(
            f"line {line_no}: invalid velocity {_safe_repr(raw)}"
        ) from exc

    if not math.isfinite(value) or value < 0.0 or value > 1.0:
        raise ScoreParseError(f"line {line_no}: velocity must be within [0.0, 1.0]")
    return value


def _parse_directive(line: str, line_no: int) -> tuple[str, str] | None:
    """Parse a directive line and return ``(key, value)`` linearly without regex."""
    if ":" not in line:
        return None

    key, value = line.split(":", 1)
    key = key.strip().lower()
    value = value.strip()
    if not key or not value:
        raise ScoreParseError(
            f"line {line_no}: malformed directive {_safe_repr(line)}"
        )

    return key, value


def parse_score(
    text: str,
    *,
    max_score_bytes: int = MAX_SCORE_BYTES,
    max_event_count: int = MAX_EVENTS,
) -> Score:
    """Parse a MUS01 text score into a Score model without regular expressions."""
    if not isinstance(text, str):
        raise ScoreParseError("score text must be a string")

    if isinstance(max_score_bytes, bool) or not isinstance(max_score_bytes, Integral):
        raise ScoreParseError("max_score_bytes must be a non-negative integer")
    byte_limit = int(max_score_bytes)
    if byte_limit < 0 or byte_limit > MAX_SCORE_BYTES:
        raise ScoreParseError(
            f"max_score_bytes must be in [0, {MAX_SCORE_BYTES}]"
        )

    if isinstance(max_event_count, bool) or not isinstance(max_event_count, Integral):
        raise ScoreParseError("max_event_count must be a non-negative integer")
    event_limit = int(max_event_count)
    if event_limit < 0 or event_limit > MAX_EVENTS:
        raise ScoreParseError(f"max_event_count must be in [0, {MAX_EVENTS}]")

    if len(text) > byte_limit:
        raise ScoreParseError(
            f"score text length {len(text)} exceeds max_score_bytes={byte_limit}"
        )

    encoded_len = len(text.encode("utf-8"))
    if encoded_len > byte_limit:
        raise ScoreParseError(
            f"score text length {encoded_len} bytes exceeds "
            f"max_score_bytes={byte_limit}"
        )

    if text.count("\n") >= MAX_LINE_COUNT:
        raise ScoreParseError(f"line count exceeds MAX_LINE_COUNT={MAX_LINE_COUNT}")

    tempo_bpm = 120.0
    title: str | None = None
    events: list[MusicalEvent] = []

    for line_no, raw_line in enumerate(text.splitlines(), start=1):
        if line_no > MAX_LINE_COUNT:
            raise ScoreParseError(
                f"line count exceeds MAX_LINE_COUNT={MAX_LINE_COUNT}"
            )
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue

        directive = _parse_directive(line, line_no)
        if directive is not None:
            key, value = directive
            if key == "tempo":
                try:
                    tempo_bpm = float(value)
                except (ValueError, TypeError) as exc:
                    raise ScoreParseError(
                        f"line {line_no}: tempo must be a finite number"
                    ) from exc

                if not math.isfinite(tempo_bpm) or tempo_bpm <= 0.0:
                    raise ScoreParseError(
                        f"line {line_no}: tempo must be a positive finite number"
                    )
                continue

            if key == "title":
                title = value
                continue

            raise ScoreParseError(
                f"line {line_no}: unknown directive {_safe_repr(key)}"
            )

        # Linear token splitting: replaces regex ^\s*(\S+)\s+(\S+)(?:\s+(\S+))?\s*$
        parts = line.split()
        if len(parts) == 2:
            token, duration_raw = parts
            velocity_raw: str | None = None
        elif len(parts) == 3:
            token, duration_raw, velocity_raw = parts
        else:
            raise ScoreParseError(
                f"line {line_no}: malformed event line {_safe_repr(raw_line)}"
            )

        if len(events) >= MAX_EVENTS:
            raise ScoreParseError(
                f"line {line_no}: event count exceeds MAX_EVENTS={MAX_EVENTS}"
            )

        duration_beats = _parse_duration(duration_raw, line_no=line_no)
        velocity = _parse_velocity(velocity_raw, line_no=line_no)

        note_token = token.strip().upper()
        if note_token == "R":
            note = None
        else:
            try:
                note = parse_note(token)
            except ValueError as exc:
                raise ScoreParseError(f"line {line_no}: {exc}") from exc

        if len(events) >= event_limit:
            raise ScoreParseError(
                f"event count exceeds max_event_count={event_limit}"
            )
        events.append(
            MusicalEvent(
                note=note,
                duration_beats=duration_beats,
                velocity=velocity,
            )
        )

    return Score(events=tuple(events), tempo_bpm=tempo_bpm, title=title)


def score_duration_seconds(score: Score) -> float:
    """Return total duration in seconds using the score tempo."""
    if not isinstance(score, Score):
        raise ScoreParseError("score must be a Score instance")
    return sum(event.duration_seconds(score.tempo_bpm) for event in score.events)


def render_score(
    score: Score,
    sample_rate_hz: int = 44100,
    *,
    max_sample_count: int = MAX_SAMPLE_COUNT,
) -> list[float]:
    """Render a monophonic score to a list of floating-point samples."""
    if not isinstance(score, Score):
        raise ScoreParseError("score must be a Score instance")

    if isinstance(max_sample_count, bool) or not isinstance(max_sample_count, Integral):
        raise ScoreParseError("max_sample_count must be a positive integer")
    sample_limit = int(max_sample_count)
    if sample_limit <= 0 or sample_limit > MAX_SAMPLE_COUNT:
        raise ScoreParseError(
            f"max_sample_count must be in [1, {MAX_SAMPLE_COUNT}]"
        )

    if isinstance(sample_rate_hz, bool) or not isinstance(
        sample_rate_hz, (Integral, Real)
    ):
        raise ScoreParseError("sample_rate_hz must be an integer")
    try:
        sr_float = float(sample_rate_hz)
    except OverflowError as exc:
        raise ScoreParseError("sample_rate_hz overflows float range") from exc
    if not math.isfinite(sr_float) or sr_float <= 0.0:
        raise ScoreParseError("sample_rate_hz must be greater than zero")
    if sr_float > MAX_SAMPLE_RATE_HZ:
        raise ScoreParseError(
            f"sample_rate_hz {sr_float} exceeds "
            f"MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
        )

    rate = int(round(sr_float))
    if rate <= 0 or not math.isfinite(1.0 / rate):
        raise ScoreParseError("sample_rate_hz must round to an integer >= 1")
    total_seconds = score_duration_seconds(score)
    if not math.isfinite(total_seconds) or not math.isfinite(total_seconds * rate):
        raise ScoreParseError("score duration produces non-finite sample count")
    expected_samples = int(math.ceil(total_seconds * rate))
    if expected_samples > sample_limit:
        raise ScoreParseError(
            f"rendered score would require {expected_samples} samples, "
            f"exceeding max_sample_count={sample_limit}"
        )

    samples: list[float] = []
    phase = 0.0

    for event in score.events:
        duration_seconds = event.duration_seconds(score.tempo_bpm)
        if not math.isfinite(duration_seconds) or not math.isfinite(
            duration_seconds * rate
        ):
            raise ScoreParseError("event duration produces non-finite sample count")
        frame_count = int(math.floor(duration_seconds * rate))

        if frame_count <= 0:
            continue

        if len(samples) + frame_count > sample_limit:
            raise ScoreParseError("sample count exceeds max_sample_count")

        if event.note is None:
            samples.extend([0.0] * frame_count)
            continue

        frequency = event.note.frequency()
        angular_step = 2.0 * math.pi * frequency / rate
        for _ in range(frame_count):
            sample_value = event.velocity * math.sin(phase)
            samples.append(sample_value)
            phase += angular_step

    return samples
