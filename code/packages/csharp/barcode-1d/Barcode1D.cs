using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;
using PixelBuffer = CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.Barcode1D;

public enum Symbology
{
    Codabar,
    Code128,
    Code39,
    Ean13,
    Itf,
    UpcA,
}

public sealed record Barcode1DOptions
{
    public Symbology Symbology { get; init; } = Symbology.Code39;

    public PaintBarcode1DOptions Paint { get; init; } = new();

    public string? CodabarStart { get; init; }

    public string? CodabarStop { get; init; }
}

public class Barcode1DException(string message) : Exception(message);

public sealed class UnsupportedSymbologyException(string message) : Barcode1DException(message);

public sealed class BackendUnavailableException(string message) : Barcode1DException(message);

public static class Barcode1D
{
    public const string Version = "0.1.0";

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static Barcode1DOptions DefaultOptions() => new();

    public static string AsString(this Symbology symbology) =>
        symbology switch
        {
            Symbology.Codabar => "codabar",
            Symbology.Code128 => "code128",
            Symbology.Code39 => "code39",
            Symbology.Ean13 => "ean13",
            Symbology.Itf => "itf",
            Symbology.UpcA => "upca",
            _ => throw new ArgumentOutOfRangeException(nameof(symbology), symbology, null),
        };

    public static string? CurrentBackend() => null;

    public static Symbology NormalizeSymbology(string symbology)
    {
        ArgumentNullException.ThrowIfNull(symbology);

        var normalized = symbology
            .Trim()
            .ToLowerInvariant()
            .Replace("-", string.Empty)
            .Replace("_", string.Empty);

        normalized = normalized.Length == 0 ? "code39" : normalized;

        return normalized switch
        {
            "codabar" => Symbology.Codabar,
            "code128" => Symbology.Code128,
            "code39" => Symbology.Code39,
            "ean13" => Symbology.Ean13,
            "itf" => Symbology.Itf,
            "upca" => Symbology.UpcA,
            _ => throw new UnsupportedSymbologyException($"unsupported symbology: {symbology}"),
        };
    }

    public static PaintScene BuildScene(string data, Barcode1DOptions? options = null)
    {
        options ??= new Barcode1DOptions();

        return options.Symbology switch
        {
            Symbology.Codabar => CodingAdventures.Codabar.Codabar.LayoutCodabar(
                data,
                options.Paint,
                options.CodabarStart ?? "A",
                options.CodabarStop ?? "A"),
            Symbology.Code128 => CodingAdventures.Code128.Code128.LayoutCode128(data, options.Paint),
            Symbology.Code39 => CodingAdventures.Code39.Code39.LayoutCode39(data, options.Paint),
            Symbology.Ean13 => CodingAdventures.Ean13.Ean13.LayoutEan13(data, options.Paint),
            Symbology.Itf => CodingAdventures.Itf.Itf.LayoutItf(data, options.Paint),
            Symbology.UpcA => CodingAdventures.UpcA.UpcA.LayoutUpcA(data, options.Paint),
            _ => throw new ArgumentOutOfRangeException(nameof(options), options.Symbology, null),
        };
    }

    public static PaintScene BuildSceneForSymbology(
        string symbology,
        string data,
        Barcode1DOptions? options = null)
    {
        var nextOptions = (options ?? new Barcode1DOptions()) with
        {
            Symbology = NormalizeSymbology(symbology),
        };

        return BuildScene(data, nextOptions);
    }

    public static PixelBuffer RenderPixels(string data, Barcode1DOptions? options = null)
    {
        _ = BuildScene(data, options);
        throw NewBackendUnavailable();
    }

    public static PixelBuffer RenderPixelsForSymbology(
        string symbology,
        string data,
        Barcode1DOptions? options = null)
    {
        _ = BuildSceneForSymbology(symbology, data, options);
        throw NewBackendUnavailable();
    }

    public static byte[] RenderPng(string data, Barcode1DOptions? options = null)
    {
        _ = RenderPixels(data, options);
        throw NewBackendUnavailable();
    }

    public static byte[] RenderPngForSymbology(
        string symbology,
        string data,
        Barcode1DOptions? options = null)
    {
        _ = RenderPixelsForSymbology(symbology, data, options);
        throw NewBackendUnavailable();
    }

    private static BackendUnavailableException NewBackendUnavailable() =>
        new("native barcode rendering is not wired for dotnet yet; BuildScene() is available, but pixel and PNG rendering await a paint backend.");
}
