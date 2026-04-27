using PixelBuffer = CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.PaintInstructions;

/// <summary>
/// A six-value affine transform that matches the Canvas and SVG conventions.
/// </summary>
public readonly record struct Transform2D(double A, double B, double C, double D, double E, double F);

public enum BlendMode
{
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

public enum GradientKind
{
    Linear,
    Radial,
}

public abstract record PathCommand
{
    public abstract string Kind { get; }
}

public sealed record MoveToCommand(double X, double Y) : PathCommand
{
    public override string Kind => "move_to";
}

public sealed record LineToCommand(double X, double Y) : PathCommand
{
    public override string Kind => "line_to";
}

public sealed record QuadToCommand(double Cx, double Cy, double X, double Y) : PathCommand
{
    public override string Kind => "quad_to";
}

public sealed record CubicToCommand(double Cx1, double Cy1, double Cx2, double Cy2, double X, double Y) : PathCommand
{
    public override string Kind => "cubic_to";
}

public sealed record ArcToCommand(double Rx, double Ry, double XRotation, bool LargeArc, bool Sweep, double X, double Y) : PathCommand
{
    public override string Kind => "arc_to";
}

public sealed record ClosePathCommand() : PathCommand
{
    public override string Kind => "close";
}

public abstract record FilterEffect
{
    public abstract string Kind { get; }
}

public sealed record BlurFilter(double Radius) : FilterEffect
{
    public override string Kind => "blur";
}

public sealed record DropShadowFilter(double Dx, double Dy, double Blur, string Color) : FilterEffect
{
    public override string Kind => "drop_shadow";
}

public sealed record ColorMatrixFilter(IReadOnlyList<double> Matrix) : FilterEffect
{
    public override string Kind => "color_matrix";
}

public sealed record BrightnessFilter(double Amount) : FilterEffect
{
    public override string Kind => "brightness";
}

public sealed record ContrastFilter(double Amount) : FilterEffect
{
    public override string Kind => "contrast";
}

public sealed record SaturateFilter(double Amount) : FilterEffect
{
    public override string Kind => "saturate";
}

public sealed record HueRotateFilter(double Angle) : FilterEffect
{
    public override string Kind => "hue_rotate";
}

public sealed record InvertFilter(double Amount) : FilterEffect
{
    public override string Kind => "invert";
}

public sealed record OpacityFilter(double Amount) : FilterEffect
{
    public override string Kind => "opacity";
}

public sealed record PaintGlyphPlacement(int GlyphId, double X, double Y);

public sealed record PaintGradientStop(double Offset, string Color);

public abstract record PaintInstructionBase
{
    public string? Id { get; init; }

    public IReadOnlyDictionary<string, object?>? Metadata { get; init; }

    public abstract string Kind { get; }
}

public sealed record PaintRect(double X, double Y, double Width, double Height) : PaintInstructionBase
{
    public override string Kind => "rect";

    public string? Fill { get; init; }

    public string? Stroke { get; init; }

    public double? StrokeWidth { get; init; }

    public double? CornerRadius { get; init; }
}

public sealed record PaintEllipse(double Cx, double Cy, double Rx, double Ry) : PaintInstructionBase
{
    public override string Kind => "ellipse";

    public string? Fill { get; init; }

    public string? Stroke { get; init; }

    public double? StrokeWidth { get; init; }
}

public sealed record PaintPath(IReadOnlyList<PathCommand> Commands) : PaintInstructionBase
{
    public override string Kind => "path";

    public string? Fill { get; init; }

    public string? FillRule { get; init; }

    public string? Stroke { get; init; }

    public double? StrokeWidth { get; init; }

    public string? StrokeCap { get; init; }

    public string? StrokeJoin { get; init; }
}

public sealed record PaintGlyphRun(IReadOnlyList<PaintGlyphPlacement> Glyphs, string FontRef, double FontSize) : PaintInstructionBase
{
    public override string Kind => "glyph_run";

    public string? Fill { get; init; }
}

public sealed record PaintGroup(IReadOnlyList<PaintInstructionBase> Children) : PaintInstructionBase
{
    public override string Kind => "group";

    public Transform2D? Transform { get; init; }

    public double? Opacity { get; init; }
}

public sealed record PaintLayer(IReadOnlyList<PaintInstructionBase> Children) : PaintInstructionBase
{
    public override string Kind => "layer";

    public IReadOnlyList<FilterEffect>? Filters { get; init; }

    public BlendMode? BlendMode { get; init; }

    public double? Opacity { get; init; }

    public Transform2D? Transform { get; init; }
}

public sealed record PaintLine(double X1, double Y1, double X2, double Y2, string Stroke) : PaintInstructionBase
{
    public override string Kind => "line";

    public double? StrokeWidth { get; init; }

    public string? StrokeCap { get; init; }
}

public sealed record PaintClip(double X, double Y, double Width, double Height, IReadOnlyList<PaintInstructionBase> Children) : PaintInstructionBase
{
    public override string Kind => "clip";
}

public sealed record PaintGradient(GradientKind GradientKind, IReadOnlyList<PaintGradientStop> Stops) : PaintInstructionBase
{
    public override string Kind => "gradient";

    public double? X1 { get; init; }

    public double? Y1 { get; init; }

    public double? X2 { get; init; }

    public double? Y2 { get; init; }

    public double? Cx { get; init; }

    public double? Cy { get; init; }

    public double? R { get; init; }
}

public abstract record PaintImageSource;

public sealed record UriPaintImageSource(string Value) : PaintImageSource;

public sealed record PixelPaintImageSource(PixelBuffer Value) : PaintImageSource;

public sealed record PaintImage(double X, double Y, double Width, double Height, PaintImageSource Src) : PaintInstructionBase
{
    public override string Kind => "image";

    public double? Opacity { get; init; }
}

public sealed record PaintScene(double Width, double Height, string Background, IReadOnlyList<PaintInstructionBase> Instructions)
{
    public string? Id { get; init; }

    public IReadOnlyDictionary<string, object?>? Metadata { get; init; }
}

public record SceneOptions
{
    public string? Id { get; init; }

    public IReadOnlyDictionary<string, object?>? Metadata { get; init; }
}

public record InstructionOptions
{
    public string? Id { get; init; }

    public IReadOnlyDictionary<string, object?>? Metadata { get; init; }
}

public sealed record PaintRectOptions : InstructionOptions
{
    public string? Fill { get; init; }

    public string? Stroke { get; init; }

    public double? StrokeWidth { get; init; }

    public double? CornerRadius { get; init; }
}

public sealed record PaintEllipseOptions : InstructionOptions
{
    public string? Fill { get; init; }

    public string? Stroke { get; init; }

    public double? StrokeWidth { get; init; }
}

public sealed record PaintPathOptions : InstructionOptions
{
    public string? Fill { get; init; }

    public string? FillRule { get; init; }

    public string? Stroke { get; init; }

    public double? StrokeWidth { get; init; }

    public string? StrokeCap { get; init; }

    public string? StrokeJoin { get; init; }
}

public sealed record PaintGlyphRunOptions : InstructionOptions
{
    public string? Fill { get; init; }
}

public sealed record PaintGroupOptions : InstructionOptions
{
    public Transform2D? Transform { get; init; }

    public double? Opacity { get; init; }
}

public sealed record PaintLayerOptions : InstructionOptions
{
    public IReadOnlyList<FilterEffect>? Filters { get; init; }

    public BlendMode? BlendMode { get; init; }

    public double? Opacity { get; init; }

    public Transform2D? Transform { get; init; }
}

public sealed record PaintLineOptions : InstructionOptions
{
    public double? StrokeWidth { get; init; }

    public string? StrokeCap { get; init; }
}

public sealed record PaintClipOptions : InstructionOptions;

public sealed record PaintGradientOptions : InstructionOptions
{
    public double? X1 { get; init; }

    public double? Y1 { get; init; }

    public double? X2 { get; init; }

    public double? Y2 { get; init; }

    public double? Cx { get; init; }

    public double? Cy { get; init; }

    public double? R { get; init; }
}

public sealed record PaintImageOptions : InstructionOptions
{
    public double? Opacity { get; init; }
}

/// <summary>
/// Builder helpers that keep the creation syntax compact while still returning
/// plain records that backends can inspect and pattern-match.
/// </summary>
public static class PaintInstructions
{
    public const string VERSION = "0.1.0";

    public static PaintScene PaintScene(
        double width,
        double height,
        string background,
        IReadOnlyList<PaintInstructionBase> instructions,
        SceneOptions? options = null) =>
        new(width, height, background, instructions)
        {
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintRect PaintRect(double x, double y, double width, double height, PaintRectOptions? options = null) =>
        new(x, y, width, height)
        {
            Fill = options?.Fill,
            Stroke = options?.Stroke,
            StrokeWidth = options?.StrokeWidth,
            CornerRadius = options?.CornerRadius,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintEllipse PaintEllipse(double cx, double cy, double rx, double ry, PaintEllipseOptions? options = null) =>
        new(cx, cy, rx, ry)
        {
            Fill = options?.Fill,
            Stroke = options?.Stroke,
            StrokeWidth = options?.StrokeWidth,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintPath PaintPath(IReadOnlyList<PathCommand> commands, PaintPathOptions? options = null) =>
        new(commands)
        {
            Fill = options?.Fill,
            FillRule = options?.FillRule,
            Stroke = options?.Stroke,
            StrokeWidth = options?.StrokeWidth,
            StrokeCap = options?.StrokeCap,
            StrokeJoin = options?.StrokeJoin,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintGlyphRun PaintGlyphRun(IReadOnlyList<PaintGlyphPlacement> glyphs, string fontRef, double fontSize, PaintGlyphRunOptions? options = null) =>
        new(glyphs, fontRef, fontSize)
        {
            Fill = options?.Fill,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintLine PaintLine(double x1, double y1, double x2, double y2, string stroke, PaintLineOptions? options = null) =>
        new(x1, y1, x2, y2, stroke)
        {
            StrokeWidth = options?.StrokeWidth,
            StrokeCap = options?.StrokeCap,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintGroup PaintGroup(IReadOnlyList<PaintInstructionBase> children, PaintGroupOptions? options = null) =>
        new(children)
        {
            Transform = options?.Transform,
            Opacity = options?.Opacity,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintLayer PaintLayer(IReadOnlyList<PaintInstructionBase> children, PaintLayerOptions? options = null) =>
        new(children)
        {
            Filters = options?.Filters,
            BlendMode = options?.BlendMode,
            Opacity = options?.Opacity,
            Transform = options?.Transform,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintClip PaintClip(double x, double y, double width, double height, IReadOnlyList<PaintInstructionBase> children, PaintClipOptions? options = null) =>
        new(x, y, width, height, children)
        {
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintGradient PaintGradient(GradientKind gradientKind, IReadOnlyList<PaintGradientStop> stops, PaintGradientOptions? options = null) =>
        new(gradientKind, stops)
        {
            X1 = options?.X1,
            Y1 = options?.Y1,
            X2 = options?.X2,
            Y2 = options?.Y2,
            Cx = options?.Cx,
            Cy = options?.Cy,
            R = options?.R,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };

    public static PaintImage PaintImage(double x, double y, double width, double height, string src, PaintImageOptions? options = null) =>
        CreatePaintImage(x, y, width, height, new UriPaintImageSource(src), options);

    public static PaintImage PaintImage(double x, double y, double width, double height, PixelBuffer src, PaintImageOptions? options = null) =>
        CreatePaintImage(x, y, width, height, new PixelPaintImageSource(src), options);

    private static PaintImage CreatePaintImage(double x, double y, double width, double height, PaintImageSource src, PaintImageOptions? options) =>
        new(x, y, width, height, src)
        {
            Opacity = options?.Opacity,
            Id = options?.Id,
            Metadata = options?.Metadata,
        };
}
