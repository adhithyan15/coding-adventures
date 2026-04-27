using CodingAdventures.BarcodeLayout1D;

namespace CodingAdventures.Barcode1D.Tests;

public sealed class Barcode1DTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Barcode1D.Version);
    }

    [Fact]
    public void NormalizeSymbologyAcceptsCommonSpellings()
    {
        Assert.Equal(Symbology.Code39, Barcode1D.NormalizeSymbology("code39"));
        Assert.Equal(Symbology.Code128, Barcode1D.NormalizeSymbology("code-128"));
        Assert.Equal(Symbology.Ean13, Barcode1D.NormalizeSymbology("ean_13"));
        Assert.Equal(Symbology.UpcA, Barcode1D.NormalizeSymbology("UPC-A"));
        Assert.Equal(Symbology.Code39, Barcode1D.NormalizeSymbology(" "));
        Assert.Equal("itf", Symbology.Itf.AsString());
    }

    [Fact]
    public void NormalizeSymbologyRejectsUnsupportedNames()
    {
        Assert.Throws<ArgumentNullException>(() => Barcode1D.NormalizeSymbology(null!));
        Assert.Throws<UnsupportedSymbologyException>(() => Barcode1D.NormalizeSymbology("qr"));
    }

    [Fact]
    public void BuildSceneRoutesToCode39ByDefault()
    {
        var scene = Barcode1D.BuildScene("HELLO-123");

        Assert.Equal("code39", scene.Metadata?["symbology"]);
        Assert.Equal("HELLO-123", scene.Metadata?["humanReadableText"]);
        Assert.True(scene.Width > 0);
        Assert.Equal(Barcode1D.DefaultRenderConfig().BarHeight, scene.Height);
    }

    [Theory]
    [InlineData("codabar", "40156", "codabar")]
    [InlineData("code128", "Code 128", "code128")]
    [InlineData("ean-13", "400638133393", "ean-13")]
    [InlineData("itf", "123456", "itf")]
    [InlineData("upc_a", "03600029145", "upc-a")]
    public void BuildSceneForSymbologyRoutesAdditionalEncoders(string symbology, string data, string expected)
    {
        var scene = Barcode1D.BuildSceneForSymbology(symbology, data);

        Assert.Equal(expected, scene.Metadata?["symbology"]);
        Assert.True(scene.Width > 0);
        Assert.True((uint)scene.Metadata!["contentModules"]! > 0);
    }

    [Fact]
    public void BuildSceneUsesTypedOptionsAndPaintOptions()
    {
        var options = new Barcode1DOptions
        {
            Symbology = Symbology.Ean13,
            Paint = new PaintBarcode1DOptions
            {
                RenderConfig = new Barcode1DRenderConfig { ModuleWidth = 2.0 },
                Metadata = new Dictionary<string, object?> { ["batch"] = "aggregate" },
            },
        };

        var scene = Barcode1D.BuildScene("400638133393", options);

        Assert.Equal("ean-13", scene.Metadata?["symbology"]);
        Assert.Equal("aggregate", scene.Metadata?["batch"]);
        Assert.Equal(2.0, scene.Metadata?["moduleWidthPx"]);
    }

    [Fact]
    public void CodabarStartStopCanBeSelected()
    {
        var scene = Barcode1D.BuildScene(
            "40156",
            new Barcode1DOptions
            {
                Symbology = Symbology.Codabar,
                CodabarStart = "B",
                CodabarStop = "C",
            });

        Assert.Equal("codabar", scene.Metadata?["symbology"]);
        Assert.Equal("B", scene.Metadata?["start"]);
        Assert.Equal("C", scene.Metadata?["stop"]);
    }

    [Fact]
    public void CurrentBackendIsHonestAboutMissingNativeRenderer()
    {
        Assert.Null(Barcode1D.CurrentBackend());
    }

    [Fact]
    public void RenderPixelsFailsUntilNativeBackendExists()
    {
        Assert.Throws<BackendUnavailableException>(() => Barcode1D.RenderPixels("HELLO-123"));
        Assert.Throws<BackendUnavailableException>(() => Barcode1D.RenderPixelsForSymbology("code-128", "Code 128"));
    }

    [Fact]
    public void RenderPngFailsUntilNativeBackendExists()
    {
        Assert.Throws<BackendUnavailableException>(() => Barcode1D.RenderPng("HELLO-123"));
        Assert.Throws<BackendUnavailableException>(() => Barcode1D.RenderPngForSymbology("ean13", "400638133393"));
    }
}
