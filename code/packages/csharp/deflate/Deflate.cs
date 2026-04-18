using System.Buffers.Binary;
using System.Text;
using CodingAdventures.HuffmanTree;
using CodingAdventures.Lzss;

namespace CodingAdventures.Deflate;

internal readonly record struct LengthEntry(int Symbol, int Base, int ExtraBits);

internal readonly record struct DistEntry(int Code, int Base, int ExtraBits);

internal sealed class BitBuilder
{
    private int _buffer;
    private int _bitPos;
    private readonly List<byte> _output = [];

    public void WriteBitString(string bits)
    {
        foreach (var bit in bits)
        {
            if (bit == '1')
            {
                _buffer |= 1 << _bitPos;
            }

            _bitPos++;
            if (_bitPos == 8)
            {
                _output.Add((byte)(_buffer & 0xFF));
                _buffer = 0;
                _bitPos = 0;
            }
        }
    }

    public void WriteRawBitsLsb(int value, int count)
    {
        for (var index = 0; index < count; index++)
        {
            if (((value >> index) & 1) != 0)
            {
                _buffer |= 1 << _bitPos;
            }

            _bitPos++;
            if (_bitPos == 8)
            {
                _output.Add((byte)(_buffer & 0xFF));
                _buffer = 0;
                _bitPos = 0;
            }
        }
    }

    public byte[] ToArray()
    {
        if (_bitPos > 0)
        {
            _output.Add((byte)(_buffer & 0xFF));
            _buffer = 0;
            _bitPos = 0;
        }

        return [.. _output];
    }
}

public static class Deflate
{
    private static readonly LengthEntry[] LengthTable =
    [
        new(257, 3, 0), new(258, 4, 0), new(259, 5, 0), new(260, 6, 0),
        new(261, 7, 0), new(262, 8, 0), new(263, 9, 0), new(264, 10, 0),
        new(265, 11, 1), new(266, 13, 1), new(267, 15, 1), new(268, 17, 1),
        new(269, 19, 2), new(270, 23, 2), new(271, 27, 2), new(272, 31, 2),
        new(273, 35, 3), new(274, 43, 3), new(275, 51, 3), new(276, 59, 3),
        new(277, 67, 4), new(278, 83, 4), new(279, 99, 4), new(280, 115, 4),
        new(281, 131, 5), new(282, 163, 5), new(283, 195, 5), new(284, 227, 5)
    ];

    private static readonly DistEntry[] DistTable =
    [
        new(0, 1, 0), new(1, 2, 0), new(2, 3, 0), new(3, 4, 0),
        new(4, 5, 1), new(5, 7, 1), new(6, 9, 2), new(7, 13, 2),
        new(8, 17, 3), new(9, 25, 3), new(10, 33, 4), new(11, 49, 4),
        new(12, 65, 5), new(13, 97, 5), new(14, 129, 6), new(15, 193, 6),
        new(16, 257, 7), new(17, 385, 7), new(18, 513, 8), new(19, 769, 8),
        new(20, 1025, 9), new(21, 1537, 9), new(22, 2049, 10), new(23, 3073, 10)
    ];

    public static byte[] Compress(
        byte[] data,
        int windowSize = Lzss.Lzss.DefaultWindowSize,
        int maxMatch = Lzss.Lzss.DefaultMaxMatch,
        int minMatch = Lzss.Lzss.DefaultMinMatch)
    {
        ArgumentNullException.ThrowIfNull(data);

        if (data.Length == 0)
        {
            var empty = new byte[12];
            BinaryPrimitives.WriteUInt32BigEndian(empty.AsSpan(0, 4), 0u);
            BinaryPrimitives.WriteUInt16BigEndian(empty.AsSpan(4, 2), 1);
            BinaryPrimitives.WriteUInt16BigEndian(empty.AsSpan(6, 2), 0);
            BinaryPrimitives.WriteUInt16BigEndian(empty.AsSpan(8, 2), 256);
            empty[10] = 1;
            empty[11] = 0;
            return empty;
        }

        var tokens = Lzss.Lzss.Encode(data, windowSize, maxMatch, minMatch);
        var llFreq = new Dictionary<int, int>();
        var distFreq = new Dictionary<int, int>();

        foreach (var token in tokens)
        {
            switch (token)
            {
                case LzssLiteral literal:
                    llFreq[literal.Byte] = llFreq.GetValueOrDefault(literal.Byte) + 1;
                    break;

                case LzssMatch match:
                {
                    var lengthSymbol = FindLengthEntry(match.Length).Symbol;
                    var distanceCode = FindDistEntry(match.Offset).Code;
                    llFreq[lengthSymbol] = llFreq.GetValueOrDefault(lengthSymbol) + 1;
                    distFreq[distanceCode] = distFreq.GetValueOrDefault(distanceCode) + 1;
                    break;
                }

                default:
                    throw new InvalidOperationException("Unknown LZSS token type");
            }
        }

        llFreq[256] = llFreq.GetValueOrDefault(256) + 1;

        var llTree = CodingAdventures.HuffmanTree.HuffmanTree.Build(llFreq.Select(pair => (pair.Key, pair.Value)));
        var llCodes = llTree.CanonicalCodeTable();

        IReadOnlyDictionary<int, string> distCodes = new Dictionary<int, string>();
        if (distFreq.Count > 0)
        {
            var distTree = CodingAdventures.HuffmanTree.HuffmanTree.Build(distFreq.Select(pair => (pair.Key, pair.Value)));
            distCodes = distTree.CanonicalCodeTable();
        }

        var bits = new BitBuilder();
        foreach (var token in tokens)
        {
            switch (token)
            {
                case LzssLiteral literal:
                    bits.WriteBitString(llCodes[literal.Byte]);
                    break;

                case LzssMatch match:
                {
                    var lengthEntry = FindLengthEntry(match.Length);
                    bits.WriteBitString(llCodes[lengthEntry.Symbol]);
                    bits.WriteRawBitsLsb(match.Length - lengthEntry.Base, lengthEntry.ExtraBits);

                    var distEntry = FindDistEntry(match.Offset);
                    bits.WriteBitString(distCodes[distEntry.Code]);
                    bits.WriteRawBitsLsb(match.Offset - distEntry.Base, distEntry.ExtraBits);
                    break;
                }
            }
        }

        bits.WriteBitString(llCodes[256]);
        var packedBits = bits.ToArray();

        var llPairs = llCodes
            .Select(pair => (Symbol: pair.Key, Length: pair.Value.Length))
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .ToList();

        var distPairs = distCodes
            .Select(pair => (Symbol: pair.Key, Length: pair.Value.Length))
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .ToList();

        var totalSize = 8 + (llPairs.Count * 3) + (distPairs.Count * 3) + packedBits.Length;
        var output = new byte[totalSize];
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)data.Length);
        BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(4, 2), (ushort)llPairs.Count);
        BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(6, 2), (ushort)distPairs.Count);

        var offset = 8;
        foreach (var pair in llPairs)
        {
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(offset, 2), (ushort)pair.Symbol);
            output[offset + 2] = (byte)pair.Length;
            offset += 3;
        }

        foreach (var pair in distPairs)
        {
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(offset, 2), (ushort)pair.Symbol);
            output[offset + 2] = (byte)pair.Length;
            offset += 3;
        }

        packedBits.CopyTo(output, offset);
        return output;
    }

    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 8)
        {
            return [];
        }

        var originalLength = (int)BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        var llEntryCount = BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(4, 2));
        var distEntryCount = BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(6, 2));

        if (originalLength == 0)
        {
            return [];
        }

        var offset = 8;
        var llLengths = new List<(int Symbol, int Length)>();
        for (var index = 0; index < llEntryCount; index++)
        {
            if (offset + 3 > data.Length)
            {
                return [];
            }

            llLengths.Add((
                BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(offset, 2)),
                data[offset + 2]));
            offset += 3;
        }

        var distLengths = new List<(int Symbol, int Length)>();
        for (var index = 0; index < distEntryCount; index++)
        {
            if (offset + 3 > data.Length)
            {
                return [];
            }

            distLengths.Add((
                BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(offset, 2)),
                data[offset + 2]));
            offset += 3;
        }

        var llCodes = ReconstructCanonicalCodes(llLengths);
        var distCodes = ReconstructCanonicalCodes(distLengths);
        var bits = UnpackBits(data.AsSpan(offset));
        var bitPos = 0;
        var output = new List<byte>(originalLength);

        while (true)
        {
            var llSymbol = NextHuffmanSymbol(llCodes, bits, ref bitPos);
            if (llSymbol == 256)
            {
                break;
            }

            if (llSymbol < 256)
            {
                output.Add((byte)llSymbol);
                continue;
            }

            var lengthEntry = FindLengthEntryBySymbol(llSymbol);
            var length = lengthEntry.Base + ReadRawBits(bits, ref bitPos, lengthEntry.ExtraBits);

            if (distCodes.Count == 0)
            {
                throw new InvalidOperationException("Compressed stream references distance codes without a distance tree");
            }

            var distSymbol = NextHuffmanSymbol(distCodes, bits, ref bitPos);
            var distEntry = FindDistEntryByCode(distSymbol);
            var distance = distEntry.Base + ReadRawBits(bits, ref bitPos, distEntry.ExtraBits);

            var start = output.Count - distance;
            if (start < 0)
            {
                throw new InvalidOperationException("Distance extends before the output buffer");
            }

            for (var index = 0; index < length; index++)
            {
                output.Add(output[start + index]);
            }
        }

        return [.. output.Take(originalLength)];
    }

    private static LengthEntry FindLengthEntry(int length)
    {
        foreach (var entry in LengthTable)
        {
            var max = entry.Base + ((1 << entry.ExtraBits) - 1);
            if (length <= max)
            {
                return entry;
            }
        }

        return LengthTable[^1];
    }

    private static LengthEntry FindLengthEntryBySymbol(int symbol) =>
        LengthTable.FirstOrDefault(entry => entry.Symbol == symbol) is var entry && entry.Symbol != 0
            ? entry
            : throw new InvalidOperationException($"Unknown length symbol {symbol}");

    private static DistEntry FindDistEntry(int distance)
    {
        foreach (var entry in DistTable)
        {
            var max = entry.Base + ((1 << entry.ExtraBits) - 1);
            if (distance <= max)
            {
                return entry;
            }
        }

        return DistTable[^1];
    }

    private static DistEntry FindDistEntryByCode(int code) =>
        DistTable.FirstOrDefault(entry => entry.Code == code) is var entry && entry.Base != 0
            ? entry
            : throw new InvalidOperationException($"Unknown distance symbol {code}");

    private static Dictionary<string, int> ReconstructCanonicalCodes(IEnumerable<(int Symbol, int Length)> lengths)
    {
        var ordered = lengths
            .Where(pair => pair.Length > 0)
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .ToList();

        var result = new Dictionary<string, int>();
        if (ordered.Count == 0)
        {
            return result;
        }

        if (ordered.Count == 1)
        {
            result["0"] = ordered[0].Symbol;
            return result;
        }

        var code = 0;
        var previousLength = ordered[0].Length;
        foreach (var (symbol, length) in ordered)
        {
            if (length > previousLength)
            {
                code <<= length - previousLength;
            }

            result[Convert.ToString(code, 2).PadLeft(length, '0')] = symbol;
            code++;
            previousLength = length;
        }

        return result;
    }

    private static string UnpackBits(ReadOnlySpan<byte> data)
    {
        var builder = new StringBuilder(data.Length * 8);
        foreach (var value in data)
        {
            for (var bit = 0; bit < 8; bit++)
            {
                builder.Append(((value >> bit) & 1) != 0 ? '1' : '0');
            }
        }

        return builder.ToString();
    }

    private static int ReadRawBits(string bits, ref int bitPos, int count)
    {
        var value = 0;
        for (var index = 0; index < count; index++)
        {
            if (bitPos + index >= bits.Length)
            {
                throw new InvalidOperationException("Unexpected end of compressed bit stream");
            }

            if (bits[bitPos + index] == '1')
            {
                value |= 1 << index;
            }
        }

        bitPos += count;
        return value;
    }

    private static int NextHuffmanSymbol(IReadOnlyDictionary<string, int> codes, string bits, ref int bitPos)
    {
        if (codes.Count == 0)
        {
            throw new InvalidOperationException("Missing Huffman codes");
        }

        var builder = new StringBuilder();
        while (bitPos < bits.Length)
        {
            builder.Append(bits[bitPos]);
            bitPos++;
            if (codes.TryGetValue(builder.ToString(), out var symbol))
            {
                return symbol;
            }
        }

        throw new InvalidOperationException("Unexpected end of compressed bit stream");
    }
}
