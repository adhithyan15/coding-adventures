using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.Code128.Tests;

public sealed class Code128Tests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Code128.Version);
    }

    [Fact]
    public void NormalizeCode128BAcceptsPrintableAscii()
    {
        Assert.Equal("Code 128", Code128.NormalizeCode128B("Code 128"));
        Assert.Equal("~", Code128.NormalizeCode128B("~"));
    }

    [Fact]
    public void NormalizeCode128BRejectsUnsupportedInput()
    {
        Assert.Throws<ArgumentNullException>(() => Code128.NormalizeCode128B(null!));
        Assert.Throws<InvalidCode128InputException>(() => Code128.NormalizeCode128B("bad\ninput"));
        Assert.Throws<InvalidCode128InputException>(() => Code128.NormalizeCode128B("cafe\u00e9"));
    }

    [Fact]
    public void ComputesReferenceChecksum()
    {
        Assert.Equal(64, Code128.ComputeCode128Checksum([35, 79, 68, 69, 0, 17, 18, 24]));
        Assert.Throws<ArgumentNullException>(() => Code128.ComputeCode128Checksum(null!));
    }

    [Fact]
    public void EncodeCode128BAddsStartChecksumAndStop()
    {
        var encoded = Code128.EncodeCode128B("Code 128");

        Assert.Equal("Start B", encoded[0].Label);
        Assert.Equal(104, encoded[0].Value);
        Assert.Equal(Barcode1DRunRole.Start, encoded[0].Role);
        Assert.Equal("Checksum 64", encoded[^2].Label);
        Assert.Equal(Barcode1DRunRole.Check, encoded[^2].Role);
        Assert.Equal("Stop", encoded[^1].Label);
        Assert.Equal(Barcode1DRunRole.Stop, encoded[^1].Role);
    }

    [Fact]
    public void ExpandCode128RunsEndsWithStopPattern()
    {
        var runs = Code128.ExpandCode128Runs("Hi");

        Assert.Equal(57u, runs.Aggregate(0u, (sum, run) => sum + run.Modules));
        Assert.Equal("Start B", runs[0].SourceLabel);
        Assert.Equal(Barcode1DRunRole.Start, runs[0].Role);
        Assert.Equal("Stop", runs[^1].SourceLabel);
        Assert.Equal(Barcode1DRunRole.Stop, runs[^1].Role);
    }

    [Fact]
    public void LayoutBuildsSceneMetadataAndSymbols()
    {
        var scene = Code128.DrawCode128("Code 128");

        Assert.Equal("code128", scene.Metadata?["symbology"]);
        Assert.Equal("B", scene.Metadata?["codeSet"]);
        Assert.Equal(64, scene.Metadata?["checksum"]);
        Assert.Equal("Code 128 barcode for Code 128", scene.Metadata?["label"]);
        Assert.Equal("Code 128", scene.Metadata?["humanReadableText"]);
        Assert.Equal(123u, scene.Metadata?["contentModules"]);
        Assert.Equal(11, scene.Metadata?["symbolCount"]);
    }

    [Fact]
    public void InvalidRenderConfigIsRejected()
    {
        var badOptions = new PaintBarcode1DOptions
        {
            RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 0 },
        };

        Assert.Throws<ArgumentException>(() => Code128.LayoutCode128("Code 128", badOptions));
    }
}
