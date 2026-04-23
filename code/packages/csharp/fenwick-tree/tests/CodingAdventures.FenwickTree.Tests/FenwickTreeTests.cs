using System;
using System.Collections.Generic;
using System.Linq;
using FenwickTreeType = CodingAdventures.FenwickTree.FenwickTree;

namespace CodingAdventures.FenwickTree.Tests;

public sealed class FenwickTreeTests
{
    private const double Epsilon = 1e-9;

    [Fact]
    public void ConstructionValidatesSizeAndTracksLength()
    {
        var empty = new FenwickTreeType(0);
        Assert.Equal(0, empty.Length);
        Assert.True(empty.IsEmpty);
        Assert.Equal(0.0, empty.PrefixSum(0));

        var sized = new FenwickTreeType(5);
        Assert.Equal(5, sized.Length);
        Assert.False(sized.IsEmpty);

        Assert.Throws<FenwickError>(() => new FenwickTreeType(-1));
    }

    [Fact]
    public void FromListBuildsExpectedPrefixTotals()
    {
        var tree = FenwickTreeType.FromList([3.0, 2.0, 1.0, 7.0, 4.0]);

        AssertClose(tree.PrefixSum(1), 3.0);
        AssertClose(tree.PrefixSum(2), 5.0);
        AssertClose(tree.PrefixSum(3), 6.0);
        AssertClose(tree.PrefixSum(4), 13.0);
        AssertClose(tree.PrefixSum(5), 17.0);
    }

    [Fact]
    public void PrefixRangeAndPointQueriesMatchExpectedValues()
    {
        var tree = FenwickTreeType.FromList([3.0, 2.0, 1.0, 7.0, 4.0]);

        AssertClose(tree.PrefixSum(0), 0.0);
        AssertClose(tree.RangeSum(1, 5), 17.0);
        AssertClose(tree.RangeSum(2, 4), 10.0);
        AssertClose(tree.PointQuery(4), 7.0);
    }

    [Fact]
    public void UpdateAppliesPositiveAndNegativeDeltas()
    {
        var tree = FenwickTreeType.FromList([3.0, 2.0, 1.0, 7.0, 4.0]);

        tree.Update(3, 5.0);
        AssertClose(tree.PointQuery(3), 6.0);
        AssertClose(tree.PrefixSum(3), 11.0);

        tree.Update(2, -1.0);
        AssertClose(tree.PointQuery(2), 1.0);
        AssertClose(tree.PrefixSum(3), 10.0);
    }

    [Fact]
    public void UpdateFromIndexOnePropagatesAcrossPowerOfTwoParents()
    {
        var tree = FenwickTreeType.FromList(new double[8]);

        tree.Update(1, 10.0);

        for (var index = 1; index <= 8; index++)
        {
            AssertClose(tree.PrefixSum(index), 10.0);
        }
    }

    [Fact]
    public void QueryOperationsValidateBounds()
    {
        var tree = FenwickTreeType.FromList([1.0, 2.0, 3.0]);

        Assert.Throws<FenwickIndexOutOfRangeError>(() => tree.PrefixSum(-1));
        Assert.Throws<FenwickIndexOutOfRangeError>(() => tree.PrefixSum(4));
        Assert.Throws<FenwickIndexOutOfRangeError>(() => tree.Update(0, 1.0));
        Assert.Throws<FenwickIndexOutOfRangeError>(() => tree.RangeSum(0, 2));
        Assert.Throws<FenwickIndexOutOfRangeError>(() => tree.PointQuery(4));
        Assert.Throws<FenwickError>(() => tree.RangeSum(3, 1));
    }

    [Fact]
    public void FindKthMatchesDocumentedExamplesAndValidationRules()
    {
        var tree = FenwickTreeType.FromList([1.0, 2.0, 3.0, 4.0, 5.0]);

        Assert.Equal(1, tree.FindKth(1.0));
        Assert.Equal(2, tree.FindKth(2.0));
        Assert.Equal(2, tree.FindKth(3.0));
        Assert.Equal(3, tree.FindKth(4.0));
        Assert.Equal(4, tree.FindKth(10.0));
        Assert.Equal(5, tree.FindKth(11.0));

        Assert.Throws<FenwickError>(() => tree.FindKth(0.0));
        Assert.Throws<FenwickError>(() => tree.FindKth(100.0));
        Assert.Throws<FenwickEmptyTreeError>(() => new FenwickTreeType(0).FindKth(1.0));
    }

    [Fact]
    public void PrefixAndRangeQueriesMatchBruteForceAcrossRandomArrays()
    {
        var seed = 1337;

        static int Next(ref int state)
        {
            state = (int)(((long)state * 1103515245 + 12345) & 0x7fffffff);
            return state;
        }

        for (var run = 0; run < 120; run++)
        {
            var n = (Next(ref seed) % 30) + 1;
            var values = Enumerable.Range(0, n)
                .Select(_ => (double)((Next(ref seed) % 101) - 50))
                .ToArray();
            var tree = FenwickTreeType.FromList(values);

            for (var index = 1; index <= n; index++)
            {
                AssertClose(tree.PrefixSum(index), BrutePrefix(values, index));
            }

            for (var left = 1; left <= n; left++)
            {
                for (var right = left; right <= n; right++)
                {
                    AssertClose(tree.RangeSum(left, right), BruteRange(values, left, right));
                }
            }
        }
    }

    [Fact]
    public void TreeStaysConsistentUnderInterleavedUpdatesAndQueries()
    {
        var seed = 99u;

        static uint Next(ref uint state)
        {
            state = state * 1664525u + 1013904223u;
            return state;
        }

        const int Size = 60;
        var values = Enumerable.Range(0, Size)
            .Select(_ => (double)((Next(ref seed) % 20) + 1))
            .ToArray();
        var tree = FenwickTreeType.FromList(values);

        for (var iteration = 0; iteration < 1200; iteration++)
        {
            if (Next(ref seed) % 10 < 4)
            {
                var left = (int)(Next(ref seed) % Size) + 1;
                var right = left + (int)(Next(ref seed) % (Size - left + 1));
                AssertClose(tree.RangeSum(left, right), BruteRange(values, left, right));
            }
            else
            {
                var index = (int)(Next(ref seed) % Size) + 1;
                var delta = (double)((int)(Next(ref seed) % 41) - 20);
                values[index - 1] += delta;
                tree.Update(index, delta);
            }
        }
    }

    [Fact]
    public void ToStringRendersLogicalShape()
    {
        var tree = FenwickTreeType.FromList([1.0, 2.0, 3.0]);
        Assert.Equal("FenwickTree(n=3, bit=[1, 3, 3])", tree.ToString());
    }

    private static void AssertClose(double actual, double expected)
    {
        Assert.True(
            Math.Abs(actual - expected) < Epsilon,
            $"Expected {expected}, got {actual}");
    }

    private static double BrutePrefix(IReadOnlyList<double> values, int index) =>
        values.Take(index).Sum();

    private static double BruteRange(IReadOnlyList<double> values, int left, int right) =>
        values.Skip(left - 1).Take(right - left + 1).Sum();
}
