using System.Buffers.Binary;
using System.Numerics;
using CodingAdventures.Blake2b;
using Blake2bAlgorithm = CodingAdventures.Blake2b.Blake2b;

namespace CodingAdventures.Argon2id;

/// <summary>Optional Argon2id inputs from RFC 9106.</summary>
public sealed class Argon2idOptions
{
    /// <summary>Optional secret key K.</summary>
    public byte[] Key { get; init; } = [];

    /// <summary>Optional associated data X.</summary>
    public byte[] AssociatedData { get; init; } = [];

    /// <summary>Argon2 version. Only RFC 9106 version 0x13 is supported.</summary>
    public uint Version { get; init; } = Argon2id.Version;
}

/// <summary>Pure C# Argon2id password hashing as specified by RFC 9106.</summary>
public static class Argon2id
{
    /// <summary>Argon2 version 1.3.</summary>
    public const uint Version = 0x13;

    private const ulong Mask32 = 0xffff_ffffUL;
    private const int BlockSize = 1024;
    private const int BlockWords = BlockSize / sizeof(ulong);
    private const int SyncPoints = 4;
    private const uint TypeId = 2;

    /// <summary>Compute an Argon2id tag.</summary>
    public static byte[] Derive(
        byte[] password,
        byte[] salt,
        int timeCost,
        int memoryCost,
        int parallelism,
        int tagLength,
        Argon2idOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(password);
        ArgumentNullException.ThrowIfNull(salt);
        options ??= new Argon2idOptions();
        ArgumentNullException.ThrowIfNull(options.Key);
        ArgumentNullException.ThrowIfNull(options.AssociatedData);
        Validate(salt, timeCost, memoryCost, parallelism, tagLength, options.Version);

        var segmentLength = memoryCost / (SyncPoints * parallelism);
        var adjustedMemoryCost = segmentLength * SyncPoints * parallelism;
        var laneLength = adjustedMemoryCost / parallelism;
        var initialHash = InitialHash(
            password,
            salt,
            timeCost,
            memoryCost,
            parallelism,
            tagLength,
            options);

        var memory = new ulong[parallelism][][];
        for (var lane = 0; lane < parallelism; lane++)
        {
            memory[lane] = new ulong[laneLength][];
            for (var column = 0; column < laneLength; column++)
            {
                memory[lane][column] = new ulong[BlockWords];
            }

            memory[lane][0] = BytesToBlock(Blake2bLong(BlockSize, Concat(initialHash, UInt32Bytes(0), UInt32Bytes((uint)lane))));
            memory[lane][1] = BytesToBlock(Blake2bLong(BlockSize, Concat(initialHash, UInt32Bytes(1), UInt32Bytes((uint)lane))));
        }

        for (var pass = 0; pass < timeCost; pass++)
        {
            for (var slice = 0; slice < SyncPoints; slice++)
            {
                for (var lane = 0; lane < parallelism; lane++)
                {
                    FillSegment(
                        memory,
                        pass,
                        lane,
                        slice,
                        laneLength,
                        segmentLength,
                        parallelism,
                        adjustedMemoryCost,
                        timeCost);
                }
            }
        }

        var finalBlock = memory[0][laneLength - 1].ToArray();
        for (var lane = 1; lane < parallelism; lane++)
        {
            for (var word = 0; word < BlockWords; word++)
            {
                finalBlock[word] ^= memory[lane][laneLength - 1][word];
            }
        }

        return Blake2bLong(tagLength, BlockToBytes(finalBlock));
    }

    /// <summary>Compute an Argon2id tag and return lowercase hexadecimal.</summary>
    public static string DeriveHex(
        byte[] password,
        byte[] salt,
        int timeCost,
        int memoryCost,
        int parallelism,
        int tagLength,
        Argon2idOptions? options = null) =>
        Convert.ToHexString(Derive(password, salt, timeCost, memoryCost, parallelism, tagLength, options))
            .ToLowerInvariant();

    private static void Validate(
        byte[] salt,
        int timeCost,
        int memoryCost,
        int parallelism,
        int tagLength,
        uint version)
    {
        if (salt.Length < 8)
        {
            throw new ArgumentException($"Salt must be at least 8 bytes, got {salt.Length}.", nameof(salt));
        }

        if (tagLength < 4)
        {
            throw new ArgumentOutOfRangeException(nameof(tagLength), $"Tag length must be at least 4 bytes, got {tagLength}.");
        }

        if (parallelism is < 1 or > 0x00ff_ffff)
        {
            throw new ArgumentOutOfRangeException(nameof(parallelism), "Parallelism must be in [1, 2^24-1].");
        }

        if (memoryCost < 8 * parallelism)
        {
            throw new ArgumentOutOfRangeException(
                nameof(memoryCost),
                $"Memory cost must be at least 8*parallelism ({8 * parallelism}), got {memoryCost}.");
        }

        if (timeCost < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(timeCost), "Time cost must be at least 1.");
        }

        if (version != Version)
        {
            throw new ArgumentOutOfRangeException(nameof(version), $"Only Argon2 v1.3 (0x13) is supported; got 0x{version:x2}.");
        }
    }

    private static byte[] InitialHash(
        byte[] password,
        byte[] salt,
        int timeCost,
        int memoryCost,
        int parallelism,
        int tagLength,
        Argon2idOptions options)
    {
        var input = Concat(
            UInt32Bytes((uint)parallelism),
            UInt32Bytes((uint)tagLength),
            UInt32Bytes((uint)memoryCost),
            UInt32Bytes((uint)timeCost),
            UInt32Bytes(options.Version),
            UInt32Bytes(TypeId),
            UInt32Bytes((uint)password.Length),
            password,
            UInt32Bytes((uint)salt.Length),
            salt,
            UInt32Bytes((uint)options.Key.Length),
            options.Key,
            UInt32Bytes((uint)options.AssociatedData.Length),
            options.AssociatedData);
        return Hash(input, 64);
    }

    private static void FillSegment(
        ulong[][][] memory,
        int pass,
        int lane,
        int slice,
        int laneLength,
        int segmentLength,
        int parallelism,
        int adjustedMemoryCost,
        int timeCost)
    {
        var dataIndependent = pass == 0 && slice < 2;
        var inputBlock = new ulong[BlockWords];
        var addressBlock = new ulong[BlockWords];
        var zeroBlock = new ulong[BlockWords];
        inputBlock[0] = (ulong)pass;
        inputBlock[1] = (ulong)lane;
        inputBlock[2] = (ulong)slice;
        inputBlock[3] = (ulong)adjustedMemoryCost;
        inputBlock[4] = (ulong)timeCost;
        inputBlock[5] = TypeId;

        void NextAddresses()
        {
            inputBlock[6] = unchecked(inputBlock[6] + 1);
            var intermediate = Compress(zeroBlock, inputBlock);
            addressBlock = Compress(zeroBlock, intermediate);
        }

        var startingColumn = pass == 0 && slice == 0 ? 2 : 0;
        if (dataIndependent && startingColumn != 0)
        {
            NextAddresses();
        }

        for (var index = startingColumn; index < segmentLength; index++)
        {
            if (dataIndependent
                && index % BlockWords == 0
                && !(pass == 0 && slice == 0 && index == 2))
            {
                NextAddresses();
            }

            var column = slice * segmentLength + index;
            var previousColumn = column > 0 ? column - 1 : laneLength - 1;
            var previousBlock = memory[lane][previousColumn];
            var pseudoRandom = dataIndependent
                ? addressBlock[index % BlockWords]
                : previousBlock[0];
            var j1 = pseudoRandom & Mask32;
            var j2 = pseudoRandom >> 32;
            var referenceLane = pass == 0 && slice == 0 ? lane : (int)(j2 % (ulong)parallelism);
            var referenceColumn = IndexAlpha(
                j1,
                pass,
                slice,
                index,
                referenceLane == lane,
                laneLength,
                segmentLength);

            var newBlock = Compress(previousBlock, memory[referenceLane][referenceColumn]);
            if (pass == 0)
            {
                memory[lane][column] = newBlock;
            }
            else
            {
                for (var word = 0; word < BlockWords; word++)
                {
                    memory[lane][column][word] ^= newBlock[word];
                }
            }
        }
    }

    private static int IndexAlpha(
        ulong j1,
        int pass,
        int slice,
        int index,
        bool sameLane,
        int laneLength,
        int segmentLength)
    {
        ulong window;
        var start = 0;

        if (pass == 0)
        {
            if (slice == 0)
            {
                window = (ulong)(index - 1);
            }
            else
            {
                window = sameLane
                    ? (ulong)(slice * segmentLength + index - 1)
                    : (ulong)(slice * segmentLength - (index == 0 ? 1 : 0));
            }
        }
        else
        {
            window = sameLane
                ? (ulong)(laneLength - segmentLength + index - 1)
                : (ulong)(laneLength - segmentLength - (index == 0 ? 1 : 0));
            start = ((slice + 1) * segmentLength) % laneLength;
        }

        var x = unchecked(j1 * j1) >> 32;
        var y = unchecked(window * x) >> 32;
        var relative = window - 1 - y;
        return (int)(((ulong)start + relative) % (ulong)laneLength);
    }

    private static ulong[] Compress(ulong[] x, ulong[] y)
    {
        var r = new ulong[BlockWords];
        for (var index = 0; index < BlockWords; index++)
        {
            r[index] = x[index] ^ y[index];
        }

        var q = r.ToArray();
        for (var rowIndex = 0; rowIndex < 8; rowIndex++)
        {
            var row = new ulong[16];
            Array.Copy(q, rowIndex * 16, row, 0, row.Length);
            Permute(row);
            Array.Copy(row, 0, q, rowIndex * 16, row.Length);
        }

        for (var columnIndex = 0; columnIndex < 8; columnIndex++)
        {
            var column = new ulong[16];
            for (var rowIndex = 0; rowIndex < 8; rowIndex++)
            {
                column[2 * rowIndex] = q[rowIndex * 16 + 2 * columnIndex];
                column[2 * rowIndex + 1] = q[rowIndex * 16 + 2 * columnIndex + 1];
            }

            Permute(column);
            for (var rowIndex = 0; rowIndex < 8; rowIndex++)
            {
                q[rowIndex * 16 + 2 * columnIndex] = column[2 * rowIndex];
                q[rowIndex * 16 + 2 * columnIndex + 1] = column[2 * rowIndex + 1];
            }
        }

        for (var index = 0; index < BlockWords; index++)
        {
            r[index] ^= q[index];
        }

        return r;
    }

    private static void Permute(ulong[] values)
    {
        Mix(values, 0, 4, 8, 12);
        Mix(values, 1, 5, 9, 13);
        Mix(values, 2, 6, 10, 14);
        Mix(values, 3, 7, 11, 15);
        Mix(values, 0, 5, 10, 15);
        Mix(values, 1, 6, 11, 12);
        Mix(values, 2, 7, 8, 13);
        Mix(values, 3, 4, 9, 14);
    }

    private static void Mix(ulong[] values, int a, int b, int c, int d)
    {
        var va = values[a];
        var vb = values[b];
        var vc = values[c];
        var vd = values[d];

        va = AddWithProduct(va, vb);
        vd = BitOperations.RotateRight(vd ^ va, 32);
        vc = AddWithProduct(vc, vd);
        vb = BitOperations.RotateRight(vb ^ vc, 24);
        va = AddWithProduct(va, vb);
        vd = BitOperations.RotateRight(vd ^ va, 16);
        vc = AddWithProduct(vc, vd);
        vb = BitOperations.RotateRight(vb ^ vc, 63);

        values[a] = va;
        values[b] = vb;
        values[c] = vc;
        values[d] = vd;
    }

    private static ulong AddWithProduct(ulong left, ulong right) =>
        unchecked(left + right + 2 * (left & Mask32) * (right & Mask32));

    private static byte[] Blake2bLong(int outputLength, byte[] input)
    {
        var prefix = UInt32Bytes((uint)outputLength);
        if (outputLength <= 64)
        {
            return Hash(Concat(prefix, input), outputLength);
        }

        var rounds = (outputLength + 31) / 32 - 2;
        var value = Hash(Concat(prefix, input), 64);
        var output = new List<byte>(outputLength);
        output.AddRange(value[..32]);
        for (var round = 0; round < rounds - 1; round++)
        {
            value = Hash(value, 64);
            output.AddRange(value[..32]);
        }

        value = Hash(value, outputLength - 32 * rounds);
        output.AddRange(value);
        return output.ToArray();
    }

    private static byte[] Hash(byte[] input, int digestSize) =>
        Blake2bAlgorithm.Hash(input, Blake2bOptions.Default.WithDigestSize(digestSize));

    private static byte[] BlockToBytes(ulong[] block)
    {
        var result = new byte[BlockSize];
        for (var index = 0; index < BlockWords; index++)
        {
            BinaryPrimitives.WriteUInt64LittleEndian(result.AsSpan(index * sizeof(ulong), sizeof(ulong)), block[index]);
        }

        return result;
    }

    private static ulong[] BytesToBlock(byte[] data)
    {
        if (data.Length != BlockSize)
        {
            throw new ArgumentException($"Block must be {BlockSize} bytes, got {data.Length}.", nameof(data));
        }

        var result = new ulong[BlockWords];
        for (var index = 0; index < BlockWords; index++)
        {
            result[index] = BinaryPrimitives.ReadUInt64LittleEndian(data.AsSpan(index * sizeof(ulong), sizeof(ulong)));
        }

        return result;
    }

    private static byte[] UInt32Bytes(uint value)
    {
        var result = new byte[sizeof(uint)];
        BinaryPrimitives.WriteUInt32LittleEndian(result, value);
        return result;
    }

    private static byte[] Concat(params byte[][] parts)
    {
        var result = new byte[parts.Sum(part => part.Length)];
        var offset = 0;
        foreach (var part in parts)
        {
            Buffer.BlockCopy(part, 0, result, offset, part.Length);
            offset += part.Length;
        }

        return result;
    }
}
