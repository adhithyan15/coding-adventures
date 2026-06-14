using System.Buffers.Binary;
using HmacAlgorithm = CodingAdventures.Hmac.Hmac;
using Sha1Algorithm = CodingAdventures.Sha1.Sha1;
using Sha256Algorithm = CodingAdventures.Sha256.Sha256;
using Sha512Algorithm = CodingAdventures.Sha512.Sha512;

namespace CodingAdventures.Pbkdf2;

/// <summary>Hash families supported by the PBKDF2 helpers.</summary>
public enum Pbkdf2Hash
{
    /// <summary>PBKDF2-HMAC-SHA1.</summary>
    Sha1,

    /// <summary>PBKDF2-HMAC-SHA256.</summary>
    Sha256,

    /// <summary>PBKDF2-HMAC-SHA512.</summary>
    Sha512,
}

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
        Pbkdf2Hash hashAlgorithm,
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

        var digestLength = HashLength(hashAlgorithm);
        var blockCount = (keyLength + digestLength - 1) / digestLength;
        var derived = new byte[blockCount * digestLength];

        for (var blockIndex = 1; blockIndex <= blockCount; blockIndex++)
        {
            var block = DeriveBlock(password, salt, iterations, hashAlgorithm, blockIndex);
            Buffer.BlockCopy(block, 0, derived, (blockIndex - 1) * digestLength, digestLength);
        }

        Array.Resize(ref derived, keyLength);
        return derived;
    }

    /// <summary>Derive key material using PBKDF2-HMAC-SHA1.</summary>
    public static byte[] Pbkdf2HmacSha1(byte[] password, byte[] salt, int iterations, int keyLength) =>
        Derive(password, salt, iterations, keyLength, Pbkdf2Hash.Sha1);

    /// <summary>Derive key material using PBKDF2-HMAC-SHA256.</summary>
    public static byte[] Pbkdf2HmacSha256(
        byte[] password,
        byte[] salt,
        int iterations,
        int keyLength,
        bool allowEmptyPassword = false) =>
        Derive(password, salt, iterations, keyLength, Pbkdf2Hash.Sha256, allowEmptyPassword);

    /// <summary>Derive key material using PBKDF2-HMAC-SHA512.</summary>
    public static byte[] Pbkdf2HmacSha512(byte[] password, byte[] salt, int iterations, int keyLength) =>
        Derive(password, salt, iterations, keyLength, Pbkdf2Hash.Sha512);

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

    private static byte[] DeriveBlock(byte[] password, byte[] salt, int iterations, Pbkdf2Hash hashAlgorithm, int blockIndex)
    {
        var firstInput = new byte[salt.Length + 4];
        Buffer.BlockCopy(salt, 0, firstInput, 0, salt.Length);
        BinaryPrimitives.WriteUInt32BigEndian(firstInput.AsSpan(salt.Length, 4), (uint)blockIndex);

        var u = ComputeHmac(hashAlgorithm, password, firstInput);
        var block = u.ToArray();

        for (var iteration = 1; iteration < iterations; iteration++)
        {
            u = ComputeHmac(hashAlgorithm, password, u);

            for (var index = 0; index < block.Length; index++)
            {
                block[index] ^= u[index];
            }
        }

        return block;
    }

    private static byte[] ComputeHmac(Pbkdf2Hash hashAlgorithm, byte[] key, byte[] message) =>
        hashAlgorithm switch
        {
            Pbkdf2Hash.Sha1 => HmacAlgorithm.ComputeAllowEmptyKey(Sha1Algorithm.Hash, 64, key, message),
            Pbkdf2Hash.Sha256 => HmacAlgorithm.ComputeAllowEmptyKey(Sha256Algorithm.Hash, 64, key, message),
            Pbkdf2Hash.Sha512 => HmacAlgorithm.ComputeAllowEmptyKey(Sha512Algorithm.Hash, 128, key, message),
            _ => throw new ArgumentOutOfRangeException(nameof(hashAlgorithm), "Unsupported PBKDF2 hash algorithm."),
        };

    private static int HashLength(Pbkdf2Hash hashAlgorithm) =>
        hashAlgorithm switch
        {
            Pbkdf2Hash.Sha1 => 20,
            Pbkdf2Hash.Sha256 => 32,
            Pbkdf2Hash.Sha512 => 64,
            _ => throw new ArgumentOutOfRangeException(nameof(hashAlgorithm), "Unsupported PBKDF2 hash algorithm."),
        };
}
