using System.Buffers.Binary;

namespace CodingAdventures.Lzss;

public abstract record LzssToken;

public sealed record LzssLiteral(byte Byte) : LzssToken;

public sealed record LzssMatch(int Offset, int Length) : LzssToken;

public static class Lzss
{
    public const int DefaultWindowSize = 4096;
    public const int DefaultMaxMatch = 255;
    public const int DefaultMinMatch = 3;

    public static LzssLiteral Literal(byte value) => new(value);

    public static LzssMatch Match(int offset, int length) => new(offset, length);

    public static List<LzssToken> Encode(
        byte[] data,
        int windowSize = DefaultWindowSize,
        int maxMatch = DefaultMaxMatch,
        int minMatch = DefaultMinMatch)
    {
        ArgumentNullException.ThrowIfNull(data);
        ValidateParameters(windowSize, maxMatch, minMatch);

        var tokens = new List<LzssToken>();
        var cursor = 0;

        while (cursor < data.Length)
        {
            var windowStart = Math.Max(0, cursor - windowSize);
            var (offset, length) = FindLongestMatch(data, cursor, windowStart, maxMatch);
            if (length >= minMatch)
            {
                tokens.Add(new LzssMatch(offset, length));
                cursor += length;
            }
            else
            {
                tokens.Add(new LzssLiteral(data[cursor]));
                cursor++;
            }
        }

        return tokens;
    }

    public static byte[] Decode(IEnumerable<LzssToken> tokens, int originalLength = -1)
    {
        ArgumentNullException.ThrowIfNull(tokens);

        var output = new List<byte>();
        foreach (var token in tokens)
        {
            switch (token)
            {
                case LzssLiteral literal:
                    output.Add(literal.Byte);
                    break;

                case LzssMatch match:
                {
                    if (match.Offset <= 0)
                    {
                        throw new InvalidOperationException("Match offsets must be positive");
                    }

                    var start = output.Count - match.Offset;
                    if (start < 0)
                    {
                        throw new InvalidOperationException("Match offset extends before the output buffer");
                    }

                    for (var index = 0; index < match.Length; index++)
                    {
                        output.Add(output[start + index]);
                    }

                    break;
                }

                default:
                    throw new InvalidOperationException("Unknown LZSS token type");
            }
        }

        return originalLength >= 0 ? [.. output.Take(originalLength)] : [.. output];
    }

    public static byte[] SerialiseTokens(IReadOnlyList<LzssToken> tokens, int originalLength)
    {
        ArgumentNullException.ThrowIfNull(tokens);
        if (originalLength < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(originalLength), "originalLength must be non-negative");
        }

        var blocks = new List<byte[]>();
        for (var tokenIndex = 0; tokenIndex < tokens.Count; tokenIndex += 8)
        {
            var chunk = tokens.Skip(tokenIndex).Take(8).ToList();
            var flag = (byte)0;
            var bytes = new List<byte>();
            for (var bit = 0; bit < chunk.Count; bit++)
            {
                switch (chunk[bit])
                {
                    case LzssMatch match:
                        flag |= (byte)(1 << bit);
                        bytes.Add((byte)((match.Offset >> 8) & 0xff));
                        bytes.Add((byte)(match.Offset & 0xff));
                        bytes.Add((byte)(match.Length & 0xff));
                        break;

                    case LzssLiteral literal:
                        bytes.Add(literal.Byte);
                        break;

                    default:
                        throw new InvalidOperationException("Unknown LZSS token type");
                }
            }

            blocks.Add([flag, .. bytes]);
        }

        var totalSize = 8 + blocks.Sum(block => block.Length);
        var output = new byte[totalSize];
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)originalLength);
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(4, 4), (uint)blocks.Count);

        var position = 8;
        foreach (var block in blocks)
        {
            block.CopyTo(output, position);
            position += block.Length;
        }

        return output;
    }

    public static (List<LzssToken> Tokens, int OriginalLength) DeserialiseTokens(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 8)
        {
            return ([], 0);
        }

        var originalLength = (int)BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        var blockCount = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(4, 4));
        var maxPossibleBlocks = (uint)(data.Length - 8);
        if (blockCount > maxPossibleBlocks)
        {
            blockCount = maxPossibleBlocks;
        }

        var tokens = new List<LzssToken>();
        var position = 8;

        for (var block = 0u; block < blockCount; block++)
        {
            if (position >= data.Length)
            {
                break;
            }

            var flag = data[position];
            position++;

            for (var bit = 0; bit < 8 && position < data.Length; bit++)
            {
                if ((flag & (1 << bit)) != 0)
                {
                    if (position + 3 > data.Length)
                    {
                        break;
                    }

                    tokens.Add(new LzssMatch(
                        (data[position] << 8) | data[position + 1],
                        data[position + 2]));
                    position += 3;
                }
                else
                {
                    tokens.Add(new LzssLiteral(data[position]));
                    position++;
                }
            }
        }

        return (tokens, originalLength);
    }

    public static byte[] Compress(
        byte[] data,
        int windowSize = DefaultWindowSize,
        int maxMatch = DefaultMaxMatch,
        int minMatch = DefaultMinMatch)
    {
        ArgumentNullException.ThrowIfNull(data);
        var tokens = Encode(data, windowSize, maxMatch, minMatch);
        return SerialiseTokens(tokens, data.Length);
    }

    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var (tokens, originalLength) = DeserialiseTokens(data);
        return Decode(tokens, originalLength);
    }

    private static void ValidateParameters(int windowSize, int maxMatch, int minMatch)
    {
        if (windowSize <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(windowSize), "windowSize must be positive");
        }

        if (maxMatch <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(maxMatch), "maxMatch must be positive");
        }

        if (minMatch <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(minMatch), "minMatch must be positive");
        }
    }

    private static (int Offset, int Length) FindLongestMatch(byte[] data, int cursor, int windowStart, int maxMatch)
    {
        var bestLength = 0;
        var bestOffset = 0;
        var lookaheadEnd = Math.Min(cursor + maxMatch, data.Length);

        for (var position = windowStart; position < cursor; position++)
        {
            var length = 0;
            while (cursor + length < lookaheadEnd && data[position + length] == data[cursor + length])
            {
                length++;
            }

            if (length > bestLength)
            {
                bestLength = length;
                bestOffset = cursor - position;
            }
        }

        return (bestOffset, bestLength);
    }
}
