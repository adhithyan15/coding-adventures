"""
Note parsing and equal-tempered pitch mapping (MUS00 / VER15).

This module turns note labels such as ``A4`` and ``C#5`` into a structured
:class:`Note` object and then into a frequency in Hertz.

The core musical reference point is:

    A4 = 440 Hz

From there, each semitone step changes frequency by a factor of ``2 ** (1/12)``:

    f(n) = 440 * 2 ** (n / 12)

Security & VER15 hardening:
- **CWE-1333 (ReDoS elimination)**: Zero regular expressions; note parsing uses
  a strict, single-pass, linear-time character-by-character scanner.
- **CWE-400 (Resource Exhaustion)**: Input strings are capped at
  ``MAX_NOTE_STRING_LENGTH = 32`` characters before any parsing or digit
  conversion, and octaves are bounded to ``[MIN_OCTAVE, MAX_OCTAVE]``.
- **CWE-20 / CWE-190 (Input Validation & Numeric Bounds)**: Rejects ``bool``,
  ``float``, ``NaN``, ``±Inf``, and non-ASCII digits; prevents ``OverflowError``
  or unbounded exponentiation by enforcing ``octave in [-100, 100]``.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import isfinite
from numbers import Real

MAX_NOTE_STRING_LENGTH: int = 32
MIN_OCTAVE: int = -100
MAX_OCTAVE: int = 100

MIN_FREQUENCY_HZ: float = 1e-30
MAX_FREQUENCY_HZ: float = 1e32
MIN_REFERENCE_FREQUENCY_HZ: float = 1.0
MAX_REFERENCE_FREQUENCY_HZ: float = 100_000.0

VALID_LETTERS: frozenset[str] = frozenset({"A", "B", "C", "D", "E", "F", "G"})
VALID_ACCIDENTALS: frozenset[str] = frozenset({"", "#", "b"})

CHROMATIC_INDEX: dict[str, int] = {
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

REFERENCE_OCTAVE: int = 4
REFERENCE_INDEX: int = CHROMATIC_INDEX["A"]
REFERENCE_FREQUENCY_HZ: float = 440.0
SEMITONES_PER_OCTAVE: int = 12


def _safe_repr(value: object) -> str:
    """Bounded O(1) representation for error messages (CWE-400)."""
    if isinstance(value, bool):
        return repr(value)
    if isinstance(value, int):
        if value < -1_000_000 or value > 1_000_000:
            return "<out-of-range int>"
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        if len(value) > MAX_NOTE_STRING_LENGTH:
            return f"<str len={len(value)}>"
        return repr(value)
    return f"<{type(value).__name__}>"


def _require_strict_int(value: object, name: str) -> int:
    """Validate that ``value`` is a strict ``int`` (rejecting ``bool`` and floats)."""
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError(
            f"{name} must be a strict int (not bool or float), got {_safe_repr(value)}."
        )
    return value


def _require_finite_frequency(
    value: object,
    name: str,
    min_hz: float,
    max_hz: float,
) -> float:
    """Validate that ``value`` is a finite real frequency in ``[min_hz, max_hz]``."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(
            f"{name} must be a finite real number (not bool), got {_safe_repr(value)}."
        )
    try:
        freq = float(value)
    except OverflowError as exc:
        raise ValueError(
            f"{name} overflows float range: {_safe_repr(value)}."
        ) from exc
    if not isfinite(freq):
        raise ValueError(
            f"{name} must be finite (not NaN or Inf), got {_safe_repr(value)}."
        )
    if freq < min_hz or freq > max_hz:
        raise ValueError(
            f"{name} ({freq}) out of valid range [{min_hz}, {max_hz}]."
        )
    return freq


def validate_frequency(freq_hz: float) -> float:
    """Validate that a frequency in Hertz is finite and within physical bounds."""
    return _require_finite_frequency(
        freq_hz,
        "frequency_hz",
        MIN_FREQUENCY_HZ,
        MAX_FREQUENCY_HZ,
    )


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
        if not isinstance(self.letter, str) or not isinstance(self.accidental, str):
            raise ValueError(
                "Note letter and accidental must be strings, "
                f"got letter={_safe_repr(self.letter)}, "
                f"accidental={_safe_repr(self.accidental)}."
            )

        if len(self.letter) != 1 or not self.letter.isascii():
            raise ValueError(
                f"Unsupported note spelling {_safe_repr(self.letter)}. "
                "Letter must be a single ASCII note letter in A..G."
            )

        if self.accidental not in VALID_ACCIDENTALS:
            if len(self.accidental) > MAX_NOTE_STRING_LENGTH:
                raise ValueError(
                    "Unsupported note spelling: accidental length "
                    f"{len(self.accidental)} exceeds maximum {MAX_NOTE_STRING_LENGTH}."
                )
            raise ValueError(
                f"Unsupported note spelling {self.letter + self.accidental!r}. "
                "Only natural notes plus single # or b accidentals are supported."
            )

        canonical_letter = self.letter.upper()
        canonical_accidental = self.accidental
        spelling = canonical_letter + canonical_accidental
        if canonical_letter not in VALID_LETTERS or spelling not in CHROMATIC_INDEX:
            raise ValueError(
                f"Unsupported note spelling {spelling!r}. "
                "Only natural notes plus single # or b accidentals are supported."
            )

        validated_octave = _require_strict_int(self.octave, "octave")
        if validated_octave < MIN_OCTAVE or validated_octave > MAX_OCTAVE:
            raise ValueError(
                f"Octave {_safe_repr(validated_octave)} out of bounds "
                f"[{MIN_OCTAVE}, {MAX_OCTAVE}]."
            )

        object.__setattr__(self, "letter", canonical_letter)
        object.__setattr__(self, "accidental", canonical_accidental)
        object.__setattr__(self, "octave", validated_octave)

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
        """Return the equal-tempered frequency in Hertz (anchored at A4 = 440 Hz)."""
        return self.frequency_with_reference(REFERENCE_FREQUENCY_HZ)

    def frequency_with_reference(self, a4_hz: float = REFERENCE_FREQUENCY_HZ) -> float:
        """Return equal-tempered frequency using a validated A4 reference pitch."""
        ref_hz = _require_finite_frequency(
            a4_hz,
            "a4_hz",
            MIN_REFERENCE_FREQUENCY_HZ,
            MAX_REFERENCE_FREQUENCY_HZ,
        )
        semitone_offset = self.semitones_from_a4()
        freq = ref_hz * (2.0 ** (semitone_offset / SEMITONES_PER_OCTAVE))
        return validate_frequency(freq)

    def __str__(self) -> str:
        return f"{self.spelling}{self.octave}"


def _is_ascii_digit(ch: str) -> bool:
    return "0" <= ch <= "9"


def parse_note(text: str) -> Note:
    """
    Parse a note label like ``A4`` or ``C#5`` into a :class:`Note`.

    Uses a deterministic O(N) character-by-character scan with a strict
    ``MAX_NOTE_STRING_LENGTH`` ceiling (no regex, immune to CWE-1333 / CWE-400).
    """
    if not isinstance(text, str):
        raise ValueError(f"Invalid note {_safe_repr(text)}. Expected a string.")

    length = len(text)
    if length == 0:
        raise ValueError(
            "Invalid note ''. Expected <letter><optional # or b><octave>, "
            "for example 'A4', 'C#5', or 'Db3'."
        )
    if length > MAX_NOTE_STRING_LENGTH:
        raise ValueError(
            f"Invalid note: input length {length} exceeds maximum "
            f"{MAX_NOTE_STRING_LENGTH}."
        )

    pos = 0
    raw_letter = text[pos]
    if not raw_letter.isascii() or raw_letter.upper() not in VALID_LETTERS:
        raise ValueError(
            f"Invalid note {text!r}. Expected <letter><optional # or b><octave>, "
            "for example 'A4', 'C#5', or 'Db3'."
        )
    pos += 1

    accidental = ""
    if pos < length and text[pos] in ("#", "b"):
        accidental = text[pos]
        pos += 1

    if pos >= length:
        raise ValueError(
            f"Invalid note {text!r}. Expected <letter><optional # or b><octave>, "
            "for example 'A4', 'C#5', or 'Db3'."
        )

    octave_start = pos
    if text[pos] == "-":
        pos += 1

    if pos >= length:
        raise ValueError(
            f"Invalid note {text!r}. Expected <letter><optional # or b><octave>, "
            "for example 'A4', 'C#5', or 'Db3'."
        )

    while pos < length:
        if not _is_ascii_digit(text[pos]):
            raise ValueError(
                f"Invalid note {text!r}. Expected <letter><optional # or b><octave>, "
                "for example 'A4', 'C#5', or 'Db3'."
            )
        pos += 1

    octave = int(text[octave_start:])
    return Note(letter=raw_letter, accidental=accidental, octave=octave)


def note_to_frequency(text: str) -> float:
    """Parse a note label and return its equal-tempered frequency in Hertz."""
    return parse_note(text).frequency()


def note_to_frequency_with_reference(
    text: str,
    a4_hz: float = REFERENCE_FREQUENCY_HZ,
) -> float:
    """Parse a note label and return its frequency relative to ``a4_hz``."""
    return parse_note(text).frequency_with_reference(a4_hz)
