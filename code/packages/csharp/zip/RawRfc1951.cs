using System.IO;

namespace CodingAdventures.Zip;

/// <summary>A stable payload-blind raw RFC 1951 inflate failure.</summary>
public sealed class RawInflateError : Exception
{
    /// <summary>The closed portable error identifier.</summary>
    public string Code { get; }

    internal RawInflateError(string code) : base(code) => Code = code;
}

/// <summary>Decoded bytes plus the exact compressed byte count reached.</summary>
public sealed record RawInflateResult(byte[] Output, int BytesConsumed);

/// <summary>
/// ZIP-owned raw RFC 1951 primitives. The implementation is an in-memory byte
/// transform: it owns no filesystem, process, network, environment, clock,
/// entropy, FFI, or credential authority.
/// </summary>
public static class RawRfc1951
{
    /// <summary>Default and hard output ceiling: 256 MiB.</summary>
    public const int MaxOutput = 256 * 1024 * 1024;

    /// <summary>The complete portable failure taxonomy, in fixture order.</summary>
    public static IReadOnlyList<string> ErrorCodes { get; } = Array.AsReadOnly(
    [
        "invalid-output-limit",
        "unexpected-eof",
        "reserved-block-type",
        "stored-length-mismatch",
        "huffman-oversubscribed",
        "incomplete-code-length-tree",
        "incomplete-literal-length-tree",
        "incomplete-distance-tree",
        "repeat-without-previous",
        "repeat-overrun",
        "invalid-literal-length-symbol",
        "reserved-distance-symbol",
        "invalid-back-reference",
        "output-limit-exceeded",
    ]);

    /// <summary>Compress bytes as raw RFC 1951 without ZIP, zlib, or gzip framing.</summary>
    public static byte[] RawDeflate(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return DeflateCompressor.Compress(data);
    }

    /// <summary>Inflate raw RFC 1951 with a caller-lowerable output ceiling.</summary>
    public static byte[] RawInflate(byte[] data, int maxOutput = MaxOutput) =>
        RawInflateCounted(data, maxOutput).Output;

    /// <summary>Inflate raw RFC 1951 and report the exact final input byte reached.</summary>
    public static RawInflateResult RawInflateCounted(byte[] data, int maxOutput = MaxOutput)
    {
        ArgumentNullException.ThrowIfNull(data);
        return RawInflater.Inflate(data, maxOutput);
    }

    /// <summary>Compute incremental ZIP CRC-32; pass the previous result as initial.</summary>
    public static uint Crc32(byte[] data, uint initial = 0)
    {
        ArgumentNullException.ThrowIfNull(data);
        return Crc32Helper.Compute(data, initial);
    }
}

internal static class RawInflater
{
    private enum Completeness { CodeLength, LiteralLength, Distance }

    private sealed class HuffmanTable
    {
        private readonly Dictionary<int, int>[] _codesByLength;
        private readonly int _maximumLength;

        public HuffmanTable(Dictionary<int, int>[] codesByLength, int maximumLength)
        {
            _codesByLength = codesByLength;
            _maximumLength = maximumLength;
        }

        public int Decode(BitReader reader)
        {
            if (_maximumLength == 0) Fail("unexpected-eof");
            var code = 0;
            for (var length = 1; length <= _maximumLength; length++)
            {
                var bit = reader.ReadLsb(1);
                if (bit is null) Fail("unexpected-eof");
                code = (code << 1) | bit!.Value;
                if (_codesByLength[length].TryGetValue(code, out var symbol)) return symbol;
            }
            return Fail<int>("unexpected-eof");
        }
    }

    public static RawInflateResult Inflate(byte[] data, int maximumOutput)
    {
        if (maximumOutput < 0 || maximumOutput > RawRfc1951.MaxOutput)
            Fail("invalid-output-limit");

        var reader = new BitReader(data);
        var output = new List<byte>();
        while (true)
        {
            var final = reader.ReadLsb(1);
            var type = reader.ReadLsb(2);
            if (final is null || type is null) Fail("unexpected-eof");

            switch (type!.Value)
            {
                case 0:
                    ReadStored(reader, output, maximumOutput);
                    break;
                case 1:
                    {
                        var tables = FixedTables();
                        DecodeCompressed(reader, output, tables.LiteralLength, tables.Distance, maximumOutput);
                        break;
                    }
                case 2:
                    {
                        var tables = ReadDynamicTables(reader);
                        DecodeCompressed(reader, output, tables.LiteralLength, tables.Distance, maximumOutput);
                        break;
                    }
                default:
                    Fail("reserved-block-type");
                    break;
            }

            if (final!.Value == 1)
                return new RawInflateResult([.. output], reader.Position);
        }
    }

    private static void ReadStored(BitReader reader, List<byte> output, int maximumOutput)
    {
        reader.Align();
        var length = reader.ReadLsb(16);
        var complement = reader.ReadLsb(16);
        if (length is null || complement is null) Fail("unexpected-eof");
        if (length!.Value != (complement!.Value ^ 0xffff)) Fail("stored-length-mismatch");
        EnsureCapacity(length.Value, output.Count, maximumOutput);
        for (var index = 0; index < length.Value; index++)
        {
            EnsureCapacity(1, output.Count, maximumOutput);
            var value = reader.ReadLsb(8);
            if (value is null) Fail("unexpected-eof");
            output.Add((byte)value!.Value);
        }
    }

    private static (HuffmanTable LiteralLength, HuffmanTable Distance) FixedTables()
    {
        var literalLengths = new int[288];
        Array.Fill(literalLengths, 8, 0, 144);
        Array.Fill(literalLengths, 9, 144, 112);
        Array.Fill(literalLengths, 7, 256, 24);
        Array.Fill(literalLengths, 8, 280, 8);
        var distanceLengths = Enumerable.Repeat(5, 32).ToArray();
        return (
            BuildHuffman(literalLengths, Completeness.LiteralLength),
            BuildHuffman(distanceLengths, Completeness.Distance));
    }

    private static (HuffmanTable LiteralLength, HuffmanTable Distance) ReadDynamicTables(BitReader reader)
    {
        var rawLiteralCount = reader.ReadLsb(5);
        var rawDistanceCount = reader.ReadLsb(5);
        var rawCodeLengthCount = reader.ReadLsb(4);
        if (rawLiteralCount is null || rawDistanceCount is null || rawCodeLengthCount is null)
            Fail("unexpected-eof");

        var literalCount = rawLiteralCount!.Value + 257;
        var distanceCount = rawDistanceCount!.Value + 1;
        var codeLengthCount = rawCodeLengthCount!.Value + 4;
        if (literalCount > 286) Fail("invalid-literal-length-symbol");

        int[] order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
        var codeLengths = new int[19];
        for (var index = 0; index < codeLengthCount; index++)
        {
            var length = reader.ReadLsb(3);
            if (length is null) Fail("unexpected-eof");
            codeLengths[order[index]] = length!.Value;
        }
        var codeLengthTable = BuildHuffman(codeLengths, Completeness.CodeLength);

        var total = literalCount + distanceCount;
        var lengths = new List<int>(total);
        while (lengths.Count < total)
        {
            switch (codeLengthTable.Decode(reader))
            {
                case var literal when literal is >= 0 and <= 15:
                    lengths.Add(literal);
                    break;
                case 16:
                    {
                        if (lengths.Count == 0) Fail("repeat-without-previous");
                        var extra = reader.ReadLsb(2);
                        if (extra is null) Fail("unexpected-eof");
                        Repeat(lengths, lengths[^1], extra!.Value + 3, total);
                        break;
                    }
                case 17:
                    {
                        var extra = reader.ReadLsb(3);
                        if (extra is null) Fail("unexpected-eof");
                        Repeat(lengths, 0, extra!.Value + 3, total);
                        break;
                    }
                case 18:
                    {
                        var extra = reader.ReadLsb(7);
                        if (extra is null) Fail("unexpected-eof");
                        Repeat(lengths, 0, extra!.Value + 11, total);
                        break;
                    }
                default:
                    Fail("unexpected-eof");
                    break;
            }
        }

        var literalLengths = lengths.Take(literalCount).ToArray();
        var distanceLengths = lengths.Skip(literalCount).ToArray();
        if (literalLengths[256] == 0) Fail("incomplete-literal-length-tree");
        return (
            BuildHuffman(literalLengths, Completeness.LiteralLength),
            BuildHuffman(distanceLengths, Completeness.Distance));
    }

    private static void Repeat(List<int> target, int value, int count, int total)
    {
        if (count > total - target.Count) Fail("repeat-overrun");
        for (var index = 0; index < count; index++) target.Add(value);
    }

    private static HuffmanTable BuildHuffman(int[] lengths, Completeness completeness)
    {
        var counts = new int[16];
        foreach (var length in lengths)
        {
            if (length > 15) Fail("huffman-oversubscribed");
            if (length > 0) counts[length]++;
        }

        var left = 1;
        for (var length = 1; length <= 15; length++)
        {
            left = left * 2 - counts[length];
            if (left < 0) Fail("huffman-oversubscribed");
        }

        var symbolCount = counts.Sum();
        if (left != 0)
        {
            switch (completeness)
            {
                case Completeness.CodeLength:
                    Fail("incomplete-code-length-tree");
                    break;
                case Completeness.LiteralLength:
                    Fail("incomplete-literal-length-tree");
                    break;
                case Completeness.Distance when symbolCount != 0 && !(symbolCount == 1 && counts[1] == 1):
                    Fail("incomplete-distance-tree");
                    break;
            }
        }

        var nextCode = new int[16];
        var code = 0;
        for (var length = 1; length <= 15; length++)
        {
            code = (code + counts[length - 1]) << 1;
            nextCode[length] = code;
        }

        var tables = Enumerable.Range(0, 16).Select(_ => new Dictionary<int, int>()).ToArray();
        var maximumLength = 0;
        for (var symbol = 0; symbol < lengths.Length; symbol++)
        {
            var length = lengths[symbol];
            if (length == 0) continue;
            tables[length][nextCode[length]++] = symbol;
            maximumLength = Math.Max(maximumLength, length);
        }
        return new HuffmanTable(tables, maximumLength);
    }

    private static void DecodeCompressed(
        BitReader reader,
        List<byte> output,
        HuffmanTable literalLength,
        HuffmanTable distance,
        int maximumOutput)
    {
        while (true)
        {
            var symbol = literalLength.Decode(reader);
            if (symbol is >= 0 and <= 255)
            {
                EnsureCapacity(1, output.Count, maximumOutput);
                output.Add((byte)symbol);
            }
            else if (symbol == 256)
            {
                return;
            }
            else if (symbol is >= 257 and <= 285)
            {
                var (baseLength, extraLengthBits) = DeflateTable.Length[symbol - 257];
                var extraLength = reader.ReadLsb(extraLengthBits);
                if (extraLength is null) Fail("unexpected-eof");
                var length = baseLength + extraLength!.Value;
                var distanceSymbol = distance.Decode(reader);
                if (distanceSymbol >= 30) Fail("reserved-distance-symbol");
                var (baseDistance, extraDistanceBits) = DeflateTable.Dist[distanceSymbol];
                var extraDistance = reader.ReadLsb(extraDistanceBits);
                if (extraDistance is null) Fail("unexpected-eof");
                var backwardDistance = baseDistance + extraDistance!.Value;
                if (backwardDistance <= 0 || backwardDistance > output.Count) Fail("invalid-back-reference");
                EnsureCapacity(length, output.Count, maximumOutput);
                for (var index = 0; index < length; index++)
                {
                    EnsureCapacity(1, output.Count, maximumOutput);
                    output.Add(output[output.Count - backwardDistance]);
                }
            }
            else
            {
                Fail("invalid-literal-length-symbol");
            }
        }
    }

    private static void EnsureCapacity(int additional, int current, int maximum)
    {
        if (additional > maximum - current) Fail("output-limit-exceeded");
    }

    private static void Fail(string code) => throw new RawInflateError(code);
    private static T Fail<T>(string code) => throw new RawInflateError(code);
}
