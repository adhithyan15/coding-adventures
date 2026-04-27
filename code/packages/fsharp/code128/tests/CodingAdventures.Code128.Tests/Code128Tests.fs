namespace CodingAdventures.Code128.Tests

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.Code128.FSharp
open Xunit

module Code128Tests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Code128.VERSION)

    [<Fact>]
    let ``normalize accepts printable ascii`` () =
        Assert.Equal("Code 128", Code128.normalizeCode128B "Code 128")
        Assert.Equal("~", Code128.normalizeCode128B "~")

    [<Fact>]
    let ``normalize rejects unsupported input`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Code128.normalizeCode128B Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<InvalidCode128InputException>(fun () -> Code128.normalizeCode128B "bad\ninput" |> ignore) |> ignore
        Assert.Throws<InvalidCode128InputException>(fun () -> Code128.normalizeCode128B "cafe\u00e9" |> ignore) |> ignore

    [<Fact>]
    let ``computes reference checksum`` () =
        Assert.Equal(64, Code128.computeCode128Checksum [ 35; 79; 68; 69; 0; 17; 18; 24 ])
        Assert.Throws<ArgumentNullException>(fun () -> Code128.computeCode128Checksum Unchecked.defaultof<int list> |> ignore) |> ignore

    [<Fact>]
    let ``encode adds start checksum and stop`` () =
        let encoded = Code128.encodeCode128B "Code 128"

        Assert.Equal("Start B", encoded[0].Label)
        Assert.Equal(104, encoded[0].Value)
        Assert.Equal(Start, encoded[0].Role)
        Assert.Equal("Checksum 64", encoded[encoded.Length - 2].Label)
        Assert.Equal(Check, encoded[encoded.Length - 2].Role)
        Assert.Equal("Stop", encoded[encoded.Length - 1].Label)
        Assert.Equal(Stop, encoded[encoded.Length - 1].Role)

    [<Fact>]
    let ``expand runs ends with stop pattern`` () =
        let runs = Code128.expandCode128Runs "Hi"

        Assert.Equal(57u, runs |> List.sumBy _.Modules)
        Assert.Equal("Start B", runs[0].SourceLabel)
        Assert.Equal(Start, runs[0].Role)
        Assert.Equal("Stop", runs[runs.Length - 1].SourceLabel)
        Assert.Equal(Stop, runs[runs.Length - 1].Role)

    [<Fact>]
    let ``layout builds scene metadata and symbols`` () =
        let scene = Code128.drawCode128 "Code 128" None

        Assert.Equal(box "code128", scene.Metadata.Value["symbology"])
        Assert.Equal(box "B", scene.Metadata.Value["codeSet"])
        Assert.Equal(box 64, scene.Metadata.Value["checksum"])
        Assert.Equal(box "Code 128 barcode for Code 128", scene.Metadata.Value["label"])
        Assert.Equal(box "Code 128", scene.Metadata.Value["humanReadableText"])
        Assert.Equal(box 123u, scene.Metadata.Value["contentModules"])
        Assert.Equal(box 11, scene.Metadata.Value["symbolCount"])

    [<Fact>]
    let ``invalid render config is rejected`` () =
        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> Code128.layoutCode128 "Code 128" (Some badOptions) |> ignore) |> ignore
