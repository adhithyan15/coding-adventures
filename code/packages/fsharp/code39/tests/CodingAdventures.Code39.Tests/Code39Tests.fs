namespace CodingAdventures.Code39.Tests

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.Code39.FSharp
open Xunit

module Code39Tests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Code39.VERSION)

    [<Fact>]
    let ``normalizes supported input`` () =
        Assert.Equal("ABC-123", Code39.normalizeCode39 "abc-123")

    [<Fact>]
    let ``encodes character and full sequence`` () =
        let encoded = Code39.encodeCode39Char "A"

        Assert.Equal("WNNNNWNNW", encoded.Pattern)
        Assert.False(encoded.IsStartStop)
        Assert.Equal<string list>([ "*"; "A"; "*" ], Code39.encodeCode39 "A" |> List.map _.Char)

    [<Fact>]
    let ``expand runs includes start stop and gaps`` () =
        let runs = Code39.expandCode39Runs "A"

        Assert.Equal(29, runs.Length)
        Assert.Equal(Bar, runs[0].Color)
        Assert.Equal(Start, runs[0].Role)
        Assert.Equal(InterCharacterGap, runs[9].Role)
        Assert.Equal(3u, runs[10].Modules)
        Assert.Equal(Stop, runs[runs.Length - 1].Role)

    [<Fact>]
    let ``layout builds paint scene metadata`` () =
        let scene = Code39.drawCode39 "A" None

        Assert.Equal(box "code39", scene.Metadata.Value["symbology"])
        Assert.Equal(box "A", scene.Metadata.Value["encodedText"])
        Assert.Equal(box "Code 39 barcode for A", scene.Metadata.Value["label"])
        Assert.True(scene.Width > 0.0)
        Assert.Equal(Code39.defaultRenderConfig.BarHeight, scene.Height)

    [<Fact>]
    let ``invalid inputs and configs are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Code39.normalizeCode39 Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<InvalidCharacterException>(fun () -> Code39.normalizeCode39 "*" |> ignore) |> ignore
        Assert.Throws<InvalidCharacterException>(fun () -> Code39.normalizeCode39 "~" |> ignore) |> ignore
        Assert.Throws<InvalidCharacterException>(fun () -> Code39.encodeCode39Char "~" |> ignore) |> ignore

        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> Code39.layoutCode39 "A" (Some badOptions) |> ignore) |> ignore
