using System.Buffers.Binary;
using System.Numerics;
using System.Security.Cryptography;

namespace CodingAdventures.ChaCha20Poly1305;

/// <summary>The result of ChaCha20-Poly1305 authenticated encryption.</summary>
public sealed record AeadResult(byte[] Ciphertext, byte[] Tag);

/// <summary>
/// Pure C# ChaCha20-Poly1305 authenticated encryption from RFC 8439.
/// </summary>
/// <remarks>
/// This educational implementation uses <see cref="BigInteger"/> for
/// Poly1305 arithmetic. It avoids data-dependent branches in tag comparison,
/// but <see cref="BigInteger"/> is not guaranteed to execute in constant time.
/// </remarks>
public static class ChaCha20Poly1305
{
    /// <summary>The required ChaCha20 key size in bytes.</summary>
    public const int KeyLength = 32;

    /// <summary>The required IETF ChaCha20 nonce size in bytes.</summary>
    public const int NonceLength = 12;

    /// <summary>The Poly1305 authentication tag size in bytes.</summary>
    public const int TagLength = 16;

    private const int BlockLength = 64;
    private static readonly BigInteger Poly1305Prime = (BigInteger.One << 130) - 5;
    private static readonly BigInteger TagMask = (BigInteger.One << 128) - 1;

    /// <summary>Generate one 64-byte ChaCha20 keystream block.</summary>
    public static byte[] ChaCha20Block(byte[] key, uint counter, byte[] nonce)
    {
        ValidateLength(key, KeyLength, nameof(key), "Key");
        ValidateLength(nonce, NonceLength, nameof(nonce), "Nonce");

        uint[] state =
        [
            0x61707865u, 0x3320646eu, 0x79622d32u, 0x6b206574u,
            ReadUInt32(key, 0), ReadUInt32(key, 4), ReadUInt32(key, 8), ReadUInt32(key, 12),
            ReadUInt32(key, 16), ReadUInt32(key, 20), ReadUInt32(key, 24), ReadUInt32(key, 28),
            counter, ReadUInt32(nonce, 0), ReadUInt32(nonce, 4), ReadUInt32(nonce, 8),
        ];

        var initial = state.ToArray();
        for (var round = 0; round < 10; round++)
        {
            QuarterRound(state, 0, 4, 8, 12);
            QuarterRound(state, 1, 5, 9, 13);
            QuarterRound(state, 2, 6, 10, 14);
            QuarterRound(state, 3, 7, 11, 15);
            QuarterRound(state, 0, 5, 10, 15);
            QuarterRound(state, 1, 6, 11, 12);
            QuarterRound(state, 2, 7, 8, 13);
            QuarterRound(state, 3, 4, 9, 14);
        }

        var block = new byte[BlockLength];
        for (var index = 0; index < state.Length; index++)
        {
            state[index] = unchecked(state[index] + initial[index]);
            BinaryPrimitives.WriteUInt32LittleEndian(block.AsSpan(index * 4, 4), state[index]);
        }

        return block;
    }

    /// <summary>
    /// Encrypt or decrypt bytes with the ChaCha20 stream cipher.
    /// </summary>
    public static byte[] ChaCha20Encrypt(
        byte[] data,
        byte[] key,
        byte[] nonce,
        uint counter = 0)
    {
        ArgumentNullException.ThrowIfNull(data);
        ValidateLength(key, KeyLength, nameof(key), "Key");
        ValidateLength(nonce, NonceLength, nameof(nonce), "Nonce");

        var result = new byte[data.Length];
        var offset = 0;
        var currentCounter = counter;

        while (offset < data.Length)
        {
            var keystream = ChaCha20Block(key, currentCounter, nonce);
            var chunkLength = Math.Min(BlockLength, data.Length - offset);
            for (var index = 0; index < chunkLength; index++)
            {
                result[offset + index] = (byte)(data[offset + index] ^ keystream[index]);
            }

            offset += chunkLength;
            currentCounter = unchecked(currentCounter + 1);
        }

        return result;
    }

    /// <summary>Compute a 16-byte Poly1305 one-time authenticator.</summary>
    public static byte[] Poly1305Mac(byte[] message, byte[] key)
    {
        ArgumentNullException.ThrowIfNull(message);
        ValidateLength(key, KeyLength, nameof(key), "Poly1305 key");

        var rBytes = key[..16].ToArray();
        rBytes[3] &= 0x0f;
        rBytes[7] &= 0x0f;
        rBytes[11] &= 0x0f;
        rBytes[15] &= 0x0f;
        rBytes[4] &= 0xfc;
        rBytes[8] &= 0xfc;
        rBytes[12] &= 0xfc;

        var r = DecodeLittleEndian(rBytes);
        var s = DecodeLittleEndian(key.AsSpan(16, 16));
        var accumulator = BigInteger.Zero;
        Span<byte> augmented = stackalloc byte[17];

        for (var offset = 0; offset < message.Length; offset += 16)
        {
            var chunkLength = Math.Min(16, message.Length - offset);
            augmented.Clear();
            message.AsSpan(offset, chunkLength).CopyTo(augmented);
            augmented[chunkLength] = 1;
            var block = DecodeLittleEndian(augmented[..(chunkLength + 1)]);
            accumulator = ((accumulator + block) * r) % Poly1305Prime;
        }

        return EncodeLittleEndian((accumulator + s) & TagMask, TagLength);
    }

    /// <summary>Encrypt and authenticate data using RFC 8439 AEAD.</summary>
    public static AeadResult AeadEncrypt(
        byte[] plaintext,
        byte[] key,
        byte[] nonce,
        byte[]? additionalData = null)
    {
        ArgumentNullException.ThrowIfNull(plaintext);
        ValidateLength(key, KeyLength, nameof(key), "Key");
        ValidateLength(nonce, NonceLength, nameof(nonce), "Nonce");
        additionalData ??= [];

        var polyKey = ChaCha20Block(key, 0, nonce)[..KeyLength];
        var ciphertext = ChaCha20Encrypt(plaintext, key, nonce, 1);
        var tag = Poly1305Mac(BuildMacData(additionalData, ciphertext), polyKey);
        return new AeadResult(ciphertext, tag);
    }

    /// <summary>
    /// Authenticate and decrypt RFC 8439 AEAD ciphertext.
    /// </summary>
    /// <exception cref="CryptographicException">Authentication fails.</exception>
    public static byte[] AeadDecrypt(
        byte[] ciphertext,
        byte[] key,
        byte[] nonce,
        byte[]? additionalData,
        byte[] tag)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);
        ValidateLength(key, KeyLength, nameof(key), "Key");
        ValidateLength(nonce, NonceLength, nameof(nonce), "Nonce");
        ValidateLength(tag, TagLength, nameof(tag), "Tag");
        additionalData ??= [];

        var polyKey = ChaCha20Block(key, 0, nonce)[..KeyLength];
        var expectedTag = Poly1305Mac(BuildMacData(additionalData, ciphertext), polyKey);
        if (!ConstantTimeEquals(expectedTag, tag))
        {
            throw new CryptographicException("Authentication failed: tag mismatch.");
        }

        return ChaCha20Encrypt(ciphertext, key, nonce, 1);
    }

    private static void QuarterRound(uint[] state, int a, int b, int c, int d)
    {
        state[a] = unchecked(state[a] + state[b]);
        state[d] = BitOperations.RotateLeft(state[d] ^ state[a], 16);
        state[c] = unchecked(state[c] + state[d]);
        state[b] = BitOperations.RotateLeft(state[b] ^ state[c], 12);
        state[a] = unchecked(state[a] + state[b]);
        state[d] = BitOperations.RotateLeft(state[d] ^ state[a], 8);
        state[c] = unchecked(state[c] + state[d]);
        state[b] = BitOperations.RotateLeft(state[b] ^ state[c], 7);
    }

    private static uint ReadUInt32(byte[] value, int offset) =>
        BinaryPrimitives.ReadUInt32LittleEndian(value.AsSpan(offset, 4));

    private static BigInteger DecodeLittleEndian(ReadOnlySpan<byte> value) =>
        new(value, isUnsigned: true, isBigEndian: false);

    private static byte[] EncodeLittleEndian(BigInteger value, int length)
    {
        var encoded = value.ToByteArray(isUnsigned: true, isBigEndian: false);
        var result = new byte[length];
        encoded.AsSpan(0, Math.Min(encoded.Length, length)).CopyTo(result);
        return result;
    }

    private static byte[] BuildMacData(byte[] additionalData, byte[] ciphertext)
    {
        var aadPadding = PaddingLength(additionalData.Length);
        var ciphertextPadding = PaddingLength(ciphertext.Length);
        var result = new byte[checked(
            additionalData.Length + aadPadding + ciphertext.Length + ciphertextPadding + 16)];
        var offset = 0;

        additionalData.CopyTo(result, offset);
        offset += additionalData.Length + aadPadding;
        ciphertext.CopyTo(result, offset);
        offset += ciphertext.Length + ciphertextPadding;
        BinaryPrimitives.WriteUInt64LittleEndian(result.AsSpan(offset, 8), (ulong)additionalData.Length);
        BinaryPrimitives.WriteUInt64LittleEndian(result.AsSpan(offset + 8, 8), (ulong)ciphertext.Length);
        return result;
    }

    private static int PaddingLength(int length) => (16 - (length % 16)) % 16;

    private static bool ConstantTimeEquals(byte[] left, byte[] right)
    {
        if (left.Length != right.Length)
        {
            return false;
        }

        var difference = 0;
        for (var index = 0; index < left.Length; index++)
        {
            difference |= left[index] ^ right[index];
        }

        return difference == 0;
    }

    private static void ValidateLength(
        byte[]? value,
        int expectedLength,
        string parameterName,
        string displayName)
    {
        ArgumentNullException.ThrowIfNull(value, parameterName);
        if (value.Length != expectedLength)
        {
            throw new ArgumentException(
                $"{displayName} must be {expectedLength} bytes, got {value.Length}.",
                parameterName);
        }
    }
}
