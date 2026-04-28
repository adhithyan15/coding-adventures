"""Text score parsing and rendering for the music machine."""

from __future__ import annotations

import math
import re
from dataclasses import dataclass

from note_frequency import Note, parse_note

_DURATION_LINE_RE = re.compile(r"^\s*(\S+)\s+(\S+)(?:\s+(\S+))?\s*$")


class ScoreParseError(ValueError):
    """Raised when a score file cannot be parsed."""


@dataclass(frozen=True)
class MusicalEvent:
    """One event in a monophonic score."""

    note: Note | None
    duration_beats: float
    velocity: float = 1.0

    def duration_seconds(self, tempo_bpm: float) -> float:
        """Convert this event duration to seconds for the given tempo."""
        return 60.0 * self.duration_beats / tempo_bpm


@dataclass(frozen=True)
class Score:
    """A parsed score with optional tempo and title."""

    events: tuple[MusicalEvent, ...]
    tempo_bpm: float = 120.0
    title: str | None = None


def _parse_duration(raw: str, *, line_no: int) -> float:
    try:
        value = float(raw)
    except ValueError as exc:  # pragma: no cover - defensive parse failure path
        raise ScoreParseError(f"line {line_no}: invalid duration '{raw}'") from exc

    if not math.isfinite(value) or value <= 0:
        raise ScoreParseError(
            f"line {line_no}: duration must be a positive finite number"
        )

    return value


def _parse_velocity(raw: str | None, *, line_no: int) -> float:
    if raw is None:
        return 1.0

    try:
        value = float(raw)
    except ValueError as exc:  # pragma: no cover - defensive parse failure path
        raise ScoreParseError(f"line {line_no}: invalid velocity '{raw}'") from exc

    if not math.isfinite(value) or value < 0.0 or value > 1.0:
        raise ScoreParseError(f"line {line_no}: velocity must be within [0.0, 1.0]")

    return value


def _parse_directive(line: str, line_no: int) -> tuple[str, str] | None:
    """Parse a directive line and return ``(key, value)``."""
    if ":" not in line:
        return None

    key, value = line.split(":", 1)
    key = key.strip().lower()
    value = value.strip()
    if not key or not value:
        raise ScoreParseError(f"line {line_no}: malformed directive '{line}'")

    return key, value


def parse_score(text: str) -> Score:
    """Parse a MUS01 text score into a Score model."""
    tempo_bpm = 120.0
    title: str | None = None
    events: list[MusicalEvent] = []

    for line_no, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue
        if line.startswith("#"):
            continue

        directive = _parse_directive(line, line_no)
        if directive is not None:
            key, value = directive
            if key == "tempo":
                try:
                    tempo_bpm = float(value)
                except ValueError as exc:
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

            raise ScoreParseError(f"line {line_no}: unknown directive '{key}'")

        match = _DURATION_LINE_RE.match(raw_line)
        if match is None:
            raise ScoreParseError(f"line {line_no}: malformed event line '{raw_line}'")

        token, duration_raw, velocity_raw = match.groups()
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
    return sum(event.duration_seconds(score.tempo_bpm) for event in score.events)


def render_score(score: Score, sample_rate_hz: int = 44100) -> list[float]:
    """Render a monophonic score to a list of samples."""
    if sample_rate_hz <= 0:
        raise ScoreParseError("sample_rate_hz must be greater than zero")

    samples: list[float] = []
    phase = 0.0

    for event in score.events:
        duration_seconds = event.duration_seconds(score.tempo_bpm)
        frame_count = int(math.floor(duration_seconds * sample_rate_hz))

        if frame_count <= 0:
            continue

        if event.note is None:
            samples.extend([0.0] * frame_count)
            continue

        frequency = event.note.frequency()
        angular_step = 2.0 * math.pi * frequency / sample_rate_hz
        for _ in range(frame_count):
            sample_value = event.velocity * math.sin(phase)
            samples.append(sample_value)
            phase += angular_step

    return samples
