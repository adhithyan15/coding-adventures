namespace CodingAdventures.Ean13.Tests

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.Ean13.FSharp
open Xunit

module Ean13Tests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Ean13.VERSION)

    [<Fact>]
    let ``computes and validates check digit`` () =
        Assert.Equal("1", Ean13.computeEan13CheckDigit "400638133393")
        Assert.Equal("4006381333931", Ean13.normalizeEan13 "400638133393")
        Assert.Equal("4006381333931", Ean13.normalizeEan13 "4006381333931")

    [<Fact>]
    let ``rejects malformed input`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Ean13.normalizeEan13 Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<InvalidEan13InputException>(fun () -> Ean13.normalizeEan13 "40063813339A" |> ignore) |> ignore
        Assert.Throws<InvalidEan13InputException>(fun () -> Ean13.normalizeEan13 "123" |> ignore) |> ignore
        Assert.Throws<InvalidEan13CheckDigitException>(fun () -> Ean13.normalizeEan13 "4006381333932" |> ignore) |> ignore

    [<Fact>]
    let ``left parity pattern matches reference`` () =
        Assert.Equal("LGLLGG", Ean13.leftParityPattern "400638133393")

    [<Fact>]
    let ``encode tracks parity and check digit`` () =
        let encoded = Ean13.encodeEan13 "400638133393"

        Assert.Equal(12, encoded.Length)
        Assert.Equal("0", encoded[0].Digit)
        Assert.Equal("L", encoded[0].Encoding)
        Assert.Equal("0", encoded[1].Digit)
        Assert.Equal("G", encoded[1].Encoding)
        Assert.Equal(Check, encoded[encoded.Length - 1].Role)
        Assert.Equal("1", encoded[encoded.Length - 1].Digit)

    [<Fact>]
    let ``expand runs total ninety five modules`` () =
        let runs = Ean13.expandEan13Runs "400638133393"

        Assert.Equal(95u, runs |> List.sumBy _.Modules)
        Assert.Equal(Guard, runs[0].Role)
        Assert.Equal("start", runs[0].SourceLabel)
        Assert.Contains(runs, fun run -> run.SourceLabel = "center" && run.Role = Guard)
        Assert.Equal("end", runs[runs.Length - 1].SourceLabel)

    [<Fact>]
    let ``layout builds scene metadata and symbols`` () =
        let scene = Ean13.drawEan13 "400638133393" None

        Assert.Equal(box "ean-13", scene.Metadata.Value["symbology"])
        Assert.Equal(box "4", scene.Metadata.Value["leadingDigit"])
        Assert.Equal(box "LGLLGG", scene.Metadata.Value["leftParity"])
        Assert.Equal(box "EAN-13 barcode for 4006381333931", scene.Metadata.Value["label"])
        Assert.Equal(box "4006381333931", scene.Metadata.Value["humanReadableText"])
        Assert.Equal(box 95u, scene.Metadata.Value["contentModules"])
        Assert.Equal(box 15, scene.Metadata.Value["symbolCount"])
        Assert.Equal(Ean13.defaultRenderConfig.BarHeight, scene.Height)

    [<Fact>]
    let ``invalid render config is rejected`` () =
        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> Ean13.layoutEan13 "400638133393" (Some badOptions) |> ignore) |> ignore
