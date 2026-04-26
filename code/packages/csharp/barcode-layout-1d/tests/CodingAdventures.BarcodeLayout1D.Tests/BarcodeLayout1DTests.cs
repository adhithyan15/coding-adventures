using CodingAdventures.PaintInstructions;

namespace CodingAdventures.BarcodeLayout1D.Tests;

public sealed class BarcodeLayout1DTests
{
    [Fact]
    public void BinaryPatternExpandsToRuns()
    {
        var runs = BarcodeLayout1D.RunsFromBinaryPattern(
            "11001",
            new RunsFromBinaryPatternOptions("start", -1, Barcode1DRunRole.Guard));

        Assert.Equal(3, runs.Count);
        Assert.Equal(Barcode1DRunColor.Bar, runs[0].Color);
        Assert.Equal(2u, runs[0].Modules);
        Assert.Equal(Barcode1DRunColor.Space, runs[1].Color);
        Assert.Equal(1u, runs[2].Modules);
        Assert.Throws<ArgumentException>(() => BarcodeLayout1D.RunsFromBinaryPattern("10x", new("bad", 0, Barcode1DRunRole.Data)));
    }

    [Fact]
    public void WidthPatternExpandsToRuns()
    {
        var runs = BarcodeLayout1D.RunsFromWidthPattern(
            "NWN",
            new RunsFromWidthPatternOptions("A", 0, Barcode1DRunRole.Data));

        Assert.Equal(3, runs.Count);
        Assert.Equal(1u, runs[0].Modules);
        Assert.Equal(3u, runs[1].Modules);
        Assert.Equal(Barcode1DRunColor.Bar, runs[2].Color);
        Assert.Throws<ArgumentException>(() => BarcodeLayout1D.RunsFromWidthPattern("NX", new("A", 0, Barcode1DRunRole.Data)));
    }

    [Fact]
    public void ComputesQuietZoneAwareLayout()
    {
        var runs = new[]
        {
            new Barcode1DRun(Barcode1DRunColor.Bar, 1, "*", 0, Barcode1DRunRole.Start),
            new Barcode1DRun(Barcode1DRunColor.Space, 1, "*", 0, Barcode1DRunRole.InterCharacterGap),
            new Barcode1DRun(Barcode1DRunColor.Bar, 2, "A", 1, Barcode1DRunRole.Data),
        };

        var layout = BarcodeLayout1D.ComputeBarcode1DLayout(runs, 10);

        Assert.Equal(4u, layout.ContentModules);
        Assert.Equal(24u, layout.TotalModules);
        Assert.Equal(2, layout.SymbolLayouts.Count);
        Assert.Equal("*", layout.SymbolLayouts[0].Label);
        Assert.Equal(2u, layout.SymbolLayouts[0].EndModule);
    }

    [Fact]
    public void ExplicitSymbolsMustMatchRunWidth()
    {
        var runs = BarcodeLayout1D.RunsFromBinaryPattern("101", new("demo", 0, Barcode1DRunRole.Guard));

        var symbols = new[]
        {
            new Barcode1DSymbolDescriptor("demo", 3, 0, Barcode1DSymbolRole.Guard),
        };

        var layout = BarcodeLayout1D.ComputeBarcode1DLayout(runs, 10, symbols);
        Assert.Single(layout.SymbolLayouts);
        Assert.Throws<ArgumentException>(() => BarcodeLayout1D.ComputeBarcode1DLayout(runs, 10, [new("bad", 2, 0, Barcode1DSymbolRole.Data)]));
    }

    [Fact]
    public void LaysOutRunsIntoPaintScene()
    {
        var runs = BarcodeLayout1D.RunsFromBinaryPattern("101", new("demo", 0, Barcode1DRunRole.Guard));
        var scene = BarcodeLayout1D.LayoutBarcode1D(
            runs,
            new PaintBarcode1DOptions { Label = "Demo barcode" });

        Assert.Equal("#ffffff", scene.Background);
        Assert.Equal(2, scene.Instructions.Count);
        Assert.All(scene.Instructions, instruction => Assert.IsType<PaintRect>(instruction));
        Assert.Equal("Demo barcode", scene.Metadata?["label"]);
        Assert.Equal(23u, scene.Metadata?["totalModules"]);
    }

    [Fact]
    public void ValidatesLayoutAndRenderConfiguration()
    {
        Assert.Throws<ArgumentException>(() => BarcodeLayout1D.ComputeBarcode1DLayout(
            [
                new Barcode1DRun(Barcode1DRunColor.Bar, 1, "a", 0, Barcode1DRunRole.Data),
                new Barcode1DRun(Barcode1DRunColor.Bar, 1, "b", 1, Barcode1DRunRole.Data),
            ],
            10));

        var runs = BarcodeLayout1D.RunsFromBinaryPattern("101", new("demo", 0, Barcode1DRunRole.Guard));
        Assert.Throws<NotSupportedException>(() => BarcodeLayout1D.LayoutBarcode1D(
            runs,
            new PaintBarcode1DOptions
            {
                RenderConfig = new Barcode1DRenderConfig { IncludeHumanReadableText = true },
                HumanReadableText = "demo",
            }));
        Assert.Throws<ArgumentException>(() => BarcodeLayout1D.LayoutBarcode1D(
            runs,
            new PaintBarcode1DOptions
            {
                RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
            }));
    }
}
