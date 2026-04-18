import pytest

from note_frequency import Note, note_to_frequency, parse_note


class TestParseNote:
    def test_parses_simple_note(self) -> None:
        note = parse_note("A4")
        assert note.letter == "A"
        assert note.accidental == ""
        assert note.octave == 4

    def test_parses_sharp(self) -> None:
        note = parse_note("C#5")
        assert note.letter == "C"
        assert note.accidental == "#"
        assert note.octave == 5

    def test_parses_flat(self) -> None:
        note = parse_note("Db3")
        assert note.letter == "D"
        assert note.accidental == "b"
        assert note.octave == 3

    def test_lowercase_letter_is_normalized(self) -> None:
        assert str(parse_note("g4")) == "G4"


class TestValidation:
    @pytest.mark.parametrize("value", ["", "A", "H4", "#4", "4A", "A##4", "Bb"])
    def test_invalid_note_strings_raise(self, value: str) -> None:
        with pytest.raises(ValueError, match="Invalid note"):
            parse_note(value)

    def test_unsupported_spelling_raises(self) -> None:
        with pytest.raises(ValueError, match="Unsupported note spelling"):
            Note(letter="E", accidental="#", octave=4)


class TestSemitoneOffsets:
    def test_a4_is_zero_semitones_from_a4(self) -> None:
        assert parse_note("A4").semitones_from_a4() == 0

    def test_a5_is_twelve_semitones_above_a4(self) -> None:
        assert parse_note("A5").semitones_from_a4() == 12

    def test_a3_is_twelve_semitones_below_a4(self) -> None:
        assert parse_note("A3").semitones_from_a4() == -12

    def test_c4_is_nine_semitones_below_a4(self) -> None:
        assert parse_note("C4").semitones_from_a4() == -9


class TestFrequencyMapping:
    def test_a4_is_440_hz(self) -> None:
        assert parse_note("A4").frequency() == pytest.approx(440.0, abs=1e-12)

    def test_a5_is_880_hz(self) -> None:
        assert parse_note("A5").frequency() == pytest.approx(880.0, abs=1e-12)

    def test_a3_is_220_hz(self) -> None:
        assert parse_note("A3").frequency() == pytest.approx(220.0, abs=1e-12)

    def test_middle_c_frequency(self) -> None:
        assert note_to_frequency("C4") == pytest.approx(261.6255653005986, rel=1e-12)

    def test_sharp_and_flat_spellings_match_same_pitch(self) -> None:
        assert note_to_frequency("C#4") == pytest.approx(note_to_frequency("Db4"), rel=1e-12)

    def test_note_to_frequency_wrapper(self) -> None:
        assert note_to_frequency("Bb3") == pytest.approx(parse_note("Bb3").frequency(), rel=1e-12)

