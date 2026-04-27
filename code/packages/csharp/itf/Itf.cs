using System.Text;
using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.Itf;

public static class Itf
{
    public const string Version = "0.1.0";

    private const string StartPattern = "1010";
    private const string StopPattern = "11101";

    private static readonly string[] DigitPatterns =
    [
        "00110",
        "10001",
        "01001",
        "11000",
        "00101",
        "10100",
        "01100",
        "00011",
        "10010",
        "01010",
    ];

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static string NormalizeItf(string data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length == 0 || data.Length % 2 != 0)
        {
            throw new InvalidItfInputException("ITF input must contain an even number of digits");
        }

        if (data.Any(ch => !char.IsDigit(ch)))
        {
            throw new InvalidItfInputException("ITF input must contain digits only");
        }

        return data;
    }

    public static IReadOnlyList<EncodedPair> EncodeItf(string data)
    {
        var normalized = NormalizeItf(data);
        var encoded = new List<EncodedPair>();
        for (var index = 0; index < normalized.Length; index += 2)
        {
            encoded.Add(EncodePair(normalized[index..(index + 2)], index / 2));
        }

        return encoded;
    }

    public static IReadOnlyList<Barcode1DRun> ExpandItfRuns(string data)
    {
        var encodedPairs = EncodeItf(data);
        var runs = new List<Barcode1DRun>();

        runs.AddRange(BarcodeLayout1D.BarcodeLayout1D.RunsFromBinaryPattern(
            StartPattern,
            new RunsFromBinaryPatternOptions("start", -1, Barcode1DRunRole.Start)));

        foreach (var pair in encodedPairs)
        {
            runs.AddRange(BarcodeLayout1D.BarcodeLayout1D.RunsFromBinaryPattern(
                pair.BinaryPattern,
                new RunsFromBinaryPatternOptions(pair.Pair, pair.SourceIndex, Barcode1DRunRole.Data)));
        }

        runs.AddRange(BarcodeLayout1D.BarcodeLayout1D.RunsFromBinaryPattern(
            StopPattern,
            new RunsFromBinaryPatternOptions("stop", -2, Barcode1DRunRole.Stop)));

        return runs;
    }

    public static PaintScene LayoutItf(string data, PaintBarcode1DOptions? options = null)
    {
        var normalized = NormalizeItf(data);
        var runs = ExpandItfRuns(normalized);
        options ??= new PaintBarcode1DOptions();

        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["symbology"] = "itf",
            ["pairCount"] = normalized.Length / 2,
        };

        return BarcodeLayout1D.BarcodeLayout1D.LayoutBarcode1D(
            runs,
            options with { Metadata = metadata });
    }

    public static PaintScene DrawItf(string data, PaintBarcode1DOptions? options = null) =>
        LayoutItf(data, options);

    private static EncodedPair EncodePair(string pair, int sourceIndex)
    {
        var barPattern = DigitPatterns[pair[0] - '0'];
        var spacePattern = DigitPatterns[pair[1] - '0'];
        var binaryPattern = new StringBuilder();

        for (var index = 0; index < barPattern.Length; index++)
        {
            binaryPattern.Append(barPattern[index] == '1' ? "111" : "1");
            binaryPattern.Append(spacePattern[index] == '1' ? "000" : "0");
        }

        return new EncodedPair(pair, barPattern, spacePattern, binaryPattern.ToString(), sourceIndex);
    }
}

public sealed record EncodedPair(
    string Pair,
    string BarPattern,
    string SpacePattern,
    string BinaryPattern,
    int SourceIndex);

public sealed class InvalidItfInputException(string message) : ArgumentException(message);
