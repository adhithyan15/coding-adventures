module CodingAdventures.NoteFrequency

open System
open System.Text.RegularExpressions

let private chromaticIndexFor spelling =
    match spelling with
    | "C" -> 0
    | "C#" | "Db" -> 1
    | "D" -> 2
    | "D#" | "Eb" -> 3
    | "E" -> 4
    | "F" -> 5
    | "F#" | "Gb" -> 6
    | "G" -> 7
    | "G#" | "Ab" -> 8
    | "A" -> 9
    | "A#" | "Bb" -> 10
    | "B" -> 11
    | _ -> invalidArg "spelling" (sprintf "Unsupported note spelling %s. Only natural notes plus single # or b accidentals are supported." spelling)

[<CLIMutable>]
type Note =
    { Letter: string
      Accidental: string
      Octave: int }
    member this.Spelling = this.Letter + this.Accidental
    member this.ChromaticIndex = chromaticIndexFor this.Spelling
    member this.SemitonesFromA4() =
        let octaveOffset = (this.Octave - 4) * 12
        let pitchOffset = this.ChromaticIndex - 9
        octaveOffset + pitchOffset
    member this.Frequency() = 440.0 * Math.Pow(2.0, float (this.SemitonesFromA4()) / 12.0)
    override this.ToString() = this.Spelling + string this.Octave

let private notePattern = Regex("^([A-Ga-g])([#b]?)(-?\d+)$", RegexOptions.Compiled)

let chromaticIndex spelling = chromaticIndexFor spelling

let createNote (letter: string) (accidental: string) (octave: int) =
    let canonicalLetter = letter.ToUpperInvariant()
    let spelling = canonicalLetter + accidental
    let _ = chromaticIndexFor spelling
    { Letter = canonicalLetter; Accidental = accidental; Octave = octave }

let parseNote (text: string) =
    let matchResult = notePattern.Match(text)
    if not matchResult.Success then
        invalidArg "text" (sprintf "Invalid note %s. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'." text)

    createNote matchResult.Groups[1].Value matchResult.Groups[2].Value (int matchResult.Groups[3].Value)

let noteToFrequency (text: string) = parseNote text |> fun note -> note.Frequency()
