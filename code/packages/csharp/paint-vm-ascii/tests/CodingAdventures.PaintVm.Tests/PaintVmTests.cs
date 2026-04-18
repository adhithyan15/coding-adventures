using CodingAdventures.PaintInstructions;
using CodingAdventures.PaintVm;
using CodingAdventures.PaintVmAscii;
using static CodingAdventures.PaintInstructions.PaintInstructions;

namespace CodingAdventures.PaintVmAscii.Tests;

public sealed class PaintVmTests
{
    [Fact]
    public void Version_IsSemver()
    {
        Assert.Equal("0.1.0", PaintVmAsciiPackage.VERSION);
    }

    [Fact]
    public void RenderToAscii_DrawsAStrokedRectangle()
    {
        var scene = PaintScene(
            5,
            3,
            "#fff",
            [PaintRect(0, 0, 4, 2, new PaintRectOptions { Fill = "transparent", Stroke = "#000", StrokeWidth = 1 })]);

        var rendered = PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 });

        Assert.Equal("┌───┐\n│   │\n└───┘", rendered);
    }

    [Fact]
    public void RenderToAscii_FillsRectanglesWithBlockCharacters()
    {
        var scene = PaintScene(
            3,
            2,
            "#fff",
            [PaintRect(0, 0, 2, 1, new PaintRectOptions { Fill = "#000" })]);

        Assert.Contains("█", PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }));
    }

    [Fact]
    public void RenderToAscii_MergesLineIntersections()
    {
        var scene = PaintScene(
            5,
            3,
            "#fff",
            [PaintLine(0, 1, 4, 1, "#000"), PaintLine(2, 0, 2, 2, "#000")]);

        var lines = PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }).Split('\n');

        Assert.Equal('│', lines[0][2]);
        Assert.Equal('┼', lines[1][2]);
        Assert.Equal('│', lines[2][2]);
    }

    [Fact]
    public void RenderToAscii_RendersGlyphRunsAsText()
    {
        var scene = PaintScene(
            5,
            1,
            "#fff",
            [
                PaintGlyphRun(
                    [new PaintGlyphPlacement('H', 0, 0), new PaintGlyphPlacement('i', 1, 0)],
                    "mono",
                    12)
            ]);

        Assert.Equal("Hi", PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }));
    }

    [Fact]
    public void RenderToAscii_ReplacesUnsafeGlyphs()
    {
        var scene = PaintScene(
            2,
            1,
            "#fff",
            [
                PaintGlyphRun(
                    [new PaintGlyphPlacement(0x1b, 0, 0), new PaintGlyphPlacement('A', 1, 0)],
                    "mono",
                    12)
            ]);

        Assert.Equal("?A", PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }));
    }

    [Fact]
    public void RenderToAscii_ClipsChildOutput()
    {
        var scene = PaintScene(
            10,
            1,
            "#fff",
            [
                PaintClip(
                    0,
                    0,
                    3,
                    1,
                    [
                        PaintGlyphRun(
                            [
                                new PaintGlyphPlacement('H', 0, 0),
                                new PaintGlyphPlacement('e', 1, 0),
                                new PaintGlyphPlacement('l', 2, 0),
                                new PaintGlyphPlacement('l', 3, 0),
                                new PaintGlyphPlacement('o', 4, 0),
                            ],
                            "mono",
                            12)
                    ])
            ]);

        Assert.Equal("Hel", PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }));
    }

    [Fact]
    public void RenderToAscii_RecursesPlainGroupsAndLayers()
    {
        var scene = PaintScene(
            5,
            1,
            "#fff",
            [
                PaintGroup(
                    [
                        PaintLayer(
                            [
                                PaintGlyphRun(
                                    [new PaintGlyphPlacement('A', 0, 0), new PaintGlyphPlacement('B', 1, 0)],
                                    "mono",
                                    12)
                            ]),
                        PaintGlyphRun(
                            [new PaintGlyphPlacement('C', 3, 0), new PaintGlyphPlacement('D', 4, 0)],
                            "mono",
                            12)
                    ])
            ]);

        Assert.Equal("AB CD", PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }));
    }

    [Fact]
    public void RenderToAscii_RejectsTransformedGroups()
    {
        var scene = PaintScene(
            5,
            1,
            "#fff",
            [PaintGroup([], new PaintGroupOptions { Transform = new Transform2D(1, 0, 0, 1, 1, 0) })]);

        var error = Assert.Throws<UnsupportedAsciiFeatureError>(() => PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 }));
        Assert.Contains("transformed groups", error.Message);
    }

    [Fact]
    public void CreateAsciiVm_ExecutesThroughPaintVm()
    {
        var vm = PaintVmAscii.CreateAsciiVm(new AsciiOptions { ScaleX = 1, ScaleY = 1 });
        var context = PaintVmAscii.CreateAsciiContext();
        var scene = PaintScene(2, 1, "#fff", [PaintGlyphRun([new PaintGlyphPlacement('O', 0, 0)], "mono", 12)]);

        vm.Execute(scene, context);

        Assert.Equal("O", context.Buffer.ToString());
    }

    [Fact]
    public void Export_ThrowsBecauseAsciiDoesNotProducePixelData()
    {
        var vm = PaintVmAscii.CreateAsciiVm(new AsciiOptions { ScaleX = 1, ScaleY = 1 });

        Assert.Throws<ExportNotSupportedError>(() => vm.Export(PaintScene(10, 10, "#fff", [])));
    }
}
