"""Unit tests for frequency_to_note conversion (MUS08, MUS00)."""

from __future__ import annotations

import math

import pytest
from note_frequency import note_to_frequency

from music_analysis import frequency_to_note
from music_analysis.types import InvalidInputError, NotePitch


def test_frequency_to_note_standard_tuning() -> None:
    res = frequency_to_note(440.0)
    assert isinstance(res, NotePitch)
    assert res.note == "A4"
    assert res.midi == 69
    assert math.isclose(res.cents, 0.0, abs_tol=0.1)

    # Tuple unpacking support
    note, midi, cents = frequency_to_note(440.0)
    assert note == "A4"
    assert midi == 69
    assert math.isclose(cents, 0.0, abs_tol=0.1)


def test_frequency_to_note_c4_middle_c() -> None:
    # C4 frequency in equal temperament
    c4_hz = 440.0 * (2.0 ** (-9.0 / 12.0))
    res = frequency_to_note(c4_hz)
    assert res.note == "C4"
    assert res.midi == 60
    assert math.isclose(res.cents, 0.0, abs_tol=0.1)


def test_frequency_to_note_octave_extremes() -> None:
    # A0 (lowest piano note): 27.5 Hz
    res_a0 = frequency_to_note(27.5)
    assert res_a0.note == "A0"
    assert res_a0.midi == 21

    # C8 (highest piano note): ~4186 Hz
    c8_hz = 440.0 * (2.0 ** ((108 - 69) / 12.0))
    res_c8 = frequency_to_note(c8_hz)
    assert res_c8.note == "C8"
    assert res_c8.midi == 108


def test_frequency_to_note_alternate_reference() -> None:
    # In European orchestral tuning A4 = 442.0 Hz
    res = frequency_to_note(442.0, a4_hz=442.0)
    assert res.note == "A4"
    assert res.midi == 69
    assert math.isclose(res.cents, 0.0, abs_tol=0.1)


def test_frequency_to_note_cents_deviation() -> None:
    # 445 Hz is sharp relative to 440 Hz (~19.56 cents)
    res = frequency_to_note(445.0, a4_hz=440.0)
    assert res.note == "A4"
    assert res.midi == 69
    assert res.cents > 15.0 and res.cents < 25.0

    # 435 Hz is flat relative to 440 Hz (~ -19.78 cents)
    res_flat = frequency_to_note(435.0, a4_hz=440.0)
    assert res_flat.note == "A4"
    assert res_flat.midi == 69
    assert res_flat.cents < -15.0 and res_flat.cents > -25.0


def test_frequency_to_note_round_trip_with_note_frequency() -> None:
    test_notes = ["C3", "D#4", "F#5", "A4", "B2", "G#3"]
    for note_str in test_notes:
        hz = note_to_frequency(note_str)
        match = frequency_to_note(hz)
        assert match.note == note_str
        assert math.isclose(match.cents, 0.0, abs_tol=0.2)


def test_frequency_to_note_invalid_inputs() -> None:
    with pytest.raises(InvalidInputError):
        frequency_to_note(0.0)
    with pytest.raises(InvalidInputError):
        frequency_to_note(-440.0)
    with pytest.raises(InvalidInputError):
        frequency_to_note(float("nan"))
    with pytest.raises(InvalidInputError):
        frequency_to_note(float("inf"))
    with pytest.raises(InvalidInputError):
        frequency_to_note(440.0, a4_hz=0.0)
    with pytest.raises(InvalidInputError):
        frequency_to_note(440.0, a4_hz=-440.0)
