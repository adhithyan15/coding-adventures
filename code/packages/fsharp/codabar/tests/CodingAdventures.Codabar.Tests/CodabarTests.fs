namespace CodingAdventures.Codabar.Tests

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.Codabar.FSharp
open Xunit

module CodabarTests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Codabar.VERSION)

    [<Fact>]
    let ``normalize adds default or requested guards`` () =
        Assert.Equal("A40156A", Codabar.normalizeCodabar "40156" None None)
        Assert.Equal("B40156D", Codabar.normalizeCodabar "40156" (Some "B") (Some "D"))
        Assert.Equal("C40156D", Codabar.normalizeCodabar "c40156d" None None)

    [<Fact>]
    let ``encode marks outer symbols`` () =
        let encoded = Codabar.encodeCodabar "40156" (Some "B") (Some "D")

        Assert.Equal("B", encoded[0].Char)
        Assert.Equal("1001001011", encoded[0].Pattern)
        Assert.Equal(Start, encoded[0].Role)
        Assert.Equal(Data, encoded[1].Role)
        Assert.Equal("D", encoded[encoded.Length - 1].Char)
        Assert.Equal(Stop, encoded[encoded.Length - 1].Role)

    [<Fact>]
    let ``expand runs includes inter character gaps`` () =
        let runs = Codabar.expandCodabarRuns "1" None None

        Assert.True(runs.Length > 0)
        Assert.Equal(Bar, runs[0].Color)
        Assert.Equal(Start, runs[0].Role)
        Assert.Contains(runs, fun run -> run.Role = InterCharacterGap)
        Assert.Equal(Stop, runs[runs.Length - 1].Role)

    [<Fact>]
    let ``layout builds paint scene metadata`` () =
        let scene = Codabar.drawCodabar "40156" None (Some "B") (Some "D")

        Assert.Equal(box "codabar", scene.Metadata.Value["symbology"])
        Assert.Equal(box "B", scene.Metadata.Value["start"])
        Assert.Equal(box "D", scene.Metadata.Value["stop"])
        Assert.Equal(box "Codabar barcode for B40156D", scene.Metadata.Value["label"])
        Assert.Equal(box "B40156D", scene.Metadata.Value["humanReadableText"])
        Assert.True(scene.Width > 0.0)
        Assert.Equal(Codabar.defaultRenderConfig.BarHeight, scene.Height)

    [<Fact>]
    let ``invalid inputs and configs are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Codabar.normalizeCodabar Unchecked.defaultof<string> None None |> ignore) |> ignore
        Assert.Throws<InvalidCodabarInputException>(fun () -> Codabar.normalizeCodabar "40*56" None None |> ignore) |> ignore
        Assert.Throws<InvalidCodabarInputException>(fun () -> Codabar.normalizeCodabar "A" None None |> ignore) |> ignore
        Assert.Throws<InvalidCodabarInputException>(fun () -> Codabar.normalizeCodabar "40156" (Some "Z") (Some "A") |> ignore) |> ignore

        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> Codabar.layoutCodabar "40156" (Some badOptions) None None |> ignore) |> ignore
