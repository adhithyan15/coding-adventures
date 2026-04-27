using Md5Algorithm = CodingAdventures.Md5.Md5;
using Sha1Algorithm = CodingAdventures.Sha1.Sha1;
using Sha256Algorithm = CodingAdventures.Sha256.Sha256;
using Sha512Algorithm = CodingAdventures.Sha512.Sha512;

namespace CodingAdventures.Hmac;

/// <summary>
/// HMAC helpers for MD5, SHA-1, SHA-256, and SHA-512.
/// </summary>
public static class Hmac
{
    private const byte Ipad = 0x36;
    private const byte Opad = 0x5c;

    /// <summary>Compute the RFC 2104 HMAC construction using a supplied hash function.</summary>
    public static byte[] Compute(Func<byte[], byte[]> hashFunction, int blockSize, byte[] key, byte[] message)
    {
        ArgumentNullException.ThrowIfNull(key);
        EnsureNonEmptyKey(key);
        return ComputeAllowEmptyKey(hashFunction, blockSize, key, message);
    }

    /// <summary>Compute the RFC 2104 HMAC construction using a supplied hash function, allowing an empty key.</summary>
    public static byte[] ComputeAllowEmptyKey(Func<byte[], byte[]> hashFunction, int blockSize, byte[] key, byte[] message)
    {
        ArgumentNullException.ThrowIfNull(hashFunction);
        ArgumentNullException.ThrowIfNull(key);
        ArgumentNullException.ThrowIfNull(message);

        if (blockSize <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(blockSize), "Block size must be positive.");
        }

        var keyPrime = NormalizeKey(hashFunction, blockSize, key);
        var innerKey = new byte[blockSize];
        var outerKey = new byte[blockSize];

        for (var i = 0; i < blockSize; i++)
        {
            innerKey[i] = (byte)(keyPrime[i] ^ Ipad);
            outerKey[i] = (byte)(keyPrime[i] ^ Opad);
        }

        var innerInput = Concat(innerKey, message);
        var inner = hashFunction(innerInput);
        return hashFunction(Concat(outerKey, inner));
    }

    /// <summary>Compute HMAC-MD5.</summary>
    public static byte[] HmacMd5(byte[] key, byte[] message) =>
        Compute(data => Md5Algorithm.SumMd5(data), 64, key, message);

    /// <summary>Compute HMAC-SHA1.</summary>
    public static byte[] HmacSha1(byte[] key, byte[] message) =>
        Compute(Sha1Algorithm.Hash, 64, key, message);

    /// <summary>Compute HMAC-SHA256.</summary>
    public static byte[] HmacSha256(byte[] key, byte[] message) =>
        Compute(Sha256Algorithm.Hash, 64, key, message);

    /// <summary>Compute HMAC-SHA512.</summary>
    public static byte[] HmacSha512(byte[] key, byte[] message) =>
        Compute(Sha512Algorithm.Hash, 128, key, message);

    /// <summary>Compute HMAC-MD5 and return lowercase hex.</summary>
    public static string HmacMd5Hex(byte[] key, byte[] message) =>
        ToHex(HmacMd5(key, message));

    /// <summary>Compute HMAC-SHA1 and return lowercase hex.</summary>
    public static string HmacSha1Hex(byte[] key, byte[] message) =>
        ToHex(HmacSha1(key, message));

    /// <summary>Compute HMAC-SHA256 and return lowercase hex.</summary>
    public static string HmacSha256Hex(byte[] key, byte[] message) =>
        ToHex(HmacSha256(key, message));

    /// <summary>Compute HMAC-SHA512 and return lowercase hex.</summary>
    public static string HmacSha512Hex(byte[] key, byte[] message) =>
        ToHex(HmacSha512(key, message));

    /// <summary>Compare two tags in constant time.</summary>
    public static bool Verify(byte[] expected, byte[] actual)
    {
        ArgumentNullException.ThrowIfNull(expected);
        ArgumentNullException.ThrowIfNull(actual);

        if (expected.Length != actual.Length)
        {
            return false;
        }

        var diff = 0;
        for (var i = 0; i < expected.Length; i++)
        {
            diff |= expected[i] ^ actual[i];
        }

        return diff == 0;
    }

    private static byte[] NormalizeKey(Func<byte[], byte[]> hashFunction, int blockSize, byte[] key)
    {
        var normalized = key.Length > blockSize ? hashFunction(key) : key.ToArray();
        if (normalized.Length > blockSize)
        {
            throw new ArgumentException("Hash output must not exceed the HMAC block size.", nameof(hashFunction));
        }

        Array.Resize(ref normalized, blockSize);
        return normalized;
    }

    private static byte[] Concat(byte[] left, byte[] right)
    {
        var result = new byte[left.Length + right.Length];
        Buffer.BlockCopy(left, 0, result, 0, left.Length);
        Buffer.BlockCopy(right, 0, result, left.Length, right.Length);
        return result;
    }

    private static string ToHex(byte[] data) =>
        Convert.ToHexString(data).ToLowerInvariant();

    private static void EnsureNonEmptyKey(byte[] key)
    {
        if (key.Length == 0)
        {
            throw new ArgumentException("HMAC key must not be empty.", nameof(key));
        }
    }
}
