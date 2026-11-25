"""Hostile input testing for CWE-1333, CWE-400, CWE-20, and CWE-190 (VER15)."""

from __future__ import annotations

import math
from fractions import Fraction

import pytest

from note_frequency import (
    MAX_FREQUENCY_HZ,
    MAX_NOTE_STRING_LENGTH,
    MAX_OCTAVE,
    MAX_REFERENCE_FREQUENCY_HZ,
    MIN_FREQUENCY_HZ,
    MIN_OCTAVE,
    MIN_REFERENCE_FREQUENCY_HZ,
    Note,
    note_to_frequency,
    note_to_frequency_with_reference,
    parse_note,
    validate_frequency,
)


class TestCwe400InputLengthCaps:
    def test_oversized_note_string_rejected_before_parsing(self) -> None:
        for length in (MAX_NOTE_STRING_LENGTH + 1, 1024, 100_000):
            huge = "A" + "0" * (length - 2) + "4"
            with pytest.raises(ValueError, match="exceeds maximum"):
                parse_note(huge)
            with pytest.raises(ValueError, match="exceeds maximum"):
                note_to_frequency(huge)

    def test_exact_max_length_valid_note_accepted(self) -> None:
        # "C#" (2) + 29 zeros + "4" (1) == 32
        boundary = "C#" + "0" * (MAX_NOTE_STRING_LENGTH - 3) + "4"
        assert len(boundary) == MAX_NOTE_STRING_LENGTH
        note = parse_note(boundary)
        assert note.letter == "C"
        assert note.accidental == "#"
        assert note.octave == 4

    def test_empty_string_rejected(self) -> None:
        with pytest.raises(ValueError, match="Invalid note"):
            parse_note("")


class TestCwe1333LinearTimeParser:
    @pytest.mark.parametrize(
        "adversarial",
        [
            "A#" + "b" * 29,
            "A" + "0" * 30 + "!",
            "A-" + "9" * 29 + "x",
            "A-",
            "A#-",
            "A+4",
            " A4",
            "A4 ",
            "A4\n",
            "A\x004",
            "A٤",
            "İ4",
            "ß4",
            "C##4",
            "Dbb3",
        ],
    )
    def test_adversarial_strings_fail_closed_in_linear_time(
        self, adversarial: str
    ) -> None:
        with pytest.raises(ValueError, match="Invalid note"):
            parse_note(adversarial)


class TestCwe20TypeAndSpellingValidation:
    @pytest.mark.parametrize(
        "bad_input",
        [None, 440, 4.0, True, False, ["A4"], {"note": "A4"}, b"A4"],
    )
    def test_parse_note_rejects_non_string_types(self, bad_input: object) -> None:
        with pytest.raises(ValueError, match="Expected a string"):
            parse_note(bad_input)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "bad_octave",
        [
            True,
            False,
            4.0,
            4.5,
            float("nan"),
            float("inf"),
            float("-inf"),
            "4",
            None,
            complex(4, 0),
        ],
    )
    def test_note_rejects_bool_float_nan_inf_and_non_int_octave(
        self, bad_octave: object
    ) -> None:
        with pytest.raises(ValueError, match="strict int"):
            Note(letter="A", accidental="", octave=bad_octave)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        ("letter", "accidental"),
        [
            (123, ""),
            ("A", 1),
            ("A", True),
            (None, ""),
        ],
    )
    def test_note_rejects_non_string_letter_or_accidental(
        self, letter: object, accidental: object
    ) -> None:
        with pytest.raises(ValueError, match="must be strings"):
            Note(letter=letter, accidental=accidental, octave=4)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        ("letter", "accidental"),
        [
            ("", ""),
            ("C#", ""),
            ("AA", ""),
            ("İ", ""),
            ("A", "##"),
            ("A", "x"),
            ("H", ""),
            ("E", "#"),
            ("B", "#"),
            ("C", "b"),
            ("F", "b"),
        ],
    )
    def test_note_rejects_invalid_letter_accidental_and_spelling(
        self, letter: str, accidental: str
    ) -> None:
        with pytest.raises(ValueError, match="Unsupported note spelling"):
            Note(letter=letter, accidental=accidental, octave=4)

    @pytest.mark.parametrize(
        "bad_freq",
        [
            float("nan"),
            float("inf"),
            float("-inf"),
            0.0,
            -0.0,
            -440.0,
            True,
            False,
            "440.0",
            None,
            MIN_FREQUENCY_HZ / 10.0,
            MAX_FREQUENCY_HZ * 10.0,
        ],
    )
    def test_validate_frequency_rejects_non_finite_and_out_of_range(
        self, bad_freq: object
    ) -> None:
        with pytest.raises(ValueError):
            validate_frequency(bad_freq)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "bad_a4",
        [
            float("nan"),
            float("inf"),
            float("-inf"),
            0.0,
            -440.0,
            True,
            False,
            "440",
            MIN_REFERENCE_FREQUENCY_HZ - 0.1,
            MAX_REFERENCE_FREQUENCY_HZ + 1.0,
        ],
    )
    def test_reference_frequency_rejects_non_finite_and_out_of_range(
        self, bad_a4: object
    ) -> None:
        with pytest.raises(ValueError):
            note_to_frequency_with_reference("A4", a4_hz=bad_a4)  # type: ignore[arg-type]


class TestCwe190OctaveBounds:
    @pytest.mark.parametrize(
        "bad_octave",
        [MIN_OCTAVE - 1, MAX_OCTAVE + 1, 10_000, -10_000, 10**18, -(10**18)],
    )
    def test_note_rejects_out_of_bounds_octave(self, bad_octave: int) -> None:
        with pytest.raises(ValueError, match="out of bounds"):
            Note(letter="A", accidental="", octave=bad_octave)

    @pytest.mark.parametrize(
        "bad_text",
        [
            f"A{MAX_OCTAVE + 1}",
            f"A{MIN_OCTAVE - 1}",
            "A999999999",
            "A-999999999",
        ],
    )
    def test_parse_note_rejects_out_of_bounds_octave_without_overflow(
        self, bad_text: str
    ) -> None:
        with pytest.raises(ValueError, match="out of bounds"):
            parse_note(bad_text)

    def test_octave_boundaries_succeed_with_finite_positive_frequencies(self) -> None:
        low = parse_note(f"C{MIN_OCTAVE}")
        high = parse_note(f"B{MAX_OCTAVE}")
        low_hz = low.frequency()
        high_hz = high.frequency()
        assert math.isfinite(low_hz) and low_hz > 0.0
        assert math.isfinite(high_hz) and high_hz > 0.0
        assert low_hz < high_hz


class TestCwe400AndOverflowErrorHardening:
    @pytest.mark.parametrize(
        "huge_val",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_frequency_raises_value_error_not_overflow_error(
        self, huge_val: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            validate_frequency(huge_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            note_to_frequency_with_reference("A4", a4_hz=huge_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_octave",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_int_octave_fails_in_o1_without_int_max_str_digits(
        self, huge_octave: int
    ) -> None:
        with pytest.raises(
            ValueError, match="<out-of-range int> out of bounds"
        ) as exc_info:
            Note(letter="A", accidental="", octave=huge_octave)
        assert len(str(exc_info.value)) < 128

    def test_oversized_constructor_strings_do_not_amplify_error_messages(self) -> None:
        huge_letter = "A" * 100_000
        with pytest.raises(ValueError) as exc_letter:
            Note(letter=huge_letter, accidental="", octave=4)
        assert len(str(exc_letter.value)) < 128

        huge_accidental = "#" * 100_000
        with pytest.raises(ValueError, match="exceeds maximum") as exc_acc:
            Note(letter="A", accidental=huge_accidental, octave=4)
        assert len(str(exc_acc.value)) < 128

        with pytest.raises(ValueError, match="<list>") as exc_list:
            parse_note(["A4"] * 1000)  # type: ignore[arg-type]
        assert len(str(exc_list.value)) < 128
