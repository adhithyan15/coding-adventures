using System.Security.Cryptography;

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
        ArgumentNullException.ThrowIfNull(hashFunction);
        ArgumentNullException.ThrowIfNull(key);
        ArgumentNullException.ThrowIfNull(message);

        if (blockSize <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(blockSize), "Block size must be positive.");
        }

        EnsureNonEmptyKey(key);

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
        Compute(MD5.HashData, 64, key, message);

    /// <summary>Compute HMAC-SHA1.</summary>
    public static byte[] HmacSha1(byte[] key, byte[] message) =>
        Compute(SHA1.HashData, 64, key, message);

    /// <summary>Compute HMAC-SHA256.</summary>
    public static byte[] HmacSha256(byte[] key, byte[] message) =>
        Compute(SHA256.HashData, 64, key, message);

    /// <summary>Compute HMAC-SHA512.</summary>
    public static byte[] HmacSha512(byte[] key, byte[] message) =>
        Compute(SHA512.HashData, 128, key, message);

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
        return CryptographicOperations.FixedTimeEquals(expected, actual);
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
