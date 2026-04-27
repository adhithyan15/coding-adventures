using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.UpcA.Tests;

public sealed class UpcATests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", UpcA.Version);
    }

    [Fact]
    public void ComputesAndValidatesCheckDigit()
    {
        Assert.Equal("2", UpcA.ComputeUpcACheckDigit("03600029145"));
        Assert.Equal("036000291452", UpcA.NormalizeUpcA("03600029145"));
        Assert.Equal("036000291452", UpcA.NormalizeUpcA("036000291452"));
    }

    [Fact]
    public void RejectsMalformedInput()
    {
        Assert.Throws<ArgumentNullException>(() => UpcA.NormalizeUpcA(null!));
        Assert.Throws<InvalidUpcAInputException>(() => UpcA.NormalizeUpcA("0360002914A"));
        Assert.Throws<InvalidUpcAInputException>(() => UpcA.NormalizeUpcA("123"));
        Assert.Throws<InvalidUpcACheckDigitException>(() => UpcA.NormalizeUpcA("036000291453"));
    }

    [Fact]
    public void EncodeTracksLeftRightAndCheckDigit()
    {
        var encoded = UpcA.EncodeUpcA("03600029145");

        Assert.Equal(12, encoded.Count);
        Assert.Equal("0", encoded[0].Digit);
        Assert.Equal("L", encoded[0].Encoding);
        Assert.Equal("0", encoded[5].Digit);
        Assert.Equal("L", encoded[5].Encoding);
        Assert.Equal("2", encoded[6].Digit);
        Assert.Equal("R", encoded[6].Encoding);
        Assert.Equal(Barcode1DRunRole.Check, encoded[^1].Role);
        Assert.Equal("2", encoded[^1].Digit);
    }

    [Fact]
    public void ExpandRunsTotalNinetyFiveModules()
    {
        var runs = UpcA.ExpandUpcARuns("03600029145");

        Assert.Equal(95u, runs.Aggregate(0u, (sum, run) => sum + run.Modules));
        Assert.Equal(Barcode1DRunRole.Guard, runs[0].Role);
        Assert.Equal("start", runs[0].SourceLabel);
        Assert.Contains(runs, run => run.SourceLabel == "center" && run.Role == Barcode1DRunRole.Guard);
        Assert.Equal("end", runs[^1].SourceLabel);
    }

    [Fact]
    public void LayoutBuildsSceneMetadataAndSymbols()
    {
        var scene = UpcA.DrawUpcA("03600029145");

        Assert.Equal("upc-a", scene.Metadata?["symbology"]);
        Assert.Equal("UPC-A barcode for 036000291452", scene.Metadata?["label"]);
        Assert.Equal("036000291452", scene.Metadata?["humanReadableText"]);
        Assert.Equal(95u, scene.Metadata?["contentModules"]);
        Assert.Equal(15, scene.Metadata?["symbolCount"]);
        Assert.Equal(UpcA.DefaultRenderConfig().BarHeight, scene.Height);
    }

    [Fact]
    public void InvalidRenderConfigIsRejected()
    {
        var badOptions = new PaintBarcode1DOptions
        {
            RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
        };

        Assert.Throws<ArgumentException>(() => UpcA.LayoutUpcA("03600029145", badOptions));
    }
}
