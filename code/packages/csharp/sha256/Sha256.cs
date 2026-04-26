using System.Security.Cryptography;

namespace CodingAdventures.Sha256;

/// <summary>
/// SHA-256 one-shot and streaming helpers.
/// </summary>
public static class Sha256
{
    /// <summary>SHA-256 digest length in bytes.</summary>
    public const int DigestLength = 32;

    /// <summary>Compute the SHA-256 digest for a complete byte array.</summary>
    public static byte[] Hash(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return SHA256.HashData(data);
    }

    /// <summary>Compute SHA-256 and return a lowercase hexadecimal digest.</summary>
    public static string HashHex(byte[] data) =>
        Convert.ToHexString(Hash(data)).ToLowerInvariant();
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
    public byte[] Digest() => SHA256.HashData(_data.ToArray());

    /// <summary>Return the current digest as a lowercase hexadecimal string.</summary>
    public string HexDigest() =>
        Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Sha256Hasher Copy()
    {
        var copy = new Sha256Hasher();
        copy._data.AddRange(_data);
        return copy;
    }
}
