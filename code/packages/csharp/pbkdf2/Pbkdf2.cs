using System.Security.Cryptography;

namespace CodingAdventures.Pbkdf2;

/// <summary>
/// PBKDF2 key derivation helpers for HMAC-SHA1, HMAC-SHA256, and HMAC-SHA512.
/// </summary>
public static class Pbkdf2
{
    private const int MaxKeyLength = 1 << 20;

    /// <summary>Derive key material using PBKDF2 and a specific HMAC hash algorithm.</summary>
    public static byte[] Derive(
        byte[] password,
        byte[] salt,
        int iterations,
        int keyLength,
        HashAlgorithmName hashAlgorithm,
        bool allowEmptyPassword = false)
    {
        ArgumentNullException.ThrowIfNull(password);
        ArgumentNullException.ThrowIfNull(salt);

        if (!allowEmptyPassword && password.Length == 0)
        {
            throw new ArgumentException("PBKDF2 password must not be empty.", nameof(password));
        }

        if (iterations <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(iterations), "Iterations must be positive.");
        }

        if (keyLength <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(keyLength), "Key length must be positive.");
        }

        if (keyLength > MaxKeyLength)
        {
            throw new ArgumentOutOfRangeException(nameof(keyLength), "Key length must not exceed 2^20 bytes.");
        }

        return Rfc2898DeriveBytes.Pbkdf2(password, salt, iterations, hashAlgorithm, keyLength);
    }

    /// <summary>Derive key material using PBKDF2-HMAC-SHA1.</summary>
    public static byte[] Pbkdf2HmacSha1(byte[] password, byte[] salt, int iterations, int keyLength) =>
        Derive(password, salt, iterations, keyLength, HashAlgorithmName.SHA1);

    /// <summary>Derive key material using PBKDF2-HMAC-SHA256.</summary>
    public static byte[] Pbkdf2HmacSha256(
        byte[] password,
        byte[] salt,
        int iterations,
        int keyLength,
        bool allowEmptyPassword = false) =>
        Derive(password, salt, iterations, keyLength, HashAlgorithmName.SHA256, allowEmptyPassword);

    /// <summary>Derive key material using PBKDF2-HMAC-SHA512.</summary>
    public static byte[] Pbkdf2HmacSha512(byte[] password, byte[] salt, int iterations, int keyLength) =>
        Derive(password, salt, iterations, keyLength, HashAlgorithmName.SHA512);

    /// <summary>Derive PBKDF2-HMAC-SHA1 and return lowercase hex.</summary>
    public static string Pbkdf2HmacSha1Hex(byte[] password, byte[] salt, int iterations, int keyLength) =>
        ToHex(Pbkdf2HmacSha1(password, salt, iterations, keyLength));

    /// <summary>Derive PBKDF2-HMAC-SHA256 and return lowercase hex.</summary>
    public static string Pbkdf2HmacSha256Hex(byte[] password, byte[] salt, int iterations, int keyLength) =>
        ToHex(Pbkdf2HmacSha256(password, salt, iterations, keyLength));

    /// <summary>Derive PBKDF2-HMAC-SHA512 and return lowercase hex.</summary>
    public static string Pbkdf2HmacSha512Hex(byte[] password, byte[] salt, int iterations, int keyLength) =>
        ToHex(Pbkdf2HmacSha512(password, salt, iterations, keyLength));

    private static string ToHex(byte[] data) =>
        Convert.ToHexString(data).ToLowerInvariant();
}
