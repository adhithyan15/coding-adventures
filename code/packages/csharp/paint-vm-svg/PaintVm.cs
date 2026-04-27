using System.Globalization;
using System.Text;
using CodingAdventures.PaintInstructions;
using CodingAdventures.PaintVm;

namespace CodingAdventures.PaintVmSvg;

public static class PaintVmSvgPackage
{
    public const string VERSION = "0.1.0";
}

public sealed class SvgContext
{
    public List<string> Defs { get; } = [];

    public List<string> Elements { get; } = [];

    public int ClipCounter { get; set; }

    public int FilterCounter { get; set; }
}

/// <summary>
/// paint-vm-svg renders a PaintScene to a standalone SVG string without a DOM.
/// That keeps the backend portable across CLI tools, tests, and server-side
/// rendering environments.
/// </summary>
public static class PaintVmSvg
{
    private const string UnsafeImagePlaceholder = "data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==";
    private const string PixelImagePlaceholder = "data:image/png;base64,";

    public static SvgContext CreateSvgContext() => new();

    public static PaintVM<SvgContext> CreateSvgVm()
    {
        var vm = new PaintVM<SvgContext>(
            (context, _, _, _) =>
            {
                context.Defs.Clear();
                context.Elements.Clear();
                context.ClipCounter = 0;
                context.FilterCounter = 0;
            },
            (_, _, _) => throw new ExportNotSupportedError("SVG"));

        vm.Register("rect", (instruction, context, _) =>
        {
            if (instruction is PaintRect rect)
            {
                HandleRect(rect, context);
            }
        });

        vm.Register("ellipse", (instruction, context, _) =>
        {
            if (instruction is PaintEllipse ellipse)
            {
                HandleEllipse(ellipse, context);
            }
        });

        vm.Register("path", (instruction, context, _) =>
        {
            if (instruction is PaintPath path)
            {
                HandlePath(path, context);
            }
        });

        vm.Register("glyph_run", (instruction, context, _) =>
        {
            if (instruction is PaintGlyphRun glyphRun)
            {
                HandleGlyphRun(glyphRun, context);
            }
        });

        vm.Register("group", (instruction, context, innerVm) =>
        {
            if (instruction is PaintGroup group)
            {
                HandleGroup(group, context, innerVm);
            }
        });

        vm.Register("layer", (instruction, context, innerVm) =>
        {
            if (instruction is PaintLayer layer)
            {
                HandleLayer(layer, context, innerVm);
            }
        });

        vm.Register("line", (instruction, context, _) =>
        {
            if (instruction is PaintLine line)
            {
                HandleLine(line, context);
            }
        });

        vm.Register("clip", (instruction, context, innerVm) =>
        {
            if (instruction is PaintClip clip)
            {
                HandleClip(clip, context, innerVm);
            }
        });

        vm.Register("gradient", (instruction, context, _) =>
        {
            if (instruction is PaintGradient gradient)
            {
                HandleGradient(gradient, context);
            }
        });

        vm.Register("image", (instruction, context, _) =>
        {
            if (instruction is PaintImage image)
            {
                HandleImage(image, context);
            }
        });

        return vm;
    }

    public static string RenderToSvgString(PaintScene scene)
    {
        var context = CreateSvgContext();
        CreateSvgVm().Execute(scene, context);
        return AssembleSvg(scene, context);
    }

    public static string AssembleSvg(PaintScene scene, SvgContext context)
    {
        var width = SafeNum(scene.Width, "scene.width");
        var height = SafeNum(scene.Height, "scene.height");
        var parts = new List<string>
        {
            $"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\">",
        };

        if (context.Defs.Count > 0)
        {
            parts.Add($"<defs>{string.Concat(context.Defs)}</defs>");
        }

        if (scene.Background is not "transparent" and not "none")
        {
            parts.Add($"<rect width=\"{width}\" height=\"{height}\" fill=\"{EscAttr(scene.Background)}\"/>");
        }

        parts.AddRange(context.Elements);
        parts.Add("</svg>");
        return string.Concat(parts);
    }

    private static void HandleRect(PaintRect instruction, SvgContext context)
    {
        var radius = instruction.CornerRadius is double cornerRadius
            ? $" rx=\"{SafeNum(cornerRadius, "rect.corner_radius")}\""
            : string.Empty;
        context.Elements.Add(
            $"<rect{IdAttr(instruction.Id)} x=\"{SafeNum(instruction.X, "rect.x")}\" y=\"{SafeNum(instruction.Y, "rect.y")}\" width=\"{SafeNum(instruction.Width, "rect.width")}\" height=\"{SafeNum(instruction.Height, "rect.height")}\"{radius} {StrokeFillAttrs(instruction.Fill, instruction.Stroke, instruction.StrokeWidth)}/>");
    }

    private static void HandleEllipse(PaintEllipse instruction, SvgContext context)
    {
        context.Elements.Add(
            $"<ellipse{IdAttr(instruction.Id)} cx=\"{SafeNum(instruction.Cx, "ellipse.cx")}\" cy=\"{SafeNum(instruction.Cy, "ellipse.cy")}\" rx=\"{SafeNum(instruction.Rx, "ellipse.rx")}\" ry=\"{SafeNum(instruction.Ry, "ellipse.ry")}\" {StrokeFillAttrs(instruction.Fill, instruction.Stroke, instruction.StrokeWidth)}/>");
    }

    private static void HandlePath(PaintPath instruction, SvgContext context)
    {
        var fillRule = instruction.FillRule is "evenodd" ? " fill-rule=\"evenodd\"" : string.Empty;
        var cap = instruction.StrokeCap is "butt" or "round" or "square"
            ? $" stroke-linecap=\"{instruction.StrokeCap}\""
            : string.Empty;
        var join = instruction.StrokeJoin is "miter" or "round" or "bevel"
            ? $" stroke-linejoin=\"{instruction.StrokeJoin}\""
            : string.Empty;
        context.Elements.Add(
            $"<path{IdAttr(instruction.Id)} d=\"{EscAttr(CommandsToPathData(instruction.Commands))}\"{fillRule}{cap}{join} {StrokeFillAttrs(instruction.Fill, instruction.Stroke, instruction.StrokeWidth)}/>");
    }

    private static void HandleGlyphRun(PaintGlyphRun instruction, SvgContext context)
    {
        var spans = instruction.Glyphs.Select(glyph =>
        {
            var glyphId = glyph.GlyphId is >= 0 and <= 0x10ffff ? glyph.GlyphId : 0xfffd;
            return $"<tspan x=\"{SafeNum(glyph.X, "glyph.x")}\" y=\"{SafeNum(glyph.Y, "glyph.y")}\">&#{glyphId};</tspan>";
        });

        var fill = instruction.Fill ?? "#000000";
        context.Elements.Add(
            $"<text{IdAttr(instruction.Id)} font-size=\"{SafeNum(instruction.FontSize, "glyph_run.font_size")}\" fill=\"{EscAttr(fill)}\">{string.Concat(spans)}</text>");
    }

    private static void HandleGroup(PaintGroup instruction, SvgContext context, PaintVM<SvgContext> vm)
    {
        var opacity = instruction.Opacity is double value && value != 1.0
            ? $" opacity=\"{SafeNum(value, "group.opacity")}\""
            : string.Empty;
        context.Elements.Add($"<g{IdAttr(instruction.Id)}{TransformAttr(instruction.Transform)}{opacity}>");
        foreach (var child in instruction.Children)
        {
            vm.Dispatch(child, context);
        }

        context.Elements.Add("</g>");
    }

    private static void HandleLayer(PaintLayer instruction, SvgContext context, PaintVM<SvgContext> vm)
    {
        var filterId = instruction.Id is not null
            ? $"filter-{instruction.Id}"
            : $"filter-{context.FilterCounter++}";
        var filter = BuildSvgFilter(filterId, instruction.Filters);
        if (filter.Length > 0)
        {
            context.Defs.Add(filter);
        }

        var filterAttr = filter.Length > 0 ? $" filter=\"url(#{EscAttr(filterId)})\"" : string.Empty;
        var blendAttr = instruction.BlendMode is CodingAdventures.PaintInstructions.BlendMode blend &&
            blend != CodingAdventures.PaintInstructions.BlendMode.Normal
            ? $" style=\"mix-blend-mode:{BlendModeToSvg(blend)}\""
            : string.Empty;
        var opacity = instruction.Opacity is double value && value != 1.0
            ? $" opacity=\"{SafeNum(value, "layer.opacity")}\""
            : string.Empty;

        context.Elements.Add($"<g{IdAttr(instruction.Id)}{TransformAttr(instruction.Transform)}{opacity}{filterAttr}{blendAttr}>");
        foreach (var child in instruction.Children)
        {
            vm.Dispatch(child, context);
        }

        context.Elements.Add("</g>");
    }

    private static void HandleLine(PaintLine instruction, SvgContext context)
    {
        var cap = instruction.StrokeCap is "butt" or "round" or "square"
            ? $" stroke-linecap=\"{instruction.StrokeCap}\""
            : string.Empty;
        context.Elements.Add(
            $"<line{IdAttr(instruction.Id)} x1=\"{SafeNum(instruction.X1, "line.x1")}\" y1=\"{SafeNum(instruction.Y1, "line.y1")}\" x2=\"{SafeNum(instruction.X2, "line.x2")}\" y2=\"{SafeNum(instruction.Y2, "line.y2")}\" stroke=\"{EscAttr(instruction.Stroke)}\" stroke-width=\"{SafeNum(instruction.StrokeWidth ?? 1.0, "line.stroke_width")}\"{cap} fill=\"none\"/>");
    }

    private static void HandleClip(PaintClip instruction, SvgContext context, PaintVM<SvgContext> vm)
    {
        var clipId = instruction.Id is not null ? $"clip-{instruction.Id}" : $"clip-{context.ClipCounter++}";
        context.Defs.Add(
            $"<clipPath id=\"{EscAttr(clipId)}\"><rect x=\"{SafeNum(instruction.X, "clip.x")}\" y=\"{SafeNum(instruction.Y, "clip.y")}\" width=\"{SafeNum(instruction.Width, "clip.width")}\" height=\"{SafeNum(instruction.Height, "clip.height")}\"/></clipPath>");
        context.Elements.Add($"<g clip-path=\"url(#{EscAttr(clipId)})\">");
        foreach (var child in instruction.Children)
        {
            vm.Dispatch(child, context);
        }

        context.Elements.Add("</g>");
    }

    private static void HandleGradient(PaintGradient instruction, SvgContext context)
    {
        if (instruction.Id is null)
        {
            return;
        }

        var stops = string.Concat(instruction.Stops.Select((stop, index) =>
            $"<stop offset=\"{SafeNum(stop.Offset, $"gradient.stops[{index}].offset")}\" stop-color=\"{EscAttr(stop.Color)}\"/>"));

        if (instruction.GradientKind == GradientKind.Linear)
        {
            context.Defs.Add(
                $"<linearGradient id=\"{EscAttr(instruction.Id)}\" x1=\"{SafeNum(instruction.X1 ?? 0.0, "gradient.x1")}\" y1=\"{SafeNum(instruction.Y1 ?? 0.0, "gradient.y1")}\" x2=\"{SafeNum(instruction.X2 ?? 0.0, "gradient.x2")}\" y2=\"{SafeNum(instruction.Y2 ?? 0.0, "gradient.y2")}\" gradientUnits=\"userSpaceOnUse\">{stops}</linearGradient>");
            return;
        }

        context.Defs.Add(
            $"<radialGradient id=\"{EscAttr(instruction.Id)}\" cx=\"{SafeNum(instruction.Cx ?? 0.0, "gradient.cx")}\" cy=\"{SafeNum(instruction.Cy ?? 0.0, "gradient.cy")}\" r=\"{SafeNum(instruction.R ?? 0.0, "gradient.r")}\" gradientUnits=\"userSpaceOnUse\">{stops}</radialGradient>");
    }

    private static void HandleImage(PaintImage instruction, SvgContext context)
    {
        var href = instruction.Src switch
        {
            UriPaintImageSource uri => SanitizeImageHref(uri.Value),
            PixelPaintImageSource => PixelImagePlaceholder,
            _ => UnsafeImagePlaceholder,
        };

        var opacity = instruction.Opacity is double value && value != 1.0
            ? $" opacity=\"{SafeNum(value, "image.opacity")}\""
            : string.Empty;

        context.Elements.Add(
            $"<image{IdAttr(instruction.Id)} x=\"{SafeNum(instruction.X, "image.x")}\" y=\"{SafeNum(instruction.Y, "image.y")}\" width=\"{SafeNum(instruction.Width, "image.width")}\" height=\"{SafeNum(instruction.Height, "image.height")}\" href=\"{EscAttr(href)}\"{opacity}/>");
    }

    private static string BuildSvgFilter(string filterId, IReadOnlyList<FilterEffect>? filters)
    {
        if (filters is null || filters.Count == 0)
        {
            return string.Empty;
        }

        var primitives = new List<string>();
        var previous = "SourceGraphic";

        for (var index = 0; index < filters.Count; index++)
        {
            var filter = filters[index];
            var result = $"f{index}";

            switch (filter)
            {
                case BlurFilter blur:
                    primitives.Add(
                        $"<feGaussianBlur in=\"{previous}\" stdDeviation=\"{SafeNum(blur.Radius, "blur.radius")}\" result=\"{result}\"/>");
                    break;
                case DropShadowFilter dropShadow:
                    primitives.Add(
                        $"<feDropShadow dx=\"{SafeNum(dropShadow.Dx, "drop_shadow.dx")}\" dy=\"{SafeNum(dropShadow.Dy, "drop_shadow.dy")}\" stdDeviation=\"{SafeNum(dropShadow.Blur, "drop_shadow.blur")}\" flood-color=\"{EscAttr(dropShadow.Color)}\" result=\"{result}\"/>");
                    break;
                case ColorMatrixFilter colorMatrix:
                    var matrix = string.Join(" ", colorMatrix.Matrix.Select((value, matrixIndex) => SafeNum(value, $"color_matrix.matrix[{matrixIndex}]")));
                    primitives.Add(
                        $"<feColorMatrix in=\"{previous}\" type=\"matrix\" values=\"{matrix}\" result=\"{result}\"/>");
                    break;
                case BrightnessFilter brightness:
                    var brightnessAmount = SafeNum(brightness.Amount, "brightness.amount");
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncR type=\"linear\" slope=\"{brightnessAmount}\"/><feFuncG type=\"linear\" slope=\"{brightnessAmount}\"/><feFuncB type=\"linear\" slope=\"{brightnessAmount}\"/></feComponentTransfer>");
                    break;
                case ContrastFilter contrast:
                    var slope = SafeNum(contrast.Amount, "contrast.amount");
                    var intercept = SafeNum(-(contrast.Amount - 1.0) / 2.0, "contrast.intercept");
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncR type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/><feFuncG type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/><feFuncB type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/></feComponentTransfer>");
                    break;
                case SaturateFilter saturate:
                    primitives.Add(
                        $"<feColorMatrix in=\"{previous}\" type=\"saturate\" values=\"{SafeNum(saturate.Amount, "saturate.amount")}\" result=\"{result}\"/>");
                    break;
                case HueRotateFilter hueRotate:
                    primitives.Add(
                        $"<feColorMatrix in=\"{previous}\" type=\"hueRotate\" values=\"{SafeNum(hueRotate.Angle, "hue_rotate.angle")}\" result=\"{result}\"/>");
                    break;
                case InvertFilter invert:
                    var invertAmount = SafeNum(invert.Amount, "invert.amount");
                    var negativeAmount = SafeNum(-invert.Amount, "invert.neg_amount");
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncR type=\"linear\" slope=\"{negativeAmount}\" intercept=\"{invertAmount}\"/><feFuncG type=\"linear\" slope=\"{negativeAmount}\" intercept=\"{invertAmount}\"/><feFuncB type=\"linear\" slope=\"{negativeAmount}\" intercept=\"{invertAmount}\"/></feComponentTransfer>");
                    break;
                case OpacityFilter opacity:
                    primitives.Add(
                        $"<feComponentTransfer in=\"{previous}\" result=\"{result}\"><feFuncA type=\"linear\" slope=\"{SafeNum(opacity.Amount, "opacity.amount")}\"/></feComponentTransfer>");
                    break;
            }

            previous = result;
        }

        return $"<filter id=\"{EscAttr(filterId)}\">{string.Concat(primitives)}</filter>";
    }

    private static string StrokeFillAttrs(string? fill, string? stroke, double? strokeWidth)
    {
        var parts = new List<string>
        {
            $"fill=\"{EscAttr(fill ?? "none")}\"",
        };

        if (!string.IsNullOrWhiteSpace(stroke))
        {
            parts.Add($"stroke=\"{EscAttr(stroke)}\"");
            parts.Add($"stroke-width=\"{SafeNum(strokeWidth ?? 1.0, "stroke_width")}\"");
        }

        return string.Join(" ", parts);
    }

    private static string CommandsToPathData(IReadOnlyList<PathCommand> commands) =>
        string.Join(" ", commands.Select(command => command switch
        {
            MoveToCommand move => $"M {RoundPathNumber(move.X)} {RoundPathNumber(move.Y)}",
            LineToCommand line => $"L {RoundPathNumber(line.X)} {RoundPathNumber(line.Y)}",
            QuadToCommand quad => $"Q {RoundPathNumber(quad.Cx)} {RoundPathNumber(quad.Cy)} {RoundPathNumber(quad.X)} {RoundPathNumber(quad.Y)}",
            CubicToCommand cubic => $"C {RoundPathNumber(cubic.Cx1)} {RoundPathNumber(cubic.Cy1)} {RoundPathNumber(cubic.Cx2)} {RoundPathNumber(cubic.Cy2)} {RoundPathNumber(cubic.X)} {RoundPathNumber(cubic.Y)}",
            ArcToCommand arc => $"A {RoundPathNumber(arc.Rx)} {RoundPathNumber(arc.Ry)} {RoundPathNumber(arc.XRotation)} {(arc.LargeArc ? 1 : 0)} {(arc.Sweep ? 1 : 0)} {RoundPathNumber(arc.X)} {RoundPathNumber(arc.Y)}",
            ClosePathCommand => "Z",
            _ => throw new InvalidOperationException("Unknown path command"),
        }));

    private static string TransformAttr(Transform2D? transform)
    {
        if (transform is null)
        {
            return string.Empty;
        }

        return $" transform=\"matrix({SafeNum(transform.Value.A, "transform.a")},{SafeNum(transform.Value.B, "transform.b")},{SafeNum(transform.Value.C, "transform.c")},{SafeNum(transform.Value.D, "transform.d")},{SafeNum(transform.Value.E, "transform.e")},{SafeNum(transform.Value.F, "transform.f")})\"";
    }

    private static string SanitizeImageHref(string href)
    {
        var lower = href.ToLowerInvariant().TrimStart();
        if (lower.StartsWith("data:") || lower.StartsWith("https:") || lower.StartsWith("http:"))
        {
            return href;
        }

        return UnsafeImagePlaceholder;
    }

    private static string BlendModeToSvg(CodingAdventures.PaintInstructions.BlendMode mode) => mode switch
    {
        CodingAdventures.PaintInstructions.BlendMode.Normal => "normal",
        CodingAdventures.PaintInstructions.BlendMode.Multiply => "multiply",
        CodingAdventures.PaintInstructions.BlendMode.Screen => "screen",
        CodingAdventures.PaintInstructions.BlendMode.Overlay => "overlay",
        CodingAdventures.PaintInstructions.BlendMode.Darken => "darken",
        CodingAdventures.PaintInstructions.BlendMode.Lighten => "lighten",
        CodingAdventures.PaintInstructions.BlendMode.ColorDodge => "color-dodge",
        CodingAdventures.PaintInstructions.BlendMode.ColorBurn => "color-burn",
        CodingAdventures.PaintInstructions.BlendMode.HardLight => "hard-light",
        CodingAdventures.PaintInstructions.BlendMode.SoftLight => "soft-light",
        CodingAdventures.PaintInstructions.BlendMode.Difference => "difference",
        CodingAdventures.PaintInstructions.BlendMode.Exclusion => "exclusion",
        CodingAdventures.PaintInstructions.BlendMode.Hue => "hue",
        CodingAdventures.PaintInstructions.BlendMode.Saturation => "saturation",
        CodingAdventures.PaintInstructions.BlendMode.Color => "color",
        CodingAdventures.PaintInstructions.BlendMode.Luminosity => "luminosity",
        _ => "normal",
    };

    private static string SafeNum(double value, string field)
    {
        if (double.IsNaN(value) || double.IsInfinity(value))
        {
            throw new ArgumentOutOfRangeException(field, $"PaintVM SVG requires a finite number for {field}, got {value}.");
        }

        return value.ToString("0.############################", CultureInfo.InvariantCulture);
    }

    private static string RoundPathNumber(double value) =>
        Math.Round(value, 4, MidpointRounding.AwayFromZero).ToString("0.####", CultureInfo.InvariantCulture);

    private static string EscAttr(string value) =>
        value
            .Replace("&", "&amp;", StringComparison.Ordinal)
            .Replace("\"", "&quot;", StringComparison.Ordinal)
            .Replace("<", "&lt;", StringComparison.Ordinal)
            .Replace(">", "&gt;", StringComparison.Ordinal);

    private static string IdAttr(string? id) =>
        id is null ? string.Empty : $" id=\"{EscAttr(id)}\"";
}
