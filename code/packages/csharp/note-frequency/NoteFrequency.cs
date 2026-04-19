using System.Text.RegularExpressions;

namespace CodingAdventures.NoteFrequency;

/// <summary>
/// A parsed musical note label such as A4, C#5, or Db3.
/// </summary>
public sealed class Note
{
    private static readonly IReadOnlyDictionary<string, int> ChromaticIndex = new Dictionary<string, int>
    {
        ["C"] = 0,
        ["C#"] = 1,
        ["Db"] = 1,
        ["D"] = 2,
        ["D#"] = 3,
        ["Eb"] = 3,
        ["E"] = 4,
        ["F"] = 5,
        ["F#"] = 6,
        ["Gb"] = 6,
        ["G"] = 7,
        ["G#"] = 8,
        ["Ab"] = 8,
        ["A"] = 9,
        ["A#"] = 10,
        ["Bb"] = 10,
        ["B"] = 11,
    };

    private const int ReferenceOctave = 4;
    private const int ReferenceIndex = 9;
    private const double ReferenceFrequencyHz = 440.0;
    private const int SemitonesPerOctave = 12;

    /// <summary>
    /// Creates a note from a letter, optional accidental, and octave number.
    /// </summary>
    public Note(string letter, string accidental, int octave)
    {
        Letter = letter.ToUpperInvariant();
        Accidental = accidental;
        Octave = octave;

        if (!ChromaticIndex.ContainsKey(Spelling))
        {
            throw new ArgumentException(
                $"Unsupported note spelling {Spelling}. Only natural notes plus single # or b accidentals are supported.",
                nameof(letter));
        }
    }

    /// <summary>The uppercase note letter, from A through G.</summary>
    public string Letter { get; }

    /// <summary>The optional accidental spelling: empty, #, or b.</summary>
    public string Accidental { get; }

    /// <summary>The scientific pitch notation octave number.</summary>
    public int Octave { get; }

    /// <summary>The note spelling without its octave, such as C# or Db.</summary>
    public string Spelling => Letter + Accidental;

    /// <summary>The note's semitone index within an octave where C is 0.</summary>
    public int ChromaticIndexValue => ChromaticIndex[Spelling];

    /// <summary>
    /// Returns the signed semitone distance from the reference pitch A4.
    /// </summary>
    public int SemitonesFromA4()
    {
        var octaveOffset = (Octave - ReferenceOctave) * SemitonesPerOctave;
        var pitchOffset = ChromaticIndexValue - ReferenceIndex;
        return octaveOffset + pitchOffset;
    }

    /// <summary>Returns this note's equal-tempered frequency in Hertz.</summary>
    public double Frequency() => ReferenceFrequencyHz * Math.Pow(2.0, SemitonesFromA4() / (double)SemitonesPerOctave);

    /// <summary>Returns the normalized note label, such as A4 or C#5.</summary>
    public override string ToString() => $"{Spelling}{Octave}";
}

/// <summary>
/// Parses note labels and maps them to equal-tempered frequencies.
/// </summary>
public static class NoteFrequency
{
    private static readonly Regex NotePattern = new(@"^([A-Ga-g])([#b]?)(-?\d+)$", RegexOptions.Compiled);

    /// <summary>
    /// Parses a label like A4, C#5, or Db3 into a structured note.
    /// </summary>
    public static Note ParseNote(string text)
    {
        var match = NotePattern.Match(text);
        if (!match.Success)
        {
            throw new ArgumentException(
                $"Invalid note {text}. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.",
                nameof(text));
        }
        return new Note(match.Groups[1].Value, match.Groups[2].Value, int.Parse(match.Groups[3].Value));
    }

    /// <summary>
    /// Parses a note label and returns its equal-tempered frequency in Hertz.
    /// </summary>
    public static double NoteToFrequency(string text) => ParseNote(text).Frequency();
}
