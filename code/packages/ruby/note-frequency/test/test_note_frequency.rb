# frozen_string_literal: true

require 'minitest/autorun'
require_relative '../lib/note_frequency'

class NoteFrequencyTest < Minitest::Test
  def test_parse_note
    note = NoteFrequency.parse_note('C#5')
    assert_equal 'C', note.letter
    assert_equal '#', note.accidental
    assert_equal 5, note.octave
  end

  def test_lowercase_note_is_normalized
    assert_equal 'G4', NoteFrequency.parse_note('g4').to_s
  end

  def test_invalid_note_strings_raise
    ['', 'A', 'H4', '#4', '4A', 'A##4', 'Bb'].each do |value|
      error = assert_raises(ArgumentError) { NoteFrequency.parse_note(value) }
      assert_match(/Invalid note/, error.message)
    end
  end

  def test_unsupported_spelling_raises
    error = assert_raises(ArgumentError) { NoteFrequency::Note.new(letter: 'E', accidental: '#', octave: 4) }
    assert_match(/Unsupported note spelling/, error.message)
  end

  def test_semitones_from_a4
    assert_equal 0, NoteFrequency.parse_note('A4').semitones_from_a4
    assert_equal 12, NoteFrequency.parse_note('A5').semitones_from_a4
    assert_equal(-12, NoteFrequency.parse_note('A3').semitones_from_a4)
    assert_equal(-9, NoteFrequency.parse_note('C4').semitones_from_a4)
  end

  def test_frequency_mapping
    assert_in_delta 440.0, NoteFrequency.parse_note('A4').frequency, 1e-12
    assert_in_delta 880.0, NoteFrequency.parse_note('A5').frequency, 1e-12
    assert_in_delta 220.0, NoteFrequency.parse_note('A3').frequency, 1e-12
  end

  def test_middle_c_frequency
    assert_in_delta 261.6255653005986, NoteFrequency.note_to_frequency('C4'), 1e-12
  end

  def test_enharmonic_spellings_match
    assert_in_delta NoteFrequency.note_to_frequency('C#4'), NoteFrequency.note_to_frequency('Db4'), 1e-12
  end
end
