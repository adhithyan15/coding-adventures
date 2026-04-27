using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.Itf.Tests;

public sealed class ItfTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Itf.Version);
    }

    [Fact]
    public void NormalizeItfAcceptsEvenLengthDigitStrings()
    {
        Assert.Equal("123456", Itf.NormalizeItf("123456"));
    }

    [Fact]
    public void NormalizeItfRejectsInvalidInput()
    {
        Assert.Throws<ArgumentNullException>(() => Itf.NormalizeItf(null!));
        Assert.Throws<InvalidItfInputException>(() => Itf.NormalizeItf(""));
        Assert.Throws<InvalidItfInputException>(() => Itf.NormalizeItf("12345"));
        Assert.Throws<InvalidItfInputException>(() => Itf.NormalizeItf("12A4"));
    }

    [Fact]
    public void EncodeItfEncodesDigitPairs()
    {
        var encoded = Itf.EncodeItf("123456");

        Assert.Equal(3, encoded.Count);
        Assert.Equal("12", encoded[0].Pair);
        Assert.Equal("10001", encoded[0].BarPattern);
        Assert.Equal("01001", encoded[0].SpacePattern);
        Assert.Equal(0, encoded[0].SourceIndex);
        Assert.NotEmpty(encoded[0].BinaryPattern);
    }

    [Fact]
    public void ExpandItfRunsIncludeStartAndStopPatterns()
    {
        var runs = Itf.ExpandItfRuns("123456");

        Assert.Equal("start", runs[0].SourceLabel);
        Assert.Equal(Barcode1DRunRole.Start, runs[0].Role);
        Assert.Equal("stop", runs[^1].SourceLabel);
        Assert.Equal(Barcode1DRunRole.Stop, runs[^1].Role);
        Assert.Contains(runs, run => run.Role == Barcode1DRunRole.Data && run.SourceLabel == "12");
    }

    [Fact]
    public void DrawItfReturnsBarcodeScene()
    {
        var scene = Itf.DrawItf("123456");

        Assert.Equal("itf", scene.Metadata?["symbology"]);
        Assert.Equal(3, scene.Metadata?["pairCount"]);
        Assert.True(scene.Width > 0);
        Assert.Equal(Itf.DefaultRenderConfig().BarHeight, scene.Height);
    }

    [Fact]
    public void InvalidLayoutConfigIsRejected()
    {
        var badOptions = new PaintBarcode1DOptions
        {
            RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
        };

        Assert.Throws<ArgumentException>(() => Itf.LayoutItf("123456", badOptions));
    }
}
