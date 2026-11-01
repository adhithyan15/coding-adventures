using System.Buffers.Binary;
using System.Numerics;
using System.Runtime.CompilerServices;

[assembly: InternalsVisibleTo("CodingAdventures.Sha512.Tests")]

namespace CodingAdventures.Sha512;

/// <summary>
/// SHA-512 one-shot and streaming helpers implemented from FIPS 180-4.
/// </summary>
public static class Sha512
{
    /// <summary>SHA-512 digest length in bytes.</summary>
    public const int DigestLength = 64;

    internal static readonly ulong[] InitialState =
    [
        0x6a09e667f3bcc908UL, 0xbb67ae8584caa73bUL, 0x3c6ef372fe94f82bUL, 0xa54ff53a5f1d36f1UL,
        0x510e527fade682d1UL, 0x9b05688c2b3e6c1fUL, 0x1f83d9abfb41bd6bUL, 0x5be0cd19137e2179UL,
    ];

    private static readonly ulong[] K =
    [
        0x428a2f98d728ae22UL, 0x7137449123ef65cdUL, 0xb5c0fbcfec4d3b2fUL, 0xe9b5dba58189dbbcUL,
        0x3956c25bf348b538UL, 0x59f111f1b605d019UL, 0x923f82a4af194f9bUL, 0xab1c5ed5da6d8118UL,
        0xd807aa98a3030242UL, 0x12835b0145706fbeUL, 0x243185be4ee4b28cUL, 0x550c7dc3d5ffb4e2UL,
        0x72be5d74f27b896fUL, 0x80deb1fe3b1696b1UL, 0x9bdc06a725c71235UL, 0xc19bf174cf692694UL,
        0xe49b69c19ef14ad2UL, 0xefbe4786384f25e3UL, 0x0fc19dc68b8cd5b5UL, 0x240ca1cc77ac9c65UL,
        0x2de92c6f592b0275UL, 0x4a7484aa6ea6e483UL, 0x5cb0a9dcbd41fbd4UL, 0x76f988da831153b5UL,
        0x983e5152ee66dfabUL, 0xa831c66d2db43210UL, 0xb00327c898fb213fUL, 0xbf597fc7beef0ee4UL,
        0xc6e00bf33da88fc2UL, 0xd5a79147930aa725UL, 0x06ca6351e003826fUL, 0x142929670a0e6e70UL,
        0x27b70a8546d22ffcUL, 0x2e1b21385c26c926UL, 0x4d2c6dfc5ac42aedUL, 0x53380d139d95b3dfUL,
        0x650a73548baf63deUL, 0x766a0abb3c77b2a8UL, 0x81c2c92e47edaee6UL, 0x92722c851482353bUL,
        0xa2bfe8a14cf10364UL, 0xa81a664bbc423001UL, 0xc24b8b70d0f89791UL, 0xc76c51a30654be30UL,
        0xd192e819d6ef5218UL, 0xd69906245565a910UL, 0xf40e35855771202aUL, 0x106aa07032bbd1b8UL,
        0x19a4c116b8d2d0c8UL, 0x1e376c085141ab53UL, 0x2748774cdf8eeb99UL, 0x34b0bcb5e19b48a8UL,
        0x391c0cb3c5c95a63UL, 0x4ed8aa4ae3418acbUL, 0x5b9cca4f7763e373UL, 0x682e6ff3d6b2b8a3UL,
        0x748f82ee5defb2fcUL, 0x78a5636f43172f60UL, 0x84c87814a1f0ab72UL, 0x8cc702081a6439ecUL,
        0x90befffa23631e28UL, 0xa4506cebde82bde9UL, 0xbef9a3f7b2c67915UL, 0xc67178f2e372532bUL,
        0xca273eceea26619cUL, 0xd186b8c721c0c207UL, 0xeada7dd6cde0eb1eUL, 0xf57d4f7fee6ed178UL,
        0x06f067aa72176fbaUL, 0x0a637dc5a2c898a6UL, 0x113f9804bef90daeUL, 0x1b710b35131c471bUL,
        0x28db77f523047d84UL, 0x32caab7b40c72493UL, 0x3c9ebe0a15c9bebcUL, 0x431d67c49c100d4cUL,
        0x4cc5d4becb3e42b6UL, 0x597f299cfc657e2aUL, 0x5fcb6fab3ad6faecUL, 0x6c44198c4a475817UL,
    ];

    /// <summary>Compute the SHA-512 digest for a complete byte array.</summary>
    public static byte[] Hash(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var padded = Pad(data);
        var state = InitialState.ToArray();

        for (var offset = 0; offset < padded.Length; offset += 128)
        {
            Compress(state, padded.AsSpan(offset, 128));
        }

        return StateToBytes(state);
    }

    /// <summary>Compute SHA-512 and return a lowercase hexadecimal digest.</summary>
    public static string HashHex(byte[] data) =>
        Convert.ToHexString(Hash(data)).ToLowerInvariant();

    private static byte[] Pad(ReadOnlySpan<byte> data)
    {
        var afterBit = (data.Length + 1) % 128;
        var zeroCount = afterBit <= 112 ? 112 - afterBit : 128 + 112 - afterBit;
        var result = new byte[data.Length + 1 + zeroCount + 16];

        data.CopyTo(result);
        result[data.Length] = 0x80;
        EncodeBitLength(0, (ulong)data.Length).CopyTo(result, result.Length - 16);
        return result;
    }

    /// <summary>Encode a 128-bit byte count as SHA-512's checked bit length.</summary>
    internal static byte[] EncodeBitLength(ulong byteCountHigh, ulong byteCountLow)
    {
        if ((byteCountHigh & 0xe000000000000000UL) != 0)
        {
            throw new ArgumentOutOfRangeException(nameof(byteCountHigh), "SHA-512 message length must be less than 2^128 bits");
        }

        var result = new byte[16];
        var bitCountHigh = (byteCountHigh << 3) | (byteCountLow >> 61);
        var bitCountLow = byteCountLow << 3;
        BinaryPrimitives.WriteUInt64BigEndian(result.AsSpan(0, 8), bitCountHigh);
        BinaryPrimitives.WriteUInt64BigEndian(result.AsSpan(8, 8), bitCountLow);
        return result;
    }

    internal static void Compress(ulong[] state, ReadOnlySpan<byte> block)
    {
        Span<ulong> w = stackalloc ulong[80];
        for (var index = 0; index < 16; index++)
        {
            w[index] = BinaryPrimitives.ReadUInt64BigEndian(block.Slice(index * 8, 8));
        }

        for (var index = 16; index < 80; index++)
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

        for (var index = 0; index < 80; index++)
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

    private static ulong BigSigma0(ulong x) => BitOperations.RotateRight(x, 28) ^ BitOperations.RotateRight(x, 34) ^ BitOperations.RotateRight(x, 39);

    private static ulong BigSigma1(ulong x) => BitOperations.RotateRight(x, 14) ^ BitOperations.RotateRight(x, 18) ^ BitOperations.RotateRight(x, 41);

    private static ulong SmallSigma0(ulong x) => BitOperations.RotateRight(x, 1) ^ BitOperations.RotateRight(x, 8) ^ (x >> 7);

    private static ulong SmallSigma1(ulong x) => BitOperations.RotateRight(x, 19) ^ BitOperations.RotateRight(x, 61) ^ (x >> 6);

    private static ulong Ch(ulong x, ulong y, ulong z) => (x & y) ^ (~x & z);

    private static ulong Maj(ulong x, ulong y, ulong z) => (x & y) ^ (x & z) ^ (y & z);

    private static byte[] StateToBytes(ulong[] state)
    {
        var digest = new byte[DigestLength];
        for (var index = 0; index < state.Length; index++)
        {
            BinaryPrimitives.WriteUInt64BigEndian(digest.AsSpan(index * 8, 8), state[index]);
        }

        return digest;
    }
}

/// <summary>
/// Streaming SHA-512 hasher with non-destructive digest snapshots.
/// </summary>
public sealed class Sha512Hasher
{
    private ulong[] _state = Sha512.InitialState.ToArray();
    private readonly byte[] _buffer = new byte[128];
    private int _bufferLength;
    private ulong _byteCountHigh;
    private ulong _byteCountLow;

    /// <summary>Append bytes and return this hasher for chaining.</summary>
    public Sha512Hasher Update(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var added = (ulong)data.Length;
        var nextLow = unchecked(_byteCountLow + added);
        var nextHigh = _byteCountHigh + (nextLow < _byteCountLow ? 1UL : 0UL);
        _ = Sha512.EncodeBitLength(nextHigh, nextLow);

        _byteCountHigh = nextHigh;
        _byteCountLow = nextLow;
        var offset = 0;
        while (offset < data.Length)
        {
            var copied = Math.Min(128 - _bufferLength, data.Length - offset);
            data.AsSpan(offset, copied).CopyTo(_buffer.AsSpan(_bufferLength));
            offset += copied;
            _bufferLength += copied;
            if (_bufferLength == 128)
            {
                Sha512.Compress(_state, _buffer);
                _bufferLength = 0;
            }
        }
        return this;
    }

    /// <summary>Return the current 64-byte digest without modifying this hasher.</summary>
    public byte[] Digest()
    {
        var zeroCount = _bufferLength + 1 <= 112 ? 112 - (_bufferLength + 1) : 240 - (_bufferLength + 1);
        var tail = new byte[_bufferLength + 1 + zeroCount + 16];
        _buffer.AsSpan(0, _bufferLength).CopyTo(tail);
        tail[_bufferLength] = 0x80;
        Sha512.EncodeBitLength(_byteCountHigh, _byteCountLow).CopyTo(tail, tail.Length - 16);

        var state = _state.ToArray();
        for (var offset = 0; offset < tail.Length; offset += 128)
        {
            Sha512.Compress(state, tail.AsSpan(offset, 128));
        }

        var digest = new byte[Sha512.DigestLength];
        for (var index = 0; index < state.Length; index++)
        {
            BinaryPrimitives.WriteUInt64BigEndian(digest.AsSpan(index * 8, 8), state[index]);
        }
        return digest;
    }

    /// <summary>Return the current digest as a lowercase hexadecimal string.</summary>
    public string HexDigest() => Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Sha512Hasher Copy()
    {
        var copy = new Sha512Hasher();
        copy._state = _state.ToArray();
        _buffer.CopyTo(copy._buffer, 0);
        copy._bufferLength = _bufferLength;
        copy._byteCountHigh = _byteCountHigh;
        copy._byteCountLow = _byteCountLow;
        return copy;
    }
}
