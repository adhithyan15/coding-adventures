"""Property-based testing for note_frequency using SplitMix64 PRNG (MUS00, VER15)."""

from __future__ import annotations

import math

from note_frequency import (
    MAX_OCTAVE,
    MIN_OCTAVE,
    Note,
    note_to_frequency,
    note_to_frequency_with_reference,
    parse_note,
)

try:
    from coding_adventures_test_bootstrap.prng import SplitMix64
except ImportError:

    class SplitMix64:  # type: ignore[no-redef]
        """Minimal fallback SplitMix64 PRNG for isolated wheel test environments."""

        def __init__(self, seed: int = 0) -> None:
            self._state = seed & 0xFFFF_FFFF_FFFF_FFFF

        def next_u64(self) -> int:
            self._state = (self._state + 0x9E37_79B9_7F4A_7C15) & 0xFFFF_FFFF_FFFF_FFFF
            z = self._state
            z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & 0xFFFF_FFFF_FFFF_FFFF
            z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & 0xFFFF_FFFF_FFFF_FFFF
            return (z ^ (z >> 31)) & 0xFFFF_FFFF_FFFF_FFFF

        def bounded(self, n: int) -> int:
            threshold = (-n) % n
            while True:
                x = self.next_u64()
                if x >= threshold:
                    return x % n


VALID_SPELLINGS: tuple[tuple[str, str], ...] = (
    ("C", ""),
    ("C", "#"),
    ("D", "b"),
    ("D", ""),
    ("D", "#"),
    ("E", "b"),
    ("E", ""),
    ("F", ""),
    ("F", "#"),
    ("G", "b"),
    ("G", ""),
    ("G", "#"),
    ("A", "b"),
    ("A", ""),
    ("A", "#"),
    ("B", "b"),
    ("B", ""),
)

ENHARMONIC_PAIRS: tuple[tuple[tuple[str, str], tuple[str, str]], ...] = (
    (("C", "#"), ("D", "b")),
    (("D", "#"), ("E", "b")),
    (("F", "#"), ("G", "b")),
    (("G", "#"), ("A", "b")),
    (("A", "#"), ("B", "b")),
)


def test_pbt_octave_doubling_and_semitone_linearity() -> None:
    rng = SplitMix64(0xA440_1234_5678_9ABC)
    span = MAX_OCTAVE - MIN_OCTAVE  # [MIN_OCTAVE, MAX_OCTAVE - 1]
    for _ in range(250):
        letter, accidental = VALID_SPELLINGS[rng.bounded(len(VALID_SPELLINGS))]
        octave = MIN_OCTAVE + rng.bounded(span)

        lower = Note(letter=letter, accidental=accidental, octave=octave)
        upper = Note(letter=letter, accidental=accidental, octave=octave + 1)

        assert upper.semitones_from_a4() - lower.semitones_from_a4() == 12
        assert math.isclose(
            upper.frequency() / lower.frequency(),
            2.0,
            rel_tol=1e-12,
            abs_tol=0.0,
        )


def test_pbt_enharmonic_equivalence_and_roundtrip() -> None:
    rng = SplitMix64(0xCAFE_BABE_DEAD_BEEF)
    octave_count = MAX_OCTAVE - MIN_OCTAVE + 1
    for _ in range(250):
        (l1, a1), (l2, a2) = ENHARMONIC_PAIRS[rng.bounded(len(ENHARMONIC_PAIRS))]
        octave = MIN_OCTAVE + rng.bounded(octave_count)

        sharp = Note(letter=l1, accidental=a1, octave=octave)
        flat = Note(letter=l2, accidental=a2, octave=octave)

        assert sharp.chromatic_index == flat.chromatic_index
        assert sharp.semitones_from_a4() == flat.semitones_from_a4()
        assert math.isclose(sharp.frequency(), flat.frequency(), rel_tol=1e-12)

        reparsed_sharp = parse_note(str(sharp))
        assert reparsed_sharp == sharp
        assert math.isclose(
            note_to_frequency(str(sharp).lower()),
            sharp.frequency(),
            rel_tol=1e-12,
        )


def test_pbt_reference_pitch_scaling() -> None:
    rng = SplitMix64(0x1357_2468_ACE0_BDF1)
    for _ in range(150):
        letter, accidental = VALID_SPELLINGS[rng.bounded(len(VALID_SPELLINGS))]
        octave = -20 + rng.bounded(41)
        label = f"{letter}{accidental}{octave}"
        a4_hz = 220.0 + (rng.next_u64() & 0xFFFF) * (660.0 / 65535.0)

        f_std = note_to_frequency(label)
        f_alt = note_to_frequency_with_reference(label, a4_hz=a4_hz)
        assert math.isclose(f_alt / f_std, a4_hz / 440.0, rel_tol=1e-12)


def test_pbt_hostile_random_strings_fail_closed() -> None:
    rng = SplitMix64(0xFEED_FACE_0102_0304)
    alphabet = "ABCDEFGHabcdefg#b-+0123456789xyz \t\n\x00"
    for _ in range(500):
        length = rng.bounded(48)
        candidate = "".join(
            alphabet[rng.bounded(len(alphabet))] for _ in range(length)
        )
        try:
            note = parse_note(candidate)
            freq = note.frequency()
            assert math.isfinite(freq) and freq > 0.0
            assert MIN_OCTAVE <= note.octave <= MAX_OCTAVE
        except ValueError:
            pass
