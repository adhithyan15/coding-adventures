namespace CodingAdventures.Aes;

/// <summary>
/// AES single-block encryption and decryption helpers implemented from FIPS 197.
/// </summary>
public static class AesBlock
{
    private const int BlockSizeBytes = 16;
    private static readonly byte[] Rcon = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36, 0x6c, 0xd8, 0xab, 0x4d];
    private static readonly byte[] SBox;
    private static readonly byte[] InvSBox;

    static AesBlock()
    {
        (SBox, InvSBox) = BuildSBoxes();
    }

    /// <summary>Encrypt one 16-byte block with a 16-, 24-, or 32-byte AES key.</summary>
    public static byte[] EncryptBlock(byte[] block, byte[] key)
    {
        ValidateBlock(block);
        ValidateKey(key);

        var expandedKey = ExpandKey(key);
        var rounds = expandedKey.Length / BlockSizeBytes - 1;
        var state = block.ToArray();

        AddRoundKey(state, expandedKey, 0);

        for (var round = 1; round < rounds; round++)
        {
            SubBytes(state);
            ShiftRows(state);
            MixColumns(state);
            AddRoundKey(state, expandedKey, round);
        }

        SubBytes(state);
        ShiftRows(state);
        AddRoundKey(state, expandedKey, rounds);
        return state;
    }

    /// <summary>Decrypt one 16-byte block with a 16-, 24-, or 32-byte AES key.</summary>
    public static byte[] DecryptBlock(byte[] block, byte[] key)
    {
        ValidateBlock(block);
        ValidateKey(key);

        var expandedKey = ExpandKey(key);
        var rounds = expandedKey.Length / BlockSizeBytes - 1;
        var state = block.ToArray();

        AddRoundKey(state, expandedKey, rounds);

        for (var round = rounds - 1; round >= 1; round--)
        {
            InvShiftRows(state);
            InvSubBytes(state);
            AddRoundKey(state, expandedKey, round);
            InvMixColumns(state);
        }

        InvShiftRows(state);
        InvSubBytes(state);
        AddRoundKey(state, expandedKey, 0);
        return state;
    }

    private static byte[] ExpandKey(byte[] key)
    {
        var keyWords = key.Length / 4;
        var rounds = keyWords switch
        {
            4 => 10,
            6 => 12,
            8 => 14,
            _ => throw new ArgumentException("AES key must be 16, 24, or 32 bytes.", nameof(key)),
        };

        var expanded = new byte[BlockSizeBytes * (rounds + 1)];
        Buffer.BlockCopy(key, 0, expanded, 0, key.Length);

        Span<byte> temp = stackalloc byte[4];
        var bytesGenerated = key.Length;
        var rconIndex = 1;

        while (bytesGenerated < expanded.Length)
        {
            for (var index = 0; index < 4; index++)
            {
                temp[index] = expanded[bytesGenerated - 4 + index];
            }

            if (bytesGenerated % key.Length == 0)
            {
                RotWord(temp);
                SubWord(temp);
                temp[0] ^= Rcon[rconIndex++];
            }
            else if (key.Length == 32 && bytesGenerated % key.Length == 16)
            {
                SubWord(temp);
            }

            for (var index = 0; index < 4; index++)
            {
                expanded[bytesGenerated] = (byte)(expanded[bytesGenerated - key.Length] ^ temp[index]);
                bytesGenerated++;
            }
        }

        return expanded;
    }

    private static void AddRoundKey(byte[] state, byte[] expandedKey, int round)
    {
        var offset = round * BlockSizeBytes;
        for (var index = 0; index < BlockSizeBytes; index++)
        {
            state[index] ^= expandedKey[offset + index];
        }
    }

    private static void SubBytes(byte[] state)
    {
        for (var index = 0; index < state.Length; index++)
        {
            state[index] = SBox[state[index]];
        }
    }

    private static void InvSubBytes(byte[] state)
    {
        for (var index = 0; index < state.Length; index++)
        {
            state[index] = InvSBox[state[index]];
        }
    }

    private static void ShiftRows(byte[] state)
    {
        var copy = state.ToArray();
        for (var row = 0; row < 4; row++)
        {
            for (var column = 0; column < 4; column++)
            {
                state[row + 4 * column] = copy[row + 4 * ((column + row) & 3)];
            }
        }
    }

    private static void InvShiftRows(byte[] state)
    {
        var copy = state.ToArray();
        for (var row = 0; row < 4; row++)
        {
            for (var column = 0; column < 4; column++)
            {
                state[row + 4 * column] = copy[row + 4 * ((column - row + 4) & 3)];
            }
        }
    }

    private static void MixColumns(byte[] state)
    {
        for (var column = 0; column < 4; column++)
        {
            var offset = 4 * column;
            var s0 = state[offset];
            var s1 = state[offset + 1];
            var s2 = state[offset + 2];
            var s3 = state[offset + 3];

            state[offset] = (byte)(Multiply(0x02, s0) ^ Multiply(0x03, s1) ^ s2 ^ s3);
            state[offset + 1] = (byte)(s0 ^ Multiply(0x02, s1) ^ Multiply(0x03, s2) ^ s3);
            state[offset + 2] = (byte)(s0 ^ s1 ^ Multiply(0x02, s2) ^ Multiply(0x03, s3));
            state[offset + 3] = (byte)(Multiply(0x03, s0) ^ s1 ^ s2 ^ Multiply(0x02, s3));
        }
    }

    private static void InvMixColumns(byte[] state)
    {
        for (var column = 0; column < 4; column++)
        {
            var offset = 4 * column;
            var s0 = state[offset];
            var s1 = state[offset + 1];
            var s2 = state[offset + 2];
            var s3 = state[offset + 3];

            state[offset] = (byte)(Multiply(0x0e, s0) ^ Multiply(0x0b, s1) ^ Multiply(0x0d, s2) ^ Multiply(0x09, s3));
            state[offset + 1] = (byte)(Multiply(0x09, s0) ^ Multiply(0x0e, s1) ^ Multiply(0x0b, s2) ^ Multiply(0x0d, s3));
            state[offset + 2] = (byte)(Multiply(0x0d, s0) ^ Multiply(0x09, s1) ^ Multiply(0x0e, s2) ^ Multiply(0x0b, s3));
            state[offset + 3] = (byte)(Multiply(0x0b, s0) ^ Multiply(0x0d, s1) ^ Multiply(0x09, s2) ^ Multiply(0x0e, s3));
        }
    }

    private static void RotWord(Span<byte> word)
    {
        var first = word[0];
        word[0] = word[1];
        word[1] = word[2];
        word[2] = word[3];
        word[3] = first;
    }

    private static void SubWord(Span<byte> word)
    {
        for (var index = 0; index < word.Length; index++)
        {
            word[index] = SBox[word[index]];
        }
    }

    private static (byte[] SBox, byte[] InvSBox) BuildSBoxes()
    {
        var sbox = new byte[256];
        var invSbox = new byte[256];

        for (var value = 0; value < 256; value++)
        {
            var substituted = AffineTransform(MultiplicativeInverse((byte)value));
            sbox[value] = substituted;
            invSbox[substituted] = (byte)value;
        }

        return (sbox, invSbox);
    }

    private static byte AffineTransform(byte value) =>
        (byte)(value ^ RotateLeft(value, 1) ^ RotateLeft(value, 2) ^ RotateLeft(value, 3) ^ RotateLeft(value, 4) ^ 0x63);

    private static byte MultiplicativeInverse(byte value)
    {
        if (value == 0)
        {
            return 0;
        }

        var result = (byte)1;
        for (var index = 0; index < 254; index++)
        {
            result = Multiply(result, value);
        }

        return result;
    }

    private static byte Multiply(byte left, byte right)
    {
        var result = 0;
        var a = left;
        var b = right;

        for (var index = 0; index < 8; index++)
        {
            if ((b & 1) != 0)
            {
                result ^= a;
            }

            var carry = (a & 0x80) != 0;
            a = (byte)(a << 1);
            if (carry)
            {
                a ^= 0x1b;
            }

            b >>= 1;
        }

        return (byte)result;
    }

    private static byte RotateLeft(byte value, int count) =>
        (byte)(((value << count) | (value >> (8 - count))) & 0xff);

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
