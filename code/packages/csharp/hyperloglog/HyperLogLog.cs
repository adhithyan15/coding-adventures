using System.Numerics;
using System.Text;
using System.Text.Json;

namespace CodingAdventures.HyperLogLog;

public sealed class HyperLogLogError : Exception
{
    public HyperLogLogError(string message)
        : base(message)
    {
    }
}

public sealed class HyperLogLog : IEquatable<HyperLogLog>
{
    private const int DefaultPrecision = 14;
    private const int MinPrecision = 4;
    private const int MaxPrecision = 16;
    private const ulong FnvOffsetBasis = 0xcbf29ce484222325;
    private const ulong FnvPrime = 0x100000001b3;

    private readonly byte[] _registers;
    private readonly int _precisionBits;

    public HyperLogLog(int precision = DefaultPrecision)
    {
        if (precision < MinPrecision || precision > MaxPrecision)
        {
            throw new HyperLogLogError($"precision must be between {MinPrecision} and {MaxPrecision}, got {precision}");
        }

        _precisionBits = precision;
        _registers = new byte[1 << precision];
    }

    public static HyperLogLog WithPrecision(int precision) => new(precision);

    public static HyperLogLog? TryWithPrecision(int precision)
    {
        try
        {
            return new HyperLogLog(precision);
        }
        catch
        {
            return null;
        }
    }

    public HyperLogLog Add(object? element)
    {
        AddBytes(ValueToBytes(element));
        return this;
    }

    public void AddBytes(byte[] bytes)
    {
        var hash = Fmix64(Fnv1a64(bytes));
        var bucket = (int)(hash >> (64 - _precisionBits));
        var remainingBits = 64 - _precisionBits;
        var mask = remainingBits == 64 ? ulong.MaxValue : ((1UL << remainingBits) - 1);
        var remaining = hash & mask;
        var rho = CountLeadingZeros(remaining, remainingBits) + 1;
        if (rho > _registers[bucket])
        {
            _registers[bucket] = (byte)rho;
        }
    }

    public HyperLogLog Clone()
    {
        var clone = new HyperLogLog(_precisionBits);
        Array.Copy(_registers, clone._registers, _registers.Length);
        return clone;
    }

    public bool Equals(HyperLogLog? other)
    {
        if (other is null || _precisionBits != other._precisionBits)
        {
            return false;
        }

        return _registers.AsSpan().SequenceEqual(other._registers);
    }

    public override bool Equals(object? obj) => obj is HyperLogLog other && Equals(other);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(_precisionBits);
        foreach (var register in _registers)
        {
            hash.Add(register);
        }

        return hash.ToHashCode();
    }

    public int Count()
    {
        var registers = NumRegisters();
        var zSum = _registers.Sum(register => Math.Pow(2, -register));
        var alpha = AlphaForRegisters(registers);
        var estimate = alpha * registers * registers / zSum;

        if (estimate <= 2.5 * registers)
        {
            var zeros = _registers.Count(register => register == 0);
            if (zeros > 0)
            {
                estimate = registers * Math.Log(registers / (double)zeros);
            }
        }

        const double Two32 = 4294967296d;
        if (estimate > Two32 / 30)
        {
            var ratio = 1 - (estimate / Two32);
            if (ratio > 0)
            {
                estimate = -Two32 * Math.Log(ratio);
            }
        }

        return Math.Max(0, (int)Math.Round(estimate));
    }

    public HyperLogLog Merge(HyperLogLog other)
    {
        var merged = TryMerge(other);
        return merged ?? throw new HyperLogLogError($"precision mismatch: {_precisionBits} vs {other.Precision()}");
    }

    public HyperLogLog? TryMerge(HyperLogLog other)
    {
        ArgumentNullException.ThrowIfNull(other);
        if (_precisionBits != other._precisionBits)
        {
            return null;
        }

        var merged = new HyperLogLog(_precisionBits);
        for (var i = 0; i < _registers.Length; i++)
        {
            merged._registers[i] = Math.Max(_registers[i], other._registers[i]);
        }

        return merged;
    }

    public int Len() => Count();

    public int Precision() => _precisionBits;

    public int NumRegisters() => _registers.Length;

    public double ErrorRate() => ErrorRateForPrecision(_precisionBits);

    public static double ErrorRateForPrecision(int precision) => 1.04 / Math.Sqrt(1 << precision);

    public static int MemoryBytes(int precision) => ((1 << precision) * 6) / 8;

    public static int OptimalPrecision(double desiredError)
    {
        var minM = Math.Pow(1.04 / desiredError, 2);
        var precision = (int)Math.Ceiling(Math.Log2(minM));
        return Math.Min(MaxPrecision, Math.Max(MinPrecision, precision));
    }

    public override string ToString() =>
        $"HyperLogLog(precision={_precisionBits}, registers={NumRegisters()}, error_rate={(ErrorRate() * 100):F2}%)";

    private static byte[] ValueToBytes(object? value) =>
        value switch
        {
            byte[] bytes => [.. bytes],
            string text => Encoding.UTF8.GetBytes(text),
            null => Encoding.UTF8.GetBytes("null"),
            bool or byte or sbyte or short or ushort or int or uint or long or ulong or float or double or decimal or char => Encoding.UTF8.GetBytes(value.ToString()!),
            _ => Encoding.UTF8.GetBytes(JsonSerializer.Serialize(value))
        };

    private static ulong Fnv1a64(IEnumerable<byte> bytes)
    {
        var hash = FnvOffsetBasis;
        foreach (var value in bytes)
        {
            hash ^= value;
            hash *= FnvPrime;
        }

        return hash;
    }

    private static ulong Fmix64(ulong value)
    {
        value ^= value >> 33;
        value *= 0xff51afd7ed558ccd;
        value ^= value >> 33;
        value *= 0xc4ceb9fe1a85ec53;
        value ^= value >> 33;
        return value;
    }

    private static int CountLeadingZeros(ulong value, int bitWidth)
    {
        if (bitWidth <= 0)
        {
            return 0;
        }

        if (value == 0)
        {
            return bitWidth;
        }

        return BitOperations.LeadingZeroCount(value) - (64 - bitWidth);
    }

    private static double AlphaForRegisters(int registers) =>
        registers switch
        {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1 + (1.079 / registers))
        };
}
