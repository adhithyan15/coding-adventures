namespace CodingAdventures.PaintInstructions.Tests

open System.Collections.Generic
open CodingAdventures.PaintInstructions
open CodingAdventures.PixelContainer
open Xunit

type PaintInstructionsTests() =
    [<Fact>]
    member _.``Version is semver``() =
        Assert.Equal("0.1.0", PaintInstructions.VERSION)

    [<Fact>]
    member _.``paintRect creates a minimal rectangle``() =
        match PaintInstructions.paintRect 10 20 100 50 with
        | Rect rect ->
            Assert.Equal("rect", (Rect rect).Kind)
            Assert.Equal(10.0, rect.X)
            Assert.Equal(20.0, rect.Y)
            Assert.Equal(100.0, rect.Width)
            Assert.Equal(50.0, rect.Height)
            Assert.True(rect.Fill.IsNone)
        | _ -> failwith "expected rect"

    [<Fact>]
    member _.``paintRectWith applies optional fields``() =
        let metadataMap = Dictionary<string, obj>()
        metadataMap["source"] <- box "chart-bar-3"
        let metadata = metadataMap :> IReadOnlyDictionary<string, obj>

        let options =
            {
                PaintInstructions.defaultPaintRectOptions with
                    Fill = Some "#2563eb"
                    Stroke = Some "#ffffff"
                    StrokeWidth = Some 2.0
                    CornerRadius = Some 8.0
                    Id = Some "card-bg"
                    Metadata = Some metadata
            }

        match PaintInstructions.paintRectWith options 0 0 20 30 with
        | Rect rect ->
            Assert.Equal(Some "#2563eb", rect.Fill)
            Assert.Equal(Some "#ffffff", rect.Stroke)
            Assert.Equal(Some 2.0, rect.StrokeWidth)
            Assert.Equal(Some 8.0, rect.CornerRadius)
            Assert.Equal(Some "card-bg", rect.Base.Id)
            Assert.Equal("chart-bar-3", unbox<string> rect.Base.Metadata.Value["source"])
        | _ -> failwith "expected rect"

    [<Fact>]
    member _.``paintPath preserves commands and stroke settings``() =
        let options =
            {
                PaintInstructions.defaultPaintPathOptions with
                    Fill = Some "#ef4444"
                    FillRule = Some "evenodd"
                    Stroke = Some "#111111"
                    StrokeCap = Some "round"
                    StrokeJoin = Some "bevel"
            }

        match PaintInstructions.paintPathWith options [ MoveTo(0, 0); LineTo(100, 0); Close ] with
        | Path path ->
            Assert.Equal(3, path.Commands.Length)
            Assert.Equal("move_to", path.Commands[0].Kind)
            Assert.Equal("close", path.Commands[2].Kind)
            Assert.Equal(Some "evenodd", path.FillRule)
            Assert.Equal(Some "round", path.StrokeCap)
            Assert.Equal(Some "bevel", path.StrokeJoin)
        | _ -> failwith "expected path"

    [<Fact>]
    member _.``path commands expose stable kinds for all variants``() =
        let commands =
            [
                MoveTo(0, 0)
                LineTo(1, 1)
                QuadTo(2, 3, 4, 5)
                CubicTo(1, 2, 3, 4, 5, 6)
                ArcTo(7, 8, 45, true, false, 9, 10)
                Close
            ]

        Assert.Equal<string>([| "move_to"; "line_to"; "quad_to"; "cubic_to"; "arc_to"; "close" |], commands |> List.map _.Kind |> List.toArray)

    [<Fact>]
    member _.``filter effects expose stable kinds for all variants``() =
        let filters =
            [
                Blur 1.0
                DropShadow(2.0, 3.0, 4.0, "#000")
                ColorMatrix [ 1.0; 0.0; 0.0; 0.0; 0.0 ]
                Brightness 1.1
                Contrast 0.9
                Saturate 0.8
                HueRotate 120.0
                Invert 0.5
                Opacity 0.25
            ]

        Assert.Equal<string>(
            [| "blur"; "drop_shadow"; "color_matrix"; "brightness"; "contrast"; "saturate"; "hue_rotate"; "invert"; "opacity" |],
            filters |> List.map _.Kind |> List.toArray
        )

    [<Fact>]
    member _.``paintEllipseWith applies optional fields``() =
        let options =
            {
                PaintInstructions.defaultPaintEllipseOptions with
                    Fill = Some "#10b981"
                    Stroke = Some "#064e3b"
                    StrokeWidth = Some 1.5
                    Id = Some "orbit"
            }

        match PaintInstructions.paintEllipseWith options 30 40 15 10 with
        | Ellipse ellipse ->
            Assert.Equal(Some "#10b981", ellipse.Fill)
            Assert.Equal(Some "#064e3b", ellipse.Stroke)
            Assert.Equal(Some 1.5, ellipse.StrokeWidth)
            Assert.Equal(Some "orbit", ellipse.Base.Id)
        | _ -> failwith "expected ellipse"

    [<Fact>]
    member _.``paintGlyphRunWith stores placements and fill``() =
        let options =
            {
                PaintInstructions.defaultPaintGlyphRunOptions with
                    Fill = Some "#111827"
                    Id = Some "title"
            }

        let glyphs =
            [
                { GlyphId = 65; X = 10.0; Y = 20.0 }
                { GlyphId = 66; X = 22.0; Y = 20.0 }
            ]

        match PaintInstructions.paintGlyphRunWith options glyphs "font://plex-sans" 18 with
        | GlyphRun glyphRun ->
            Assert.Equal("glyph_run", (GlyphRun glyphRun).Kind)
            Assert.Equal("font://plex-sans", glyphRun.FontRef)
            Assert.Equal(18.0, glyphRun.FontSize)
            Assert.Equal(2, glyphRun.Glyphs.Length)
            Assert.Equal(Some "#111827", glyphRun.Fill)
            Assert.Equal(Some "title", glyphRun.Base.Id)
        | _ -> failwith "expected glyph run"

    [<Fact>]
    member _.``paintLineWith applies stroke presentation options``() =
        let options =
            {
                PaintInstructions.defaultPaintLineOptions with
                    StrokeWidth = Some 3.0
                    StrokeCap = Some "square"
                    Id = Some "baseline"
            }

        match PaintInstructions.paintLineWith options 0 1 20 21 "#334155" with
        | Line line ->
            Assert.Equal("#334155", line.Stroke)
            Assert.Equal(Some 3.0, line.StrokeWidth)
            Assert.Equal(Some "square", line.StrokeCap)
            Assert.Equal(Some "baseline", line.Base.Id)
        | _ -> failwith "expected line"

    [<Fact>]
    member _.``paintGroup stores children transform and opacity``() =
        let transform = { A = 1.0; B = 0.0; C = 0.0; D = 1.0; E = 100.0; F = 50.0 }
        let options = { PaintInstructions.defaultPaintGroupOptions with Transform = Some transform; Opacity = Some 0.5 }

        match PaintInstructions.paintGroupWith options [ PaintInstructions.paintRect 0 0 10 10 ] with
        | Group group ->
            Assert.Single(group.Children) |> ignore
            Assert.Equal(Some transform, group.Transform)
            Assert.Equal(Some 0.5, group.Opacity)
        | _ -> failwith "expected group"

    [<Fact>]
    member _.``paintLayer stores filters and blend mode``() =
        let options =
            {
                PaintInstructions.defaultPaintLayerOptions with
                    Filters = Some [ Blur 10.0; Brightness 1.2 ]
                    BlendMode = Some Multiply
                    Opacity = Some 0.7
            }

        match PaintInstructions.paintLayerWith options [ PaintInstructions.paintRect 0 0 10 10 ] with
        | Layer layer ->
            Assert.Equal(Some Multiply, layer.BlendMode)
            Assert.Equal(Some 0.7, layer.Opacity)
            Assert.Equal("blur", layer.Filters.Value[0].Kind)
            Assert.Equal("brightness", layer.Filters.Value[1].Kind)
        | _ -> failwith "expected layer"

    [<Fact>]
    member _.``paintClip stores rectangle and children``() =
        match PaintInstructions.paintClip 0 0 400 300 [ PaintInstructions.paintRect -10 -10 420 320 ] with
        | Clip clip ->
            Assert.Equal(400.0, clip.Width)
            Assert.Equal(300.0, clip.Height)
            Assert.Single(clip.Children) |> ignore
        | _ -> failwith "expected clip"

    [<Fact>]
    member _.``paintGradient supports linear geometry``() =
        let options =
            {
                PaintInstructions.defaultPaintGradientOptions with
                    Id = Some "blue-purple"
                    X1 = Some 0.0
                    Y1 = Some 0.0
                    X2 = Some 400.0
                    Y2 = Some 0.0
            }

        let stops = [ { Offset = 0.0; Color = "#3b82f6" }; { Offset = 1.0; Color = "#8b5cf6" } ]

        match PaintInstructions.paintGradientWith options Linear stops with
        | Gradient gradient ->
            Assert.Equal(Linear, gradient.GradientKind)
            Assert.Equal(Some "blue-purple", gradient.Base.Id)
            Assert.Equal(2, gradient.Stops.Length)
            Assert.Equal(Some 400.0, gradient.X2)
        | _ -> failwith "expected gradient"

    [<Fact>]
    member _.``paintGradientWith supports radial geometry``() =
        let stops = [ { Offset = 0.25; Color = "#f59e0b" }; { Offset = 1.0; Color = "#7c2d12" } ]

        let options =
            {
                PaintInstructions.defaultPaintGradientOptions with
                    Cx = Some 50.0
                    Cy = Some 60.0
                    R = Some 25.0
            }

        match PaintInstructions.paintGradientWith options Radial stops with
        | Gradient gradient ->
            Assert.Equal(Radial, gradient.GradientKind)
            Assert.Equal(Some 50.0, gradient.Cx)
            Assert.Equal(Some 60.0, gradient.Cy)
            Assert.Equal(Some 25.0, gradient.R)
        | _ -> failwith "expected gradient"

    [<Fact>]
    member _.``paintImage accepts uri and pixel sources``() =
        match PaintInstructions.paintImage 10 20 300 200 (ImageUri "file:///assets/logo.png") with
        | Image uriImage ->
            match uriImage.Src with
            | ImageUri value -> Assert.Equal("file:///assets/logo.png", value)
            | _ -> failwith "expected uri source"
        | _ -> failwith "expected image"

        let pixels = PixelContainers.create 2 2

        let options =
            {
                PaintInstructions.defaultPaintImageOptions with
                    Opacity = Some 0.75
                    Id = Some "embedded"
                    Metadata =
                        let metadataMap = Dictionary<string, obj>()
                        metadataMap["purpose"] <- box "preview"
                        Some (metadataMap :> IReadOnlyDictionary<string, obj>)
            }

        match PaintInstructions.paintImageWith options 0 0 20 20 (ImagePixels pixels) with
        | Image pixelImage ->
            match pixelImage.Src with
            | ImagePixels value -> Assert.Same(pixels, value)
            | _ -> failwith "expected pixel source"
            Assert.Equal(Some 0.75, pixelImage.Opacity)
            Assert.Equal(Some "embedded", pixelImage.Base.Id)
            Assert.Equal("preview", unbox<string> pixelImage.Base.Metadata.Value["purpose"])
        | _ -> failwith "expected image"

    [<Fact>]
    member _.``paintScene stores ordered instructions``() =
        let sceneOptions = { PaintInstructions.defaultSceneOptions with Id = Some "chart" }
        let scene =
            PaintInstructions.paintSceneWith
                sceneOptions
                800
                600
                "#f8fafc"
                [ PaintInstructions.paintRect 0 0 100 50; PaintInstructions.paintEllipse 50 50 20 20 ]

        Assert.Equal(800.0, scene.Width)
        Assert.Equal(600.0, scene.Height)
        Assert.Equal("#f8fafc", scene.Background)
        Assert.Equal(Some "chart", scene.Id)
        Assert.Equal<string>([| "rect"; "ellipse" |], scene.Instructions |> List.map _.Kind |> List.toArray)

    [<Fact>]
    member _.``default option records start empty``() =
        Assert.True(PaintInstructions.defaultSceneOptions.Id.IsNone)
        Assert.True(PaintInstructions.defaultPaintRectOptions.Fill.IsNone)
        Assert.True(PaintInstructions.defaultPaintEllipseOptions.Stroke.IsNone)
        Assert.True(PaintInstructions.defaultPaintPathOptions.StrokeCap.IsNone)
        Assert.True(PaintInstructions.defaultPaintGlyphRunOptions.Fill.IsNone)
        Assert.True(PaintInstructions.defaultPaintGroupOptions.Transform.IsNone)
        Assert.True(PaintInstructions.defaultPaintLayerOptions.Filters.IsNone)
        Assert.True(PaintInstructions.defaultPaintLineOptions.StrokeWidth.IsNone)
        Assert.True(PaintInstructions.defaultPaintClipOptions.Id.IsNone)
        Assert.True(PaintInstructions.defaultPaintGradientOptions.R.IsNone)
        Assert.True(PaintInstructions.defaultPaintImageOptions.Opacity.IsNone)
