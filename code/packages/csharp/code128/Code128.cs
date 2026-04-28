using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.Code128;

public static class Code128
{
    public const string Version = "0.1.0";

    private const int StartB = 104;
    private const int Stop = 106;

    private static readonly string[] Patterns =
    [
        "11011001100", "11001101100", "11001100110", "10010011000", "10010001100",
        "10001001100", "10011001000", "10011000100", "10001100100", "11001001000",
        "11001000100", "11000100100", "10110011100", "10011011100", "10011001110",
        "10111001100", "10011101100", "10011100110", "11001110010", "11001011100",
        "11001001110", "11011100100", "11001110100", "11101101110", "11101001100",
        "11100101100", "11100100110", "11101100100", "11100110100", "11100110010",
        "11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
        "10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
        "11000101000", "11000100010", "10110111000", "10110001110", "10001101110",
        "10111011000", "10111000110", "10001110110", "11101110110", "11010001110",
        "11000101110", "11011101000", "11011100010", "11011101110", "11101011000",
        "11101000110", "11100010110", "11101101000", "11101100010", "11100011010",
        "11101111010", "11001000010", "11110001010", "10100110000", "10100001100",
        "10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
        "10110000100", "10011010000", "10011000010", "10000110100", "10000110010",
        "11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
        "10100111100", "10010111100", "10010011110", "10111100100", "10011110100",
        "10011110010", "11110100100", "11110010100", "11110010010", "11011011110",
        "11011110110", "11110110110", "10101111000", "10100011110", "10001011110",
        "10111101000", "10111100010", "11110101000", "11110100010", "10111011110",
        "10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
        "11010011100", "1100011101011",
    ];

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static string NormalizeCode128B(string data)
    {
        ArgumentNullException.ThrowIfNull(data);
        foreach (var ch in data)
        {
            if (ch is < ' ' or > '~')
            {
                throw new InvalidCode128InputException("Code 128 Code Set B supports printable ASCII characters only");
            }
        }

        return data;
    }

    public static int ComputeCode128Checksum(IReadOnlyList<int> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        var total = StartB;
        for (var index = 0; index < values.Count; index++)
        {
            total += values[index] * (index + 1);
        }

        return total % 103;
    }

    public static IReadOnlyList<EncodedCode128Symbol> EncodeCode128B(string data)
    {
        var normalized = NormalizeCode128B(data);
        var dataSymbols = normalized
            .Select((ch, index) =>
            {
                var value = ValueForCode128BChar(ch);
                return new EncodedCode128Symbol(ch.ToString(), value, Patterns[value], index, Barcode1DRunRole.Data);
            })
            .ToArray();
        var checksum = ComputeCode128Checksum(dataSymbols.Select(symbol => symbol.Value).ToArray());

        return new[]
        {
            new EncodedCode128Symbol("Start B", StartB, Patterns[StartB], -1, Barcode1DRunRole.Start),
        }
        .Concat(dataSymbols)
        .Concat(
        [
            new EncodedCode128Symbol($"Checksum {checksum}", checksum, Patterns[checksum], normalized.Length, Barcode1DRunRole.Check),
            new EncodedCode128Symbol("Stop", Stop, Patterns[Stop], normalized.Length + 1, Barcode1DRunRole.Stop),
        ])
        .ToArray();
    }

    public static IReadOnlyList<Barcode1DRun> ExpandCode128Runs(string data)
    {
        var encoded = EncodeCode128B(data);
        var runs = new List<Barcode1DRun>();

        foreach (var symbol in encoded)
        {
            runs.AddRange(BarcodeLayout1D.BarcodeLayout1D.RunsFromBinaryPattern(
                symbol.Pattern,
                new RunsFromBinaryPatternOptions(symbol.Label, symbol.SourceIndex, symbol.Role)));
        }

        return runs;
    }

    public static PaintScene LayoutCode128(string data, PaintBarcode1DOptions? options = null)
    {
        var normalized = NormalizeCode128B(data);
        var encoded = EncodeCode128B(normalized);
        var checksum = encoded[^2].Value;
        var runs = ExpandCode128Runs(normalized);
        options ??= new PaintBarcode1DOptions();

        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["symbology"] = "code128",
            ["codeSet"] = "B",
            ["checksum"] = checksum,
        };

        var layoutOptions = options with
        {
            Label = options.Label ?? $"Code 128 barcode for {normalized}",
            HumanReadableText = options.HumanReadableText ?? normalized,
            Metadata = metadata,
            Symbols = BuildSymbols(encoded),
        };

        return BarcodeLayout1D.BarcodeLayout1D.LayoutBarcode1D(runs, layoutOptions);
    }

    public static PaintScene DrawCode128(string data, PaintBarcode1DOptions? options = null) =>
        LayoutCode128(data, options);

    private static int ValueForCode128BChar(char ch) => ch - 32;

    private static IReadOnlyList<Barcode1DSymbolDescriptor> BuildSymbols(IReadOnlyList<EncodedCode128Symbol> encoded) =>
        encoded
            .Select(symbol => new Barcode1DSymbolDescriptor(
                symbol.Label,
                symbol.Role == Barcode1DRunRole.Stop ? 13u : 11u,
                symbol.SourceIndex,
                symbol.Role switch
                {
                    Barcode1DRunRole.Start => Barcode1DSymbolRole.Start,
                    Barcode1DRunRole.Check => Barcode1DSymbolRole.Check,
                    Barcode1DRunRole.Stop => Barcode1DSymbolRole.Stop,
                    _ => Barcode1DSymbolRole.Data,
                }))
            .ToArray();
}

public sealed record EncodedCode128Symbol(
    string Label,
    int Value,
    string Pattern,
    int SourceIndex,
    Barcode1DRunRole Role);

public sealed class InvalidCode128InputException(string message) : ArgumentException(message);
