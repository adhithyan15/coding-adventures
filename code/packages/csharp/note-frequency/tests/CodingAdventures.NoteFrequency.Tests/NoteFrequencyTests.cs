using CodingAdventures.NoteFrequency;

namespace CodingAdventures.NoteFrequency.Tests;

public class NoteFrequencyTests
{
    [Fact]
    public void ParseNoteExtractsFields()
    {
        var note = NoteFrequency.ParseNote("C#5");
        Assert.Equal("C", note.Letter);
        Assert.Equal("#", note.Accidental);
        Assert.Equal(5, note.Octave);
    }

    [Fact]
    public void LowercaseLettersAreNormalized()
    {
        Assert.Equal("G4", NoteFrequency.ParseNote("g4").ToString());
    }

    [Fact]
    public void MalformedNotesThrow()
    {
        foreach (var value in new[] { "", "A", "H4", "#4", "4A", "A##4", "Bb" })
        {
            Assert.Throws<ArgumentException>(() => NoteFrequency.ParseNote(value));
        }
    }

    [Fact]
    public void UnsupportedSpellingsThrow()
    {
        Assert.Throws<ArgumentException>(() => new Note("E", "#", 4));
    }

    [Fact]
    public void SemitoneOffsetsMatchExamples()
    {
        Assert.Equal(0, NoteFrequency.ParseNote("A4").SemitonesFromA4());
        Assert.Equal(12, NoteFrequency.ParseNote("A5").SemitonesFromA4());
        Assert.Equal(-12, NoteFrequency.ParseNote("A3").SemitonesFromA4());
        Assert.Equal(-9, NoteFrequency.ParseNote("C4").SemitonesFromA4());
    }

    [Fact]
    public void FrequenciesMatchExamples()
    {
        Assert.Equal(440.0, NoteFrequency.ParseNote("A4").Frequency(), 12);
        Assert.Equal(880.0, NoteFrequency.ParseNote("A5").Frequency(), 12);
        Assert.Equal(220.0, NoteFrequency.ParseNote("A3").Frequency(), 12);
        Assert.Equal(261.6255653005986, NoteFrequency.NoteToFrequency("C4"), 12);
        Assert.Equal(NoteFrequency.NoteToFrequency("C#4"), NoteFrequency.NoteToFrequency("Db4"), 12);
    }
}
