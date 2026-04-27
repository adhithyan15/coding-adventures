namespace CodingAdventures.NoteFrequency.Tests

open CodingAdventures.NoteFrequency
open Xunit

module NoteFrequencyTests =
    [<Fact>]
    let ``parseNote extracts fields`` () =
        let note = parseNote "C#5"
        Assert.Equal("C", note.Letter)
        Assert.Equal("#", note.Accidental)
        Assert.Equal(5, note.Octave)

    [<Fact>]
    let ``lowercase letters are normalized`` () =
        Assert.Equal("G4", parseNote("g4").ToString())

    [<Fact>]
    let ``malformed notes throw`` () =
        for value in [ ""; "A"; "H4"; "#4"; "4A"; "A##4"; "Bb" ] do
            Assert.Throws<System.ArgumentException>(fun () -> parseNote value |> ignore) |> ignore

    [<Fact>]
    let ``unsupported spellings throw`` () =
        Assert.Throws<System.ArgumentException>(fun () -> createNote "E" "#" 4 |> ignore) |> ignore

    [<Fact>]
    let ``chromatic index covers the supported spellings`` () =
        Assert.Equal(0, chromaticIndex "C")
        Assert.Equal(1, chromaticIndex "C#")
        Assert.Equal(1, chromaticIndex "Db")
        Assert.Equal(2, chromaticIndex "D")
        Assert.Equal(3, chromaticIndex "D#")
        Assert.Equal(3, chromaticIndex "Eb")
        Assert.Equal(4, chromaticIndex "E")
        Assert.Equal(5, chromaticIndex "F")
        Assert.Equal(6, chromaticIndex "F#")
        Assert.Equal(6, chromaticIndex "Gb")
        Assert.Equal(7, chromaticIndex "G")
        Assert.Equal(8, chromaticIndex "G#")
        Assert.Equal(8, chromaticIndex "Ab")
        Assert.Equal(9, chromaticIndex "A")
        Assert.Equal(10, chromaticIndex "A#")
        Assert.Equal(10, chromaticIndex "Bb")
        Assert.Equal(11, chromaticIndex "B")

    [<Fact>]
    let ``semitone offsets match examples`` () =
        Assert.Equal(0, parseNote("A4").SemitonesFromA4())
        Assert.Equal(12, parseNote("A5").SemitonesFromA4())
        Assert.Equal(-12, parseNote("A3").SemitonesFromA4())
        Assert.Equal(-9, parseNote("C4").SemitonesFromA4())

    [<Fact>]
    let ``frequencies match examples`` () =
        Assert.Equal(440.0, parseNote("A4").Frequency(), 12)
        Assert.Equal(880.0, parseNote("A5").Frequency(), 12)
        Assert.Equal(220.0, parseNote("A3").Frequency(), 12)
        Assert.Equal(261.6255653005986, noteToFrequency("C4"), 12)
        Assert.Equal(noteToFrequency("C#4"), noteToFrequency("Db4"), 12)
