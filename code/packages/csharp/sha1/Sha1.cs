using System.Buffers.Binary;
using System.Numerics;

namespace CodingAdventures.Sha1;

/// <summary>
/// SHA-1 one-shot and streaming helpers implemented from FIPS 180-4.
/// </summary>
public static class Sha1
{
    /// <summary>SHA-1 digest length in bytes.</summary>
    public const int DigestLength = 20;

    private static readonly uint[] InitialState =
    [
        0x67452301u,
        0xefcdab89u,
        0x98badcfeu,
        0x10325476u,
        0xc3d2e1f0u,
    ];

    private static readonly uint[] RoundConstants =
    [
        0x5a827999u,
        0x6ed9eba1u,
        0x8f1bbcdcu,
        0xca62c1d6u,
    ];

    /// <summary>Compute the SHA-1 digest for a complete byte array.</summary>
    public static byte[] Hash(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var padded = Pad(data);
        var state = InitialState.ToArray();

        for (var offset = 0; offset < padded.Length; offset += 64)
        {
            Compress(state, padded.AsSpan(offset, 64));
        }

        return StateToBytes(state);
    }

    /// <summary>Compute SHA-1 and return a lowercase hexadecimal digest.</summary>
    public static string HashHex(byte[] data) =>
        Convert.ToHexString(Hash(data)).ToLowerInvariant();

    private static byte[] Pad(ReadOnlySpan<byte> data)
    {
        var bitLength = (ulong)data.Length * 8UL;
        var afterBit = (data.Length + 1) % 64;
        var zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit;
        var result = new byte[data.Length + 1 + zeroCount + 8];

        data.CopyTo(result);
        result[data.Length] = 0x80;
        BinaryPrimitives.WriteUInt64BigEndian(result.AsSpan(result.Length - 8), bitLength);
        return result;
    }

    private static void Compress(uint[] state, ReadOnlySpan<byte> block)
    {
        Span<uint> w = stackalloc uint[80];
        for (var index = 0; index < 16; index++)
        {
            w[index] = BinaryPrimitives.ReadUInt32BigEndian(block.Slice(index * 4, 4));
        }

        for (var index = 16; index < 80; index++)
        {
            w[index] = BitOperations.RotateLeft(w[index - 3] ^ w[index - 8] ^ w[index - 14] ^ w[index - 16], 1);
        }

        var a = state[0];
        var b = state[1];
        var c = state[2];
        var d = state[3];
        var e = state[4];

        for (var index = 0; index < 80; index++)
        {
            uint f;
            uint k;
            if (index < 20)
            {
                f = (b & c) | (~b & d);
                k = RoundConstants[0];
            }
            else if (index < 40)
            {
                f = b ^ c ^ d;
                k = RoundConstants[1];
            }
            else if (index < 60)
            {
                f = (b & c) | (b & d) | (c & d);
                k = RoundConstants[2];
            }
            else
            {
                f = b ^ c ^ d;
                k = RoundConstants[3];
            }

            var temp = unchecked(BitOperations.RotateLeft(a, 5) + f + e + k + w[index]);
            e = d;
            d = c;
            c = BitOperations.RotateLeft(b, 30);
            b = a;
            a = temp;
        }

        state[0] = unchecked(state[0] + a);
        state[1] = unchecked(state[1] + b);
        state[2] = unchecked(state[2] + c);
        state[3] = unchecked(state[3] + d);
        state[4] = unchecked(state[4] + e);
    }

    private static byte[] StateToBytes(uint[] state)
    {
        var digest = new byte[DigestLength];
        for (var index = 0; index < state.Length; index++)
        {
            BinaryPrimitives.WriteUInt32BigEndian(digest.AsSpan(index * 4, 4), state[index]);
        }

        return digest;
    }
}

/// <summary>
/// Streaming SHA-1 hasher with non-destructive digest snapshots.
/// </summary>
public sealed class Sha1Hasher
{
    private readonly List<byte> _data = [];

    /// <summary>Append bytes and return this hasher for chaining.</summary>
    public Sha1Hasher Update(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        _data.AddRange(data);
        return this;
    }

    /// <summary>Return the current 20-byte digest without modifying this hasher.</summary>
    public byte[] Digest() => Sha1.Hash(_data.ToArray());

    /// <summary>Return the current digest as a lowercase hexadecimal string.</summary>
    public string HexDigest() => Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Sha1Hasher Copy()
    {
        var copy = new Sha1Hasher();
        copy._data.AddRange(_data);
        return copy;
    }
}
