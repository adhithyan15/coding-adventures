using CodingAdventures.PaintInstructions;
using CodingAdventures.PaintVm;
using CodingAdventures.PaintVmSvg;
using static CodingAdventures.PaintInstructions.PaintInstructions;

namespace CodingAdventures.PaintVmSvg.Tests;

public sealed class PaintVmTests
{
    [Fact]
    public void Version_IsSemver()
    {
        Assert.Equal("0.1.0", PaintVmSvgPackage.VERSION);
    }

    [Fact]
    public void RenderToSvgString_EmitsSvgRootAndBackground()
    {
        var svg = PaintVmSvg.RenderToSvgString(PaintScene(100, 80, "#f8fafc", []));

        Assert.StartsWith("<svg", svg);
        Assert.Contains("xmlns=\"http://www.w3.org/2000/svg\"", svg);
        Assert.Contains("fill=\"#f8fafc\"", svg);
        Assert.EndsWith("</svg>", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsRectAttributes()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(200, 100, "transparent", [PaintRect(10, 20, 50, 30, new PaintRectOptions { Fill = "#ef4444", CornerRadius = 8, Id = "card" })]));

        Assert.Contains("<rect", svg);
        Assert.Contains("id=\"card\"", svg);
        Assert.Contains("x=\"10\"", svg);
        Assert.Contains("y=\"20\"", svg);
        Assert.Contains("rx=\"8\"", svg);
        Assert.Contains("fill=\"#ef4444\"", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsEllipseAndLine()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                200,
                200,
                "transparent",
                [
                    PaintEllipse(100, 90, 50, 20, new PaintEllipseOptions { Fill = "#3b82f6" }),
                    PaintLine(0, 50, 200, 50, "#111111", new PaintLineOptions { StrokeWidth = 2, StrokeCap = "round" }),
                ]));

        Assert.Contains("<ellipse", svg);
        Assert.Contains("cx=\"100\"", svg);
        Assert.Contains("<line", svg);
        Assert.Contains("stroke-linecap=\"round\"", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsPathCommands()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                200,
                200,
                "transparent",
                [
                    PaintPath(
                        [
                            new MoveToCommand(0, 0),
                            new CubicToCommand(10, 20, 30, 40, 100, 100),
                            new ArcToCommand(50, 50, 0, false, true, 120, 100),
                            new ClosePathCommand(),
                        ],
                        new PaintPathOptions { Stroke = "#000", FillRule = "evenodd", StrokeJoin = "round", StrokeCap = "square" }),
                ]));

        Assert.Contains("C 10 20 30 40 100 100", svg);
        Assert.Contains("A 50 50 0 0 1 120 100", svg);
        Assert.Contains("fill-rule=\"evenodd\"", svg);
        Assert.Contains("stroke-linejoin=\"round\"", svg);
        Assert.Contains("stroke-linecap=\"square\"", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsTextForGlyphRuns()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                200,
                100,
                "transparent",
                [PaintGlyphRun([new PaintGlyphPlacement(65, 10, 20), new PaintGlyphPlacement(0x200000, 20, 20)], "Inter", 16)]));

        Assert.Contains("<text", svg);
        Assert.Contains("&#65;", svg);
        Assert.Contains("&#65533;", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsGroupsLayersAndFilters()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                200,
                200,
                "transparent",
                [
                    PaintGroup([PaintRect(0, 0, 50, 50, new PaintRectOptions { Fill = "#3b82f6" })], new PaintGroupOptions
                    {
                        Transform = new Transform2D(1, 0, 0, 1, 10, 20),
                        Opacity = 0.5,
                    }),
                    PaintLayer([], new PaintLayerOptions
                    {
                        Id = "glow",
                        Filters = [new BlurFilter(5)],
                        BlendMode = CodingAdventures.PaintInstructions.BlendMode.Multiply,
                    }),
                ]));

        Assert.Contains("transform=\"matrix(1,0,0,1,10,20)\"", svg);
        Assert.Contains("opacity=\"0.5\"", svg);
        Assert.Contains("<defs>", svg);
        Assert.Contains("feGaussianBlur", svg);
        Assert.Contains("mix-blend-mode:multiply", svg);
        Assert.Contains("filter=\"url(#filter-glow)\"", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsClipPaths()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                200,
                100,
                "transparent",
                [PaintClip(0, 0, 20, 10, [PaintRect(0, 0, 100, 50, new PaintRectOptions { Fill = "#fff" })])]));

        Assert.Contains("<clipPath", svg);
        Assert.Contains("clip-path=\"url(#", svg);
    }

    [Fact]
    public void RenderToSvgString_EmitsGradients()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                300,
                100,
                "transparent",
                [
                    PaintGradient(
                        GradientKind.Linear,
                        [new PaintGradientStop(0, "#3b82f6"), new PaintGradientStop(1, "#8b5cf6")],
                        new PaintGradientOptions { Id = "grad1", X1 = 0, Y1 = 0, X2 = 300, Y2 = 0 }),
                    PaintRect(0, 0, 300, 100, new PaintRectOptions { Fill = "url(#grad1)" }),
                ]));

        Assert.Contains("<linearGradient", svg);
        Assert.Contains("id=\"grad1\"", svg);
        Assert.Contains("fill=\"url(#grad1)\"", svg);
    }

    [Fact]
    public void RenderToSvgString_IgnoresGradientsWithoutIds()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(100, 100, "transparent", [PaintGradient(GradientKind.Linear, [new PaintGradientStop(0, "#000")])]));

        Assert.DoesNotContain("<linearGradient", svg);
    }

    [Fact]
    public void RenderToSvgString_SanitizesImageUris()
    {
        var svg = PaintVmSvg.RenderToSvgString(
            PaintScene(
                200,
                100,
                "transparent",
                [
                    PaintImage(0, 0, 50, 50, "https://example.com/logo.png"),
                    PaintImage(50, 0, 50, 50, "javascript:alert(1)"),
                    PaintImage(100, 0, 50, 50, new CodingAdventures.PixelContainer.PixelContainer(1, 1)),
                ]));

        Assert.Contains("https://example.com/logo.png", svg);
        Assert.DoesNotContain("javascript:alert(1)", svg);
        Assert.Contains("data:image/gif;base64,", svg);
        Assert.Contains("data:image/png;base64,", svg);
    }

    [Fact]
    public void AssembleSvg_ComposesManualVmExecution()
    {
        var vm = PaintVmSvg.CreateSvgVm();
        var context = PaintVmSvg.CreateSvgContext();
        var scene = PaintScene(100, 100, "transparent", [PaintRect(0, 0, 50, 50, new PaintRectOptions { Fill = "#fff" })]);

        vm.Execute(scene, context);
        var svg = PaintVmSvg.AssembleSvg(scene, context);

        Assert.Contains("<rect", svg);
    }

    [Fact]
    public void Export_ThrowsBecauseSvgDoesNotProducePixelData()
    {
        var vm = PaintVmSvg.CreateSvgVm();

        Assert.Throws<ExportNotSupportedError>(() => vm.Export(PaintScene(10, 10, "#fff", [])));
    }

    [Fact]
    public void RenderToSvgString_RejectsNonFiniteSceneNumbers()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => PaintVmSvg.RenderToSvgString(PaintScene(double.NaN, 10, "transparent", [])));
    }
}
