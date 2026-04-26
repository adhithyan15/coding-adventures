using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.Code39.Tests;

public sealed class Code39Tests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Code39.Version);
    }

    [Fact]
    public void NormalizesSupportedInput()
    {
        Assert.Equal("ABC-123", Code39.NormalizeCode39("abc-123"));
    }

    [Fact]
    public void EncodesCharacterAndFullSequence()
    {
        var encoded = Code39.EncodeCode39Char("A");

        Assert.Equal("WNNNNWNNW", encoded.Pattern);
        Assert.False(encoded.IsStartStop);
        Assert.Equal(["*", "A", "*"], Code39.EncodeCode39("A").Select(item => item.Char));
    }

    [Fact]
    public void ExpandRunsIncludesStartStopAndInterCharacterGaps()
    {
        var runs = Code39.ExpandCode39Runs("A");

        Assert.Equal(29, runs.Count);
        Assert.Equal(Barcode1DRunColor.Bar, runs[0].Color);
        Assert.Equal(Barcode1DRunRole.Start, runs[0].Role);
        Assert.Equal(Barcode1DRunRole.InterCharacterGap, runs[9].Role);
        Assert.Equal(3u, runs[10].Modules);
        Assert.Equal(Barcode1DRunRole.Stop, runs[^1].Role);
    }

    [Fact]
    public void LayoutCode39BuildsPaintSceneMetadata()
    {
        var scene = Code39.DrawCode39("A");

        Assert.Equal("code39", scene.Metadata?["symbology"]);
        Assert.Equal("A", scene.Metadata?["encodedText"]);
        Assert.Equal("Code 39 barcode for A", scene.Metadata?["label"]);
        Assert.True(scene.Width > 0);
        Assert.Equal(Code39.DefaultRenderConfig().BarHeight, scene.Height);
    }

    [Fact]
    public void InvalidInputsAndConfigsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => Code39.NormalizeCode39(null!));
        Assert.Throws<InvalidCharacterException>(() => Code39.NormalizeCode39("*"));
        Assert.Throws<InvalidCharacterException>(() => Code39.NormalizeCode39("~"));
        Assert.Throws<InvalidCharacterException>(() => Code39.EncodeCode39Char("~"));

        var badOptions = new PaintBarcode1DOptions
        {
            RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
        };
        Assert.Throws<ArgumentException>(() => Code39.LayoutCode39("A", badOptions));
    }
}
