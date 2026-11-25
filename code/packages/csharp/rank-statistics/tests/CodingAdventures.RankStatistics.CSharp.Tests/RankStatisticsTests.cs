using System.Text.Json;
using CodingAdventures.RankStatistics;

namespace CodingAdventures.RankStatistics.CSharp.Tests;

public sealed class RankStatisticsTests
{
    [Fact]
    public void MedianCopiesInputAndHandlesOddEvenAndFloatingBoundaries()
    {
        var source = new[] { 9.0, 1.0, 5.0, 3.0 };
        Assert.Equal(4.0, RankStatistics.Median(source));
        Assert.Equal(new[] { 9.0, 1.0, 5.0, 3.0 }, source);
        Assert.Equal(7.0, RankStatistics.Median(new[] { 7.0 }));

        var denorm = double.Epsilon;
        Assert.Equal(2 * denorm, RankStatistics.Median(new[] { denorm, 2 * denorm }));
        Assert.Equal(4 * denorm, RankStatistics.Median(new[] { 3 * denorm, 4 * denorm }));
        Assert.Equal(-2 * denorm, RankStatistics.Median(new[] { -2 * denorm, -denorm }));
        Assert.Equal(-2 * denorm, RankStatistics.Median(new[] { -3 * denorm, -2 * denorm }));
        var minNormal = Math.ScaleB(1.0, -1022);
        Assert.Equal(minNormal, RankStatistics.Median(new[] { Math.BitDecrement(minNormal), minNormal }));
        Assert.Equal(double.MaxValue, RankStatistics.Median(new[] { double.MaxValue, double.MaxValue }));
        Assert.Equal(0.0, RankStatistics.Median(new[] { -double.MaxValue, double.MaxValue }));
        Assert.Equal(double.MaxValue * 0.75, RankStatistics.Median(new[] { double.MaxValue / 2.0, double.MaxValue }));
    }

    [Fact]
    public void RejectsEmptyAndNonFiniteSamplesAndInvalidConfidence()
    {
        Assert.Throws<ArgumentException>(() => RankStatistics.Median(Array.Empty<double>()));
        Assert.Throws<ArgumentException>(() => RankStatistics.Median(new[] { 1.0, double.NaN }));
        Assert.Throws<ArgumentException>(() => RankStatistics.MedianInterval(Array.Empty<double>(), 95));
        Assert.Throws<ArgumentException>(() => RankStatistics.MedianInterval(new[] { 1.0, double.PositiveInfinity }, 95));
        Assert.Throws<ArgumentOutOfRangeException>(() => RankStatistics.MedianInterval(new[] { 1.0 }, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => RankStatistics.MedianInterval(new[] { 1.0 }, 100));
    }

    [Fact]
    public void ExactCutoffMatchesIndependentBoundedSignOracle()
    {
        for (var n = 1; n <= 10; n++)
        {
            var values = Enumerable.Range(0, n).Select(i => (double)i).ToArray();
            for (var confidence = 1; confidence <= 99; confidence++)
            {
                var rank = OracleRank(n, confidence);
                var interval = RankStatistics.MedianInterval(values, confidence);
                if (rank == 0)
                    Assert.Null(interval);
                else
                    Assert.Equal(new MedianInterval(rank - 1, n - rank), interval);
            }
        }
    }

    [Fact]
    public void WideCutoffsMatchFrozenReferenceVectors()
    {
        var cases = new (int N, int Confidence, int Low, int High)[]
        {
            (64, 95, 23, 40), (65, 99, 21, 43), (127, 95, 51, 75),
            (128, 95, 52, 75), (256, 99, 106, 149), (1000, 95, 468, 531),
            (10000, 50, 4965, 5034), (10000, 95, 4901, 5098),
            (10000, 99, 4870, 5129),
        };
        foreach (var item in cases)
        {
            var values = Enumerable.Range(0, item.N).Select(i => (double)i).ToArray();
            Assert.Equal(new MedianInterval(item.Low, item.High), RankStatistics.MedianInterval(values, item.Confidence));
        }
    }

    [Fact]
    public void SharedFixtureMatchesCanonicalAndAnalyzedRatios()
    {
        var localPath = Path.Combine(AppContext.BaseDirectory, "testdata", "microbenchmark-pairs-v1.json");
        var local = File.ReadAllBytes(localPath);
        var root = new DirectoryInfo(AppContext.BaseDirectory);
        string? canonicalPath = null;
        for (var current = root; current is not null; current = current.Parent)
        {
            var candidate = Path.Combine(current.FullName, "code", "specs", "fixtures", "microbenchmark-pairs-v1.json");
            if (File.Exists(candidate)) { canonicalPath = candidate; break; }
        }
        if (canonicalPath is not null)
            Assert.Equal(File.ReadAllBytes(canonicalPath), local);

        using var document = JsonDocument.Parse(local);
        var fixture = document.RootElement;
        var confidence = fixture.GetProperty("confidence_percent").GetInt32();
        foreach (var item in fixture.GetProperty("cases").EnumerateArray())
        {
            var baseline = item.GetProperty("baseline_ns").GetInt64();
            var ratios = item.GetProperty("candidate_ns").EnumerateArray()
                .Select(candidate => (double)candidate.GetInt64() / (double)baseline).ToArray();
            Assert.Equal(item.GetProperty("ratio").GetDouble(), RankStatistics.Median(ratios));
            var interval = RankStatistics.MedianInterval(ratios, confidence);
            if (item.GetProperty("interval").ValueKind == JsonValueKind.Null)
                Assert.Null(interval);
            else
            {
                Assert.NotNull(interval);
                var expected = item.GetProperty("interval").EnumerateArray().Select(x => x.GetDouble()).ToArray();
                Assert.Equal(expected[0], interval!.Value.Low);
                Assert.Equal(expected[1], interval.Value.High);
            }
        }
    }

    [Fact]
    public void GeneratedSamplesPreservePermutationAndEndpointProperties()
    {
        var random = new Random(0x4d454449);
        for (var attempt = 0; attempt < 512; attempt++)
        {
            var values = Enumerable.Range(0, random.Next(1, 48)).Select(_ => random.Next(-100_000, 100_001) / 17.0).ToArray();
            var reversed = values.Reverse().ToArray();
            Assert.Equal(RankStatistics.Median(values), RankStatistics.Median(reversed));
            var interval = RankStatistics.MedianInterval(values, random.Next(1, 100));
            if (interval is not null)
            {
                Assert.Contains(interval.Value.Low, values);
                Assert.Contains(interval.Value.High, values);
            }
        }
    }

    private static int OracleRank(int n, int confidence)
    {
        var assignments = 1 << n;
        var best = 0;
        for (var rank = 1; rank <= n; rank++)
        {
            var below = 0;
            for (var mask = 0; mask < assignments; mask++)
                if (System.Numerics.BitOperations.PopCount((uint)mask) <= rank - 1) below++;
            if (200 * below <= (100 - confidence) * assignments) best = rank;
        }
        return best;
    }
}
