using System.Buffers.Binary;
using System.Numerics;

namespace CodingAdventures.Sha256;

/// <summary>
/// SHA-256 one-shot and streaming helpers implemented from FIPS 180-4.
/// </summary>
public static class Sha256
{
    /// <summary>SHA-256 digest length in bytes.</summary>
    public const int DigestLength = 32;

    private static readonly uint[] InitialState =
    [
        0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
        0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u,
    ];

    private static readonly uint[] K =
    [
        0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
        0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u, 0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
        0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
        0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
        0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u, 0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
        0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
        0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
        0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u, 0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u,
    ];

    /// <summary>Compute the SHA-256 digest for a complete byte array.</summary>
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

    /// <summary>Compute SHA-256 and return a lowercase hexadecimal digest.</summary>
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
        Span<uint> w = stackalloc uint[64];
        for (var index = 0; index < 16; index++)
        {
            w[index] = BinaryPrimitives.ReadUInt32BigEndian(block.Slice(index * 4, 4));
        }

        for (var index = 16; index < 64; index++)
        {
            w[index] = unchecked(SmallSigma1(w[index - 2]) + w[index - 7] + SmallSigma0(w[index - 15]) + w[index - 16]);
        }

        var a = state[0];
        var b = state[1];
        var c = state[2];
        var d = state[3];
        var e = state[4];
        var f = state[5];
        var g = state[6];
        var h = state[7];

        for (var index = 0; index < 64; index++)
        {
            var t1 = unchecked(h + BigSigma1(e) + Ch(e, f, g) + K[index] + w[index]);
            var t2 = unchecked(BigSigma0(a) + Maj(a, b, c));
            h = g;
            g = f;
            f = e;
            e = unchecked(d + t1);
            d = c;
            c = b;
            b = a;
            a = unchecked(t1 + t2);
        }

        state[0] = unchecked(state[0] + a);
        state[1] = unchecked(state[1] + b);
        state[2] = unchecked(state[2] + c);
        state[3] = unchecked(state[3] + d);
        state[4] = unchecked(state[4] + e);
        state[5] = unchecked(state[5] + f);
        state[6] = unchecked(state[6] + g);
        state[7] = unchecked(state[7] + h);
    }

    private static uint BigSigma0(uint x) => BitOperations.RotateRight(x, 2) ^ BitOperations.RotateRight(x, 13) ^ BitOperations.RotateRight(x, 22);

    private static uint BigSigma1(uint x) => BitOperations.RotateRight(x, 6) ^ BitOperations.RotateRight(x, 11) ^ BitOperations.RotateRight(x, 25);

    private static uint SmallSigma0(uint x) => BitOperations.RotateRight(x, 7) ^ BitOperations.RotateRight(x, 18) ^ (x >> 3);

    private static uint SmallSigma1(uint x) => BitOperations.RotateRight(x, 17) ^ BitOperations.RotateRight(x, 19) ^ (x >> 10);

    private static uint Ch(uint x, uint y, uint z) => (x & y) ^ (~x & z);

    private static uint Maj(uint x, uint y, uint z) => (x & y) ^ (x & z) ^ (y & z);

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
/// Streaming SHA-256 hasher with non-destructive digest snapshots.
/// </summary>
public sealed class Sha256Hasher
{
    private readonly List<byte> _data = [];

    /// <summary>Append bytes and return this hasher for chaining.</summary>
    public Sha256Hasher Update(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        _data.AddRange(data);
        return this;
    }

    /// <summary>Return the current 32-byte digest without modifying this hasher.</summary>
    public byte[] Digest() => Sha256.Hash(_data.ToArray());

    /// <summary>Return the current digest as a lowercase hexadecimal string.</summary>
    public string HexDigest() => Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Sha256Hasher Copy()
    {
        var copy = new Sha256Hasher();
        copy._data.AddRange(_data);
        return copy;
    }
}
