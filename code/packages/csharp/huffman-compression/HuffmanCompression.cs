using System.Buffers.Binary;
using System.Text;
using CodingAdventures.HuffmanTree;

namespace CodingAdventures.HuffmanCompression;

/// <summary>
/// CMP04 Huffman lossless compression over byte arrays.
/// </summary>
public static class HuffmanCompression
{
    // Symbols are single bytes, so a valid table has at most 256 entries.
    private const int MaxSymbols = 256;

    // Longest code length the decoder accepts; keeps every 64-bit shift safe.
    private const int MaxCodeLength = 56;

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

        // SECURITY: the header is attacker-controlled. Symbols are single bytes,
        // so a table can name at most 256 of them. Check before computing the
        // table size or sizing anything from it.
        if (symbolCount > MaxSymbols)
        {
            throw new InvalidOperationException($"symbol_count {symbolCount} exceeds {MaxSymbols}");
        }

        var tableLength = (int)symbolCount * 2;
        var bitStreamOffset = 8 + tableLength;
        if (bitStreamOffset > data.Length)
        {
            throw new InvalidOperationException("Compressed data ended before the code-length table completed");
        }

        var lengths = new List<(int Symbol, int Length)>((int)symbolCount);
        var seen = new bool[256];
        for (var index = 0; index < (int)symbolCount; index++)
        {
            var offset = 8 + (index * 2);
            var symbol = data[offset];
            var length = data[offset + 1];

            // A byte value can have only one code; a repeat makes the table ambiguous.
            if (seen[symbol])
            {
                throw new InvalidOperationException($"Symbol {symbol} appears twice in the code-length table");
            }

            seen[symbol] = true;
            lengths.Add((symbol, length));
        }

        var codeToSymbol = BuildCanonicalDecodeTable(lengths, out var maxLength);

        // SECURITY: every code is at least one bit, so N payload bytes hold at
        // most 8*N symbols. Reject a larger declared length BEFORE allocating
        // the output array (which would otherwise be up to 2 GiB).
        var totalBits = (long)(data.Length - bitStreamOffset) * 8;
        if (originalLength > totalBits)
        {
            throw new InvalidOperationException(
                $"Bit stream exhausted: declared length {originalLength} exceeds the {totalBits} bits available");
        }

        if (originalLength > int.MaxValue)
        {
            throw new InvalidOperationException("CMP04 header fields exceed supported .NET array sizes");
        }

        // Decode straight from the payload bytes. Bits are LSB-first within each
        // byte, so bit i is (data[offset + i/8] >> (i%8)) & 1 -- O(1), no
        // expanded bit string. The candidate code is built MSB-first as an
        // integer; canonical codes are prefix-free so the first (length, value)
        // hit is the symbol. Once the accumulated length reaches the longest
        // code without a match, no later bit can match (an incomplete table fed
        // unmatched bits), so we fail immediately instead of scanning on.
        var output = new byte[(int)originalLength];
        var decoded = 0;
        ulong accumulated = 0;
        var accumulatedLength = 0;

        for (long position = 0; decoded < output.Length; position++)
        {
            if (position >= totalBits)
            {
                throw new InvalidOperationException("Bit stream exhausted before decoding all symbols");
            }

            var bit = (data[bitStreamOffset + (int)(position >> 3)] >> (int)(position & 7)) & 1;
            accumulated = (accumulated << 1) | (uint)bit;
            accumulatedLength++;
            if (codeToSymbol.TryGetValue(((ulong)accumulatedLength << 56) | accumulated, out var symbol))
            {
                output[decoded] = symbol;
                decoded++;
                accumulated = 0;
                accumulatedLength = 0;
            }
            else if (accumulatedLength >= maxLength)
            {
                throw new InvalidOperationException("Bit pattern does not match any code");
            }
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

    // The canonical reconstruction rule (same as DEFLATE):
    //   code = 0
    //   for each (symbol, length) in sorted order:
    //     if length > previous: code <<= (length - previous)
    //     assign code to symbol; code++
    //
    // SECURITY: the table is hostile input, so we reject
    //   * lengths of 0 or above MaxCodeLength (a zero-length code matches
    //     nothing; a long one could overflow the 64-bit shift),
    //   * tables not sorted by length (the encoder always sorts), and
    //   * over-subscribed tables (Kraft sum > 1): before each assignment the
    //     running code must still fit in `length` bits (code >> length == 0).
    // C# masks a ulong shift count to 6 bits; the 1..56 bound keeps every shift
    // below 64 so the mask can never change a result.
    // Codes are keyed by (length << 56 | code) so no strings are built.
    private static Dictionary<ulong, byte> BuildCanonicalDecodeTable(
        IEnumerable<(int Symbol, int Length)> lengths,
        out int maxLength)
    {
        var table = new Dictionary<ulong, byte>();
        var ordered = lengths.ToList();
        ulong codeValue = 0;
        var previousLength = ordered.Count == 0 ? 0 : ordered[0].Length;
        maxLength = 0;

        foreach (var (symbol, length) in ordered)
        {
            if (length == 0 || length > MaxCodeLength)
            {
                throw new InvalidOperationException($"Code lengths must be positive and at most {MaxCodeLength}");
            }

            if (length < previousLength)
            {
                throw new InvalidOperationException("Code-length table is not sorted by length");
            }

            codeValue <<= length - previousLength;
            if ((codeValue >> length) != 0)
            {
                throw new InvalidOperationException("Code-length table over-subscribes the code space");
            }

            table[((ulong)length << 56) | codeValue] = (byte)symbol;
            codeValue++;
            previousLength = length;
            maxLength = length;
        }

        return table;
    }
}
