# frozen_string_literal: true

module NoteFrequency
  NOTE_PATTERN = /\A([A-Ga-g])([#b]?)(-?\d+)\z/
  CHROMATIC_INDEX = {
    'C' => 0, 'C#' => 1, 'Db' => 1, 'D' => 2, 'D#' => 3, 'Eb' => 3,
    'E' => 4, 'F' => 5, 'F#' => 6, 'Gb' => 6, 'G' => 7, 'G#' => 8,
    'Ab' => 8, 'A' => 9, 'A#' => 10, 'Bb' => 10, 'B' => 11
  }.freeze
  REFERENCE_OCTAVE = 4
  REFERENCE_INDEX = CHROMATIC_INDEX['A']
  REFERENCE_FREQUENCY_HZ = 440.0
  SEMITONES_PER_OCTAVE = 12

  class Note
    attr_reader :letter, :accidental, :octave

    def initialize(letter:, accidental:, octave:)
      @letter = letter.upcase
      @accidental = accidental
      @octave = Integer(octave)
      return if CHROMATIC_INDEX.key?(spelling)

      raise ArgumentError,
            "Unsupported note spelling #{spelling.inspect}. "             'Only natural notes plus single # or b accidentals are supported.'
    end

    def spelling
      "#{@letter}#{@accidental}"
    end

    def chromatic_index
      CHROMATIC_INDEX.fetch(spelling)
    end

    def semitones_from_a4
      octave_offset = (@octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE
      pitch_offset = chromatic_index - REFERENCE_INDEX
      octave_offset + pitch_offset
    end

    def frequency
      REFERENCE_FREQUENCY_HZ * (2**(semitones_from_a4.to_f / SEMITONES_PER_OCTAVE))
    end

    def to_s
      "#{spelling}#{@octave}"
    end
  end

  def self.parse_note(text)
    match = NOTE_PATTERN.match(text)
    raise ArgumentError, invalid_note_message(text) unless match

    Note.new(letter: match[1], accidental: match[2], octave: match[3])
  end

  def self.note_to_frequency(text)
    parse_note(text).frequency
  end

  def self.invalid_note_message(text)
    "Invalid note #{text.inspect}. Expected <letter><optional # or b><octave>, "       "for example 'A4', 'C#5', or 'Db3'."
  end
  private_class_method :invalid_note_message
end
