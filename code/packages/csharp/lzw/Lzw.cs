using System.Buffers.Binary;

namespace CodingAdventures.Lzw;

public sealed class BitWriter
{
    private ulong _buffer;
    private int _bitCount;
    private readonly List<byte> _bytes = [];

    public void Write(int code, int codeSize)
    {
        if (codeSize <= 0 || codeSize > Lzw.MAX_CODE_SIZE)
        {
            throw new ArgumentOutOfRangeException(nameof(codeSize), "codeSize must be between 1 and MAX_CODE_SIZE");
        }

        _buffer |= (ulong)(uint)code << _bitCount;
        _bitCount += codeSize;

        while (_bitCount >= 8)
        {
            _bytes.Add((byte)(_buffer & 0xFF));
            _buffer >>= 8;
            _bitCount -= 8;
        }
    }

    public void Flush()
    {
        if (_bitCount > 0)
        {
            _bytes.Add((byte)(_buffer & 0xFF));
            _buffer = 0;
            _bitCount = 0;
        }
    }

    public byte[] ToArray() => [.. _bytes];
}

public sealed class BitReader(byte[] data)
{
    private readonly byte[] _data = data ?? throw new ArgumentNullException(nameof(data));
    private int _position;
    private ulong _buffer;
    private int _bitCount;

    public int Read(int codeSize)
    {
        if (codeSize <= 0 || codeSize > Lzw.MAX_CODE_SIZE)
        {
            throw new ArgumentOutOfRangeException(nameof(codeSize), "codeSize must be between 1 and MAX_CODE_SIZE");
        }

        while (_bitCount < codeSize)
        {
            if (_position >= _data.Length)
            {
                throw new InvalidOperationException("unexpected end of bit stream");
            }

            _buffer |= (ulong)_data[_position] << _bitCount;
            _position++;
            _bitCount += 8;
        }

        var mask = (1UL << codeSize) - 1UL;
        var code = (int)(_buffer & mask);
        _buffer >>= codeSize;
        _bitCount -= codeSize;
        return code;
    }

    public bool Exhausted() => _position >= _data.Length && _bitCount == 0;
}

public static class Lzw
{
    public const int CLEAR_CODE = 256;
    public const int STOP_CODE = 257;
    public const int INITIAL_NEXT_CODE = 258;
    public const int INITIAL_CODE_SIZE = 9;
    public const int MAX_CODE_SIZE = 16;

    private const int MaxEntries = 1 << MAX_CODE_SIZE;

    public static (List<int> Codes, int OriginalLength) EncodeCodes(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);

        var dictionary = CreateEncoderDictionary();
        var codes = new List<int> { CLEAR_CODE };
        var nextCode = INITIAL_NEXT_CODE;
        var current = string.Empty;

        foreach (var value in data)
        {
            var candidate = current + (char)value;
            if (dictionary.ContainsKey(candidate))
            {
                current = candidate;
                continue;
            }

            codes.Add(dictionary[current]);

            if (nextCode < MaxEntries)
            {
                dictionary[candidate] = nextCode;
                nextCode++;
            }
            else
            {
                codes.Add(CLEAR_CODE);
                dictionary = CreateEncoderDictionary();
                nextCode = INITIAL_NEXT_CODE;
            }

            current = SingleByteString(value);
        }

        if (current.Length > 0)
        {
            codes.Add(dictionary[current]);
        }

        codes.Add(STOP_CODE);
        return (codes, data.Length);
    }

    public static byte[] DecodeCodes(IEnumerable<int> codes, int originalLength = -1)
    {
        ArgumentNullException.ThrowIfNull(codes);

        var dictionary = CreateDecoderDictionary();
        var output = new List<byte>();
        var nextCode = INITIAL_NEXT_CODE;
        int? previousCode = null;

        foreach (var code in codes)
        {
            if (code == CLEAR_CODE)
            {
                ResetDecoderDictionary(dictionary);
                nextCode = INITIAL_NEXT_CODE;
                previousCode = null;
                continue;
            }

            if (code == STOP_CODE)
            {
                break;
            }

            byte[] entry;
            if (code >= 0 && code < dictionary.Count)
            {
                entry = dictionary[code];
            }
            else if (code == nextCode && previousCode.HasValue)
            {
                var previous = dictionary[previousCode.Value];
                entry = AppendByte(previous, previous[0]);
            }
            else
            {
                throw new InvalidOperationException("invalid LZW code");
            }

            output.AddRange(entry);

            if (previousCode.HasValue && nextCode < MaxEntries)
            {
                var previous = dictionary[previousCode.Value];
                dictionary.Add(AppendByte(previous, entry[0]));
                nextCode++;
            }

            previousCode = code;
        }

        return originalLength >= 0 ? [.. output.Take(originalLength)] : [.. output];
    }

    public static byte[] PackCodes(IReadOnlyList<int> codes, int originalLength)
    {
        ArgumentNullException.ThrowIfNull(codes);
        if (originalLength < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(originalLength), "originalLength must be non-negative");
        }

        var writer = new BitWriter();
        var nextCode = INITIAL_NEXT_CODE;
        var codeSize = INITIAL_CODE_SIZE;

        foreach (var code in codes)
        {
            if (code < 0 || code >= MaxEntries)
            {
                throw new InvalidOperationException("Code does not fit in the CMP03 code space");
            }

            writer.Write(code, codeSize);

            if (code == CLEAR_CODE)
            {
                nextCode = INITIAL_NEXT_CODE;
                codeSize = INITIAL_CODE_SIZE;
            }
            else if (code != STOP_CODE && nextCode < MaxEntries)
            {
                nextCode++;
                if (nextCode > (1 << codeSize) && codeSize < MAX_CODE_SIZE)
                {
                    codeSize++;
                }
            }
        }

        writer.Flush();
        var payload = writer.ToArray();
        var output = new byte[4 + payload.Length];
        BinaryPrimitives.WriteUInt32BigEndian(output.AsSpan(0, 4), (uint)originalLength);
        payload.CopyTo(output, 4);
        return output;
    }

    public static (List<int> Codes, int OriginalLength) UnpackCodes(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 4)
        {
            return ([], 0);
        }

        var originalLength = (int)BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(0, 4));
        var reader = new BitReader(data[4..]);
        var codes = new List<int>();
        var nextCode = INITIAL_NEXT_CODE;
        var codeSize = INITIAL_CODE_SIZE;

        while (true)
        {
            int code;
            try
            {
                code = reader.Read(codeSize);
            }
            catch (InvalidOperationException)
            {
                break;
            }

            codes.Add(code);

            if (code == STOP_CODE)
            {
                break;
            }

            if (code == CLEAR_CODE)
            {
                nextCode = INITIAL_NEXT_CODE;
                codeSize = INITIAL_CODE_SIZE;
            }
            else if (nextCode < MaxEntries)
            {
                nextCode++;
                if (nextCode > (1 << codeSize) && codeSize < MAX_CODE_SIZE)
                {
                    codeSize++;
                }
            }
        }

        return (codes, originalLength);
    }

    public static byte[] Compress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var (codes, originalLength) = EncodeCodes(data);
        return PackCodes(codes, originalLength);
    }

    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var (codes, originalLength) = UnpackCodes(data);
        return DecodeCodes(codes, originalLength);
    }

    private static Dictionary<string, int> CreateEncoderDictionary()
    {
        var dictionary = new Dictionary<string, int>(512);
        for (var value = 0; value < 256; value++)
        {
            dictionary[SingleByteString((byte)value)] = value;
        }

        return dictionary;
    }

    private static List<byte[]> CreateDecoderDictionary()
    {
        var dictionary = new List<byte[]>(258);
        ResetDecoderDictionary(dictionary);
        return dictionary;
    }

    private static void ResetDecoderDictionary(List<byte[]> dictionary)
    {
        dictionary.Clear();
        for (var value = 0; value < 256; value++)
        {
            dictionary.Add([(byte)value]);
        }

        dictionary.Add([]);
        dictionary.Add([]);
    }

    private static string SingleByteString(byte value) => new((char)value, 1);

    private static byte[] AppendByte(byte[] prefix, byte value)
    {
        var output = new byte[prefix.Length + 1];
        prefix.CopyTo(output, 0);
        output[^1] = value;
        return output;
    }
}
