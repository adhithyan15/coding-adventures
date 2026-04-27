using System.Buffers.Binary;

namespace CodingAdventures.Lz77;

public readonly record struct Lz77Token(int Offset, int Length, byte NextChar);

public static class Lz77
{
    public static Lz77Token Token(int offset, int length, byte nextChar) => new(offset, length, nextChar);

    public static List<Lz77Token> Encode(byte[] data, int windowSize = 4096, int maxMatch = 255, int minMatch = 3)
    {
        ArgumentNullException.ThrowIfNull(data);
        ValidateParameters(windowSize, maxMatch, minMatch);

        var tokens = new List<Lz77Token>();
        var cursor = 0;

        while (cursor < data.Length)
        {
            if (cursor == data.Length - 1)
            {
                tokens.Add(new Lz77Token(0, 0, data[cursor]));
                cursor++;
                continue;
            }

            var (offset, length) = FindLongestMatch(data, cursor, windowSize, maxMatch);
            if (length >= minMatch)
            {
                tokens.Add(new Lz77Token(offset, length, data[cursor + length]));
                cursor += length + 1;
            }
            else
            {
                tokens.Add(new Lz77Token(0, 0, data[cursor]));
                cursor++;
            }
        }

        return tokens;
    }

    public static byte[] Decode(IEnumerable<Lz77Token> tokens, byte[]? initialBuffer = null)
    {
        ArgumentNullException.ThrowIfNull(tokens);

        var output = new List<byte>(initialBuffer ?? []);
        foreach (var token in tokens)
        {
            if (token.Length > 0)
            {
                if (token.Offset <= 0)
                {
                    throw new InvalidOperationException("Backreference offsets must be positive");
                }

                var start = output.Count - token.Offset;
                if (start < 0)
                {
                    throw new InvalidOperationException("Backreference offset extends before the output buffer");
                }

                for (var i = 0; i < token.Length; i++)
                {
                    output.Add(output[start + i]);
                }
            }

            output.Add(token.NextChar);
        }

        return [.. output];
    }

    public static byte[] SerialiseTokens(IReadOnlyList<Lz77Token> tokens)
    {
        ArgumentNullException.ThrowIfNull(tokens);

        var output = new byte[4 + (tokens.Count * 4)];
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)tokens.Count);
        for (var i = 0; i < tokens.Count; i++)
        {
            var token = tokens[i];
            if (token.Offset < 0 || token.Offset > ushort.MaxValue)
            {
                throw new InvalidOperationException("Offset does not fit in the teaching uint16 token format");
            }

            if (token.Length < 0 || token.Length > byte.MaxValue)
            {
                throw new InvalidOperationException("Length does not fit in the teaching uint8 token format");
            }

            var index = 4 + (i * 4);
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(index, 2), (ushort)token.Offset);
            output[index + 2] = (byte)token.Length;
            output[index + 3] = token.NextChar;
        }

        return output;
    }

    public static List<Lz77Token> DeserialiseTokens(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 4)
        {
            return [];
        }

        var count = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        var tokens = new List<Lz77Token>();
        for (var i = 0; i < count; i++)
        {
            var index = 4 + ((int)i * 4);
            if (index + 4 > data.Length)
            {
                break;
            }

            tokens.Add(new Lz77Token(
                BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(index, 2)),
                data[index + 2],
                data[index + 3]));
        }

        return tokens;
    }

    public static byte[] Compress(byte[] data, int windowSize = 4096, int maxMatch = 255, int minMatch = 3) =>
        SerialiseTokens(Encode(data, windowSize, maxMatch, minMatch));

    public static byte[] Decompress(byte[] data) => Decode(DeserialiseTokens(data));

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

    private static (int Offset, int Length) FindLongestMatch(byte[] data, int cursor, int windowSize, int maxMatch)
    {
        var bestOffset = 0;
        var bestLength = 0;
        var searchStart = Math.Max(0, cursor - windowSize);
        var lookaheadEnd = Math.Min(cursor + maxMatch, data.Length - 1);

        for (var position = searchStart; position < cursor; position++)
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
