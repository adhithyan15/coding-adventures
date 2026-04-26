using System.Security.Cryptography;

namespace CodingAdventures.Sha512;

/// <summary>
/// SHA-512 one-shot and streaming helpers.
/// </summary>
public static class Sha512
{
    /// <summary>SHA-512 digest length in bytes.</summary>
    public const int DigestLength = 64;

    /// <summary>Compute the SHA-512 digest for a complete byte array.</summary>
    public static byte[] Hash(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return SHA512.HashData(data);
    }

    /// <summary>Compute SHA-512 and return a lowercase hexadecimal digest.</summary>
    public static string HashHex(byte[] data) =>
        Convert.ToHexString(Hash(data)).ToLowerInvariant();
}

/// <summary>
/// Streaming SHA-512 hasher with non-destructive digest snapshots.
/// </summary>
public sealed class Sha512Hasher
{
    private readonly List<byte> _data = [];

    /// <summary>Append bytes and return this hasher for chaining.</summary>
    public Sha512Hasher Update(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        _data.AddRange(data);
        return this;
    }

    /// <summary>Return the current 64-byte digest without modifying this hasher.</summary>
    public byte[] Digest() => SHA512.HashData(_data.ToArray());

    /// <summary>Return the current digest as a lowercase hexadecimal string.</summary>
    public string HexDigest() =>
        Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Sha512Hasher Copy()
    {
        var copy = new Sha512Hasher();
        copy._data.AddRange(_data);
        return copy;
    }
}
