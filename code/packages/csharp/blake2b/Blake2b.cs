using System.Buffers.Binary;
using System.Numerics;

namespace CodingAdventures.Blake2b;

/// <summary>
/// BLAKE2b one-shot and streaming helpers implemented from RFC 7693.
/// </summary>
public static class Blake2b
{
    /// <summary>BLAKE2b block size in bytes.</summary>
    public const int BlockSize = 128;

    /// <summary>Maximum BLAKE2b digest length in bytes.</summary>
    public const int MaxDigestLength = 64;

    /// <summary>Maximum BLAKE2b key length in bytes.</summary>
    public const int MaxKeyLength = 64;

    private static readonly ulong[] Iv =
    [
        0x6A09_E667_F3BC_C908UL,
        0xBB67_AE85_84CA_A73BUL,
        0x3C6E_F372_FE94_F82BUL,
        0xA54F_F53A_5F1D_36F1UL,
        0x510E_527F_ADE6_82D1UL,
        0x9B05_688C_2B3E_6C1FUL,
        0x1F83_D9AB_FB41_BD6BUL,
        0x5BE0_CD19_137E_2179UL,
    ];

    private static readonly int[][] Sigma =
    [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    ];

    /// <summary>Compute a BLAKE2b digest for a complete byte array.</summary>
    public static byte[] Hash(byte[] data, Blake2bOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(data);
        var hasher = new Blake2bHasher(options);
        hasher.Update(data);
        return hasher.Digest();
    }

    /// <summary>Compute BLAKE2b and return a lowercase hexadecimal digest.</summary>
    public static string HashHex(byte[] data, Blake2bOptions? options = null) =>
        Convert.ToHexString(Hash(data, options)).ToLowerInvariant();

    internal static void Validate(Blake2bOptions options)
    {
        if (options.DigestSize < 1 || options.DigestSize > MaxDigestLength)
        {
            throw new ArgumentOutOfRangeException(nameof(options), options.DigestSize, "Digest size must be in [1, 64].");
        }

        if (options.Key.Length > MaxKeyLength)
        {
            throw new ArgumentException("Key length must be in [0, 64].", nameof(options));
        }

        if (options.Salt.Length is not 0 and not 16)
        {
            throw new ArgumentException("Salt must be empty or exactly 16 bytes.", nameof(options));
        }

        if (options.Personal.Length is not 0 and not 16)
        {
            throw new ArgumentException("Personal must be empty or exactly 16 bytes.", nameof(options));
        }
    }

    internal static ulong[] InitialState(Blake2bOptions options)
    {
        var parameter = new byte[64];
        parameter[0] = checked((byte)options.DigestSize);
        parameter[1] = checked((byte)options.Key.Length);
        parameter[2] = 1;
        parameter[3] = 1;

        if (options.Salt.Length > 0)
        {
            options.Salt.CopyTo(parameter.AsSpan(32, 16));
        }

        if (options.Personal.Length > 0)
        {
            options.Personal.CopyTo(parameter.AsSpan(48, 16));
        }

        var state = Iv.ToArray();
        for (var index = 0; index < state.Length; index++)
        {
            state[index] ^= BinaryPrimitives.ReadUInt64LittleEndian(parameter.AsSpan(index * 8, 8));
        }

        return state;
    }

    internal static void Compress(ulong[] state, ReadOnlySpan<byte> block, ulong counterLow, ulong counterHigh, bool isFinal)
    {
        Span<ulong> m = stackalloc ulong[16];
        for (var index = 0; index < m.Length; index++)
        {
            m[index] = BinaryPrimitives.ReadUInt64LittleEndian(block.Slice(index * 8, 8));
        }

        Span<ulong> v = stackalloc ulong[16];
        for (var index = 0; index < 8; index++)
        {
            v[index] = state[index];
            v[index + 8] = Iv[index];
        }

        v[12] ^= counterLow;
        v[13] ^= counterHigh;
        if (isFinal)
        {
            v[14] ^= ulong.MaxValue;
        }

        for (var round = 0; round < 12; round++)
        {
            var s = Sigma[round % 10];
            Mix(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
            Mix(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
            Mix(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
            Mix(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
            Mix(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
            Mix(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
            Mix(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
            Mix(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
        }

        for (var index = 0; index < 8; index++)
        {
            state[index] ^= v[index] ^ v[index + 8];
        }
    }

    internal static void WriteDigest(ulong[] state, Span<byte> destination)
    {
        Span<byte> full = stackalloc byte[MaxDigestLength];
        for (var index = 0; index < state.Length; index++)
        {
            BinaryPrimitives.WriteUInt64LittleEndian(full.Slice(index * 8, 8), state[index]);
        }

        full[..destination.Length].CopyTo(destination);
    }

    private static void Mix(Span<ulong> v, int a, int b, int c, int d, ulong x, ulong y)
    {
        v[a] = unchecked(v[a] + v[b] + x);
        v[d] = BitOperations.RotateRight(v[d] ^ v[a], 32);
        v[c] = unchecked(v[c] + v[d]);
        v[b] = BitOperations.RotateRight(v[b] ^ v[c], 24);
        v[a] = unchecked(v[a] + v[b] + y);
        v[d] = BitOperations.RotateRight(v[d] ^ v[a], 16);
        v[c] = unchecked(v[c] + v[d]);
        v[b] = BitOperations.RotateRight(v[b] ^ v[c], 63);
    }
}

/// <summary>BLAKE2b parameter block options for sequential mode.</summary>
public sealed class Blake2bOptions
{
    /// <summary>Create BLAKE2b options. Arrays are defensively copied.</summary>
    public Blake2bOptions(int digestSize = Blake2b.MaxDigestLength, byte[]? key = null, byte[]? salt = null, byte[]? personal = null)
    {
        DigestSize = digestSize;
        Key = key?.ToArray() ?? [];
        Salt = salt?.ToArray() ?? [];
        Personal = personal?.ToArray() ?? [];
    }

    /// <summary>Requested digest length in bytes.</summary>
    public int DigestSize { get; }

    /// <summary>Optional key for MAC mode.</summary>
    public byte[] Key { get; }

    /// <summary>Optional 16-byte salt.</summary>
    public byte[] Salt { get; }

    /// <summary>Optional 16-byte personalization string.</summary>
    public byte[] Personal { get; }

    /// <summary>Default BLAKE2b options: 64-byte digest, unkeyed, no salt, no personalization.</summary>
    public static Blake2bOptions Default => new();

    /// <summary>Return a copy with a different digest size.</summary>
    public Blake2bOptions WithDigestSize(int digestSize) => new(digestSize, Key, Salt, Personal);

    /// <summary>Return a copy with a different key.</summary>
    public Blake2bOptions WithKey(byte[] key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return new Blake2bOptions(DigestSize, key, Salt, Personal);
    }

    /// <summary>Return a copy with a different salt.</summary>
    public Blake2bOptions WithSalt(byte[] salt)
    {
        ArgumentNullException.ThrowIfNull(salt);
        return new Blake2bOptions(DigestSize, Key, salt, Personal);
    }

    /// <summary>Return a copy with a different personalization string.</summary>
    public Blake2bOptions WithPersonal(byte[] personal)
    {
        ArgumentNullException.ThrowIfNull(personal);
        return new Blake2bOptions(DigestSize, Key, Salt, personal);
    }
}

/// <summary>Streaming BLAKE2b hasher with non-destructive digest snapshots.</summary>
public sealed class Blake2bHasher
{
    private readonly List<byte> _buffer;
    private readonly int _digestSize;
    private readonly ulong[] _state;
    private ulong _counterLow;
    private ulong _counterHigh;

    /// <summary>Create a hasher with optional BLAKE2b parameters.</summary>
    public Blake2bHasher(Blake2bOptions? options = null)
    {
        options ??= Blake2bOptions.Default;
        Blake2b.Validate(options);

        _digestSize = options.DigestSize;
        _state = Blake2b.InitialState(options);
        _buffer = new List<byte>(Blake2b.BlockSize * 2);

        if (options.Key.Length > 0)
        {
            var keyBlock = new byte[Blake2b.BlockSize];
            options.Key.CopyTo(keyBlock.AsSpan());
            _buffer.AddRange(keyBlock);
        }
    }

    private Blake2bHasher(ulong[] state, byte[] buffer, ulong counterLow, ulong counterHigh, int digestSize)
    {
        _state = state.ToArray();
        _buffer = new List<byte>(buffer);
        _counterLow = counterLow;
        _counterHigh = counterHigh;
        _digestSize = digestSize;
    }

    /// <summary>Append bytes and return this hasher for chaining.</summary>
    public Blake2bHasher Update(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        _buffer.AddRange(data);

        while (_buffer.Count > Blake2b.BlockSize)
        {
            AddCount(Blake2b.BlockSize);
            var block = new byte[Blake2b.BlockSize];
            _buffer.CopyTo(0, block, 0, Blake2b.BlockSize);
            Blake2b.Compress(_state, block, _counterLow, _counterHigh, isFinal: false);
            _buffer.RemoveRange(0, Blake2b.BlockSize);
        }

        return this;
    }

    /// <summary>Return the current digest without modifying this hasher.</summary>
    public byte[] Digest()
    {
        var state = _state.ToArray();
        var finalBlock = new byte[Blake2b.BlockSize];
        _buffer.CopyTo(finalBlock);

        var finalLow = _counterLow;
        var finalHigh = _counterHigh;
        AddToCount(ref finalLow, ref finalHigh, checked((ulong)_buffer.Count));
        Blake2b.Compress(state, finalBlock, finalLow, finalHigh, isFinal: true);

        var digest = new byte[_digestSize];
        Blake2b.WriteDigest(state, digest);
        return digest;
    }

    /// <summary>Return the current digest as lowercase hexadecimal text.</summary>
    public string HexDigest() => Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Blake2bHasher Copy() => new(_state, _buffer.ToArray(), _counterLow, _counterHigh, _digestSize);

    private void AddCount(ulong value) => AddToCount(ref _counterLow, ref _counterHigh, value);

    private static void AddToCount(ref ulong low, ref ulong high, ulong value)
    {
        var previous = low;
        low = unchecked(low + value);
        if (low < previous)
        {
            high = unchecked(high + 1);
        }
    }
}
