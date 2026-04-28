namespace CodingAdventures.Barcode1D.Tests

open System
open System.Collections.Generic
open CodingAdventures.Barcode1D.FSharp
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions
open Xunit

module Barcode1DTests =
    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Barcode1D.VERSION)

    [<Fact>]
    let ``normalize accepts common spellings`` () =
        Assert.Equal(Symbology.Code39, Barcode1D.normalizeSymbology "code39")
        Assert.Equal(Symbology.Code128, Barcode1D.normalizeSymbology "code-128")
        Assert.Equal(Symbology.Ean13, Barcode1D.normalizeSymbology "ean_13")
        Assert.Equal(Symbology.UpcA, Barcode1D.normalizeSymbology "UPC-A")
        Assert.Equal(Symbology.Code39, Barcode1D.normalizeSymbology " ")
        Assert.Equal("itf", Barcode1D.symbologyAsString Symbology.Itf)

    [<Fact>]
    let ``normalize rejects unsupported names`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Barcode1D.normalizeSymbology Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<UnsupportedSymbologyException>(fun () -> Barcode1D.normalizeSymbology "qr" |> ignore) |> ignore

    [<Fact>]
    let ``build scene routes to code39 by default`` () =
        let scene = Barcode1D.buildScene "HELLO-123" None

        Assert.Equal(box "code39", scene.Metadata.Value["symbology"])
        Assert.Equal(box "HELLO-123", scene.Metadata.Value["humanReadableText"])
        Assert.True(scene.Width > 0.0)
        Assert.Equal(Barcode1D.defaultRenderConfig.BarHeight, scene.Height)

    [<Theory>]
    [<InlineData("codabar", "40156", "codabar")>]
    [<InlineData("code128", "Code 128", "code128")>]
    [<InlineData("ean-13", "400638133393", "ean-13")>]
    [<InlineData("itf", "123456", "itf")>]
    [<InlineData("upc_a", "03600029145", "upc-a")>]
    let ``build scene for symbology routes additional encoders`` symbology data expected =
        let scene = Barcode1D.buildSceneForSymbology symbology data None

        Assert.Equal(box expected, scene.Metadata.Value["symbology"])
        Assert.True(scene.Width > 0.0)
        Assert.True(unbox<uint32> scene.Metadata.Value["contentModules"] > 0u)

    [<Fact>]
    let ``build scene uses typed options and paint options`` () =
        let metadata = Dictionary<string, obj>()
        metadata.["batch"] <- box "aggregate"

        let options =
            { Barcode1D.defaultOptions with
                Symbology = Symbology.Ean13
                Paint =
                    { BarcodeLayout1D.defaultPaintOptions with
                        RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 2.0 }
                        Metadata = metadata :> Metadata } }

        let scene = Barcode1D.buildScene "400638133393" (Some options)

        Assert.Equal(box "ean-13", scene.Metadata.Value["symbology"])
        Assert.Equal(box "aggregate", scene.Metadata.Value["batch"])
        Assert.Equal(box 2.0, scene.Metadata.Value["moduleWidthPx"])

    [<Fact>]
    let ``codabar start stop can be selected`` () =
        let scene =
            Barcode1D.buildScene
                "40156"
                (Some
                    { Barcode1D.defaultOptions with
                        Symbology = Symbology.Codabar
                        CodabarStart = Some "B"
                        CodabarStop = Some "C" })

        Assert.Equal(box "codabar", scene.Metadata.Value["symbology"])
        Assert.Equal(box "B", scene.Metadata.Value["start"])
        Assert.Equal(box "C", scene.Metadata.Value["stop"])

    [<Fact>]
    let ``current backend is honest about missing native renderer`` () =
        Assert.True(Barcode1D.currentBackend().IsNone)

    [<Fact>]
    let ``render pixels fails until native backend exists`` () =
        Assert.Throws<BackendUnavailableException>(fun () -> Barcode1D.renderPixels "HELLO-123" None |> ignore) |> ignore
        Assert.Throws<BackendUnavailableException>(fun () -> Barcode1D.renderPixelsForSymbology "code-128" "Code 128" None |> ignore) |> ignore

    [<Fact>]
    let ``render png fails until native backend exists`` () =
        Assert.Throws<BackendUnavailableException>(fun () -> Barcode1D.renderPng "HELLO-123" None |> ignore) |> ignore
        Assert.Throws<BackendUnavailableException>(fun () -> Barcode1D.renderPngForSymbology "ean13" "400638133393" None |> ignore) |> ignore
