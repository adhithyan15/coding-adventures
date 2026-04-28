using System.Text;

namespace CodingAdventures.BloomFilter;

/// <summary>
/// Space-efficient probabilistic set membership filter.
/// </summary>
public sealed class BloomFilter<T>
    where T : notnull
{
    private const int Fnv32OffsetBasis = unchecked((int)0x811C9DC5);
    private const int Fnv32Prime = 0x01000193;
    private const int MaxBits = 1 << 30;

    private readonly int _bitCount;
    private readonly int _hashCount;
    private readonly int _expectedItems;
    private readonly byte[] _bits;
    private int _bitsSet;
    private int _count;

    /// <summary>
    /// Create a filter sized for the requested expected count and false-positive rate.
    /// </summary>
    public BloomFilter(int expectedItems, double falsePositiveRate)
    {
        if (expectedItems <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(expectedItems), "Expected items must be positive.");
        }

        if (falsePositiveRate <= 0.0 || falsePositiveRate >= 1.0)
        {
            throw new ArgumentOutOfRangeException(nameof(falsePositiveRate), "False-positive rate must be in (0, 1).");
        }

        var bitCount = OptimalM(expectedItems, falsePositiveRate);
        if (bitCount > MaxBits)
        {
            throw new ArgumentOutOfRangeException(nameof(expectedItems), "Required bit array exceeds the maximum size.");
        }

        _bitCount = checked((int)bitCount);
        _hashCount = OptimalK(_bitCount, expectedItems);
        _expectedItems = expectedItems;
        _bits = new byte[(_bitCount + 7) / 8];
    }

    /// <summary>
    /// Create a filter with explicit bit and hash counts.
    /// </summary>
    public BloomFilter(int bitCount, int hashCount, bool explicitParameters)
    {
        if (bitCount <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(bitCount), "Bit count must be positive.");
        }

        if (hashCount <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(hashCount), "Hash count must be positive.");
        }

        if (bitCount > MaxBits)
        {
            throw new ArgumentOutOfRangeException(nameof(bitCount), "Bit count exceeds the maximum size.");
        }

        _bitCount = bitCount;
        _hashCount = hashCount;
        _expectedItems = 0;
        _bits = new byte[(bitCount + 7) / 8];
    }

    /// <summary>Total number of bits in the filter.</summary>
    public int BitCount => _bitCount;

    /// <summary>Number of hash probes per element.</summary>
    public int HashCount => _hashCount;

    /// <summary>Number of bits currently set.</summary>
    public int BitsSet => _bitsSet;

    /// <summary>Number of elements added.</summary>
    public int Count => _count;

    /// <summary>Number of elements added.</summary>
    public int Size => _count;

    /// <summary>Memory used by the packed bit array.</summary>
    public int SizeBytes => _bits.Length;

    /// <summary>Current fraction of set bits.</summary>
    public double FillRatio => (double)_bitsSet / _bitCount;

    /// <summary>Estimated false-positive rate based on the current fill ratio.</summary>
    public double EstimatedFalsePositiveRate => _bitsSet == 0 ? 0.0 : Math.Pow(FillRatio, _hashCount);

    /// <summary>True when more elements were added than the auto-sizing target.</summary>
    public bool IsOverCapacity => _expectedItems != 0 && _count > _expectedItems;

    /// <summary>Add an element to the filter.</summary>
    public void Add(T element)
    {
        ArgumentNullException.ThrowIfNull(element);

        foreach (var index in HashIndices(element))
        {
            var byteIndex = index >>> 3;
            var bitMask = 1 << (index & 7);
            if ((_bits[byteIndex] & bitMask) == 0)
            {
                _bits[byteIndex] = (byte)(_bits[byteIndex] | bitMask);
                _bitsSet++;
            }
        }

        _count++;
    }

    /// <summary>Return true if the element is probably present.</summary>
    public bool Contains(T element)
    {
        ArgumentNullException.ThrowIfNull(element);

        foreach (var index in HashIndices(element))
        {
            var byteIndex = index >>> 3;
            var bitMask = 1 << (index & 7);
            if ((_bits[byteIndex] & bitMask) == 0)
            {
                return false;
            }
        }

        return true;
    }

    private int[] HashIndices(T element)
    {
        var text = element.ToString() ?? string.Empty;
        var raw = Encoding.UTF8.GetBytes(text);
        var h1 = (uint)Fmix32(Fnv1a32(raw));
        var h2 = (uint)Fmix32(Djb2_32(raw)) | 1U;

        var indices = new int[_hashCount];
        for (var i = 0; i < _hashCount; i++)
        {
            indices[i] = (int)(((ulong)h1 + (ulong)i * h2) % (ulong)_bitCount);
        }

        return indices;
    }

    private static int Fnv1a32(byte[] data)
    {
        unchecked
        {
            var hash = Fnv32OffsetBasis;
            foreach (var value in data)
            {
                hash ^= value;
                hash *= Fnv32Prime;
            }

            return hash;
        }
    }

    private static int Djb2_32(byte[] data)
    {
        unchecked
        {
            var hash = 5381UL;
            foreach (var value in data)
            {
                hash = (hash << 5) + hash + value;
            }

            return (int)((hash ^ (hash >> 32)) & 0xFFFFFFFFUL);
        }
    }

    private static int Fmix32(int value)
    {
        unchecked
        {
            var hash = (uint)value;
            hash ^= hash >> 16;
            hash *= 0x85EBCA6BU;
            hash ^= hash >> 13;
            hash *= 0xC2B2AE35U;
            hash ^= hash >> 16;
            return (int)hash;
        }
    }

    /// <summary>Optimal bit count for n elements and false-positive rate p.</summary>
    public static long OptimalM(long n, double p)
    {
        if (n <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(n), "n must be positive.");
        }

        if (p <= 0.0 || p >= 1.0)
        {
            throw new ArgumentOutOfRangeException(nameof(p), "p must be in (0, 1).");
        }

        var ln2 = Math.Log(2.0);
        return (long)Math.Ceiling(-n * Math.Log(p) / (ln2 * ln2));
    }

    /// <summary>Optimal hash count for m bits and n expected elements.</summary>
    public static int OptimalK(long m, long n)
    {
        if (n <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(n), "n must be positive.");
        }

        return Math.Max(1, (int)Math.Round((double)m / n * Math.Log(2.0)));
    }

    /// <summary>Estimate capacity for a memory budget and false-positive rate.</summary>
    public static long CapacityForMemory(long memoryBytes, double p)
    {
        if (memoryBytes <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(memoryBytes), "Memory budget must be positive.");
        }

        if (p <= 0.0 || p >= 1.0)
        {
            throw new ArgumentOutOfRangeException(nameof(p), "p must be in (0, 1).");
        }

        var bits = memoryBytes * 8.0;
        var ln2 = Math.Log(2.0);
        return (long)(-bits * ln2 * ln2 / Math.Log(p));
    }

    /// <summary>
    /// Return a compact summary of sizing and current saturation.
    /// </summary>
    public override string ToString() =>
        $"BloomFilter(m={_bitCount}, k={_hashCount}, bitsSet={_bitsSet}/{_bitCount} ({FillRatio * 100.0:F2}%), ~fp={EstimatedFalsePositiveRate * 100.0:F4}%)";
}
