using System.Security.Cryptography;

namespace CodingAdventures.Hkdf;

/// <summary>Hash families supported by the HKDF helpers.</summary>
public enum HkdfHash
{
    /// <summary>HKDF-HMAC-SHA256.</summary>
    Sha256,

    /// <summary>HKDF-HMAC-SHA512.</summary>
    Sha512,
}

/// <summary>
/// HKDF extract-and-expand helpers from RFC 5869.
/// </summary>
public static class Hkdf
{
    /// <summary>HKDF-Extract: concentrate input keying material into a pseudorandom key.</summary>
    public static byte[] Extract(byte[] salt, byte[] ikm, HkdfHash hash = HkdfHash.Sha256)
    {
        ArgumentNullException.ThrowIfNull(salt);
        ArgumentNullException.ThrowIfNull(ikm);

        var hashLength = HashLength(hash);
        var actualSalt = salt.Length == 0 ? new byte[hashLength] : salt;
        using var hmac = CreateHmac(hash, actualSalt);
        return hmac.ComputeHash(ikm);
    }

    /// <summary>HKDF-Expand: derive output keying material from a pseudorandom key.</summary>
    public static byte[] Expand(byte[] prk, byte[] info, int length, HkdfHash hash = HkdfHash.Sha256)
    {
        ArgumentNullException.ThrowIfNull(prk);
        ArgumentNullException.ThrowIfNull(info);

        var hashLength = HashLength(hash);
        if (length <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(length), "HKDF output length must be positive.");
        }

        var maxLength = 255 * hashLength;
        if (length > maxLength)
        {
            throw new ArgumentOutOfRangeException(nameof(length), $"HKDF output length must not exceed {maxLength} bytes.");
        }

        var okm = new byte[length];
        var previous = Array.Empty<byte>();
        var offset = 0;
        var counter = 1;

        while (offset < length)
        {
            using var hmac = CreateHmac(hash, prk);
            var input = new byte[previous.Length + info.Length + 1];
            Buffer.BlockCopy(previous, 0, input, 0, previous.Length);
            Buffer.BlockCopy(info, 0, input, previous.Length, info.Length);
            input[^1] = (byte)counter;

            previous = hmac.ComputeHash(input);
            var toCopy = Math.Min(previous.Length, length - offset);
            Buffer.BlockCopy(previous, 0, okm, offset, toCopy);
            offset += toCopy;
            counter++;
        }

        return okm;
    }

    /// <summary>HKDF extract followed by expand.</summary>
    public static byte[] Derive(byte[] salt, byte[] ikm, byte[] info, int length, HkdfHash hash = HkdfHash.Sha256) =>
        Expand(Extract(salt, ikm, hash), info, length, hash);

    /// <summary>SHA-256 HKDF-Extract convenience helper.</summary>
    public static byte[] ExtractSha256(byte[] salt, byte[] ikm) =>
        Extract(salt, ikm, HkdfHash.Sha256);

    /// <summary>SHA-256 HKDF-Expand convenience helper.</summary>
    public static byte[] ExpandSha256(byte[] prk, byte[] info, int length) =>
        Expand(prk, info, length, HkdfHash.Sha256);

    /// <summary>SHA-256 HKDF convenience helper.</summary>
    public static byte[] DeriveSha256(byte[] salt, byte[] ikm, byte[] info, int length) =>
        Derive(salt, ikm, info, length, HkdfHash.Sha256);

    /// <summary>SHA-512 HKDF-Extract convenience helper.</summary>
    public static byte[] ExtractSha512(byte[] salt, byte[] ikm) =>
        Extract(salt, ikm, HkdfHash.Sha512);

    /// <summary>SHA-512 HKDF-Expand convenience helper.</summary>
    public static byte[] ExpandSha512(byte[] prk, byte[] info, int length) =>
        Expand(prk, info, length, HkdfHash.Sha512);

    /// <summary>SHA-512 HKDF convenience helper.</summary>
    public static byte[] DeriveSha512(byte[] salt, byte[] ikm, byte[] info, int length) =>
        Derive(salt, ikm, info, length, HkdfHash.Sha512);

    private static HMAC CreateHmac(HkdfHash hash, byte[] key) =>
        hash switch
        {
            HkdfHash.Sha256 => new HMACSHA256(key),
            HkdfHash.Sha512 => new HMACSHA512(key),
            _ => throw new ArgumentOutOfRangeException(nameof(hash), "Unsupported HKDF hash algorithm."),
        };

    private static int HashLength(HkdfHash hash) =>
        hash switch
        {
            HkdfHash.Sha256 => 32,
            HkdfHash.Sha512 => 64,
            _ => throw new ArgumentOutOfRangeException(nameof(hash), "Unsupported HKDF hash algorithm."),
        };
}
