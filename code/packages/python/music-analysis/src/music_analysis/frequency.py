"""Equal-tempered frequency to note inversion (MUS08, MUS00)."""

from __future__ import annotations

import math

from music_analysis.types import (
    MAX_SAMPLE_RATE_HZ,
    NotePitch,
    validate_positive_float,
)

PITCH_NAMES: tuple[str, ...] = (
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


def frequency_to_note(hz: float, a4_hz: float = 440.0) -> NotePitch:
    """Convert positive frequency in Hz to nearest note name, MIDI index, and cents.

    Args:
        hz: Fundamental frequency in Hertz (> 0.0, finite).
        a4_hz: Standard tuning reference frequency for A4 (default 440.0 Hz).

    Returns:
        NotePitch named tuple with fields (note, midi, cents).

    Raises:
        InvalidInputError: If hz or a4_hz is non-positive or non-finite.
    """
    valid_hz = validate_positive_float(hz, "hz", max_val=MAX_SAMPLE_RATE_HZ / 2.0)
    valid_a4 = validate_positive_float(a4_hz, "a4_hz", max_val=10_000.0)

    # Invert equal-tempered frequency formula: f = a4 * 2^((midi - 69) / 12)
    semitones_from_a4 = 12.0 * math.log2(valid_hz / valid_a4)
    exact_midi = 69.0 + semitones_from_a4
    nearest_midi = int(math.floor(exact_midi + 0.5))
    cents = (exact_midi - nearest_midi) * 100.0

    pitch_class = PITCH_NAMES[nearest_midi % 12]
    octave = (nearest_midi // 12) - 1
    note_name = f"{pitch_class}{octave}"

    return NotePitch(note=note_name, midi=nearest_midi, cents=round(cents, 2))
