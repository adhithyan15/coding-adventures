"""
Note parsing and equal-tempered pitch mapping.

This module turns note labels such as ``A4`` and ``C#5`` into a structured
``Note`` object and then into a frequency in Hertz.

The core musical reference point is:

    A4 = 440 Hz

From there, each semitone step changes frequency by a factor of ``2 ** (1/12)``.
"""

from __future__ import annotations

from dataclasses import dataclass
import re

NOTE_PATTERN = re.compile(r"^([A-Ga-g])([#b]?)(-?\d+)$")

CHROMATIC_INDEX = {
    "C": 0,
    "C#": 1,
    "Db": 1,
    "D": 2,
    "D#": 3,
    "Eb": 3,
    "E": 4,
    "F": 5,
    "F#": 6,
    "Gb": 6,
    "G": 7,
    "G#": 8,
    "Ab": 8,
    "A": 9,
    "A#": 10,
    "Bb": 10,
    "B": 11,
}

REFERENCE_OCTAVE = 4
REFERENCE_INDEX = CHROMATIC_INDEX["A"]
REFERENCE_FREQUENCY_HZ = 440.0
SEMITONES_PER_OCTAVE = 12


@dataclass(frozen=True)
class Note:
    """
    A parsed musical note label.

    Example:

    - ``A4`` means note letter A in octave 4
    - ``C#5`` means C-sharp in octave 5
    - ``Db3`` means D-flat in octave 3
    """

    letter: str
    accidental: str
    octave: int

    def __post_init__(self) -> None:
        canonical_letter = self.letter.upper()
        canonical_accidental = self.accidental

        spelling = canonical_letter + canonical_accidental
        if spelling not in CHROMATIC_INDEX:
            raise ValueError(
                f"Unsupported note spelling {spelling!r}. "
                "Only natural notes plus single # or b accidentals are supported."
            )

        object.__setattr__(self, "letter", canonical_letter)
        object.__setattr__(self, "accidental", canonical_accidental)
        object.__setattr__(self, "octave", int(self.octave))

    @property
    def spelling(self) -> str:
        """Return the canonical pitch spelling without the octave."""

        return self.letter + self.accidental

    @property
    def chromatic_index(self) -> int:
        """Return this note's index within the 12-note chromatic octave."""

        return CHROMATIC_INDEX[self.spelling]

    def semitones_from_a4(self) -> int:
        """Return the signed semitone offset from the reference note A4."""

        octave_offset = (self.octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE
        pitch_offset = self.chromatic_index - REFERENCE_INDEX
        return octave_offset + pitch_offset

    def frequency(self) -> float:
        """Return the equal-tempered frequency in Hertz."""

        semitone_offset = self.semitones_from_a4()
        return REFERENCE_FREQUENCY_HZ * (2 ** (semitone_offset / SEMITONES_PER_OCTAVE))

    def __str__(self) -> str:
        return f"{self.spelling}{self.octave}"


def parse_note(text: str) -> Note:
    """Parse a note label like ``A4`` or ``C#5`` into a :class:`Note`."""

    match = NOTE_PATTERN.fullmatch(text)
    if match is None:
        raise ValueError(
            f"Invalid note {text!r}. Expected <letter><optional # or b><octave>, "
            "for example 'A4', 'C#5', or 'Db3'."
        )

    letter, accidental, octave_text = match.groups()
    return Note(letter=letter, accidental=accidental, octave=int(octave_text))


def note_to_frequency(text: str) -> float:
    """Parse a note label and return its equal-tempered frequency in Hertz."""

    return parse_note(text).frequency()

