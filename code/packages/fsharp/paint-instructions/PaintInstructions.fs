namespace CodingAdventures.PaintInstructions

open System.Collections.Generic
open CodingAdventures.PixelContainer

type Metadata = IReadOnlyDictionary<string, obj>

[<Struct>]
type Transform2D =
    {
        A: float
        B: float
        C: float
        D: float
        E: float
        F: float
    }

type BlendMode =
    | Normal
    | Multiply
    | Screen
    | Overlay
    | Darken
    | Lighten
    | ColorDodge
    | ColorBurn
    | HardLight
    | SoftLight
    | Difference
    | Exclusion
    | Hue
    | Saturation
    | Color
    | Luminosity

type GradientKind =
    | Linear
    | Radial

type PathCommand =
    | MoveTo of x: float * y: float
    | LineTo of x: float * y: float
    | QuadTo of cx: float * cy: float * x: float * y: float
    | CubicTo of cx1: float * cy1: float * cx2: float * cy2: float * x: float * y: float
    | ArcTo of rx: float * ry: float * xRotation: float * largeArc: bool * sweep: bool * x: float * y: float
    | Close
    member this.Kind =
        match this with
        | MoveTo _ -> "move_to"
        | LineTo _ -> "line_to"
        | QuadTo _ -> "quad_to"
        | CubicTo _ -> "cubic_to"
        | ArcTo _ -> "arc_to"
        | Close -> "close"

type FilterEffect =
    | Blur of radius: float
    | DropShadow of dx: float * dy: float * blur: float * color: string
    | ColorMatrix of matrix: float list
    | Brightness of amount: float
    | Contrast of amount: float
    | Saturate of amount: float
    | HueRotate of angle: float
    | Invert of amount: float
    | Opacity of amount: float
    member this.Kind =
        match this with
        | Blur _ -> "blur"
        | DropShadow _ -> "drop_shadow"
        | ColorMatrix _ -> "color_matrix"
        | Brightness _ -> "brightness"
        | Contrast _ -> "contrast"
        | Saturate _ -> "saturate"
        | HueRotate _ -> "hue_rotate"
        | Invert _ -> "invert"
        | Opacity _ -> "opacity"

type PaintBase =
    {
        Id: string option
        Metadata: Metadata option
    }

type PaintGlyphPlacement =
    {
        GlyphId: int
        X: float
        Y: float
    }

type PaintGradientStop =
    {
        Offset: float
        Color: string
    }

type PaintRect =
    {
        Base: PaintBase
        X: float
        Y: float
        Width: float
        Height: float
        Fill: string option
        Stroke: string option
        StrokeWidth: float option
        CornerRadius: float option
    }

type PaintEllipse =
    {
        Base: PaintBase
        Cx: float
        Cy: float
        Rx: float
        Ry: float
        Fill: string option
        Stroke: string option
        StrokeWidth: float option
    }

type PaintPath =
    {
        Base: PaintBase
        Commands: PathCommand list
        Fill: string option
        FillRule: string option
        Stroke: string option
        StrokeWidth: float option
        StrokeCap: string option
        StrokeJoin: string option
    }

type PaintGlyphRun =
    {
        Base: PaintBase
        Glyphs: PaintGlyphPlacement list
        FontRef: string
        FontSize: float
        Fill: string option
    }

type PaintImageSource =
    | ImageUri of string
    | ImagePixels of PixelContainer

type PaintRectOptions =
    {
        Id: string option
        Metadata: Metadata option
        Fill: string option
        Stroke: string option
        StrokeWidth: float option
        CornerRadius: float option
    }

type PaintEllipseOptions =
    {
        Id: string option
        Metadata: Metadata option
        Fill: string option
        Stroke: string option
        StrokeWidth: float option
    }

type PaintPathOptions =
    {
        Id: string option
        Metadata: Metadata option
        Fill: string option
        FillRule: string option
        Stroke: string option
        StrokeWidth: float option
        StrokeCap: string option
        StrokeJoin: string option
    }

type PaintGlyphRunOptions =
    {
        Id: string option
        Metadata: Metadata option
        Fill: string option
    }

type PaintGroupOptions =
    {
        Id: string option
        Metadata: Metadata option
        Transform: Transform2D option
        Opacity: float option
    }

type PaintLayerOptions =
    {
        Id: string option
        Metadata: Metadata option
        Filters: FilterEffect list option
        BlendMode: BlendMode option
        Opacity: float option
        Transform: Transform2D option
    }

type PaintLineOptions =
    {
        Id: string option
        Metadata: Metadata option
        StrokeWidth: float option
        StrokeCap: string option
    }

type PaintClipOptions =
    {
        Id: string option
        Metadata: Metadata option
    }

type PaintGradientOptions =
    {
        Id: string option
        Metadata: Metadata option
        X1: float option
        Y1: float option
        X2: float option
        Y2: float option
        Cx: float option
        Cy: float option
        R: float option
    }

type PaintImageOptions =
    {
        Id: string option
        Metadata: Metadata option
        Opacity: float option
    }

type SceneOptions =
    {
        Id: string option
        Metadata: Metadata option
    }

type PaintScene =
    {
        Width: float
        Height: float
        Background: string
        Instructions: PaintInstruction list
        Id: string option
        Metadata: Metadata option
    }

and PaintGroup =
    {
        Base: PaintBase
        Children: PaintInstruction list
        Transform: Transform2D option
        Opacity: float option
    }

and PaintLayer =
    {
        Base: PaintBase
        Children: PaintInstruction list
        Filters: FilterEffect list option
        BlendMode: BlendMode option
        Opacity: float option
        Transform: Transform2D option
    }

and PaintLine =
    {
        Base: PaintBase
        X1: float
        Y1: float
        X2: float
        Y2: float
        Stroke: string
        StrokeWidth: float option
        StrokeCap: string option
    }

and PaintClip =
    {
        Base: PaintBase
        X: float
        Y: float
        Width: float
        Height: float
        Children: PaintInstruction list
    }

and PaintGradient =
    {
        Base: PaintBase
        GradientKind: GradientKind
        Stops: PaintGradientStop list
        X1: float option
        Y1: float option
        X2: float option
        Y2: float option
        Cx: float option
        Cy: float option
        R: float option
    }

and PaintImage =
    {
        Base: PaintBase
        X: float
        Y: float
        Width: float
        Height: float
        Src: PaintImageSource
        Opacity: float option
    }

and PaintInstruction =
    | Rect of PaintRect
    | Ellipse of PaintEllipse
    | Path of PaintPath
    | GlyphRun of PaintGlyphRun
    | Group of PaintGroup
    | Layer of PaintLayer
    | Line of PaintLine
    | Clip of PaintClip
    | Gradient of PaintGradient
    | Image of PaintImage
    member this.Kind =
        match this with
        | Rect _ -> "rect"
        | Ellipse _ -> "ellipse"
        | Path _ -> "path"
        | GlyphRun _ -> "glyph_run"
        | Group _ -> "group"
        | Layer _ -> "layer"
        | Line _ -> "line"
        | Clip _ -> "clip"
        | Gradient _ -> "gradient"
        | Image _ -> "image"
    member this.Id =
        match this with
        | Rect value -> value.Base.Id
        | Ellipse value -> value.Base.Id
        | Path value -> value.Base.Id
        | GlyphRun value -> value.Base.Id
        | Group value -> value.Base.Id
        | Layer value -> value.Base.Id
        | Line value -> value.Base.Id
        | Clip value -> value.Base.Id
        | Gradient value -> value.Base.Id
        | Image value -> value.Base.Id

[<RequireQualifiedAccess>]
module PaintInstructions =
    [<Literal>]
    let VERSION = "0.1.0"

    let defaultSceneOptions : SceneOptions = { Id = None; Metadata = None }

    let defaultPaintRectOptions : PaintRectOptions =
        {
            Id = None
            Metadata = None
            Fill = None
            Stroke = None
            StrokeWidth = None
            CornerRadius = None
        }

    let defaultPaintEllipseOptions : PaintEllipseOptions =
        {
            Id = None
            Metadata = None
            Fill = None
            Stroke = None
            StrokeWidth = None
        }

    let defaultPaintPathOptions : PaintPathOptions =
        {
            Id = None
            Metadata = None
            Fill = None
            FillRule = None
            Stroke = None
            StrokeWidth = None
            StrokeCap = None
            StrokeJoin = None
        }

    let defaultPaintGlyphRunOptions : PaintGlyphRunOptions = { Id = None; Metadata = None; Fill = None }

    let defaultPaintGroupOptions : PaintGroupOptions =
        {
            Id = None
            Metadata = None
            Transform = None
            Opacity = None
        }

    let defaultPaintLayerOptions : PaintLayerOptions =
        {
            Id = None
            Metadata = None
            Filters = None
            BlendMode = None
            Opacity = None
            Transform = None
        }

    let defaultPaintLineOptions : PaintLineOptions =
        {
            Id = None
            Metadata = None
            StrokeWidth = None
            StrokeCap = None
        }

    let defaultPaintClipOptions : PaintClipOptions = { Id = None; Metadata = None }

    let defaultPaintGradientOptions : PaintGradientOptions =
        {
            Id = None
            Metadata = None
            X1 = None
            Y1 = None
            X2 = None
            Y2 = None
            Cx = None
            Cy = None
            R = None
        }

    let defaultPaintImageOptions : PaintImageOptions = { Id = None; Metadata = None; Opacity = None }

    let private makeBase (id: string option) (metadata: Metadata option) : PaintBase =
        { Id = id; Metadata = metadata }

    let rec paintScene width height background instructions =
        paintSceneWith defaultSceneOptions width height background instructions

    and paintSceneWith (options: SceneOptions) width height background instructions : PaintScene =
        ({
            Width = width
            Height = height
            Background = background
            Instructions = instructions
            Id = options.Id
            Metadata = options.Metadata
        }: PaintScene)

    and paintRect x y width height =
        paintRectWith defaultPaintRectOptions x y width height

    and paintRectWith (options: PaintRectOptions) x y width height : PaintInstruction =
        Rect
            ({
                Base = makeBase options.Id options.Metadata
                X = x
                Y = y
                Width = width
                Height = height
                Fill = options.Fill
                Stroke = options.Stroke
                StrokeWidth = options.StrokeWidth
                CornerRadius = options.CornerRadius
            }: PaintRect)

    and paintEllipse cx cy rx ry =
        paintEllipseWith defaultPaintEllipseOptions cx cy rx ry

    and paintEllipseWith (options: PaintEllipseOptions) cx cy rx ry : PaintInstruction =
        Ellipse
            ({
                Base = makeBase options.Id options.Metadata
                Cx = cx
                Cy = cy
                Rx = rx
                Ry = ry
                Fill = options.Fill
                Stroke = options.Stroke
                StrokeWidth = options.StrokeWidth
            }: PaintEllipse)

    and paintPath commands =
        paintPathWith defaultPaintPathOptions commands

    and paintPathWith (options: PaintPathOptions) (commands: PathCommand list) : PaintInstruction =
        Path
            ({
                Base = makeBase options.Id options.Metadata
                Commands = commands
                Fill = options.Fill
                FillRule = options.FillRule
                Stroke = options.Stroke
                StrokeWidth = options.StrokeWidth
                StrokeCap = options.StrokeCap
                StrokeJoin = options.StrokeJoin
            }: PaintPath)

    and paintGlyphRun glyphs fontRef fontSize =
        paintGlyphRunWith defaultPaintGlyphRunOptions glyphs fontRef fontSize

    and paintGlyphRunWith (options: PaintGlyphRunOptions) (glyphs: PaintGlyphPlacement list) fontRef fontSize : PaintInstruction =
        GlyphRun
            ({
                Base = makeBase options.Id options.Metadata
                Glyphs = glyphs
                FontRef = fontRef
                FontSize = fontSize
                Fill = options.Fill
            }: PaintGlyphRun)

    and paintGroup children =
        paintGroupWith defaultPaintGroupOptions children

    and paintGroupWith (options: PaintGroupOptions) (children: PaintInstruction list) : PaintInstruction =
        Group
            ({
                Base = makeBase options.Id options.Metadata
                Children = children
                Transform = options.Transform
                Opacity = options.Opacity
            }: PaintGroup)

    and paintLayer children =
        paintLayerWith defaultPaintLayerOptions children

    and paintLayerWith (options: PaintLayerOptions) (children: PaintInstruction list) : PaintInstruction =
        Layer
            ({
                Base = makeBase options.Id options.Metadata
                Children = children
                Filters = options.Filters
                BlendMode = options.BlendMode
                Opacity = options.Opacity
                Transform = options.Transform
            }: PaintLayer)

    and paintLine x1 y1 x2 y2 stroke =
        paintLineWith defaultPaintLineOptions x1 y1 x2 y2 stroke

    and paintLineWith (options: PaintLineOptions) x1 y1 x2 y2 stroke : PaintInstruction =
        Line
            ({
                Base = makeBase options.Id options.Metadata
                X1 = x1
                Y1 = y1
                X2 = x2
                Y2 = y2
                Stroke = stroke
                StrokeWidth = options.StrokeWidth
                StrokeCap = options.StrokeCap
            }: PaintLine)

    and paintClip x y width height children =
        paintClipWith defaultPaintClipOptions x y width height children

    and paintClipWith (options: PaintClipOptions) x y width height (children: PaintInstruction list) : PaintInstruction =
        Clip
            ({
                Base = makeBase options.Id options.Metadata
                X = x
                Y = y
                Width = width
                Height = height
                Children = children
            }: PaintClip)

    and paintGradient gradientKind stops =
        paintGradientWith defaultPaintGradientOptions gradientKind stops

    and paintGradientWith (options: PaintGradientOptions) gradientKind (stops: PaintGradientStop list) : PaintInstruction =
        Gradient
            ({
                Base = makeBase options.Id options.Metadata
                GradientKind = gradientKind
                Stops = stops
                X1 = options.X1
                Y1 = options.Y1
                X2 = options.X2
                Y2 = options.Y2
                Cx = options.Cx
                Cy = options.Cy
                R = options.R
            }: PaintGradient)

    and paintImage x y width height src =
        paintImageWith defaultPaintImageOptions x y width height src

    and paintImageWith (options: PaintImageOptions) x y width height (src: PaintImageSource) : PaintInstruction =
        Image
            ({
                Base = makeBase options.Id options.Metadata
                X = x
                Y = y
                Width = width
                Height = height
                Src = src
                Opacity = options.Opacity
            }: PaintImage)
