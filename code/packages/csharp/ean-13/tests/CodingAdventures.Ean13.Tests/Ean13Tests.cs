using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.Ean13.Tests;

public sealed class Ean13Tests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Ean13.Version);
    }

    [Fact]
    public void ComputesAndValidatesCheckDigit()
    {
        Assert.Equal("1", Ean13.ComputeEan13CheckDigit("400638133393"));
        Assert.Equal("4006381333931", Ean13.NormalizeEan13("400638133393"));
        Assert.Equal("4006381333931", Ean13.NormalizeEan13("4006381333931"));
    }

    [Fact]
    public void RejectsMalformedInput()
    {
        Assert.Throws<ArgumentNullException>(() => Ean13.NormalizeEan13(null!));
        Assert.Throws<InvalidEan13InputException>(() => Ean13.NormalizeEan13("40063813339A"));
        Assert.Throws<InvalidEan13InputException>(() => Ean13.NormalizeEan13("123"));
        Assert.Throws<InvalidEan13CheckDigitException>(() => Ean13.NormalizeEan13("4006381333932"));
    }

    [Fact]
    public void LeftParityPatternMatchesReference()
    {
        Assert.Equal("LGLLGG", Ean13.LeftParityPattern("400638133393"));
    }

    [Fact]
    public void EncodeEan13TracksParityAndCheckDigit()
    {
        var encoded = Ean13.EncodeEan13("400638133393");

        Assert.Equal(12, encoded.Count);
        Assert.Equal("0", encoded[0].Digit);
        Assert.Equal("L", encoded[0].Encoding);
        Assert.Equal("0", encoded[1].Digit);
        Assert.Equal("G", encoded[1].Encoding);
        Assert.Equal(Barcode1DRunRole.Check, encoded[^1].Role);
        Assert.Equal("1", encoded[^1].Digit);
    }

    [Fact]
    public void ExpandRunsTotalNinetyFiveModules()
    {
        var runs = Ean13.ExpandEan13Runs("400638133393");

        Assert.Equal(95u, runs.Aggregate(0u, (sum, run) => sum + run.Modules));
        Assert.Equal(Barcode1DRunRole.Guard, runs[0].Role);
        Assert.Equal("start", runs[0].SourceLabel);
        Assert.Contains(runs, run => run.SourceLabel == "center" && run.Role == Barcode1DRunRole.Guard);
        Assert.Equal("end", runs[^1].SourceLabel);
    }

    [Fact]
    public void LayoutBuildsSceneMetadataAndSymbols()
    {
        var scene = Ean13.DrawEan13("400638133393");

        Assert.Equal("ean-13", scene.Metadata?["symbology"]);
        Assert.Equal("4", scene.Metadata?["leadingDigit"]);
        Assert.Equal("LGLLGG", scene.Metadata?["leftParity"]);
        Assert.Equal("EAN-13 barcode for 4006381333931", scene.Metadata?["label"]);
        Assert.Equal("4006381333931", scene.Metadata?["humanReadableText"]);
        Assert.Equal(95u, scene.Metadata?["contentModules"]);
        Assert.Equal(15, scene.Metadata?["symbolCount"]);
        Assert.Equal(Ean13.DefaultRenderConfig().BarHeight, scene.Height);
    }

    [Fact]
    public void InvalidRenderConfigIsRejected()
    {
        var badOptions = new PaintBarcode1DOptions
        {
            RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
        };

        Assert.Throws<ArgumentException>(() => Ean13.LayoutEan13("400638133393", badOptions));
    }
}
