using System.Security.Cryptography;

namespace CodingAdventures.Sha1;

/// <summary>
/// SHA-1 one-shot and streaming helpers.
/// </summary>
public static class Sha1
{
    /// <summary>SHA-1 digest length in bytes.</summary>
    public const int DigestLength = 20;

    /// <summary>Compute the SHA-1 digest for a complete byte array.</summary>
    public static byte[] Hash(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return SHA1.HashData(data);
    }

    /// <summary>Compute SHA-1 and return a lowercase hexadecimal digest.</summary>
    public static string HashHex(byte[] data) =>
        Convert.ToHexString(Hash(data)).ToLowerInvariant();
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
    public byte[] Digest() => SHA1.HashData(_data.ToArray());

    /// <summary>Return the current digest as a lowercase hexadecimal string.</summary>
    public string HexDigest() =>
        Convert.ToHexString(Digest()).ToLowerInvariant();

    /// <summary>Return an independent copy of the current hasher state.</summary>
    public Sha1Hasher Copy()
    {
        var copy = new Sha1Hasher();
        copy._data.AddRange(_data);
        return copy;
    }
}
