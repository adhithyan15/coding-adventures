namespace CodingAdventures.HyperLogLog.Tests;

public sealed class HyperLogLogTests
{
    [Fact]
    public void StartsEmpty()
    {
        var hll = new HyperLogLog();
        Assert.Equal(0, hll.Count());
        Assert.Equal(0, hll.Len());
    }

    [Fact]
    public void IgnoresDuplicatesAndGrowsForUniqueValues()
    {
        var hll = new HyperLogLog();
        for (var i = 0; i < 1000; i++)
        {
            hll.Add("same");
        }

        Assert.True(hll.Count() < 10);

        var spread = new HyperLogLog();
        for (var i = 0; i < 1000; i++)
        {
            spread.Add($"item-{i}");
        }

        Assert.InRange(spread.Count(), 800, 1200);
    }

    [Fact]
    public void MergesSketchesWithSamePrecision()
    {
        var left = new HyperLogLog(10);
        var right = new HyperLogLog(10);
        for (var i = 0; i < 200; i++)
        {
            left.Add($"left-{i}");
            right.Add($"right-{i}");
        }

        var merged = left.Merge(right);
        Assert.True(merged.Count() >= left.Count());
        Assert.True(merged.Count() >= right.Count());
    }

    [Fact]
    public void RejectsPrecisionMismatches()
    {
        var left = new HyperLogLog(10);
        var right = new HyperLogLog(14);
        Assert.Null(left.TryMerge(right));
        Assert.Throws<HyperLogLogError>(() => left.Merge(right));
    }

    [Fact]
    public void ExposesHelperMath()
    {
        Assert.Equal(12288, HyperLogLog.MemoryBytes(14));
        Assert.Equal(14, HyperLogLog.OptimalPrecision(0.01));
        Assert.True(HyperLogLog.ErrorRateForPrecision(14) > 0.008);
    }
}
