using CodingAdventures.PixelContainer;
using static CodingAdventures.PaintInstructions.PaintInstructions;

namespace CodingAdventures.PaintInstructions.Tests;

public sealed class PaintInstructionsTests
{
    [Fact]
    public void Version_IsSemver()
    {
        Assert.Equal("0.1.0", PaintInstructions.VERSION);
    }

    [Fact]
    public void PaintRect_CreatesMinimalRectangle()
    {
        var rect = PaintRect(10, 20, 100, 50);

        Assert.Equal("rect", rect.Kind);
        Assert.Equal(10, rect.X);
        Assert.Equal(20, rect.Y);
        Assert.Equal(100, rect.Width);
        Assert.Equal(50, rect.Height);
        Assert.Null(rect.Fill);
    }

    [Fact]
    public void PaintRect_AppliesOptionalFields()
    {
        var rect = PaintRect(0, 0, 20, 30, new PaintRectOptions
        {
            Fill = "#2563eb",
            Stroke = "#ffffff",
            StrokeWidth = 2,
            CornerRadius = 8,
            Id = "card-bg",
            Metadata = new Dictionary<string, object?> { ["source"] = "chart-bar-3" },
        });

        Assert.Equal("#2563eb", rect.Fill);
        Assert.Equal("#ffffff", rect.Stroke);
        Assert.Equal(2, rect.StrokeWidth);
        Assert.Equal(8, rect.CornerRadius);
        Assert.Equal("card-bg", rect.Id);
        Assert.Equal("chart-bar-3", rect.Metadata?["source"]);
    }

    [Fact]
    public void PaintPath_PreservesCommandsAndStrokeSettings()
    {
        var path = PaintPath(
            [
                new MoveToCommand(0, 0),
                new LineToCommand(100, 0),
                new ClosePathCommand(),
            ],
            new PaintPathOptions
            {
                Fill = "#ef4444",
                FillRule = "evenodd",
                Stroke = "#111111",
                StrokeCap = "round",
                StrokeJoin = "bevel",
            });

        Assert.Equal("path", path.Kind);
        Assert.Equal(3, path.Commands.Count);
        Assert.Equal("move_to", path.Commands[0].Kind);
        Assert.Equal("close", path.Commands[2].Kind);
        Assert.Equal("evenodd", path.FillRule);
        Assert.Equal("round", path.StrokeCap);
        Assert.Equal("bevel", path.StrokeJoin);
    }

    [Fact]
    public void PathCommands_ExposeStableKindsForAllVariants()
    {
        PathCommand[] commands =
        [
            new MoveToCommand(0, 0),
            new LineToCommand(1, 1),
            new QuadToCommand(2, 3, 4, 5),
            new CubicToCommand(1, 2, 3, 4, 5, 6),
            new ArcToCommand(7, 8, 45, true, false, 9, 10),
            new ClosePathCommand(),
        ];

        Assert.Equal(
            ["move_to", "line_to", "quad_to", "cubic_to", "arc_to", "close"],
            commands.Select(command => command.Kind).ToArray());
    }

    [Fact]
    public void FilterEffects_ExposeStableKindsForAllVariants()
    {
        FilterEffect[] filters =
        [
            new BlurFilter(1),
            new DropShadowFilter(2, 3, 4, "#000"),
            new ColorMatrixFilter([1, 0, 0, 0, 0]),
            new BrightnessFilter(1.1),
            new ContrastFilter(0.9),
            new SaturateFilter(0.8),
            new HueRotateFilter(120),
            new InvertFilter(0.5),
            new OpacityFilter(0.25),
        ];

        Assert.Equal(
            ["blur", "drop_shadow", "color_matrix", "brightness", "contrast", "saturate", "hue_rotate", "invert", "opacity"],
            filters.Select(filter => filter.Kind).ToArray());
    }

    [Fact]
    public void PaintEllipse_AppliesOptionalFields()
    {
        var ellipse = PaintEllipse(30, 40, 15, 10, new PaintEllipseOptions
        {
            Fill = "#10b981",
            Stroke = "#064e3b",
            StrokeWidth = 1.5,
            Id = "orbit",
        });

        Assert.Equal("ellipse", ellipse.Kind);
        Assert.Equal("#10b981", ellipse.Fill);
        Assert.Equal("#064e3b", ellipse.Stroke);
        Assert.Equal(1.5, ellipse.StrokeWidth);
        Assert.Equal("orbit", ellipse.Id);
    }

    [Fact]
    public void PaintGlyphRun_StoresPlacementsAndFill()
    {
        var glyphRun = PaintGlyphRun(
            [new PaintGlyphPlacement(65, 10, 20), new PaintGlyphPlacement(66, 22, 20)],
            "font://plex-sans",
            18,
            new PaintGlyphRunOptions
            {
                Fill = "#111827",
                Id = "title",
            });

        Assert.Equal("glyph_run", glyphRun.Kind);
        Assert.Equal("font://plex-sans", glyphRun.FontRef);
        Assert.Equal(18, glyphRun.FontSize);
        Assert.Equal(2, glyphRun.Glyphs.Count);
        Assert.Equal("#111827", glyphRun.Fill);
        Assert.Equal("title", glyphRun.Id);
    }

    [Fact]
    public void PaintLine_AppliesStrokePresentationOptions()
    {
        var line = PaintLine(0, 1, 20, 21, "#334155", new PaintLineOptions
        {
            StrokeWidth = 3,
            StrokeCap = "square",
            Id = "baseline",
        });

        Assert.Equal("line", line.Kind);
        Assert.Equal("#334155", line.Stroke);
        Assert.Equal(3, line.StrokeWidth);
        Assert.Equal("square", line.StrokeCap);
        Assert.Equal("baseline", line.Id);
    }

    [Fact]
    public void PaintGroup_StoresChildrenTransformAndOpacity()
    {
        var group = PaintGroup(
            [PaintRect(0, 0, 10, 10)],
            new PaintGroupOptions
            {
                Transform = new Transform2D(1, 0, 0, 1, 100, 50),
                Opacity = 0.5,
            });

        Assert.Equal("group", group.Kind);
        Assert.Single(group.Children);
        Assert.Equal(new Transform2D(1, 0, 0, 1, 100, 50), group.Transform);
        Assert.Equal(0.5, group.Opacity);
    }

    [Fact]
    public void PaintLayer_StoresFiltersAndBlendMode()
    {
        var layer = PaintLayer(
            [PaintRect(0, 0, 10, 10)],
            new PaintLayerOptions
            {
                Filters =
                [
                    new BlurFilter(10),
                    new BrightnessFilter(1.2),
                ],
                BlendMode = BlendMode.Multiply,
                Opacity = 0.7,
            });

        Assert.Equal("layer", layer.Kind);
        Assert.Equal(2, layer.Filters?.Count);
        Assert.Equal("blur", layer.Filters?[0].Kind);
        Assert.Equal("brightness", layer.Filters?[1].Kind);
        Assert.Equal(BlendMode.Multiply, layer.BlendMode);
        Assert.Equal(0.7, layer.Opacity);
    }

    [Fact]
    public void PaintClip_StoresRectangleAndChildren()
    {
        var clip = PaintClip(0, 0, 400, 300, [PaintRect(-10, -10, 420, 320)]);

        Assert.Equal("clip", clip.Kind);
        Assert.Equal(400, clip.Width);
        Assert.Equal(300, clip.Height);
        Assert.Single(clip.Children);
    }

    [Fact]
    public void PaintGradient_SupportsLinearGeometry()
    {
        var gradient = PaintGradient(
            GradientKind.Linear,
            [new PaintGradientStop(0, "#3b82f6"), new PaintGradientStop(1, "#8b5cf6")],
            new PaintGradientOptions
            {
                Id = "blue-purple",
                X1 = 0,
                Y1 = 0,
                X2 = 400,
                Y2 = 0,
            });

        Assert.Equal("gradient", gradient.Kind);
        Assert.Equal(GradientKind.Linear, gradient.GradientKind);
        Assert.Equal(2, gradient.Stops.Count);
        Assert.Equal("blue-purple", gradient.Id);
        Assert.Equal(400, gradient.X2);
    }

    [Fact]
    public void PaintGradient_SupportsRadialGeometry()
    {
        var gradient = PaintGradient(
            GradientKind.Radial,
            [new PaintGradientStop(0.25, "#f59e0b"), new PaintGradientStop(1.0, "#7c2d12")],
            new PaintGradientOptions
            {
                Cx = 50,
                Cy = 60,
                R = 25,
            });

        Assert.Equal(GradientKind.Radial, gradient.GradientKind);
        Assert.Equal(50, gradient.Cx);
        Assert.Equal(60, gradient.Cy);
        Assert.Equal(25, gradient.R);
    }

    [Fact]
    public void PaintImage_AcceptsUriSources()
    {
        var image = PaintImage(10, 20, 300, 200, "file:///assets/logo.png", new PaintImageOptions { Opacity = 0.5 });

        Assert.Equal("image", image.Kind);
        Assert.IsType<UriPaintImageSource>(image.Src);
        Assert.Equal(0.5, image.Opacity);
    }

    [Fact]
    public void PaintImage_AcceptsDecodedPixelBuffers()
    {
        var pixels = PixelContainers.Create(2, 2);
        var image = PaintImage(0, 0, 20, 20, pixels, new PaintImageOptions
        {
            Opacity = 0.75,
            Id = "embedded",
            Metadata = new Dictionary<string, object?> { ["purpose"] = "preview" },
        });

        var src = Assert.IsType<PixelPaintImageSource>(image.Src);
        Assert.Same(pixels, src.Value);
        Assert.Equal(0.75, image.Opacity);
        Assert.Equal("embedded", image.Id);
        Assert.Equal("preview", image.Metadata?["purpose"]);
    }

    [Fact]
    public void PaintScene_StoresOrderedInstructions()
    {
        var scene = PaintScene(
            800,
            600,
            "#f8fafc",
            [
                PaintRect(0, 0, 100, 50),
                PaintEllipse(50, 50, 20, 20),
            ],
            new SceneOptions { Id = "chart" });

        Assert.Equal(800, scene.Width);
        Assert.Equal(600, scene.Height);
        Assert.Equal("#f8fafc", scene.Background);
        Assert.Equal("chart", scene.Id);
        Assert.Equal(new[] { "rect", "ellipse" }, scene.Instructions.Select(instruction => instruction.Kind).ToArray());
    }
}
