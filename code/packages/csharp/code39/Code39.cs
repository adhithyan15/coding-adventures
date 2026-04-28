using CodingAdventures.BarcodeLayout1D;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.Code39;

public static class Code39
{
    public const string Version = "0.1.0";

    private static readonly IReadOnlyDictionary<string, string> Patterns = new Dictionary<string, string>
    {
        ["0"] = "bwbWBwBwb", ["1"] = "BwbWbwbwB", ["2"] = "bwBWbwbwB", ["3"] = "BwBWbwbwb",
        ["4"] = "bwbWBwbwB", ["5"] = "BwbWBwbwb", ["6"] = "bwBWBwbwb", ["7"] = "bwbWbwBwB",
        ["8"] = "BwbWbwBwb", ["9"] = "bwBWbwBwb", ["A"] = "BwbwbWbwB", ["B"] = "bwBwbWbwB",
        ["C"] = "BwBwbWbwb", ["D"] = "bwbwBWbwB", ["E"] = "BwbwBWbwb", ["F"] = "bwBwBWbwb",
        ["G"] = "bwbwbWBwB", ["H"] = "BwbwbWBwb", ["I"] = "bwBwbWBwb", ["J"] = "bwbwBWBwb",
        ["K"] = "BwbwbwbWB", ["L"] = "bwBwbwbWB", ["M"] = "BwBwbwbWb", ["N"] = "bwbwBwbWB",
        ["O"] = "BwbwBwbWb", ["P"] = "bwBwBwbWb", ["Q"] = "bwbwbwBWB", ["R"] = "BwbwbwBWb",
        ["S"] = "bwBwbwBWb", ["T"] = "bwbwBwBWb", ["U"] = "BWbwbwbwB", ["V"] = "bWBwbwbwB",
        ["W"] = "BWBwbwbwb", ["X"] = "bWbwBwbwB", ["Y"] = "BWbwBwbwb", ["Z"] = "bWBwBwbwb",
        ["-"] = "bWbwbwBwB", ["."] = "BWbwbwBwb", [" "] = "bWBwbwBwb", ["$"] = "bWbWbWbwb",
        ["/"] = "bWbWbwbWb", ["+"] = "bWbwbWbWb", ["%"] = "bwbWbWbWb", ["*"] = "bWbwBwBwb",
    };

    public static Barcode1DRenderConfig DefaultRenderConfig() => new();

    public static string NormalizeCode39(string data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var normalized = data.ToUpperInvariant();
        foreach (var ch in normalized)
        {
            var value = ch.ToString();
            if (value == "*")
            {
                throw new InvalidCharacterException("input must not contain \"*\" because it is reserved for start/stop");
            }

            if (!Patterns.ContainsKey(value))
            {
                throw new InvalidCharacterException($"invalid character: \"{value}\" is not supported by Code 39");
            }
        }

        return normalized;
    }

    public static EncodedCharacter EncodeCode39Char(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (!Patterns.TryGetValue(value, out var pattern))
        {
            throw new InvalidCharacterException($"invalid character: \"{value}\" is not supported by Code 39");
        }

        return new EncodedCharacter(value, value == "*", WidthPattern(pattern));
    }

    public static IReadOnlyList<EncodedCharacter> EncodeCode39(string data)
    {
        var normalized = NormalizeCode39(data);
        return $"*{normalized}*"
            .Select(ch => EncodeCode39Char(ch.ToString()))
            .ToArray();
    }

    public static IReadOnlyList<Barcode1DRun> ExpandCode39Runs(string data)
    {
        var encoded = EncodeCode39(data);
        var runs = new List<Barcode1DRun>();

        for (var sourceIndex = 0; sourceIndex < encoded.Count; sourceIndex++)
        {
            var encodedChar = encoded[sourceIndex];
            var role = RunRoleFor(sourceIndex, encoded.Count, encodedChar);
            foreach (var run in BarcodeLayout1D.BarcodeLayout1D.RunsFromWidthPattern(
                         encodedChar.Pattern,
                         new RunsFromWidthPatternOptions(encodedChar.Char, sourceIndex, role)))
            {
                runs.Add(run);
            }

            if (sourceIndex < encoded.Count - 1)
            {
                runs.Add(new Barcode1DRun(
                    Barcode1DRunColor.Space,
                    1,
                    encodedChar.Char,
                    sourceIndex,
                    Barcode1DRunRole.InterCharacterGap));
            }
        }

        return runs;
    }

    public static PaintScene LayoutCode39(string data, PaintBarcode1DOptions? options = null)
    {
        var normalized = NormalizeCode39(data);
        var runs = ExpandCode39Runs(normalized);
        options ??= new PaintBarcode1DOptions();

        var metadata = new Dictionary<string, object?>(options.Metadata)
        {
            ["symbology"] = "code39",
            ["encodedText"] = normalized,
        };

        var layoutOptions = options with
        {
            Label = options.Label ?? (normalized.Length == 0 ? "Code 39 barcode" : $"Code 39 barcode for {normalized}"),
            HumanReadableText = options.HumanReadableText ?? normalized,
            Metadata = metadata,
        };

        return BarcodeLayout1D.BarcodeLayout1D.LayoutBarcode1D(runs, layoutOptions);
    }

    public static PaintScene DrawCode39(string data, PaintBarcode1DOptions? options = null) =>
        LayoutCode39(data, options);

    private static string WidthPattern(string pattern) =>
        new(pattern.Select(part => char.IsUpper(part) ? 'W' : 'N').ToArray());

    private static Barcode1DRunRole RunRoleFor(int sourceIndex, int encodedLength, EncodedCharacter encodedCharacter)
    {
        if (!encodedCharacter.IsStartStop)
        {
            return Barcode1DRunRole.Data;
        }

        if (sourceIndex == 0)
        {
            return Barcode1DRunRole.Start;
        }

        return sourceIndex == encodedLength - 1 ? Barcode1DRunRole.Stop : Barcode1DRunRole.Guard;
    }
}

public sealed record EncodedCharacter(string Char, bool IsStartStop, string Pattern);

public sealed class InvalidCharacterException(string message) : ArgumentException(message);
