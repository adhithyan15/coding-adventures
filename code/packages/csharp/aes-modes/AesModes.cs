using System.Security.Cryptography;
using CodingAdventures.Aes;

namespace CodingAdventures.AesModes;

/// <summary>
/// AES modes of operation helpers for educational use.
/// </summary>
public static class AesModes
{
    /// <summary>AES block size in bytes.</summary>
    public const int BlockSizeBytes = 16;

    private const int GcmNonceSizeBytes = 12;
    private const int GcmTagSizeBytes = 16;

    /// <summary>Pad data to a 16-byte boundary using PKCS#7.</summary>
    public static byte[] Pkcs7Pad(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var padLength = BlockSizeBytes - (data.Length % BlockSizeBytes);
        var result = new byte[data.Length + padLength];
        Buffer.BlockCopy(data, 0, result, 0, data.Length);
        Array.Fill(result, (byte)padLength, data.Length, padLength);
        return result;
    }

    /// <summary>Remove PKCS#7 padding from block-aligned data.</summary>
    public static byte[] Pkcs7Unpad(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length == 0 || data.Length % BlockSizeBytes != 0)
        {
            throw new ArgumentException("Padded data must be a positive multiple of 16 bytes.", nameof(data));
        }

        var padLength = data[^1];
        if (padLength is < 1 or > BlockSizeBytes)
        {
            throw new ArgumentException("Invalid PKCS#7 padding.", nameof(data));
        }

        var diff = 0;
        for (var index = data.Length - padLength; index < data.Length; index++)
        {
            diff |= data[index] ^ padLength;
        }

        if (diff != 0)
        {
            throw new ArgumentException("Invalid PKCS#7 padding.", nameof(data));
        }

        var result = new byte[data.Length - padLength];
        Buffer.BlockCopy(data, 0, result, 0, result.Length);
        return result;
    }

    /// <summary>XOR two byte arrays of equal length.</summary>
    public static byte[] XorBytes(byte[] left, byte[] right)
    {
        ArgumentNullException.ThrowIfNull(left);
        ArgumentNullException.ThrowIfNull(right);
        if (left.Length != right.Length)
        {
            throw new ArgumentException("Byte arrays must have the same length.", nameof(right));
        }

        var result = new byte[left.Length];
        for (var index = 0; index < result.Length; index++)
        {
            result[index] = (byte)(left[index] ^ right[index]);
        }

        return result;
    }

    /// <summary>Encrypt with AES-ECB. ECB is insecure and included for parity and education.</summary>
    public static byte[] EcbEncrypt(byte[] plaintext, byte[] key)
    {
        ArgumentNullException.ThrowIfNull(plaintext);
        ValidateKey(key);

        var padded = Pkcs7Pad(plaintext);
        var result = new byte[padded.Length];
        for (var offset = 0; offset < padded.Length; offset += BlockSizeBytes)
        {
            var encrypted = AesBlock.EncryptBlock(ReadBlock(padded, offset), key);
            Buffer.BlockCopy(encrypted, 0, result, offset, BlockSizeBytes);
        }

        return result;
    }

    /// <summary>Decrypt with AES-ECB and remove PKCS#7 padding.</summary>
    public static byte[] EcbDecrypt(byte[] ciphertext, byte[] key)
    {
        ValidateCiphertext(ciphertext, nameof(ciphertext), "ECB ciphertext");
        ValidateKey(key);

        var padded = new byte[ciphertext.Length];
        for (var offset = 0; offset < ciphertext.Length; offset += BlockSizeBytes)
        {
            var decrypted = AesBlock.DecryptBlock(ReadBlock(ciphertext, offset), key);
            Buffer.BlockCopy(decrypted, 0, padded, offset, BlockSizeBytes);
        }

        return Pkcs7Unpad(padded);
    }

    /// <summary>Encrypt with AES-CBC using a 16-byte IV and PKCS#7 padding.</summary>
    public static byte[] CbcEncrypt(byte[] plaintext, byte[] key, byte[] iv)
    {
        ArgumentNullException.ThrowIfNull(plaintext);
        ValidateKey(key);
        ValidateLength(iv, BlockSizeBytes, nameof(iv), "CBC IV");

        var padded = Pkcs7Pad(plaintext);
        var result = new byte[padded.Length];
        var previous = iv.ToArray();

        for (var offset = 0; offset < padded.Length; offset += BlockSizeBytes)
        {
            var block = XorBytes(ReadBlock(padded, offset), previous);
            var encrypted = AesBlock.EncryptBlock(block, key);
            Buffer.BlockCopy(encrypted, 0, result, offset, BlockSizeBytes);
            previous = encrypted;
        }

        return result;
    }

    /// <summary>Decrypt with AES-CBC using a 16-byte IV and PKCS#7 padding.</summary>
    public static byte[] CbcDecrypt(byte[] ciphertext, byte[] key, byte[] iv)
    {
        ValidateCiphertext(ciphertext, nameof(ciphertext), "CBC ciphertext");
        ValidateKey(key);
        ValidateLength(iv, BlockSizeBytes, nameof(iv), "CBC IV");

        var padded = new byte[ciphertext.Length];
        var previous = iv.ToArray();

        for (var offset = 0; offset < ciphertext.Length; offset += BlockSizeBytes)
        {
            var cipherBlock = ReadBlock(ciphertext, offset);
            var decrypted = AesBlock.DecryptBlock(cipherBlock, key);
            var plainBlock = XorBytes(decrypted, previous);
            Buffer.BlockCopy(plainBlock, 0, padded, offset, BlockSizeBytes);
            previous = cipherBlock;
        }

        return Pkcs7Unpad(padded);
    }

    /// <summary>Encrypt with AES-CTR using a 12-byte nonce and a 32-bit big-endian counter.</summary>
    public static byte[] CtrEncrypt(byte[] plaintext, byte[] key, byte[] nonce)
    {
        ArgumentNullException.ThrowIfNull(plaintext);
        ValidateKey(key);
        ValidateLength(nonce, GcmNonceSizeBytes, nameof(nonce), "CTR nonce");

        var result = new byte[plaintext.Length];
        var counter = 1u;

        for (var offset = 0; offset < plaintext.Length; offset += BlockSizeBytes)
        {
            var keystream = AesBlock.EncryptBlock(BuildCounterBlock(nonce, counter), key);
            var count = Math.Min(BlockSizeBytes, plaintext.Length - offset);
            for (var index = 0; index < count; index++)
            {
                result[offset + index] = (byte)(plaintext[offset + index] ^ keystream[index]);
            }

            counter++;
        }

        return result;
    }

    /// <summary>Decrypt with AES-CTR. CTR decryption is the same operation as encryption.</summary>
    public static byte[] CtrDecrypt(byte[] ciphertext, byte[] key, byte[] nonce) =>
        CtrEncrypt(ciphertext, key, nonce);

    /// <summary>Encrypt and authenticate with AES-GCM using a 12-byte IV.</summary>
    public static (byte[] Ciphertext, byte[] Tag) GcmEncrypt(
        byte[] plaintext,
        byte[] key,
        byte[] iv,
        byte[]? aad = null)
    {
        ArgumentNullException.ThrowIfNull(plaintext);
        ValidateKey(key);
        ValidateLength(iv, GcmNonceSizeBytes, nameof(iv), "GCM IV");

        var ciphertext = new byte[plaintext.Length];
        var tag = new byte[GcmTagSizeBytes];
        using var gcm = new AesGcm(key, GcmTagSizeBytes);
        gcm.Encrypt(iv, plaintext, ciphertext, tag, aad ?? Array.Empty<byte>());
        return (ciphertext, tag);
    }

    /// <summary>Decrypt and authenticate with AES-GCM using a 12-byte IV and 16-byte tag.</summary>
    public static byte[] GcmDecrypt(
        byte[] ciphertext,
        byte[] key,
        byte[] iv,
        byte[]? aad,
        byte[] tag)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);
        ValidateKey(key);
        ValidateLength(iv, GcmNonceSizeBytes, nameof(iv), "GCM IV");
        ValidateLength(tag, GcmTagSizeBytes, nameof(tag), "GCM tag");

        var plaintext = new byte[ciphertext.Length];
        using var gcm = new AesGcm(key, GcmTagSizeBytes);
        gcm.Decrypt(iv, ciphertext, tag, plaintext, aad ?? Array.Empty<byte>());
        return plaintext;
    }

    private static byte[] ReadBlock(byte[] data, int offset)
    {
        var block = new byte[BlockSizeBytes];
        Buffer.BlockCopy(data, offset, block, 0, BlockSizeBytes);
        return block;
    }

    private static byte[] BuildCounterBlock(byte[] nonce, uint counter)
    {
        var block = new byte[BlockSizeBytes];
        Buffer.BlockCopy(nonce, 0, block, 0, nonce.Length);
        block[12] = (byte)(counter >> 24);
        block[13] = (byte)(counter >> 16);
        block[14] = (byte)(counter >> 8);
        block[15] = (byte)counter;
        return block;
    }

    private static void ValidateCiphertext(byte[] ciphertext, string paramName, string label)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);
        if (ciphertext.Length == 0 || ciphertext.Length % BlockSizeBytes != 0)
        {
            throw new ArgumentException($"{label} must be a positive multiple of 16 bytes.", paramName);
        }
    }

    private static void ValidateLength(byte[] value, int length, string paramName, string label)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (value.Length != length)
        {
            throw new ArgumentException($"{label} must be {length} bytes.", paramName);
        }
    }

    private static void ValidateKey(byte[] key)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (key.Length is not (16 or 24 or 32))
        {
            throw new ArgumentException("AES key must be 16, 24, or 32 bytes.", nameof(key));
        }
    }
}
