using System.Numerics;
using System.Security.Cryptography;
using System.Text;

namespace CodingAdventures.HashFunctions;

/// <summary>Common interface for fixed-output non-cryptographic hash functions.</summary>
public interface IHashFunction
{
    /// <summary>Hash the supplied bytes.</summary>
    ulong Hash(byte[] data);

    /// <summary>Number of output bits produced by the hash.</summary>
    int OutputBits { get; }
}

public readonly struct Fnv1a32 : IHashFunction
{
    public ulong Hash(byte[] data) => HashFunctions.Fnv1a32(data);
    public int OutputBits => 32;
}

public readonly struct Fnv1a64 : IHashFunction
{
    public ulong Hash(byte[] data) => HashFunctions.Fnv1a64(data);
    public int OutputBits => 64;
}

public readonly struct Djb2Hash : IHashFunction
{
    public ulong Hash(byte[] data) => HashFunctions.Djb2(data);
    public int OutputBits => 64;
}

public readonly struct PolynomialRollingHash(ulong @base, ulong modulus) : IHashFunction
{
    public PolynomialRollingHash()
        : this(HashFunctions.PolynomialRollingDefaultBase, HashFunctions.PolynomialRollingDefaultModulus)
    {
    }

    public ulong Base { get; } = @base;
    public ulong Modulus { get; } = modulus;
    public ulong Hash(byte[] data) => HashFunctions.PolynomialRolling(data, Base, Modulus);
    public int OutputBits => 64;
}

public readonly struct Murmur3_32(uint seed) : IHashFunction
{
    public Murmur3_32()
        : this(0)
    {
    }

    public uint Seed { get; } = seed;
    public ulong Hash(byte[] data) => HashFunctions.Murmur3_32(data, Seed);
    public int OutputBits => 32;
}

public readonly struct SipHash24(byte[] key) : IHashFunction
{
    public byte[] Key { get; } = key.ToArray();
    public ulong Hash(byte[] data) => HashFunctions.SipHash24(data, Key);
    public int OutputBits => 64;
}

/// <summary>Non-cryptographic hash functions and quality analysis helpers.</summary>
public static class HashFunctions
{
    public const uint Fnv32OffsetBasis = 0x811C9DC5;
    public const uint Fnv32Prime = 0x01000193;
    public const ulong Fnv64OffsetBasis = 0xCBF29CE484222325;
    public const ulong Fnv64Prime = 0x00000100000001B3;
    public const ulong Djb2OffsetBasis = 5381;
    public const ulong PolynomialRollingDefaultBase = 31;
    public const ulong PolynomialRollingDefaultModulus = (1UL << 61) - 1;

    private const uint MurmurC1 = 0xCC9E2D51;
    private const uint MurmurC2 = 0x1B873593;

    private const ulong SipHashV0 = 0x736F6D6570736575;
    private const ulong SipHashV1 = 0x646F72616E646F6D;
    private const ulong SipHashV2 = 0x6C7967656E657261;
    private const ulong SipHashV3 = 0x7465646279746573;

    public static uint Fnv1a32(string data) => Fnv1a32(ToBytes(data));

    public static uint Fnv1a32(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var hash = Fnv32OffsetBasis;
        foreach (var value in data)
        {
            hash ^= value;
            hash = unchecked(hash * Fnv32Prime);
        }

        return hash;
    }

    public static ulong Fnv1a64(string data) => Fnv1a64(ToBytes(data));

    public static ulong Fnv1a64(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var hash = Fnv64OffsetBasis;
        foreach (var value in data)
        {
            hash ^= value;
            hash = unchecked(hash * Fnv64Prime);
        }

        return hash;
    }

    public static ulong Djb2(string data) => Djb2(ToBytes(data));

    public static ulong Djb2(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var hash = Djb2OffsetBasis;
        foreach (var value in data)
        {
            hash = unchecked((hash << 5) + hash + value);
        }

        return hash;
    }

    public static ulong PolynomialRolling(
        string data,
        ulong @base = PolynomialRollingDefaultBase,
        ulong modulus = PolynomialRollingDefaultModulus) =>
        PolynomialRolling(ToBytes(data), @base, modulus);

    public static ulong PolynomialRolling(
        byte[] data,
        ulong @base = PolynomialRollingDefaultBase,
        ulong modulus = PolynomialRollingDefaultModulus)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (modulus == 0)
        {
            throw new ArgumentOutOfRangeException(nameof(modulus), "Modulus must be positive.");
        }

        UInt128 hash = 0;
        var baseValue = (UInt128)@base;
        var modulusValue = (UInt128)modulus;
        foreach (var value in data)
        {
            hash = ((hash * baseValue) + value) % modulusValue;
        }

        return (ulong)hash;
    }

    public static uint Murmur3_32(string data, uint seed = 0) => Murmur3_32(ToBytes(data), seed);

    public static uint Murmur3_32(byte[] data, uint seed = 0)
    {
        ArgumentNullException.ThrowIfNull(data);
        var hash = seed;
        var offset = 0;

        while (offset + 4 <= data.Length)
        {
            var k = (uint)(data[offset]
                | (data[offset + 1] << 8)
                | (data[offset + 2] << 16)
                | (data[offset + 3] << 24));
            k = unchecked(k * MurmurC1);
            k = BitOperations.RotateLeft(k, 15);
            k = unchecked(k * MurmurC2);

            hash ^= k;
            hash = BitOperations.RotateLeft(hash, 13);
            hash = unchecked((hash * 5) + 0xE6546B64);
            offset += 4;
        }

        var tail = 0u;
        var remaining = data.Length - offset;
        if (remaining >= 3)
        {
            tail ^= (uint)data[offset + 2] << 16;
        }

        if (remaining >= 2)
        {
            tail ^= (uint)data[offset + 1] << 8;
        }

        if (remaining >= 1)
        {
            tail ^= data[offset];
            tail = unchecked(tail * MurmurC1);
            tail = BitOperations.RotateLeft(tail, 15);
            tail = unchecked(tail * MurmurC2);
            hash ^= tail;
        }

        hash ^= (uint)data.Length;
        return Fmix32(hash);
    }

    public static uint HashStringFnv1a32(string data) => Fnv1a32(data);

    public static ulong HashStringSipHash(string data, byte[] key) => SipHash24(ToBytes(data), key);

    public static ulong SipHash24(byte[] data, byte[] key)
    {
        ArgumentNullException.ThrowIfNull(data);
        ValidateKey(key, 16, nameof(key), "SipHash key");

        var k0 = ReadUInt64LittleEndian(key, 0);
        var k1 = ReadUInt64LittleEndian(key, 8);
        var v0 = SipHashV0 ^ k0;
        var v1 = SipHashV1 ^ k1;
        var v2 = SipHashV2 ^ k0;
        var v3 = SipHashV3 ^ k1;

        var offset = 0;
        while (offset + 8 <= data.Length)
        {
            var m = ReadUInt64LittleEndian(data, offset);
            v3 ^= m;
            SipRound(ref v0, ref v1, ref v2, ref v3);
            SipRound(ref v0, ref v1, ref v2, ref v3);
            v0 ^= m;
            offset += 8;
        }

        var last = ((ulong)data.Length & 0xff) << 56;
        for (var index = 0; index < data.Length - offset; index++)
        {
            last |= (ulong)data[offset + index] << (index * 8);
        }

        v3 ^= last;
        SipRound(ref v0, ref v1, ref v2, ref v3);
        SipRound(ref v0, ref v1, ref v2, ref v3);
        v0 ^= last;

        v2 ^= 0xff;
        SipRound(ref v0, ref v1, ref v2, ref v3);
        SipRound(ref v0, ref v1, ref v2, ref v3);
        SipRound(ref v0, ref v1, ref v2, ref v3);
        SipRound(ref v0, ref v1, ref v2, ref v3);

        return v0 ^ v1 ^ v2 ^ v3;
    }

    public static double AvalancheScore(Func<byte[], ulong> hashFunction, int outputBits, int sampleSize = 1000)
    {
        ArgumentNullException.ThrowIfNull(hashFunction);
        if (outputBits is < 1 or > 64)
        {
            throw new ArgumentOutOfRangeException(nameof(outputBits), "Output bits must be in 1..=64.");
        }

        if (sampleSize <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(sampleSize), "Sample size must be positive.");
        }

        var input = new byte[8];
        ulong totalBitFlips = 0;
        ulong totalTrials = 0;

        for (var sample = 0; sample < sampleSize; sample++)
        {
            RandomNumberGenerator.Fill(input);
            var h1 = hashFunction(input);

            for (var bitPosition = 0; bitPosition < input.Length * 8; bitPosition++)
            {
                var flipped = input.ToArray();
                flipped[bitPosition / 8] ^= (byte)(1 << (bitPosition % 8));
                var h2 = hashFunction(flipped);
                totalBitFlips += (ulong)BitOperations.PopCount(h1 ^ h2);
                totalTrials += (ulong)outputBits;
            }
        }

        return (double)totalBitFlips / totalTrials;
    }

    public static double DistributionTest(Func<byte[], ulong> hashFunction, IEnumerable<byte[]> inputs, int numBuckets)
    {
        ArgumentNullException.ThrowIfNull(hashFunction);
        ArgumentNullException.ThrowIfNull(inputs);
        if (numBuckets <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(numBuckets), "Number of buckets must be positive.");
        }

        var counts = new ulong[numBuckets];
        ulong total = 0;
        foreach (var input in inputs)
        {
            var bucket = (int)(hashFunction(input) % (ulong)numBuckets);
            counts[bucket]++;
            total++;
        }

        if (total == 0)
        {
            throw new ArgumentException("Inputs must not be empty.", nameof(inputs));
        }

        var expected = (double)total / numBuckets;
        var chiSquared = 0.0;
        foreach (var count in counts)
        {
            var delta = count - expected;
            chiSquared += delta * delta / expected;
        }

        return chiSquared;
    }

    private static uint Fmix32(uint hash)
    {
        hash ^= hash >> 16;
        hash = unchecked(hash * 0x85EBCA6B);
        hash ^= hash >> 13;
        hash = unchecked(hash * 0xC2B2AE35);
        hash ^= hash >> 16;
        return hash;
    }

    private static void SipRound(ref ulong v0, ref ulong v1, ref ulong v2, ref ulong v3)
    {
        v0 = unchecked(v0 + v1);
        v1 = BitOperations.RotateLeft(v1, 13);
        v1 ^= v0;
        v0 = BitOperations.RotateLeft(v0, 32);

        v2 = unchecked(v2 + v3);
        v3 = BitOperations.RotateLeft(v3, 16);
        v3 ^= v2;

        v0 = unchecked(v0 + v3);
        v3 = BitOperations.RotateLeft(v3, 21);
        v3 ^= v0;

        v2 = unchecked(v2 + v1);
        v1 = BitOperations.RotateLeft(v1, 17);
        v1 ^= v2;
        v2 = BitOperations.RotateLeft(v2, 32);
    }

    private static ulong ReadUInt64LittleEndian(byte[] data, int offset)
    {
        var value = 0UL;
        for (var index = 0; index < 8; index++)
        {
            value |= (ulong)data[offset + index] << (index * 8);
        }

        return value;
    }

    private static byte[] ToBytes(string data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return Encoding.UTF8.GetBytes(data);
    }

    private static void ValidateKey(byte[] key, int length, string paramName, string label)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (key.Length != length)
        {
            throw new ArgumentException($"{label} must be {length} bytes.", paramName);
        }
    }
}
