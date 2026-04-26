using System.Buffers.Binary;
using System.Security.Cryptography;

namespace CodingAdventures.Csprng;

/// <summary>
/// Cryptographically secure random byte and integer helpers.
/// </summary>
public static class Csprng
{
    /// <summary>Fill an existing buffer with cryptographically secure random bytes.</summary>
    public static void FillRandom(byte[] buffer)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        if (buffer.Length == 0)
        {
            throw new ArgumentException("Random byte request must not be empty.", nameof(buffer));
        }

        RandomNumberGenerator.Fill(buffer);
    }

    /// <summary>Return a new buffer of cryptographically secure random bytes.</summary>
    public static byte[] RandomBytes(int length)
    {
        if (length <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(length), "Random byte request length must be positive.");
        }

        var buffer = new byte[length];
        RandomNumberGenerator.Fill(buffer);
        return buffer;
    }

    /// <summary>Return a random unsigned 32-bit integer.</summary>
    public static uint RandomUInt32()
    {
        Span<byte> bytes = stackalloc byte[4];
        RandomNumberGenerator.Fill(bytes);
        return BinaryPrimitives.ReadUInt32LittleEndian(bytes);
    }

    /// <summary>Return a random unsigned 64-bit integer.</summary>
    public static ulong RandomUInt64()
    {
        Span<byte> bytes = stackalloc byte[8];
        RandomNumberGenerator.Fill(bytes);
        return BinaryPrimitives.ReadUInt64LittleEndian(bytes);
    }
}
