namespace CodingAdventures.UpcA.Tests

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.UpcA.FSharp
open Xunit

module UpcATests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", UpcA.VERSION)

    [<Fact>]
    let ``computes and validates check digit`` () =
        Assert.Equal("2", UpcA.computeUpcACheckDigit "03600029145")
        Assert.Equal("036000291452", UpcA.normalizeUpcA "03600029145")
        Assert.Equal("036000291452", UpcA.normalizeUpcA "036000291452")

    [<Fact>]
    let ``rejects malformed input`` () =
        Assert.Throws<ArgumentNullException>(fun () -> UpcA.normalizeUpcA Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<InvalidUpcAInputException>(fun () -> UpcA.normalizeUpcA "0360002914A" |> ignore) |> ignore
        Assert.Throws<InvalidUpcAInputException>(fun () -> UpcA.normalizeUpcA "123" |> ignore) |> ignore
        Assert.Throws<InvalidUpcACheckDigitException>(fun () -> UpcA.normalizeUpcA "036000291453" |> ignore) |> ignore

    [<Fact>]
    let ``encode tracks left right and check digit`` () =
        let encoded = UpcA.encodeUpcA "03600029145"

        Assert.Equal(12, encoded.Length)
        Assert.Equal("0", encoded[0].Digit)
        Assert.Equal("L", encoded[0].Encoding)
        Assert.Equal("0", encoded[5].Digit)
        Assert.Equal("L", encoded[5].Encoding)
        Assert.Equal("2", encoded[6].Digit)
        Assert.Equal("R", encoded[6].Encoding)
        Assert.Equal(Check, encoded[encoded.Length - 1].Role)
        Assert.Equal("2", encoded[encoded.Length - 1].Digit)

    [<Fact>]
    let ``expand runs total ninety five modules`` () =
        let runs = UpcA.expandUpcARuns "03600029145"

        Assert.Equal(95u, runs |> List.sumBy _.Modules)
        Assert.Equal(Guard, runs[0].Role)
        Assert.Equal("start", runs[0].SourceLabel)
        Assert.Contains(runs, fun run -> run.SourceLabel = "center" && run.Role = Guard)
        Assert.Equal("end", runs[runs.Length - 1].SourceLabel)

    [<Fact>]
    let ``layout builds scene metadata and symbols`` () =
        let scene = UpcA.drawUpcA "03600029145" None

        Assert.Equal(box "upc-a", scene.Metadata.Value["symbology"])
        Assert.Equal(box "UPC-A barcode for 036000291452", scene.Metadata.Value["label"])
        Assert.Equal(box "036000291452", scene.Metadata.Value["humanReadableText"])
        Assert.Equal(box 95u, scene.Metadata.Value["contentModules"])
        Assert.Equal(box 15, scene.Metadata.Value["symbolCount"])
        Assert.Equal(UpcA.defaultRenderConfig.BarHeight, scene.Height)

    [<Fact>]
    let ``invalid render config is rejected`` () =
        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> UpcA.layoutUpcA "03600029145" (Some badOptions) |> ignore) |> ignore
