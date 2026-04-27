namespace CodingAdventures.WasmLeb128;

public static class WasmLeb128Version
{
    public const string VERSION = "0.1.0";
}

public sealed class LEB128Error : Exception
{
    public LEB128Error(string message) : base(message)
    {
    }
}

public static class WasmLeb128
{
    private const int ContinuationBit = 0x80;
    private const int PayloadMask = 0x7F;
    private const int MaxLeb128Bytes32 = 5;

    public static (uint Value, int BytesConsumed) DecodeUnsigned(ReadOnlySpan<byte> data, int offset = 0)
    {
        var result = 0u;
        var shift = 0;
        var bytesConsumed = 0;

        for (var i = offset; i < data.Length; i++)
        {
            if (bytesConsumed >= MaxLeb128Bytes32)
            {
                throw new LEB128Error($"LEB128 sequence exceeds maximum {MaxLeb128Bytes32} bytes for a 32-bit value");
            }

            var currentByte = data[i];
            var payload = (uint)(currentByte & PayloadMask);
            result |= payload << shift;
            shift += 7;
            bytesConsumed++;

            if ((currentByte & ContinuationBit) == 0)
            {
                return (result, bytesConsumed);
            }
        }

        throw new LEB128Error($"LEB128 sequence is unterminated: reached end of data at offset {offset + bytesConsumed} without finding a byte with continuation bit = 0");
    }

    public static (int Value, int BytesConsumed) DecodeSigned(ReadOnlySpan<byte> data, int offset = 0)
    {
        var result = 0;
        var shift = 0;
        var bytesConsumed = 0;
        byte lastByte = 0;

        for (var i = offset; i < data.Length; i++)
        {
            if (bytesConsumed >= MaxLeb128Bytes32)
            {
                throw new LEB128Error($"LEB128 sequence exceeds maximum {MaxLeb128Bytes32} bytes for a 32-bit value");
            }

            lastByte = data[i];
            var payload = lastByte & PayloadMask;
            result |= payload << shift;
            shift += 7;
            bytesConsumed++;

            if ((lastByte & ContinuationBit) == 0)
            {
                if (shift < 32 && (lastByte & 0x40) != 0)
                {
                    result |= -(1 << shift);
                }

                return (result, bytesConsumed);
            }
        }

        throw new LEB128Error($"LEB128 sequence is unterminated: reached end of data at offset {offset + bytesConsumed} without finding a byte with continuation bit = 0");
    }

    public static byte[] EncodeUnsigned(uint value)
    {
        var bytes = new List<byte>();
        var remaining = value;

        do
        {
            var currentByte = (byte)(remaining & PayloadMask);
            remaining >>= 7;
            if (remaining != 0)
            {
                currentByte |= ContinuationBit;
            }

            bytes.Add(currentByte);
        } while (remaining != 0);

        return bytes.ToArray();
    }

    public static byte[] EncodeUnsigned(int value)
    {
        return EncodeUnsigned(unchecked((uint)value));
    }

    public static byte[] EncodeSigned(int value)
    {
        var bytes = new List<byte>();
        var remaining = value;
        var done = false;

        do
        {
            var currentByte = (byte)(remaining & PayloadMask);
            remaining >>= 7;

            done = (remaining == 0 && (currentByte & 0x40) == 0) || (remaining == -1 && (currentByte & 0x40) != 0);
            if (!done)
            {
                currentByte |= ContinuationBit;
            }

            bytes.Add(currentByte);
        } while (!done);

        return bytes.ToArray();
    }
}
