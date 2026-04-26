using System.Buffers.Binary;
using System.Text;
using CodingAdventures.HuffmanTree;

namespace CodingAdventures.HuffmanCompression;

/// <summary>
/// CMP04 Huffman lossless compression over byte arrays.
/// </summary>
public static class HuffmanCompression
{
    /// <summary>
    /// Compresses bytes using canonical Huffman codes and the CMP04 wire format.
    /// </summary>
    public static byte[] Compress(byte[]? data)
    {
        if (data is null || data.Length == 0)
        {
            return new byte[8];
        }

        var frequencies = new int[256];
        foreach (var value in data)
        {
            frequencies[value]++;
        }

        var weights = new List<(int Symbol, int Frequency)>();
        for (var symbol = 0; symbol < frequencies.Length; symbol++)
        {
            if (frequencies[symbol] > 0)
            {
                weights.Add((symbol, frequencies[symbol]));
            }
        }

        var table = HuffmanTree.HuffmanTree.Build(weights).CanonicalCodeTable();
        var lengths = table
            .Select(pair => (Symbol: pair.Key, Length: pair.Value.Length))
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .ToList();

        var bitBytes = PackBits(data, table);
        var tableLength = lengths.Count * 2;
        var output = new byte[8 + tableLength + bitBytes.Length];

        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)data.Length);
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(4, 4), (uint)lengths.Count);

        for (var index = 0; index < lengths.Count; index++)
        {
            var (symbol, length) = lengths[index];
            var offset = 8 + (index * 2);
            output[offset] = (byte)symbol;
            output[offset + 1] = (byte)length;
        }

        bitBytes.CopyTo(output.AsSpan(8 + tableLength));
        return output;
    }

    /// <summary>
    /// Decompresses bytes written in the CMP04 wire format.
    /// </summary>
    public static byte[] Decompress(byte[]? data)
    {
        if (data is null || data.Length < 8)
        {
            return [];
        }

        var originalLength = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        var symbolCount = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(4, 4));

        if (originalLength == 0)
        {
            return [];
        }

        if (originalLength > int.MaxValue || symbolCount > int.MaxValue)
        {
            throw new InvalidOperationException("CMP04 header fields exceed supported .NET array sizes");
        }

        var tableLength = checked((int)symbolCount * 2);
        var bitStreamOffset = 8 + tableLength;
        if (bitStreamOffset > data.Length)
        {
            throw new InvalidOperationException("Compressed data ended before the code-length table completed");
        }

        var lengths = new List<(int Symbol, int Length)>((int)symbolCount);
        for (var index = 0; index < symbolCount; index++)
        {
            var offset = 8 + ((int)index * 2);
            var symbol = data[offset];
            var length = data[offset + 1];
            if (length == 0)
            {
                throw new InvalidOperationException("Code lengths must be positive");
            }

            lengths.Add((symbol, length));
        }

        var codeToSymbol = BuildCanonicalDecodeTable(lengths);
        var output = new byte[originalLength];
        var accumulated = new StringBuilder();
        var decoded = 0;

        for (var offset = bitStreamOffset; offset < data.Length && decoded < output.Length; offset++)
        {
            var value = data[offset];
            for (var bit = 0; bit < 8 && decoded < output.Length; bit++)
            {
                accumulated.Append(((value >> bit) & 1) == 1 ? '1' : '0');
                if (codeToSymbol.TryGetValue(accumulated.ToString(), out var symbol))
                {
                    output[decoded] = symbol;
                    decoded++;
                    accumulated.Clear();
                }
            }
        }

        if (decoded < output.Length)
        {
            throw new InvalidOperationException("Bit stream exhausted before decoding all symbols");
        }

        return output;
    }

    private static byte[] PackBits(IReadOnlyList<byte> data, IReadOnlyDictionary<int, string> table)
    {
        var bitCount = 0;
        foreach (var value in data)
        {
            bitCount += table[value].Length;
        }

        var output = new byte[(bitCount + 7) / 8];
        var bitIndex = 0;
        foreach (var value in data)
        {
            foreach (var bit in table[value])
            {
                if (bit == '1')
                {
                    output[bitIndex / 8] |= (byte)(1 << (bitIndex % 8));
                }

                bitIndex++;
            }
        }

        return output;
    }

    private static Dictionary<string, byte> BuildCanonicalDecodeTable(IEnumerable<(int Symbol, int Length)> lengths)
    {
        var table = new Dictionary<string, byte>();
        var ordered = lengths.ToList();
        var codeValue = 0;
        var previousLength = ordered.Count == 0 ? 0 : ordered[0].Length;

        foreach (var (symbol, length) in ordered)
        {
            if (length > previousLength)
            {
                codeValue <<= length - previousLength;
            }

            table[Convert.ToString(codeValue, 2).PadLeft(length, '0')] = (byte)symbol;
            codeValue++;
            previousLength = length;
        }

        return table;
    }
}
