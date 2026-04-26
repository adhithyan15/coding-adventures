using System.Security.Cryptography;

namespace CodingAdventures.Aes;

/// <summary>
/// AES single-block encryption and decryption helpers.
/// </summary>
public static class AesBlock
{
    private const int BlockSizeBytes = 16;

    /// <summary>Encrypt one 16-byte block with a 16-, 24-, or 32-byte AES key.</summary>
    public static byte[] EncryptBlock(byte[] block, byte[] key)
    {
        ValidateBlock(block);
        ValidateKey(key);
        using var aes = CreateAes(key);
        using var encryptor = aes.CreateEncryptor();
        return encryptor.TransformFinalBlock(block, 0, block.Length);
    }

    /// <summary>Decrypt one 16-byte block with a 16-, 24-, or 32-byte AES key.</summary>
    public static byte[] DecryptBlock(byte[] block, byte[] key)
    {
        ValidateBlock(block);
        ValidateKey(key);
        using var aes = CreateAes(key);
        using var decryptor = aes.CreateDecryptor();
        return decryptor.TransformFinalBlock(block, 0, block.Length);
    }

    private static System.Security.Cryptography.Aes CreateAes(byte[] key)
    {
        var aes = System.Security.Cryptography.Aes.Create();
        aes.Mode = CipherMode.ECB;
        aes.Padding = PaddingMode.None;
        aes.Key = key.ToArray();
        return aes;
    }

    private static void ValidateBlock(byte[] block)
    {
        ArgumentNullException.ThrowIfNull(block);
        if (block.Length != BlockSizeBytes)
        {
            throw new ArgumentException("AES block must be exactly 16 bytes.", nameof(block));
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
