using System.Buffers.Binary;

namespace CodingAdventures.Lz78;

public readonly record struct Lz78Token(int DictIndex, byte NextChar);

public static class Lz78
{
    public static Lz78Token Token(int dictIndex, byte nextChar) => new(dictIndex, nextChar);

    public static List<Lz78Token> Encode(byte[] data, int maxDictSize = 65_536)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (maxDictSize <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(maxDictSize), "maxDictSize must be positive");
        }

        var dictionary = new Dictionary<(int ParentId, byte Byte), int>();
        var nextId = 1;
        var currentId = 0;
        var tokens = new List<Lz78Token>();

        foreach (var value in data)
        {
            if (dictionary.TryGetValue((currentId, value), out var childId))
            {
                currentId = childId;
            }
            else
            {
                tokens.Add(new Lz78Token(currentId, value));
                if (nextId < maxDictSize)
                {
                    dictionary[(currentId, value)] = nextId;
                    nextId++;
                }

                currentId = 0;
            }
        }

        if (currentId != 0)
        {
            tokens.Add(new Lz78Token(currentId, 0));
        }

        return tokens;
    }

    public static byte[] Decode(IEnumerable<Lz78Token> tokens, int originalLength = -1)
    {
        ArgumentNullException.ThrowIfNull(tokens);

        var table = new List<(int ParentId, byte Byte)> { (0, 0) };
        var output = new List<byte>();

        foreach (var token in tokens)
        {
            if (token.DictIndex < 0 || token.DictIndex >= table.Count)
            {
                throw new InvalidOperationException("Token references a dictionary entry that does not exist");
            }

            output.AddRange(Reconstruct(table, token.DictIndex));
            if (originalLength < 0 || output.Count < originalLength)
            {
                output.Add(token.NextChar);
            }

            table.Add((token.DictIndex, token.NextChar));
        }

        return originalLength >= 0 ? [.. output.Take(originalLength)] : [.. output];
    }

    public static byte[] SerialiseTokens(IReadOnlyList<Lz78Token> tokens, int originalLength)
    {
        ArgumentNullException.ThrowIfNull(tokens);
        if (originalLength < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(originalLength), "originalLength must be non-negative");
        }

        var output = new byte[8 + (tokens.Count * 4)];
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)originalLength);
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(4, 4), (uint)tokens.Count);

        for (var i = 0; i < tokens.Count; i++)
        {
            var token = tokens[i];
            if (token.DictIndex < 0 || token.DictIndex > ushort.MaxValue)
            {
                throw new InvalidOperationException("Dictionary index does not fit in the teaching uint16 token format");
            }

            var index = 8 + (i * 4);
            BinaryPrimitives.WriteUInt16BigEndian(output.AsSpan(index, 2), (ushort)token.DictIndex);
            output[index + 2] = token.NextChar;
            output[index + 3] = 0;
        }

        return output;
    }

    public static (List<Lz78Token> Tokens, int OriginalLength) DeserialiseTokens(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 8)
        {
            return ([], 0);
        }

        var originalLength = (int)BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        var tokenCount = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(4, 4));
        var tokens = new List<Lz78Token>();

        for (var i = 0u; i < tokenCount; i++)
        {
            var index = 8 + ((int)i * 4);
            if (index + 4 > data.Length)
            {
                break;
            }

            tokens.Add(new Lz78Token(
                BinaryPrimitives.ReadUInt16BigEndian(data.AsSpan(index, 2)),
                data[index + 2]));
        }

        return (tokens, originalLength);
    }

    public static byte[] Compress(byte[] data, int maxDictSize = 65_536)
    {
        ArgumentNullException.ThrowIfNull(data);
        var tokens = Encode(data, maxDictSize);
        return SerialiseTokens(tokens, data.Length);
    }

    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var (tokens, originalLength) = DeserialiseTokens(data);
        return Decode(tokens, originalLength);
    }

    private static IEnumerable<byte> Reconstruct(IReadOnlyList<(int ParentId, byte Byte)> table, int index)
    {
        if (index == 0)
        {
            return [];
        }

        var reversed = new List<byte>();
        var current = index;
        while (current != 0)
        {
            var entry = table[current];
            reversed.Add(entry.Byte);
            current = entry.ParentId;
        }

        reversed.Reverse();
        return reversed;
    }
}
