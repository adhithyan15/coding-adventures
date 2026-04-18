namespace CodingAdventures.PaintVmSvg

open System
open System.Collections.Generic
open System.Globalization
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVm

[<RequireQualifiedAccess>]
module PaintVmSvgPackage =
    [<Literal>]
    let VERSION = "0.1.0"

type SvgContext() =
    member val Defs = ResizeArray<string>() with get
    member val Elements = ResizeArray<string>() with get
    member val ClipCounter = 0 with get, set
    member val FilterCounter = 0 with get, set

[<RequireQualifiedAccess>]
module PaintVmSvg =
    let private unsafeImagePlaceholder = "data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw=="
    let private pixelImagePlaceholder = "data:image/png;base64,"

    let private escAttr (value: string) =
        value
            .Replace("&", "&amp;")
            .Replace("\"", "&quot;")
            .Replace("<", "&lt;")
            .Replace(">", "&gt;")

    let private safeNum (value: float) (field: string) =
        if Double.IsNaN(value) || Double.IsInfinity(value) then
            raise (ArgumentOutOfRangeException(field, $"PaintVM SVG requires a finite number for {field}, got {value}."))

        value.ToString("0.############################", CultureInfo.InvariantCulture)

    let private roundPathNumber (value: float) =
        Math.Round(value, 4, MidpointRounding.AwayFromZero).ToString("0.####", CultureInfo.InvariantCulture)

    let private idAttr id =
        match id with
        | Some value -> $" id=\"{escAttr value}\""
        | None -> String.Empty

    let private strokeFillAttrs fill stroke strokeWidth =
        let parts = ResizeArray<string>()
        let fillValue = escAttr (defaultArg fill "none")
        parts.Add(sprintf "fill=\"%s\"" fillValue)

        match stroke with
        | Some value when value <> String.Empty ->
            let safeStroke = escAttr value
            let safeStrokeWidth = safeNum (defaultArg strokeWidth 1.0) "stroke_width"
            parts.Add(sprintf "stroke=\"%s\"" safeStroke)
            parts.Add(sprintf "stroke-width=\"%s\"" safeStrokeWidth)
        | _ -> ()

        String.concat " " parts

    let private commandsToPathData commands =
        commands
        |> List.map (function
            | MoveTo(x, y) -> $"M {roundPathNumber x} {roundPathNumber y}"
            | LineTo(x, y) -> $"L {roundPathNumber x} {roundPathNumber y}"
            | QuadTo(cx, cy, x, y) -> $"Q {roundPathNumber cx} {roundPathNumber cy} {roundPathNumber x} {roundPathNumber y}"
            | CubicTo(cx1, cy1, cx2, cy2, x, y) ->
                $"C {roundPathNumber cx1} {roundPathNumber cy1} {roundPathNumber cx2} {roundPathNumber cy2} {roundPathNumber x} {roundPathNumber y}"
            | ArcTo(rx, ry, xRotation, largeArc, sweep, x, y) ->
                $"A {roundPathNumber rx} {roundPathNumber ry} {roundPathNumber xRotation} {(if largeArc then 1 else 0)} {(if sweep then 1 else 0)} {roundPathNumber x} {roundPathNumber y}"
            | Close -> "Z")
        |> String.concat " "

    let private transformAttr transform =
        match transform with
        | None -> String.Empty
        | Some transform ->
            let a = safeNum transform.A "transform.a"
            let b = safeNum transform.B "transform.b"
            let c = safeNum transform.C "transform.c"
            let d = safeNum transform.D "transform.d"
            let e = safeNum transform.E "transform.e"
            let f = safeNum transform.F "transform.f"
            sprintf " transform=\"matrix(%s,%s,%s,%s,%s,%s)\"" a b c d e f

    let private sanitizeImageHref (href: string) =
        let lower = href.ToLowerInvariant().TrimStart()

        if lower.StartsWith("data:") || lower.StartsWith("https:") || lower.StartsWith("http:") then
            href
        else
            unsafeImagePlaceholder

    let private blendModeToSvg mode =
        match mode with
        | BlendMode.Normal -> "normal"
        | BlendMode.Multiply -> "multiply"
        | BlendMode.Screen -> "screen"
        | BlendMode.Overlay -> "overlay"
        | BlendMode.Darken -> "darken"
        | BlendMode.Lighten -> "lighten"
        | BlendMode.ColorDodge -> "color-dodge"
        | BlendMode.ColorBurn -> "color-burn"
        | BlendMode.HardLight -> "hard-light"
        | BlendMode.SoftLight -> "soft-light"
        | BlendMode.Difference -> "difference"
        | BlendMode.Exclusion -> "exclusion"
        | BlendMode.Hue -> "hue"
        | BlendMode.Saturation -> "saturation"
        | BlendMode.Color -> "color"
        | BlendMode.Luminosity -> "luminosity"

    let private buildSvgFilter filterId filters =
        match filters with
        | None
        | Some [] -> String.Empty
        | Some filters ->
            let primitives = ResizeArray<string>()
            let mutable previous = "SourceGraphic"

            for index, filter in filters |> List.indexed do
                let result = $"f{index}"

                match filter with
                | Blur radius ->
                    let safeRadius = safeNum radius "blur.radius"
                    primitives.Add(sprintf "<feGaussianBlur in=\"%s\" stdDeviation=\"%s\" result=\"%s\"/>" previous safeRadius result)
                | DropShadow(dx, dy, blur, color) ->
                    let safeDx = safeNum dx "drop_shadow.dx"
                    let safeDy = safeNum dy "drop_shadow.dy"
                    let safeBlur = safeNum blur "drop_shadow.blur"
                    let safeColor = escAttr color
                    primitives.Add(sprintf "<feDropShadow dx=\"%s\" dy=\"%s\" stdDeviation=\"%s\" flood-color=\"%s\" result=\"%s\"/>" safeDx safeDy safeBlur safeColor result)
                | ColorMatrix matrix ->
                    let safeMatrix =
                        matrix
                        |> List.mapi (fun matrixIndex value -> safeNum value $"color_matrix.matrix[{matrixIndex}]")
                        |> String.concat " "

                    primitives.Add($"<feColorMatrix in=\"{previous}\" type=\"matrix\" values=\"{safeMatrix}\" result=\"{result}\"/>")
                | Brightness amount ->
                    let slope = safeNum amount "brightness.amount"
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncR type=\"linear\" slope=\"{slope}\"/><feFuncG type=\"linear\" slope=\"{slope}\"/><feFuncB type=\"linear\" slope=\"{slope}\"/></feComponentTransfer>")
                | Contrast amount ->
                    let slope = safeNum amount "contrast.amount"
                    let intercept = safeNum (-(amount - 1.0) / 2.0) "contrast.intercept"
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncR type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/><feFuncG type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/><feFuncB type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/></feComponentTransfer>")
                | Saturate amount ->
                    let safeAmount = safeNum amount "saturate.amount"
                    primitives.Add(sprintf "<feColorMatrix in=\"%s\" type=\"saturate\" values=\"%s\" result=\"%s\"/>" previous safeAmount result)
                | HueRotate angle ->
                    let safeAngle = safeNum angle "hue_rotate.angle"
                    primitives.Add(sprintf "<feColorMatrix in=\"%s\" type=\"hueRotate\" values=\"%s\" result=\"%s\"/>" previous safeAngle result)
                | Invert amount ->
                    let positive = safeNum amount "invert.amount"
                    let negative = safeNum (-amount) "invert.neg_amount"
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncR type=\"linear\" slope=\"{negative}\" intercept=\"{positive}\"/><feFuncG type=\"linear\" slope=\"{negative}\" intercept=\"{positive}\"/><feFuncB type=\"linear\" slope=\"{negative}\" intercept=\"{positive}\"/></feComponentTransfer>")
                | Opacity amount ->
                    let safeAmount = safeNum amount "opacity.amount"
                    primitives.Add(sprintf "<feComponentTransfer in=\"%s\" result=\"%s\"><feFuncA type=\"linear\" slope=\"%s\"/></feComponentTransfer>" previous result safeAmount)

                previous <- result

            $"<filter id=\"{escAttr filterId}\">{String.concat String.Empty primitives}</filter>"

    let private handleRect (instruction: PaintRect) (context: SvgContext) =
        let radius =
            match instruction.CornerRadius with
            | Some value ->
                let safeRadius = safeNum value "rect.corner_radius"
                sprintf " rx=\"%s\"" safeRadius
            | None -> String.Empty

        let x = safeNum instruction.X "rect.x"
        let y = safeNum instruction.Y "rect.y"
        let width = safeNum instruction.Width "rect.width"
        let height = safeNum instruction.Height "rect.height"
        let attrs = strokeFillAttrs instruction.Fill instruction.Stroke instruction.StrokeWidth
        context.Elements.Add(sprintf "<rect%s x=\"%s\" y=\"%s\" width=\"%s\" height=\"%s\"%s %s/>" (idAttr instruction.Base.Id) x y width height radius attrs)

    let private handleEllipse (instruction: PaintEllipse) (context: SvgContext) =
        let cx = safeNum instruction.Cx "ellipse.cx"
        let cy = safeNum instruction.Cy "ellipse.cy"
        let rx = safeNum instruction.Rx "ellipse.rx"
        let ry = safeNum instruction.Ry "ellipse.ry"
        let attrs = strokeFillAttrs instruction.Fill instruction.Stroke instruction.StrokeWidth
        context.Elements.Add(sprintf "<ellipse%s cx=\"%s\" cy=\"%s\" rx=\"%s\" ry=\"%s\" %s/>" (idAttr instruction.Base.Id) cx cy rx ry attrs)

    let private handlePath (instruction: PaintPath) (context: SvgContext) =
        let fillRule =
            match instruction.FillRule with
            | Some "evenodd" -> " fill-rule=\"evenodd\""
            | _ -> String.Empty

        let cap =
            match instruction.StrokeCap with
            | Some ("butt" | "round" | "square" as value) -> $" stroke-linecap=\"{value}\""
            | _ -> String.Empty

        let join =
            match instruction.StrokeJoin with
            | Some ("miter" | "round" | "bevel" as value) -> $" stroke-linejoin=\"{value}\""
            | _ -> String.Empty

        let data = escAttr (commandsToPathData instruction.Commands)
        let attrs = strokeFillAttrs instruction.Fill instruction.Stroke instruction.StrokeWidth
        context.Elements.Add(sprintf "<path%s d=\"%s\"%s%s%s %s/>" (idAttr instruction.Base.Id) data fillRule cap join attrs)

    let private handleGlyphRun (instruction: PaintGlyphRun) (context: SvgContext) =
        let spans =
            instruction.Glyphs
            |> List.map (fun glyph ->
                let glyphId =
                    if glyph.GlyphId >= 0 && glyph.GlyphId <= 0x10ffff then
                        glyph.GlyphId
                    else
                        0xfffd

                let x = safeNum glyph.X "glyph.x"
                let y = safeNum glyph.Y "glyph.y"
                sprintf "<tspan x=\"%s\" y=\"%s\">&#%d;</tspan>" x y glyphId)
            |> String.concat String.Empty

        let fill = defaultArg instruction.Fill "#000000"
        let fontSize = safeNum instruction.FontSize "glyph_run.font_size"
        let safeFill = escAttr fill

        context.Elements.Add(sprintf "<text%s font-size=\"%s\" fill=\"%s\">%s</text>" (idAttr instruction.Base.Id) fontSize safeFill spans)

    let private handleGroup (instruction: PaintGroup) (context: SvgContext) (vm: PaintVM<SvgContext>) =
        let opacity =
            match instruction.Opacity with
            | Some value when value <> 1.0 ->
                let safeOpacity = safeNum value "group.opacity"
                sprintf " opacity=\"%s\"" safeOpacity
            | _ -> String.Empty

        context.Elements.Add($"<g{idAttr instruction.Base.Id}{transformAttr instruction.Transform}{opacity}>")
        instruction.Children |> List.iter (fun child -> vm.Dispatch(child, context))
        context.Elements.Add("</g>")

    let private handleLayer (instruction: PaintLayer) (context: SvgContext) (vm: PaintVM<SvgContext>) =
        let filterId =
            match instruction.Base.Id with
            | Some id -> $"filter-{id}"
            | None ->
                let id = context.FilterCounter
                context.FilterCounter <- context.FilterCounter + 1
                $"filter-{id}"

        let filter = buildSvgFilter filterId instruction.Filters

        if filter <> String.Empty then
            context.Defs.Add(filter)

        let filterAttr =
            if filter <> String.Empty then
                $" filter=\"url(#{escAttr filterId})\""
            else
                String.Empty

        let blendAttr =
            match instruction.BlendMode with
            | Some mode when mode <> BlendMode.Normal -> $" style=\"mix-blend-mode:{blendModeToSvg mode}\""
            | _ -> String.Empty

        let opacity =
            match instruction.Opacity with
            | Some value when value <> 1.0 ->
                let safeOpacity = safeNum value "layer.opacity"
                sprintf " opacity=\"%s\"" safeOpacity
            | _ -> String.Empty

        context.Elements.Add($"<g{idAttr instruction.Base.Id}{transformAttr instruction.Transform}{opacity}{filterAttr}{blendAttr}>")
        instruction.Children |> List.iter (fun child -> vm.Dispatch(child, context))
        context.Elements.Add("</g>")

    let private handleLine (instruction: PaintLine) (context: SvgContext) =
        let cap =
            match instruction.StrokeCap with
            | Some ("butt" | "round" | "square" as value) -> $" stroke-linecap=\"{value}\""
            | _ -> String.Empty

        let x1 = safeNum instruction.X1 "line.x1"
        let y1 = safeNum instruction.Y1 "line.y1"
        let x2 = safeNum instruction.X2 "line.x2"
        let y2 = safeNum instruction.Y2 "line.y2"
        let stroke = escAttr instruction.Stroke
        let strokeWidth = safeNum (defaultArg instruction.StrokeWidth 1.0) "line.stroke_width"
        context.Elements.Add(sprintf "<line%s x1=\"%s\" y1=\"%s\" x2=\"%s\" y2=\"%s\" stroke=\"%s\" stroke-width=\"%s\"%s fill=\"none\"/>" (idAttr instruction.Base.Id) x1 y1 x2 y2 stroke strokeWidth cap)

    let private handleClip (instruction: PaintClip) (context: SvgContext) (vm: PaintVM<SvgContext>) =
        let clipId =
            match instruction.Base.Id with
            | Some id -> $"clip-{id}"
            | None ->
                let id = context.ClipCounter
                context.ClipCounter <- context.ClipCounter + 1
                $"clip-{id}"

        let safeClipId = escAttr clipId
        let x = safeNum instruction.X "clip.x"
        let y = safeNum instruction.Y "clip.y"
        let width = safeNum instruction.Width "clip.width"
        let height = safeNum instruction.Height "clip.height"
        context.Defs.Add(sprintf "<clipPath id=\"%s\"><rect x=\"%s\" y=\"%s\" width=\"%s\" height=\"%s\"/></clipPath>" safeClipId x y width height)
        context.Elements.Add($"<g clip-path=\"url(#{escAttr clipId})\">")
        instruction.Children |> List.iter (fun child -> vm.Dispatch(child, context))
        context.Elements.Add("</g>")

    let private handleGradient (instruction: PaintGradient) (context: SvgContext) =
        match instruction.Base.Id with
        | None -> ()
        | Some id ->
            let stops =
                instruction.Stops
                |> List.mapi (fun index stop ->
                    let offset = safeNum stop.Offset (sprintf "gradient.stops[%d].offset" index)
                    let color = escAttr stop.Color
                    sprintf "<stop offset=\"%s\" stop-color=\"%s\"/>" offset color)
                |> String.concat String.Empty

            match instruction.GradientKind with
            | GradientKind.Linear ->
                let safeId = escAttr id
                let x1 = safeNum (defaultArg instruction.X1 0.0) "gradient.x1"
                let y1 = safeNum (defaultArg instruction.Y1 0.0) "gradient.y1"
                let x2 = safeNum (defaultArg instruction.X2 0.0) "gradient.x2"
                let y2 = safeNum (defaultArg instruction.Y2 0.0) "gradient.y2"
                context.Defs.Add(sprintf "<linearGradient id=\"%s\" x1=\"%s\" y1=\"%s\" x2=\"%s\" y2=\"%s\" gradientUnits=\"userSpaceOnUse\">%s</linearGradient>" safeId x1 y1 x2 y2 stops)
            | GradientKind.Radial ->
                let safeId = escAttr id
                let cx = safeNum (defaultArg instruction.Cx 0.0) "gradient.cx"
                let cy = safeNum (defaultArg instruction.Cy 0.0) "gradient.cy"
                let r = safeNum (defaultArg instruction.R 0.0) "gradient.r"
                context.Defs.Add(sprintf "<radialGradient id=\"%s\" cx=\"%s\" cy=\"%s\" r=\"%s\" gradientUnits=\"userSpaceOnUse\">%s</radialGradient>" safeId cx cy r stops)

    let private handleImage (instruction: PaintImage) (context: SvgContext) =
        let href =
            match instruction.Src with
            | ImageUri uri -> sanitizeImageHref uri
            | ImagePixels _ -> pixelImagePlaceholder

        let opacity =
            match instruction.Opacity with
            | Some value when value <> 1.0 ->
                let safeOpacity = safeNum value "image.opacity"
                sprintf " opacity=\"%s\"" safeOpacity
            | _ -> String.Empty

        let x = safeNum instruction.X "image.x"
        let y = safeNum instruction.Y "image.y"
        let width = safeNum instruction.Width "image.width"
        let height = safeNum instruction.Height "image.height"
        let safeHref = escAttr href
        context.Elements.Add(sprintf "<image%s x=\"%s\" y=\"%s\" width=\"%s\" height=\"%s\" href=\"%s\"%s/>" (idAttr instruction.Base.Id) x y width height safeHref opacity)

    let createSvgContext () = SvgContext()

    let createSvgVM () =
        let vm =
            PaintVM<SvgContext>(
                (fun context _ _ _ ->
                    context.Defs.Clear()
                    context.Elements.Clear()
                    context.ClipCounter <- 0
                    context.FilterCounter <- 0),
                (fun _ _ _ -> raise (ExportNotSupportedError("SVG"))))

        vm.Register(
            "rect",
            fun instruction context _ ->
                match instruction with
                | Rect rect -> handleRect rect context
                | _ -> ())

        vm.Register(
            "ellipse",
            fun instruction context _ ->
                match instruction with
                | Ellipse ellipse -> handleEllipse ellipse context
                | _ -> ())

        vm.Register(
            "path",
            fun instruction context _ ->
                match instruction with
                | Path path -> handlePath path context
                | _ -> ())

        vm.Register(
            "glyph_run",
            fun instruction context _ ->
                match instruction with
                | GlyphRun glyphRun -> handleGlyphRun glyphRun context
                | _ -> ())

        vm.Register(
            "group",
            fun instruction context innerVm ->
                match instruction with
                | Group group -> handleGroup group context innerVm
                | _ -> ())

        vm.Register(
            "layer",
            fun instruction context innerVm ->
                match instruction with
                | Layer layer -> handleLayer layer context innerVm
                | _ -> ())

        vm.Register(
            "line",
            fun instruction context _ ->
                match instruction with
                | Line line -> handleLine line context
                | _ -> ())

        vm.Register(
            "clip",
            fun instruction context innerVm ->
                match instruction with
                | Clip clip -> handleClip clip context innerVm
                | _ -> ())

        vm.Register(
            "gradient",
            fun instruction context _ ->
                match instruction with
                | Gradient gradient -> handleGradient gradient context
                | _ -> ())

        vm.Register(
            "image",
            fun instruction context _ ->
                match instruction with
                | Image image -> handleImage image context
                | _ -> ())

        vm

    let assembleSvg (scene: PaintScene) (context: SvgContext) =
        let width = safeNum scene.Width "scene.width"
        let height = safeNum scene.Height "scene.height"
        let parts = ResizeArray<string>()
        parts.Add($"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\">")

        if context.Defs.Count > 0 then
            parts.Add($"<defs>{String.concat String.Empty context.Defs}</defs>")

        if scene.Background <> "transparent" && scene.Background <> "none" then
            parts.Add($"<rect width=\"{width}\" height=\"{height}\" fill=\"{escAttr scene.Background}\"/>")

        context.Elements |> Seq.iter parts.Add
        parts.Add("</svg>")
        String.concat String.Empty parts

    let renderToSvgString (scene: PaintScene) =
        let context = createSvgContext ()
        let vm = createSvgVM ()
        vm.Execute(scene, context)
        assembleSvg scene context
