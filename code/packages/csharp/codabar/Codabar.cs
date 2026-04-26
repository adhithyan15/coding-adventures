using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.Codabar;

public static class Codabar
{
    public const string Version = "0.1.0";

    private static readonly HashSet<string> Guards = ["A", "B", "C", "D"];

    private static readonly IReadOnlyDictionary<string, string> Patterns = new Dictionary<string, string>
    {
        ["0"] = "101010011",
        ["1"] = "101011001",
        ["2"] = "101001011",
        ["3"] = "110010101",
        ["4"] = "101101001",
        ["5"] = "110101001",
        ["6"] = "100101011",
        ["7"] = "100101101",
        ["8"] = "100110101",
        ["9"] = "110100101",
        ["-"] = "101001101",
        ["$"] = "101100101",
        [":"] = "1101011011",
        ["/"] = "1101101011",
        ["."] = "1101101101",
        ["+"] = "1011011011",
        ["A"] = "1011001001",
        ["B"] = "1001001011",
        ["C"] = "1010010011",
        ["D"] = "1010011001",
    };

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static string NormalizeCodabar(string data, string start = "A", string stop = "A")
    {
        ArgumentNullException.ThrowIfNull(data);
        ArgumentNullException.ThrowIfNull(start);
        ArgumentNullException.ThrowIfNull(stop);
        var normalized = data.ToUpperInvariant();

        if (normalized.Length >= 2)
        {
            var first = normalized[0].ToString();
            var last = normalized[^1].ToString();
            if (IsGuard(first) && IsGuard(last))
            {
                AssertBodyChars(normalized[1..^1]);
                return normalized;
            }
        }

        ValidateGuard(start, nameof(start));
        ValidateGuard(stop, nameof(stop));
        AssertBodyChars(normalized);
        return $"{start.ToUpperInvariant()}{normalized}{stop.ToUpperInvariant()}";
    }

    public static IReadOnlyList<EncodedCodabarSymbol> EncodeCodabar(string data, string start = "A", string stop = "A")
    {
        var normalized = NormalizeCodabar(data, start, stop);
        return normalized
            .Select((ch, index) =>
            {
                var value = ch.ToString();
                return new EncodedCodabarSymbol(
                    value,
                    Patterns[value],
                    index,
                    index == 0 ? Barcode1DRunRole.Start : index == normalized.Length - 1 ? Barcode1DRunRole.Stop : Barcode1DRunRole.Data);
            })
            .ToArray();
    }

    public static IReadOnlyList<Barcode1DRun> ExpandCodabarRuns(string data, string start = "A", string stop = "A")
    {
        var encoded = EncodeCodabar(data, start, stop);
        var runs = new List<Barcode1DRun>();

        for (var index = 0; index < encoded.Count; index++)
        {
            var symbol = encoded[index];
            foreach (var run in BarcodeLayout1D.BarcodeLayout1D.RunsFromBinaryPattern(
                         symbol.Pattern,
                         new RunsFromBinaryPatternOptions(symbol.Char, symbol.SourceIndex, symbol.Role)))
            {
                runs.Add(run);
            }

            if (index < encoded.Count - 1)
            {
                runs.Add(new Barcode1DRun(
                    Barcode1DRunColor.Space,
                    1,
                    symbol.Char,
                    symbol.SourceIndex,
                    Barcode1DRunRole.InterCharacterGap));
            }
        }

        return runs;
    }

    public static PaintScene LayoutCodabar(
        string data,
        PaintBarcode1DOptions? options = null,
        string start = "A",
        string stop = "A")
    {
        var normalized = NormalizeCodabar(data, start, stop);
        var runs = ExpandCodabarRuns(normalized);
        options ??= new PaintBarcode1DOptions();

        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["symbology"] = "codabar",
            ["start"] = normalized[0].ToString(),
            ["stop"] = normalized[^1].ToString(),
        };

        var layoutOptions = options with
        {
            Label = options.Label ?? $"Codabar barcode for {normalized}",
            HumanReadableText = options.HumanReadableText ?? normalized,
            Metadata = metadata,
        };

        return BarcodeLayout1D.BarcodeLayout1D.LayoutBarcode1D(runs, layoutOptions);
    }

    public static PaintScene DrawCodabar(
        string data,
        PaintBarcode1DOptions? options = null,
        string start = "A",
        string stop = "A") =>
        LayoutCodabar(data, options, start, stop);

    private static bool IsGuard(string value) => Guards.Contains(value);

    private static void ValidateGuard(string value, string paramName)
    {
        var normalized = value.ToUpperInvariant();
        if (!IsGuard(normalized))
        {
            throw new InvalidCodabarInputException($"Codabar {paramName} guard must be one of A, B, C, or D");
        }
    }

    private static void AssertBodyChars(string body)
    {
        foreach (var ch in body)
        {
            var value = ch.ToString();
            if (!Patterns.ContainsKey(value) || IsGuard(value))
            {
                throw new InvalidCodabarInputException($"invalid Codabar body character \"{value}\"");
            }
        }
    }
}

public sealed record EncodedCodabarSymbol(
    string Char,
    string Pattern,
    int SourceIndex,
    Barcode1DRunRole Role);

public sealed class InvalidCodabarInputException(string message) : ArgumentException(message);
