using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.Codabar.Tests;

public sealed class CodabarTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Codabar.Version);
    }

    [Fact]
    public void NormalizeCodabarAddsDefaultOrRequestedGuards()
    {
        Assert.Equal("A40156A", Codabar.NormalizeCodabar("40156"));
        Assert.Equal("B40156D", Codabar.NormalizeCodabar("40156", "B", "D"));
        Assert.Equal("C40156D", Codabar.NormalizeCodabar("c40156d"));
    }

    [Fact]
    public void EncodeCodabarMarksOuterSymbols()
    {
        var encoded = Codabar.EncodeCodabar("40156", "B", "D");

        Assert.Equal("B", encoded[0].Char);
        Assert.Equal("1001001011", encoded[0].Pattern);
        Assert.Equal(Barcode1DRunRole.Start, encoded[0].Role);
        Assert.Equal(Barcode1DRunRole.Data, encoded[1].Role);
        Assert.Equal("D", encoded[^1].Char);
        Assert.Equal(Barcode1DRunRole.Stop, encoded[^1].Role);
    }

    [Fact]
    public void ExpandRunsIncludesInterCharacterGaps()
    {
        var runs = Codabar.ExpandCodabarRuns("1");

        Assert.True(runs.Count > 0);
        Assert.Equal(Barcode1DRunColor.Bar, runs[0].Color);
        Assert.Equal(Barcode1DRunRole.Start, runs[0].Role);
        Assert.Contains(runs, run => run.Role == Barcode1DRunRole.InterCharacterGap);
        Assert.Equal(Barcode1DRunRole.Stop, runs[^1].Role);
    }

    [Fact]
    public void LayoutCodabarBuildsPaintSceneMetadata()
    {
        var scene = Codabar.DrawCodabar("40156", start: "B", stop: "D");

        Assert.Equal("codabar", scene.Metadata?["symbology"]);
        Assert.Equal("B", scene.Metadata?["start"]);
        Assert.Equal("D", scene.Metadata?["stop"]);
        Assert.Equal("Codabar barcode for B40156D", scene.Metadata?["label"]);
        Assert.Equal("B40156D", scene.Metadata?["humanReadableText"]);
        Assert.True(scene.Width > 0);
        Assert.Equal(Codabar.DefaultRenderConfig().BarHeight, scene.Height);
    }

    [Fact]
    public void InvalidInputsAndConfigsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => Codabar.NormalizeCodabar(null!));
        Assert.Throws<InvalidCodabarInputException>(() => Codabar.NormalizeCodabar("40*56"));
        Assert.Throws<InvalidCodabarInputException>(() => Codabar.NormalizeCodabar("A"));
        Assert.Throws<InvalidCodabarInputException>(() => Codabar.NormalizeCodabar("40156", "Z", "A"));

        var badOptions = new PaintBarcode1DOptions
        {
            RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
        };
        Assert.Throws<ArgumentException>(() => Codabar.LayoutCodabar("40156", badOptions));
    }
}
