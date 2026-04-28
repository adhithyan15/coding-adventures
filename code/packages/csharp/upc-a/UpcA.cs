using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.UpcA;

public static class UpcA
{
    public const string Version = "0.1.0";

    private const string SideGuard = "101";
    private const string CenterGuard = "01010";

    private static readonly string[] LeftPatterns =
    [
        "0001101", "0011001", "0010011", "0111101", "0100011",
        "0110001", "0101111", "0111011", "0110111", "0001011",
    ];

    private static readonly string[] RightPatterns =
    [
        "1110010", "1100110", "1101100", "1000010", "1011100",
        "1001110", "1010000", "1000100", "1001000", "1110100",
    ];

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static string ComputeUpcACheckDigit(string payload11)
    {
        AssertDigits(payload11, 11);
        var oddSum = 0;
        var evenSum = 0;

        for (var index = 0; index < payload11.Length; index++)
        {
            var value = DigitValue(payload11[index]);
            if (index % 2 == 0)
            {
                oddSum += value;
            }
            else
            {
                evenSum += value;
            }
        }

        return ((10 - ((oddSum * 3 + evenSum) % 10)) % 10).ToString();
    }

    public static string NormalizeUpcA(string data)
    {
        AssertDigits(data, 11, 12);

        if (data.Length == 11)
        {
            return $"{data}{ComputeUpcACheckDigit(data)}";
        }

        var expected = ComputeUpcACheckDigit(data[..11]);
        var actual = data[11].ToString();
        if (expected != actual)
        {
            throw new InvalidUpcACheckDigitException($"invalid UPC-A check digit: expected {expected} but received {actual}");
        }

        return data;
    }

    public static IReadOnlyList<EncodedDigit> EncodeUpcA(string data)
    {
        var normalized = NormalizeUpcA(data);
        return normalized
            .Select((digit, index) => new EncodedDigit(
                digit.ToString(),
                index < 6 ? "L" : "R",
                index < 6 ? LeftPatterns[DigitValue(digit)] : RightPatterns[DigitValue(digit)],
                index,
                index == 11 ? Barcode1DRunRole.Check : Barcode1DRunRole.Data))
            .ToArray();
    }

    public static IReadOnlyList<Barcode1DRun> ExpandUpcARuns(string data)
    {
        var encoded = EncodeUpcA(data);
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

    public static PaintScene LayoutUpcA(string data, PaintBarcode1DOptions? options = null)
    {
        var normalized = NormalizeUpcA(data);
        var encoded = EncodeUpcA(normalized);
        var runs = ExpandUpcARuns(normalized);
        options ??= new PaintBarcode1DOptions();

        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["symbology"] = "upc-a",
        };

        var layoutOptions = options with
        {
            Label = options.Label ?? $"UPC-A barcode for {normalized}",
            HumanReadableText = options.HumanReadableText ?? normalized,
            Metadata = metadata,
            Symbols = BuildSymbols(encoded),
        };

        return BarcodeLayout1D.BarcodeLayout1D.LayoutBarcode1D(runs, layoutOptions);
    }

    public static PaintScene DrawUpcA(string data, PaintBarcode1DOptions? options = null) =>
        LayoutUpcA(data, options);

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
            throw new InvalidUpcAInputException("UPC-A input must contain digits only");
        }

        if (!expectedLengths.Contains(data.Length))
        {
            throw new InvalidUpcAInputException("UPC-A input must contain 11 digits or 12 digits");
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

public class UpcAException(string message) : ArgumentException(message);

public sealed class InvalidUpcAInputException(string message) : UpcAException(message);

public sealed class InvalidUpcACheckDigitException(string message) : UpcAException(message);
