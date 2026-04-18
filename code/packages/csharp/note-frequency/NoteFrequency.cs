using System.Text.RegularExpressions;

namespace CodingAdventures.NoteFrequency;

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

    public string Letter { get; }
    public string Accidental { get; }
    public int Octave { get; }
    public string Spelling => Letter + Accidental;
    public int ChromaticIndexValue => ChromaticIndex[Spelling];

    public int SemitonesFromA4()
    {
        var octaveOffset = (Octave - ReferenceOctave) * SemitonesPerOctave;
        var pitchOffset = ChromaticIndexValue - ReferenceIndex;
        return octaveOffset + pitchOffset;
    }

    public double Frequency() => ReferenceFrequencyHz * Math.Pow(2.0, SemitonesFromA4() / (double)SemitonesPerOctave);
    public override string ToString() => $"{Spelling}{Octave}";
}

public static class NoteFrequency
{
    private static readonly Regex NotePattern = new(@"^([A-Ga-g])([#b]?)(-?\d+)$", RegexOptions.Compiled);

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

    public static double NoteToFrequency(string text) => ParseNote(text).Frequency();
}
