using System.Numerics;

namespace CodingAdventures.RankStatistics;

/// <summary>A closed interval whose endpoints are sample order statistics.</summary>
public readonly record struct MedianInterval(double Low, double High);

/// <summary>Sample median and exact distribution-free interval operations.</summary>
public static class RankStatistics
{
    /// <summary>Returns the binary64 sample median without changing the input.</summary>
    public static double Median(IReadOnlyList<double> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (values.Count == 0)
            throw new ArgumentException("values must not be empty", nameof(values));

        var sorted = CopyFinite(values);
        Array.Sort(sorted);
        var middle = sorted.Length / 2;
        if ((sorted.Length & 1) != 0)
            return sorted[middle];

        var low = sorted[middle - 1];
        var high = sorted[middle];
        var sum = low + high;
        // Sum first so subnormal ties round at the final divide. If same-sign
        // large values overflow the sum, halving first keeps the result finite.
        return double.IsFinite(sum) ? sum / 2.0 : low / 2.0 + high / 2.0;
    }

    /// <summary>
    /// Returns the largest exact order-statistic confidence interval for the
    /// population median, or null if no rank meets the requested coverage.
    /// </summary>
    public static MedianInterval? MedianInterval(
        IReadOnlyList<double> values,
        int confidencePercent)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (values.Count == 0)
            throw new ArgumentException("values must not be empty", nameof(values));
        if (confidencePercent is < 1 or > 99)
            throw new ArgumentOutOfRangeException(nameof(confidencePercent), "confidence must be from 1 through 99");

        var sorted = CopyFinite(values);
        Array.Sort(sorted);
        var n = sorted.Length;
        var denominator = BigInteger.One << n;
        var choose = BigInteger.One;
        var cumulative = BigInteger.Zero;
        var largestRank = 0;

        for (var j = 0; j <= n / 2; j++)
        {
            cumulative += choose;
            if (200 * cumulative > (100 - confidencePercent) * denominator)
                break;
            largestRank = j + 1;
            choose = choose * (n - j) / (j + 1);
        }

        return largestRank == 0
            ? null
            : new MedianInterval(sorted[largestRank - 1], sorted[n - largestRank]);
    }

    private static double[] CopyFinite(IReadOnlyList<double> values)
    {
        var sorted = new double[values.Count];
        for (var i = 0; i < sorted.Length; i++)
        {
            var value = values[i];
            if (!double.IsFinite(value))
                throw new ArgumentException("all values must be finite", nameof(values));
            sorted[i] = value;
        }
        return sorted;
    }
}
