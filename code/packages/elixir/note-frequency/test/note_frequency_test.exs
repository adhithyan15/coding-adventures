defmodule NoteFrequencyTest do
  use ExUnit.Case, async: true

  alias NoteFrequency.Note

  test "parse_note extracts the note fields" do
    note = NoteFrequency.parse_note("C#5")
    assert note.letter == "C"
    assert note.accidental == "#"
    assert note.octave == 5
  end

  test "lowercase letters are normalized" do
    assert NoteFrequency.parse_note("g4") |> Note.to_string() == "G4"
  end

  test "malformed notes raise" do
    for value <- ["", "A", "H4", "#4", "4A", "A##4", "Bb"] do
      assert_raise ArgumentError, ~r/Invalid note/, fn ->
        NoteFrequency.parse_note(value)
      end
    end
  end

  test "unsupported spellings raise" do
    assert_raise ArgumentError, ~r/Unsupported note spelling/, fn ->
      Note.new!("E", "#", 4)
    end
  end

  test "semitone offsets match the reference examples" do
    assert NoteFrequency.parse_note("A4") |> Note.semitones_from_a4() == 0
    assert NoteFrequency.parse_note("A5") |> Note.semitones_from_a4() == 12
    assert NoteFrequency.parse_note("A3") |> Note.semitones_from_a4() == -12
    assert NoteFrequency.parse_note("C4") |> Note.semitones_from_a4() == -9
  end

  test "frequencies match the reference examples" do
    assert_in_delta NoteFrequency.parse_note("A4") |> Note.frequency(), 440.0, 1.0e-12
    assert_in_delta NoteFrequency.parse_note("A5") |> Note.frequency(), 880.0, 1.0e-12
    assert_in_delta NoteFrequency.parse_note("A3") |> Note.frequency(), 220.0, 1.0e-12
    assert_in_delta NoteFrequency.note_to_frequency("C4"), 261.6255653005986, 1.0e-12
    assert_in_delta NoteFrequency.note_to_frequency("C#4"), NoteFrequency.note_to_frequency("Db4"), 1.0e-12
  end
end
