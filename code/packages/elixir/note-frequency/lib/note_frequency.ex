defmodule NoteFrequency.Note do
  @enforce_keys [:letter, :accidental, :octave]
  defstruct [:letter, :accidental, :octave]

  @chromatic_index %{
    "C" => 0,
    "C#" => 1,
    "Db" => 1,
    "D" => 2,
    "D#" => 3,
    "Eb" => 3,
    "E" => 4,
    "F" => 5,
    "F#" => 6,
    "Gb" => 6,
    "G" => 7,
    "G#" => 8,
    "Ab" => 8,
    "A" => 9,
    "A#" => 10,
    "Bb" => 10,
    "B" => 11
  }
  @reference_octave 4
  @reference_index Map.fetch!(@chromatic_index, "A")
  @reference_frequency_hz 440.0
  @semitones_per_octave 12

  def new!(letter, accidental, octave) do
    canonical_letter = String.upcase(letter)
    spelling = canonical_letter <> accidental

    unless Map.has_key?(@chromatic_index, spelling) do
      raise ArgumentError,
            "Unsupported note spelling #{inspect(spelling)}. " <>
              "Only natural notes plus single # or b accidentals are supported."
    end

    %__MODULE__{letter: canonical_letter, accidental: accidental, octave: octave}
  end

  def spelling(note), do: note.letter <> note.accidental
  def chromatic_index(note), do: Map.fetch!(@chromatic_index, spelling(note))

  def semitones_from_a4(note) do
    octave_offset = (note.octave - @reference_octave) * @semitones_per_octave
    pitch_offset = chromatic_index(note) - @reference_index
    octave_offset + pitch_offset
  end

  def frequency(note) do
    @reference_frequency_hz * :math.pow(2.0, semitones_from_a4(note) / @semitones_per_octave)
  end

  def to_string(note), do: spelling(note) <> Integer.to_string(note.octave)
end

defmodule NoteFrequency do
  alias NoteFrequency.Note
  @note_pattern ~r/^([A-Ga-g])([#b]?)(-?\d+)$/

  def parse_note(text) when is_binary(text) do
    case Regex.run(@note_pattern, text) do
      [_, letter, accidental, octave_text] ->
        Note.new!(letter, accidental, String.to_integer(octave_text))

      nil ->
        raise ArgumentError,
              "Invalid note #{inspect(text)}. Expected <letter><optional # or b><octave>, " <>
                "for example 'A4', 'C#5', or 'Db3'."
    end
  end

  def note_to_frequency(text) do
    text |> parse_note() |> Note.frequency()
  end
end
