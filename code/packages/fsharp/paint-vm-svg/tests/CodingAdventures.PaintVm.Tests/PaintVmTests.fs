namespace CodingAdventures.PaintVmSvg.Tests

open System
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVm
open CodingAdventures.PaintVmSvg
open CodingAdventures.PixelContainer
open Xunit

type PaintVmTests() =
    [<Fact>]
    member _.``Version is semver``() =
        Assert.Equal("0.1.0", PaintVmSvgPackage.VERSION)

    [<Fact>]
    member _.``RenderToSvgString emits svg root and background``() =
        let svg = PaintVmSvg.renderToSvgString (PaintInstructions.paintScene 100 80 "#f8fafc" [])
        Assert.StartsWith("<svg", svg)
        Assert.Contains("xmlns=\"http://www.w3.org/2000/svg\"", svg)
        Assert.Contains("fill=\"#f8fafc\"", svg)
        Assert.EndsWith("</svg>", svg)

    [<Fact>]
    member _.``RenderToSvgString emits rect attributes``() =
        let rectOptions =
            { PaintInstructions.defaultPaintRectOptions with
                Fill = Some "#ef4444"
                CornerRadius = Some 8.0
                Id = Some "card" }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene 200 100 "transparent" [ PaintInstructions.paintRectWith rectOptions 10 20 50 30 ])

        Assert.Contains("<rect", svg)
        Assert.Contains("id=\"card\"", svg)
        Assert.Contains("x=\"10\"", svg)
        Assert.Contains("y=\"20\"", svg)
        Assert.Contains("rx=\"8\"", svg)
        Assert.Contains("fill=\"#ef4444\"", svg)

    [<Fact>]
    member _.``RenderToSvgString emits ellipse and line``() =
        let ellipseOptions = { PaintInstructions.defaultPaintEllipseOptions with Fill = Some "#3b82f6" }
        let lineOptions = { PaintInstructions.defaultPaintLineOptions with StrokeWidth = Some 2.0; StrokeCap = Some "round" }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    200
                    200
                    "transparent"
                    [ PaintInstructions.paintEllipseWith ellipseOptions 100 90 50 20
                      PaintInstructions.paintLineWith lineOptions 0 50 200 50 "#111111" ])

        Assert.Contains("<ellipse", svg)
        Assert.Contains("cx=\"100\"", svg)
        Assert.Contains("<line", svg)
        Assert.Contains("stroke-linecap=\"round\"", svg)

    [<Fact>]
    member _.``RenderToSvgString emits path commands``() =
        let pathOptions =
            { PaintInstructions.defaultPaintPathOptions with
                Stroke = Some "#000"
                FillRule = Some "evenodd"
                StrokeJoin = Some "round"
                StrokeCap = Some "square" }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    200
                    200
                    "transparent"
                    [ PaintInstructions.paintPathWith
                          pathOptions
                          [ MoveTo(0, 0)
                            CubicTo(10, 20, 30, 40, 100, 100)
                            ArcTo(50, 50, 0, false, true, 120, 100)
                            Close ] ])

        Assert.Contains("C 10 20 30 40 100 100", svg)
        Assert.Contains("A 50 50 0 0 1 120 100", svg)
        Assert.Contains("fill-rule=\"evenodd\"", svg)
        Assert.Contains("stroke-linejoin=\"round\"", svg)
        Assert.Contains("stroke-linecap=\"square\"", svg)

    [<Fact>]
    member _.``RenderToSvgString emits text for glyph runs``() =
        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    200
                    100
                    "transparent"
                    [ PaintInstructions.paintGlyphRun [ { GlyphId = 65; X = 10; Y = 20 }; { GlyphId = 0x200000; X = 20; Y = 20 } ] "Inter" 16 ])

        Assert.Contains("<text", svg)
        Assert.Contains("&#65;", svg)
        Assert.Contains("&#65533;", svg)

    [<Fact>]
    member _.``RenderToSvgString emits groups layers and filters``() =
        let groupOptions =
            { PaintInstructions.defaultPaintGroupOptions with
                Transform = Some { A = 1.0; B = 0.0; C = 0.0; D = 1.0; E = 10.0; F = 20.0 }
                Opacity = Some 0.5 }

        let layerOptions =
            { PaintInstructions.defaultPaintLayerOptions with
                Id = Some "glow"
                Filters = Some [ Blur 5.0 ]
                BlendMode = Some BlendMode.Multiply }

        let rectOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some "#3b82f6" }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    200
                    200
                    "transparent"
                    [ PaintInstructions.paintGroupWith groupOptions [ PaintInstructions.paintRectWith rectOptions 0 0 50 50 ]
                      PaintInstructions.paintLayerWith layerOptions [] ])

        Assert.Contains("transform=\"matrix(1,0,0,1,10,20)\"", svg)
        Assert.Contains("opacity=\"0.5\"", svg)
        Assert.Contains("<defs>", svg)
        Assert.Contains("feGaussianBlur", svg)
        Assert.Contains("mix-blend-mode:multiply", svg)
        Assert.Contains("filter=\"url(#filter-glow)\"", svg)

    [<Fact>]
    member _.``RenderToSvgString emits remaining filter kinds``() =
        let layerOptions =
            { PaintInstructions.defaultPaintLayerOptions with
                Id = Some "fx"
                Filters =
                    Some
                        [ DropShadow(2.0, 3.0, 4.0, "#000")
                          ColorMatrix([ 1.0; 0.0; 0.0; 0.0; 0.0
                                        0.0; 1.0; 0.0; 0.0; 0.0
                                        0.0; 0.0; 1.0; 0.0; 0.0
                                        0.0; 0.0; 0.0; 1.0; 0.0 ])
                          Brightness 1.2
                          Contrast 0.8
                          Saturate 1.5
                          HueRotate 90.0
                          Invert 1.0
                          Opacity 0.5 ] }

        let svg = PaintVmSvg.renderToSvgString (PaintInstructions.paintScene 200 100 "transparent" [ PaintInstructions.paintLayerWith layerOptions [] ])

        Assert.Contains("feDropShadow", svg)
        Assert.Contains("type=\"matrix\"", svg)
        Assert.Contains("type=\"saturate\"", svg)
        Assert.Contains("type=\"hueRotate\"", svg)
        Assert.Contains("feFuncA", svg)

    [<Fact>]
    member _.``RenderToSvgString emits clip paths``() =
        let rectOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some "#fff" }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    200
                    100
                    "transparent"
                    [ PaintInstructions.paintClip 0 0 20 10 [ PaintInstructions.paintRectWith rectOptions 0 0 100 50 ] ])

        Assert.Contains("<clipPath", svg)
        Assert.Contains("clip-path=\"url(#", svg)

    [<Fact>]
    member _.``RenderToSvgString emits gradients``() =
        let gradientOptions =
            { PaintInstructions.defaultPaintGradientOptions with
                Id = Some "grad1"
                X1 = Some 0.0
                Y1 = Some 0.0
                X2 = Some 300.0
                Y2 = Some 0.0 }

        let rectOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some "url(#grad1)" }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    300
                    100
                    "transparent"
                    [ PaintInstructions.paintGradientWith gradientOptions GradientKind.Linear [ { Offset = 0.0; Color = "#3b82f6" }; { Offset = 1.0; Color = "#8b5cf6" } ]
                      PaintInstructions.paintRectWith rectOptions 0 0 300 100 ])

        Assert.Contains("<linearGradient", svg)
        Assert.Contains("id=\"grad1\"", svg)
        Assert.Contains("fill=\"url(#grad1)\"", svg)

    [<Fact>]
    member _.``RenderToSvgString ignores gradients without ids``() =
        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene 100 100 "transparent" [ PaintInstructions.paintGradient GradientKind.Linear [ { Offset = 0.0; Color = "#000" } ] ])

        Assert.DoesNotContain("<linearGradient", svg)

    [<Fact>]
    member _.``RenderToSvgString emits radial gradients and image opacity``() =
        let gradientOptions =
            { PaintInstructions.defaultPaintGradientOptions with
                Id = Some "radial1"
                Cx = Some 50.0
                Cy = Some 50.0
                R = Some 25.0 }

        let imageOptions = { PaintInstructions.defaultPaintImageOptions with Opacity = Some 0.5 }

        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    100
                    100
                    "transparent"
                    [ PaintInstructions.paintGradientWith gradientOptions GradientKind.Radial [ { Offset = 0.0; Color = "#fff" }; { Offset = 1.0; Color = "#000" } ]
                      PaintInstructions.paintImageWith imageOptions 0 0 20 20 (ImageUri "https://example.com/logo.png") ])

        Assert.Contains("<radialGradient", svg)
        Assert.Contains("opacity=\"0.5\"", svg)

    [<Fact>]
    member _.``RenderToSvgString sanitizes image uris``() =
        let svg =
            PaintVmSvg.renderToSvgString
                (PaintInstructions.paintScene
                    200
                    100
                    "transparent"
                    [ PaintInstructions.paintImage 0 0 50 50 (ImageUri "https://example.com/logo.png")
                      PaintInstructions.paintImage 50 0 50 50 (ImageUri "javascript:alert(1)")
                      PaintInstructions.paintImage 100 0 50 50 (ImagePixels(PixelContainer(1, 1))) ])

        Assert.Contains("https://example.com/logo.png", svg)
        Assert.DoesNotContain("javascript:alert(1)", svg)
        Assert.Contains("data:image/gif;base64,", svg)
        Assert.Contains("data:image/png;base64,", svg)

    [<Fact>]
    member _.``AssembleSvg composes manual vm execution``() =
        let vm = PaintVmSvg.createSvgVM ()
        let context = PaintVmSvg.createSvgContext ()
        let rectOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some "#fff" }
        let scene = PaintInstructions.paintScene 100 100 "transparent" [ PaintInstructions.paintRectWith rectOptions 0 0 50 50 ]

        vm.Execute(scene, context)
        let svg = PaintVmSvg.assembleSvg scene context

        Assert.Contains("<rect", svg)

    [<Fact>]
    member _.``Export throws because svg does not produce pixel data``() =
        let vm = PaintVmSvg.createSvgVM ()
        Assert.Throws<ExportNotSupportedError>(Action(fun () -> vm.Export(PaintInstructions.paintScene 10 10 "#fff" []) |> ignore))
        |> ignore

    [<Fact>]
    member _.``RenderToSvgString rejects non finite scene numbers``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> PaintVmSvg.renderToSvgString (PaintInstructions.paintScene Double.NaN 10 "transparent" []) |> ignore)
        |> ignore
