namespace CodingAdventures.Itf.Tests

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.Itf.FSharp
open Xunit

module ItfTests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Itf.VERSION)

    [<Fact>]
    let ``normalize accepts even length digit strings`` () =
        Assert.Equal("123456", Itf.normalizeItf "123456")

    [<Fact>]
    let ``normalize rejects invalid input`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Itf.normalizeItf Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf "" |> ignore) |> ignore
        Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf "12345" |> ignore) |> ignore
        Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf "12A4" |> ignore) |> ignore

    [<Fact>]
    let ``encode encodes digit pairs`` () =
        let encoded = Itf.encodeItf "123456"

        Assert.Equal(3, encoded.Length)
        Assert.Equal("12", encoded[0].Pair)
        Assert.Equal("10001", encoded[0].BarPattern)
        Assert.Equal("01001", encoded[0].SpacePattern)
        Assert.Equal(0, encoded[0].SourceIndex)
        Assert.NotEmpty(encoded[0].BinaryPattern)

    [<Fact>]
    let ``expand runs includes start and stop patterns`` () =
        let runs = Itf.expandItfRuns "123456"

        Assert.Equal("start", runs[0].SourceLabel)
        Assert.Equal(Start, runs[0].Role)
        Assert.Equal("stop", runs[runs.Length - 1].SourceLabel)
        Assert.Equal(Stop, runs[runs.Length - 1].Role)
        Assert.Contains(runs, fun run -> run.Role = Data && run.SourceLabel = "12")

    [<Fact>]
    let ``draw returns barcode scene`` () =
        let scene = Itf.drawItf "123456" None

        Assert.Equal(box "itf", scene.Metadata.Value["symbology"])
        Assert.Equal(box 3, scene.Metadata.Value["pairCount"])
        Assert.True(scene.Width > 0.0)
        Assert.Equal(Itf.defaultRenderConfig.BarHeight, scene.Height)

    [<Fact>]
    let ``invalid layout config is rejected`` () =
        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> Itf.layoutItf "123456" (Some badOptions) |> ignore) |> ignore
