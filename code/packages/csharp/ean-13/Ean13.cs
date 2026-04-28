using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.Ean13;

public static class Ean13
{
    public const string Version = "0.1.0";

    private const string SideGuard = "101";
    private const string CenterGuard = "01010";

    private static readonly string[] LeftPatterns =
    [
        "0001101", "0011001", "0010011", "0111101", "0100011",
        "0110001", "0101111", "0111011", "0110111", "0001011",
    ];

    private static readonly string[] GPatterns =
    [
        "0100111", "0110011", "0011011", "0100001", "0011101",
        "0111001", "0000101", "0010001", "0001001", "0010111",
    ];

    private static readonly string[] RightPatterns =
    [
        "1110010", "1100110", "1101100", "1000010", "1011100",
        "1001110", "1010000", "1000100", "1001000", "1110100",
    ];

    private static readonly string[] LeftParityPatterns =
    [
        "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG",
        "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL",
    ];

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static string ComputeEan13CheckDigit(string payload12)
    {
        AssertDigits(payload12, 12);
        var total = payload12
            .Reverse()
            .Select((digit, index) => DigitValue(digit) * (index % 2 == 0 ? 3 : 1))
            .Sum();

        return ((10 - (total % 10)) % 10).ToString();
    }

    public static string NormalizeEan13(string data)
    {
        AssertDigits(data, 12, 13);

        if (data.Length == 12)
        {
            return $"{data}{ComputeEan13CheckDigit(data)}";
        }

        var expected = ComputeEan13CheckDigit(data[..12]);
        var actual = data[12].ToString();
        if (expected != actual)
        {
            throw new InvalidEan13CheckDigitException($"invalid EAN-13 check digit: expected {expected} but received {actual}");
        }

        return data;
    }

    public static string LeftParityPattern(string data)
    {
        var normalized = NormalizeEan13(data);
        return LeftParityPatterns[DigitValue(normalized[0])];
    }

    public static IReadOnlyList<EncodedDigit> EncodeEan13(string data)
    {
        var normalized = NormalizeEan13(data);
        var parity = LeftParityPatterns[DigitValue(normalized[0])];
        var encoded = new List<EncodedDigit>();

        for (var offset = 0; offset < 6; offset++)
        {
            var digit = normalized[offset + 1];
            var encoding = parity[offset].ToString();
            var pattern = encoding == "L" ? LeftPatterns[DigitValue(digit)] : GPatterns[DigitValue(digit)];
            encoded.Add(new EncodedDigit(digit.ToString(), encoding, pattern, offset + 1, Barcode1DRunRole.Data));
        }

        for (var offset = 0; offset < 6; offset++)
        {
            var digit = normalized[offset + 7];
            encoded.Add(new EncodedDigit(
                digit.ToString(),
                "R",
                RightPatterns[DigitValue(digit)],
                offset + 7,
                offset == 5 ? Barcode1DRunRole.Check : Barcode1DRunRole.Data));
        }

        return encoded;
    }

    public static IReadOnlyList<Barcode1DRun> ExpandEan13Runs(string data)
    {
        var encoded = EncodeEan13(data);
        var runs = new List<Barcode1DRun>();

        AddPatternRuns(runs, SideGuard, "start", -1, Barcode1DRunRole.Guard);
        foreach (var digit in encoded.Take(6))
        {
            AddPatternRuns(runs, digit.Pattern, digit.Digit, digit.SourceIndex, digit.Role);
        }

        AddPatternRuns(runs, CenterGuard, "center", -2, Barcode1DRunRole.Guard);
        foreach (var digit in encoded.Skip(6))
        {
            AddPatternRuns(runs, digit.Pattern, digit.Digit, digit.SourceIndex, digit.Role);
        }

        AddPatternRuns(runs, SideGuard, "end", -3, Barcode1DRunRole.Guard);
        return runs;
    }

    public static PaintScene LayoutEan13(string data, PaintBarcode1DOptions? options = null)
    {
        var normalized = NormalizeEan13(data);
        var encoded = EncodeEan13(normalized);
        var runs = ExpandEan13Runs(normalized);
        options ??= new PaintBarcode1DOptions();

        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["symbology"] = "ean-13",
            ["leadingDigit"] = normalized[0].ToString(),
            ["leftParity"] = LeftParityPatterns[DigitValue(normalized[0])],
        };

        var layoutOptions = options with
        {
            Label = options.Label ?? $"EAN-13 barcode for {normalized}",
            HumanReadableText = options.HumanReadableText ?? normalized,
            Metadata = metadata,
            Symbols = BuildSymbols(encoded),
        };

        return BarcodeLayout1D.BarcodeLayout1D.LayoutBarcode1D(runs, layoutOptions);
    }

    public static PaintScene DrawEan13(string data, PaintBarcode1DOptions? options = null) =>
        LayoutEan13(data, options);

    private static IReadOnlyList<Barcode1DSymbolDescriptor> BuildSymbols(IReadOnlyList<EncodedDigit> encoded)
    {
        var symbols = new List<Barcode1DSymbolDescriptor>
        {
            new("start", 3, -1, Barcode1DSymbolRole.Guard),
        };

        symbols.AddRange(encoded.Take(6).Select(SymbolFor));
        symbols.Add(new Barcode1DSymbolDescriptor("center", 5, -2, Barcode1DSymbolRole.Guard));
        symbols.AddRange(encoded.Skip(6).Select(SymbolFor));
        symbols.Add(new Barcode1DSymbolDescriptor("end", 3, -3, Barcode1DSymbolRole.Guard));
        return symbols;
    }

    private static Barcode1DSymbolDescriptor SymbolFor(EncodedDigit digit) =>
        new(
            digit.Digit,
            7,
            digit.SourceIndex,
            digit.Role == Barcode1DRunRole.Check ? Barcode1DSymbolRole.Check : Barcode1DSymbolRole.Data);

    private static void AddPatternRuns(
        List<Barcode1DRun> runs,
        string pattern,
        string sourceLabel,
        int sourceIndex,
        Barcode1DRunRole role)
    {
        runs.AddRange(BarcodeLayout1D.BarcodeLayout1D.RunsFromBinaryPattern(
            pattern,
            new RunsFromBinaryPatternOptions(sourceLabel, sourceIndex, role)));
    }

    private static void AssertDigits(string data, params int[] expectedLengths)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Any(ch => ch is < '0' or > '9'))
        {
            throw new InvalidEan13InputException("EAN-13 input must contain digits only");
        }

        if (!expectedLengths.Contains(data.Length))
        {
            throw new InvalidEan13InputException("EAN-13 input must contain 12 digits or 13 digits");
        }
    }

    private static int DigitValue(char digit) => digit - '0';
}

public sealed record EncodedDigit(
    string Digit,
    string Encoding,
    string Pattern,
    int SourceIndex,
    Barcode1DRunRole Role);

public class Ean13Exception(string message) : ArgumentException(message);

public sealed class InvalidEan13InputException(string message) : Ean13Exception(message);

public sealed class InvalidEan13CheckDigitException(string message) : Ean13Exception(message);
