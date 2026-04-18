using System.Buffers.Binary;
using System.Text;
using CodingAdventures.HuffmanTree;
using HuffmanCodec = CodingAdventures.HuffmanTree.HuffmanTree;

namespace CodingAdventures.Brotli;

internal readonly record struct IccEntry(int InsertBase, int InsertExtra, int CopyBase, int CopyExtra);

internal readonly record struct DistEntry(int Code, int Base, int ExtraBits);

internal readonly record struct Command(int InsertLength, int CopyLength, int CopyDistance, byte[] Literals);

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

public static class Brotli
{
    private const int MaxWindow = 65535;
    private const int MinMatch = 4;
    private const int MaxMatch = 258;
    private const int MaxInsertPerIcc = 32;

    private static readonly IccEntry[] IccTable =
    [
        new(0, 0, 4, 0), new(0, 0, 5, 0), new(0, 0, 6, 0), new(0, 0, 8, 1),
        new(0, 0, 10, 1), new(0, 0, 14, 2), new(0, 0, 18, 2), new(0, 0, 26, 3),
        new(0, 0, 34, 3), new(0, 0, 50, 4), new(0, 0, 66, 4), new(0, 0, 98, 5),
        new(0, 0, 130, 5), new(0, 0, 194, 6), new(0, 0, 258, 7), new(0, 0, 514, 8),
        new(1, 0, 4, 0), new(1, 0, 5, 0), new(1, 0, 6, 0), new(1, 0, 8, 1),
        new(1, 0, 10, 1), new(1, 0, 14, 2), new(1, 0, 18, 2), new(1, 0, 26, 3),
        new(2, 0, 4, 0), new(2, 0, 5, 0), new(2, 0, 6, 0), new(2, 0, 8, 1),
        new(2, 0, 10, 1), new(2, 0, 14, 2), new(2, 0, 18, 2), new(2, 0, 26, 3),
        new(3, 1, 4, 0), new(3, 1, 5, 0), new(3, 1, 6, 0), new(3, 1, 8, 1),
        new(3, 1, 10, 1), new(3, 1, 14, 2), new(3, 1, 18, 2), new(3, 1, 26, 3),
        new(5, 2, 4, 0), new(5, 2, 5, 0), new(5, 2, 6, 0), new(5, 2, 8, 1),
        new(5, 2, 10, 1), new(5, 2, 14, 2), new(5, 2, 18, 2), new(5, 2, 26, 3),
        new(9, 3, 4, 0), new(9, 3, 5, 0), new(9, 3, 6, 0), new(9, 3, 8, 1),
        new(9, 3, 10, 1), new(9, 3, 14, 2), new(9, 3, 18, 2), new(9, 3, 26, 3),
        new(17, 4, 4, 0), new(17, 4, 5, 0), new(17, 4, 6, 0), new(17, 4, 8, 1),
        new(17, 4, 10, 1), new(17, 4, 14, 2), new(17, 4, 18, 2), new(0, 0, 0, 0)
    ];

    private static readonly DistEntry[] DistTable =
    [
        new(0, 1, 0), new(1, 2, 0), new(2, 3, 0), new(3, 4, 0),
        new(4, 5, 1), new(5, 7, 1), new(6, 9, 2), new(7, 13, 2),
        new(8, 17, 3), new(9, 25, 3), new(10, 33, 4), new(11, 49, 4),
        new(12, 65, 5), new(13, 97, 5), new(14, 129, 6), new(15, 193, 6),
        new(16, 257, 7), new(17, 385, 7), new(18, 513, 8), new(19, 769, 8),
        new(20, 1025, 9), new(21, 1537, 9), new(22, 2049, 10), new(23, 3073, 10),
        new(24, 4097, 11), new(25, 6145, 11), new(26, 8193, 12), new(27, 12289, 12),
        new(28, 16385, 13), new(29, 24577, 13), new(30, 32769, 14), new(31, 49153, 14)
    ];

    public static byte[] Compress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);

        if (data.Length == 0)
        {
            return
            [
                0, 0, 0, 0,
                1, 0, 0, 0, 0, 0,
                63, 1,
                0
            ];
        }

        var (commands, flushLiterals) = BuildCommands(data);

        var literalFrequencies = Enumerable.Range(0, 4)
            .Select(_ => new Dictionary<int, int>())
            .ToArray();
        var iccFrequencies = new Dictionary<int, int>();
        var distFrequencies = new Dictionary<int, int>();
        var history = new List<byte>();

        foreach (var command in commands)
        {
            if (command.CopyLength == 0)
            {
                break;
            }

            var icc = FindIccCode(command.InsertLength, command.CopyLength);
            iccFrequencies[icc] = iccFrequencies.GetValueOrDefault(icc) + 1;

            var distanceCode = DistCode(command.CopyDistance);
            distFrequencies[distanceCode] = distFrequencies.GetValueOrDefault(distanceCode) + 1;

            foreach (var literal in command.Literals)
            {
                var ctx = LiteralContext(history.Count > 0 ? history[^1] : -1);
                literalFrequencies[ctx][literal] = literalFrequencies[ctx].GetValueOrDefault(literal) + 1;
                history.Add(literal);
            }

            var start = history.Count - command.CopyDistance;
            for (var index = 0; index < command.CopyLength; index++)
            {
                history.Add(history[start + index]);
            }
        }

        iccFrequencies[63] = iccFrequencies.GetValueOrDefault(63) + 1;

        var previousFlush = history.Count > 0 ? history[^1] : -1;
        foreach (var literal in flushLiterals)
        {
            var ctx = LiteralContext(previousFlush);
            literalFrequencies[ctx][literal] = literalFrequencies[ctx].GetValueOrDefault(literal) + 1;
            previousFlush = literal;
        }

        var iccCodeTable = HuffmanCodec.Build(iccFrequencies.Select(pair => (pair.Key, pair.Value))).CanonicalCodeTable();

        IReadOnlyDictionary<int, string> distCodeTable = new Dictionary<int, string>();
        if (distFrequencies.Count > 0)
        {
            distCodeTable = HuffmanCodec.Build(distFrequencies.Select(pair => (pair.Key, pair.Value))).CanonicalCodeTable();
        }

        var literalCodeTables = new IReadOnlyDictionary<int, string>[4];
        for (var ctx = 0; ctx < 4; ctx++)
        {
            literalCodeTables[ctx] = literalFrequencies[ctx].Count > 0
                ? HuffmanCodec.Build(literalFrequencies[ctx].Select(pair => (pair.Key, pair.Value))).CanonicalCodeTable()
                : new Dictionary<int, string>();
        }

        var builder = new BitBuilder();
        var encodedHistory = new List<byte>();

        foreach (var command in commands)
        {
            if (command.CopyLength == 0)
            {
                builder.WriteBitString(iccCodeTable[63]);

                var flushPrevious = encodedHistory.Count > 0 ? encodedHistory[^1] : -1;
                foreach (var literal in flushLiterals)
                {
                    var ctx = LiteralContext(flushPrevious);
                    builder.WriteBitString(literalCodeTables[ctx][literal]);
                    flushPrevious = literal;
                }

                break;
            }

            var icc = FindIccCode(command.InsertLength, command.CopyLength);
            var entry = IccTable[icc];
            builder.WriteBitString(iccCodeTable[icc]);
            builder.WriteRawBitsLsb(command.InsertLength - entry.InsertBase, entry.InsertExtra);
            builder.WriteRawBitsLsb(command.CopyLength - entry.CopyBase, entry.CopyExtra);

            foreach (var literal in command.Literals)
            {
                var ctx = LiteralContext(encodedHistory.Count > 0 ? encodedHistory[^1] : -1);
                builder.WriteBitString(literalCodeTables[ctx][literal]);
                encodedHistory.Add(literal);
            }

            var distanceCode = DistCode(command.CopyDistance);
            var distanceEntry = DistTable[distanceCode];
            builder.WriteBitString(distCodeTable[distanceCode]);
            builder.WriteRawBitsLsb(command.CopyDistance - distanceEntry.Base, distanceEntry.ExtraBits);

            var start = encodedHistory.Count - command.CopyDistance;
            for (var index = 0; index < command.CopyLength; index++)
            {
                encodedHistory.Add(encodedHistory[start + index]);
            }
        }

        var packedBits = builder.ToArray();
        var iccPairs = SortedPairs(iccCodeTable);
        var distPairs = SortedPairs(distCodeTable);
        var literalPairs = literalCodeTables.Select(SortedPairs).ToArray();

        EnsureCountFits("ICC entries", iccPairs.Count);
        EnsureCountFits("distance entries", distPairs.Count);
        for (var ctx = 0; ctx < literalPairs.Length; ctx++)
        {
            EnsureCountFits($"literal context {ctx} entries", literalPairs[ctx].Count);
        }

        var totalSize = 10
            + (iccPairs.Count * 2)
            + (distPairs.Count * 2)
            + literalPairs.Sum(pairs => pairs.Count * 3)
            + packedBits.Length;

        var output = new byte[totalSize];
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)data.Length);
        output[4] = (byte)iccPairs.Count;
        output[5] = (byte)distPairs.Count;
        output[6] = (byte)literalPairs[0].Count;
        output[7] = (byte)literalPairs[1].Count;
        output[8] = (byte)literalPairs[2].Count;
        output[9] = (byte)literalPairs[3].Count;

        var offset = 10;
        foreach (var (symbol, length) in iccPairs)
        {
            output[offset] = (byte)symbol;
            output[offset + 1] = (byte)length;
            offset += 2;
        }

        foreach (var (symbol, length) in distPairs)
        {
            output[offset] = (byte)symbol;
            output[offset + 1] = (byte)length;
            offset += 2;
        }

        foreach (var pairs in literalPairs)
        {
            foreach (var (symbol, length) in pairs)
            {
                BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(offset, 2), (ushort)symbol);
                output[offset + 2] = (byte)length;
                offset += 3;
            }
        }

        packedBits.CopyTo(output, offset);
        return output;
    }

    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);

        if (data.Length < 10)
        {
            return [];
        }

        var originalLength = (int)BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        if (originalLength == 0)
        {
            return [];
        }

        var iccEntryCount = data[4];
        var distEntryCount = data[5];
        var literalEntryCounts = new[] { data[6], data[7], data[8], data[9] };

        var offset = 10;
        if (offset + (iccEntryCount * 2) > data.Length)
        {
            return [];
        }

        var iccLengths = new List<(int Symbol, int Length)>(iccEntryCount);
        for (var index = 0; index < iccEntryCount; index++)
        {
            iccLengths.Add((data[offset], data[offset + 1]));
            offset += 2;
        }

        if (offset + (distEntryCount * 2) > data.Length)
        {
            return [];
        }

        var distLengths = new List<(int Symbol, int Length)>(distEntryCount);
        for (var index = 0; index < distEntryCount; index++)
        {
            distLengths.Add((data[offset], data[offset + 1]));
            offset += 2;
        }

        var literalLengths = new List<(int Symbol, int Length)>[4];
        for (var ctx = 0; ctx < 4; ctx++)
        {
            var count = literalEntryCounts[ctx];
            if (offset + (count * 3) > data.Length)
            {
                return [];
            }

            literalLengths[ctx] = [];
            for (var index = 0; index < count; index++)
            {
                literalLengths[ctx].Add((
                    BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(offset, 2)),
                    data[offset + 2]));
                offset += 3;
            }
        }

        var iccCodes = ReconstructCanonicalCodes(iccLengths);
        var distCodes = ReconstructCanonicalCodes(distLengths);
        var literalCodes = literalLengths.Select(ReconstructCanonicalCodes).ToArray();
        var bits = UnpackBits(data.AsSpan(offset));
        var bitPos = 0;
        var output = new List<byte>(originalLength);
        var previous = -1;

        while (output.Count < originalLength)
        {
            var icc = NextHuffmanSymbol(iccCodes, bits, ref bitPos);

            if (icc == 63)
            {
                while (output.Count < originalLength)
                {
                    var ctx = LiteralContext(previous);
                    var literal = NextHuffmanSymbol(literalCodes[ctx], bits, ref bitPos);
                    output.Add((byte)literal);
                    previous = literal;
                }

                break;
            }

            var entry = IccTable[icc];
            var insertLength = entry.InsertBase + ReadRawBits(bits, ref bitPos, entry.InsertExtra);
            var copyLength = entry.CopyBase + ReadRawBits(bits, ref bitPos, entry.CopyExtra);

            for (var index = 0; index < insertLength; index++)
            {
                var ctx = LiteralContext(previous);
                var literal = NextHuffmanSymbol(literalCodes[ctx], bits, ref bitPos);
                output.Add((byte)literal);
                previous = literal;
            }

            if (copyLength == 0)
            {
                continue;
            }

            if (distCodes.Count == 0)
            {
                throw new InvalidOperationException("Compressed stream references a distance code without a distance tree");
            }

            var distanceCode = NextHuffmanSymbol(distCodes, bits, ref bitPos);
            var distanceEntry = DistTable[distanceCode];
            var copyDistance = distanceEntry.Base + ReadRawBits(bits, ref bitPos, distanceEntry.ExtraBits);
            var start = output.Count - copyDistance;
            if (start < 0)
            {
                throw new InvalidOperationException("Distance extends before the output buffer");
            }

            for (var index = 0; index < copyLength; index++)
            {
                var value = output[start + index];
                output.Add(value);
                previous = value;
            }
        }

        return output.Take(originalLength).ToArray();
    }

    private static int LiteralContext(int previousByte)
    {
        if (previousByte is >= 0x61 and <= 0x7A)
        {
            return 3;
        }

        if (previousByte is >= 0x41 and <= 0x5A)
        {
            return 2;
        }

        return previousByte is >= 0x30 and <= 0x39 ? 1 : 0;
    }

    private static int FindIccCode(int insertLength, int copyLength)
    {
        for (var code = 0; code < 63; code++)
        {
            var entry = IccTable[code];
            var maxInsert = entry.InsertBase + (1 << entry.InsertExtra) - 1;
            var maxCopy = entry.CopyBase + (1 << entry.CopyExtra) - 1;
            if (insertLength >= entry.InsertBase
                && insertLength <= maxInsert
                && copyLength >= entry.CopyBase
                && copyLength <= maxCopy)
            {
                return code;
            }
        }

        for (var code = 0; code < 16; code++)
        {
            var entry = IccTable[code];
            var maxCopy = entry.CopyBase + (1 << entry.CopyExtra) - 1;
            if (copyLength >= entry.CopyBase && copyLength <= maxCopy)
            {
                return code;
            }
        }

        return 0;
    }

    private static int FindBestIccCopy(int insertLength, int copyLength)
    {
        var best = 0;
        for (var code = 0; code < 63; code++)
        {
            var entry = IccTable[code];
            var maxInsert = entry.InsertBase + (1 << entry.InsertExtra) - 1;
            if (insertLength < entry.InsertBase || insertLength > maxInsert)
            {
                continue;
            }

            var maxCopy = entry.CopyBase + (1 << entry.CopyExtra) - 1;
            if (copyLength >= entry.CopyBase && copyLength <= maxCopy)
            {
                return copyLength;
            }

            if (maxCopy <= copyLength && maxCopy > best)
            {
                best = maxCopy;
            }
        }

        return Math.Max(best, MinMatch);
    }

    private static int DistCode(int distance)
    {
        foreach (var entry in DistTable)
        {
            var maxDistance = entry.Base + (1 << entry.ExtraBits) - 1;
            if (distance <= maxDistance)
            {
                return entry.Code;
            }
        }

        return 31;
    }

    private static (int Offset, int Length) FindLongestMatch(byte[] data, int pos)
    {
        var windowStart = Math.Max(0, pos - MaxWindow);
        var bestLength = 0;
        var bestOffset = 0;

        for (var start = pos - 1; start >= windowStart; start--)
        {
            if (data[start] != data[pos])
            {
                continue;
            }

            var maxLength = Math.Min(MaxMatch, data.Length - pos);
            var matchLength = 0;
            while (matchLength < maxLength && data[start + matchLength] == data[pos + matchLength])
            {
                matchLength++;
            }

            if (matchLength > bestLength)
            {
                bestLength = matchLength;
                bestOffset = pos - start;
                if (bestLength == MaxMatch)
                {
                    break;
                }
            }
        }

        return bestLength < MinMatch ? (0, 0) : (bestOffset, bestLength);
    }

    private static (List<Command> Commands, List<byte> FlushLiterals) BuildCommands(byte[] data)
    {
        var commands = new List<Command>();
        var insertBuffer = new List<byte>();
        var pos = 0;

        while (pos < data.Length)
        {
            var (offset, length) = FindLongestMatch(data, pos);
            if (length >= MinMatch && insertBuffer.Count <= MaxInsertPerIcc)
            {
                var actualCopy = FindBestIccCopy(insertBuffer.Count, length);
                commands.Add(new Command(insertBuffer.Count, actualCopy, offset, [.. insertBuffer]));
                insertBuffer.Clear();
                pos += actualCopy;
            }
            else
            {
                insertBuffer.Add(data[pos]);
                pos++;
            }
        }

        var flushLiterals = insertBuffer.ToList();
        commands.Add(new Command(0, 0, 0, []));
        return (commands, flushLiterals);
    }

    private static List<(int Symbol, int Length)> SortedPairs(IReadOnlyDictionary<int, string> table) =>
        table.Select(pair => (Symbol: pair.Key, Length: pair.Value.Length))
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .Select(pair => (pair.Symbol, pair.Length))
            .ToList();

    private static void EnsureCountFits(string label, int count)
    {
        if (count > byte.MaxValue)
        {
            throw new InvalidOperationException($"{label} exceed the CMP06 one-byte header limit");
        }
    }

    private static Dictionary<string, int> ReconstructCanonicalCodes(IEnumerable<(int Symbol, int Length)> lengths)
    {
        var ordered = lengths
            .Where(pair => pair.Length > 0)
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .ToList();

        if (ordered.Count == 0)
        {
            return [];
        }

        if (ordered.Count == 1)
        {
            return new Dictionary<string, int> { ["0"] = ordered[0].Symbol };
        }

        var codes = new Dictionary<string, int>();
        var codeValue = 0;
        var previousLength = ordered[0].Length;
        foreach (var (symbol, length) in ordered)
        {
            if (length > previousLength)
            {
                codeValue <<= length - previousLength;
            }

            codes[Convert.ToString(codeValue, 2).PadLeft(length, '0')] = symbol;
            codeValue++;
            previousLength = length;
        }

        return codes;
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
