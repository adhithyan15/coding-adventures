namespace CodingAdventures.PaintVmAscii.Tests

open System
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVm
open CodingAdventures.PaintVmAscii
open Xunit

type PaintVmTests() =
    let options = { ScaleX = 1.0; ScaleY = 1.0 }

    [<Fact>]
    member _.``Version is semver``() =
        Assert.Equal("0.1.0", PaintVmAsciiPackage.VERSION)

    [<Fact>]
    member _.``RenderToAscii draws a stroked rectangle``() =
        let rectOptions =
            { PaintInstructions.defaultPaintRectOptions with
                Fill = Some "transparent"
                Stroke = Some "#000"
                StrokeWidth = Some 1.0 }

        let scene =
            PaintInstructions.paintScene
                5
                3
                "#fff"
                [ PaintInstructions.paintRectWith rectOptions 0 0 4 2 ]

        let rendered = PaintVmAscii.renderToAsciiWith options scene

        Assert.Equal("┌───┐\n│   │\n└───┘", rendered)

    [<Fact>]
    member _.``RenderToAscii fills rectangles with block characters``() =
        let rectOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some "#000" }
        let scene = PaintInstructions.paintScene 3 2 "#fff" [ PaintInstructions.paintRectWith rectOptions 0 0 2 1 ]

        Assert.Contains("█", PaintVmAscii.renderToAsciiWith options scene)

    [<Fact>]
    member _.``RenderToAscii merges line intersections``() =
        let scene =
            PaintInstructions.paintScene
                5
                3
                "#fff"
                [ PaintInstructions.paintLine 0 1 4 1 "#000"
                  PaintInstructions.paintLine 2 0 2 2 "#000" ]

        let lines = PaintVmAscii.renderToAsciiWith options scene |> fun rendered -> rendered.Split('\n')

        Assert.Equal('│', lines[0][2])
        Assert.Equal('┼', lines[1][2])
        Assert.Equal('│', lines[2][2])

    [<Fact>]
    member _.``RenderToAscii renders glyph runs as text``() =
        let scene =
            PaintInstructions.paintScene
                5
                1
                "#fff"
                [ PaintInstructions.paintGlyphRun
                      [ { GlyphId = int 'H'; X = 0; Y = 0 }
                        { GlyphId = int 'i'; X = 1; Y = 0 } ]
                      "mono"
                      12 ]

        Assert.Equal("Hi", PaintVmAscii.renderToAsciiWith options scene)

    [<Fact>]
    member _.``RenderToAscii replaces unsafe glyphs``() =
        let scene =
            PaintInstructions.paintScene
                2
                1
                "#fff"
                [ PaintInstructions.paintGlyphRun
                      [ { GlyphId = 0x1b; X = 0; Y = 0 }
                        { GlyphId = int 'A'; X = 1; Y = 0 } ]
                      "mono"
                      12 ]

        Assert.Equal("?A", PaintVmAscii.renderToAsciiWith options scene)

    [<Fact>]
    member _.``RenderToAscii clips child output``() =
        let scene =
            PaintInstructions.paintScene
                10
                1
                "#fff"
                [ PaintInstructions.paintClip
                      0
                      0
                      3
                      1
                      [ PaintInstructions.paintGlyphRun
                            [ { GlyphId = int 'H'; X = 0; Y = 0 }
                              { GlyphId = int 'e'; X = 1; Y = 0 }
                              { GlyphId = int 'l'; X = 2; Y = 0 }
                              { GlyphId = int 'l'; X = 3; Y = 0 }
                              { GlyphId = int 'o'; X = 4; Y = 0 } ]
                            "mono"
                            12 ] ]

        Assert.Equal("Hel", PaintVmAscii.renderToAsciiWith options scene)

    [<Fact>]
    member _.``RenderToAscii recurses plain groups and layers``() =
        let scene =
            PaintInstructions.paintScene
                5
                1
                "#fff"
                [ PaintInstructions.paintGroup
                      [ PaintInstructions.paintLayer
                            [ PaintInstructions.paintGlyphRun
                                  [ { GlyphId = int 'A'; X = 0; Y = 0 }
                                    { GlyphId = int 'B'; X = 1; Y = 0 } ]
                                  "mono"
                                  12 ]
                        PaintInstructions.paintGlyphRun
                            [ { GlyphId = int 'C'; X = 3; Y = 0 }
                              { GlyphId = int 'D'; X = 4; Y = 0 } ]
                            "mono"
                            12 ] ]

        Assert.Equal("AB CD", PaintVmAscii.renderToAsciiWith options scene)

    [<Fact>]
    member _.``RenderToAscii rejects transformed groups``() =
        let groupOptions =
            { PaintInstructions.defaultPaintGroupOptions with
                Transform = Some { A = 1.0; B = 0.0; C = 0.0; D = 1.0; E = 1.0; F = 0.0 } }

        let scene = PaintInstructions.paintScene 5 1 "#fff" [ PaintInstructions.paintGroupWith groupOptions [] ]

        let error =
            Assert.Throws<UnsupportedAsciiFeatureError>(fun () -> PaintVmAscii.renderToAsciiWith options scene |> ignore)

        Assert.Contains("transformed groups", error.Message)

    [<Fact>]
    member _.``CreateAsciiVM executes through PaintVM``() =
        let vm = PaintVmAscii.createAsciiVMWith options
        let context = PaintVmAscii.createAsciiContext ()
        let scene =
            PaintInstructions.paintScene
                2
                1
                "#fff"
                [ PaintInstructions.paintGlyphRun [ { GlyphId = int 'O'; X = 0; Y = 0 } ] "mono" 12 ]

        vm.Execute(scene, context)

        Assert.Equal("O", context.Buffer.ToString())

    [<Fact>]
    member _.``Export throws because ASCII does not produce pixel data``() =
        let vm = PaintVmAscii.createAsciiVMWith options

        Assert.Throws<ExportNotSupportedError>(Action(fun () -> vm.Export(PaintInstructions.paintScene 10 10 "#fff" []) |> ignore))
        |> ignore
