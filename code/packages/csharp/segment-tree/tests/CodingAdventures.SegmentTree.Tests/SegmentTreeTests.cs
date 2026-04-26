using CodingAdventures.SegmentTree;

namespace CodingAdventures.SegmentTree.Tests;

public sealed class SegmentTreeTests
{
    [Fact]
    public void EmptyTreeHasMetadataAndRejectsOperations()
    {
        var tree = SegmentTree<int>.SumTree(Array.Empty<int>());

        Assert.Equal(0, tree.Size);
        Assert.Equal(0, tree.Count);
        Assert.True(tree.IsEmpty);
        Assert.Empty(tree.ToList());
        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Query(0, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Update(0, 1));
    }

    [Fact]
    public void SingleElementTreesQueryAndUpdate()
    {
        var sum = SegmentTree<int>.SumTree(new[] { 42 });
        var min = SegmentTree<int>.MinTree(new[] { -7 });
        var max = SegmentTree<int>.MaxTree(new[] { 99 });

        Assert.Equal(42, sum.Query(0, 0));
        Assert.Equal(-7, min.Query(0, 0));
        Assert.Equal(99, max.Query(0, 0));
        sum.Update(0, 55);
        Assert.Equal(55, sum.Query(0, 0));
    }

    [Fact]
    public void SumTreeMatchesSpecExampleAndUpdates()
    {
        var values = new[] { 2, 1, 5, 3, 4 };
        var tree = SegmentTree<int>.SumTree(values);

        Assert.Equal(15, tree.Query(0, 4));
        Assert.Equal(9, tree.Query(1, 3));
        Assert.Equal(3, tree.Query(0, 1));
        Assert.Equal(7, tree.Query(3, 4));
        Assert.Equal(5, tree.Query(2, 2));

        tree.Update(2, 7);

        Assert.Equal(11, tree.Query(1, 3));
        Assert.Equal(17, tree.Query(0, 4));
        Assert.Equal(new[] { 2, 1, 7, 3, 4 }, tree.ToList());
    }

    [Fact]
    public void SumTreeAllRangesMatchBruteForce()
    {
        var values = new[] { -3, 1, -4, 1, 5, -9, 2, 6 };
        var tree = SegmentTree<int>.SumTree(values);

        AssertAllRanges(values, tree, BruteSum);
    }

    [Fact]
    public void MinTreeAllRangesAndUpdatesMatchBruteForce()
    {
        var values = new[] { 5, 3, 7, 1, 9, 2 };
        var tree = SegmentTree<int>.MinTree(values);

        AssertAllRanges(values, tree, BruteMin);
        values[3] = 10;
        tree.Update(3, 10);
        Assert.Equal(2, tree.Query(0, 5));
        AssertAllRanges(values, tree, BruteMin);
    }

    [Fact]
    public void MaxTreeAllRangesAndUpdatesMatchBruteForce()
    {
        var values = new[] { 3, -1, 4, 1, 5, 9, 2, 6 };
        var tree = SegmentTree<int>.MaxTree(values);

        AssertAllRanges(values, tree, BruteMax);
        values[2] = 100;
        tree.Update(2, 100);
        Assert.Equal(100, tree.Query(0, 7));
        Assert.Equal(100, tree.Query(1, 3));
        AssertAllRanges(values, tree, BruteMax);
    }

    [Fact]
    public void GcdTreeAllRangesMatchBruteForce()
    {
        var values = new[] { 12, 8, 6, 4, 9 };
        var tree = SegmentTree<int>.GcdTree(values);

        Assert.Equal(2, tree.Query(0, 2));
        Assert.Equal(1, tree.Query(1, 4));
        Assert.Equal(4, tree.Query(0, 1));
        AssertAllRanges(values, tree, BruteGcd);
    }

    [Fact]
    public void NonPowerOfTwoAndMultipleUpdatesStayConsistent()
    {
        var values = new[] { 1, 2, 3, 4, 5, 6, 7 };
        var tree = SegmentTree<int>.SumTree(values);

        Assert.Equal(28, tree.Query(0, 6));
        Assert.Equal(9, tree.Query(1, 3));
        values[0] = 10;
        values[6] = 20;
        values[2] = 0;
        tree.Update(0, 10);
        tree.Update(6, 20);
        tree.Update(2, 0);
        Assert.Equal(values, tree.ToList());
        AssertAllRanges(values, tree, BruteSum);
    }

    [Fact]
    public void InvalidRangesAndIndicesThrow()
    {
        var tree = SegmentTree<int>.SumTree(new[] { 1, 2, 3 });

        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Query(2, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Query(-1, 2));
        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Query(0, 3));
        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Update(-1, 5));
        Assert.Throws<ArgumentOutOfRangeException>(() => tree.Update(3, 5));
    }

    [Fact]
    public void CustomCombineSupportsProductAndBitwiseOr()
    {
        var product = new SegmentTree<int>(new[] { 2, 3, 4, 5 }, static (a, b) => a * b, 1);
        Assert.Equal(120, product.Query(0, 3));
        Assert.Equal(24, product.Query(0, 2));
        Assert.Equal(20, product.Query(2, 3));

        var bitwiseOr = new SegmentTree<int>(new[] { 0b0001, 0b0010, 0b0100, 0b1000 }, static (a, b) => a | b, 0);
        Assert.Equal(0b1111, bitwiseOr.Query(0, 3));
        Assert.Equal(0b0011, bitwiseOr.Query(0, 1));
        Assert.Equal(0b0110, bitwiseOr.Query(1, 2));
    }

    [Fact]
    public void RandomStressMatchesReferenceArray()
    {
        var random = new Random(12345);
        var values = Enumerable.Range(0, 200).Select(_ => random.Next(1000) - 500).ToArray();
        var sum = SegmentTree<int>.SumTree(values);
        var min = SegmentTree<int>.MinTree(values);
        var max = SegmentTree<int>.MaxTree(values);

        for (var i = 0; i < 50; i++)
        {
            var left = random.Next(values.Length);
            var right = left + random.Next(values.Length - left);
            Assert.Equal(BruteSum(values, left, right), sum.Query(left, right));
            Assert.Equal(BruteMin(values, left, right), min.Query(left, right));
            Assert.Equal(BruteMax(values, left, right), max.Query(left, right));
        }

        for (var i = 0; i < 200; i++)
        {
            var index = random.Next(values.Length);
            var value = random.Next(1000) - 500;
            values[index] = value;
            sum.Update(index, value);
            min.Update(index, value);
            max.Update(index, value);

            var left = random.Next(values.Length);
            var right = left + random.Next(values.Length - left);
            Assert.Equal(BruteSum(values, left, right), sum.Query(left, right));
            Assert.Equal(BruteMin(values, left, right), min.Query(left, right));
            Assert.Equal(BruteMax(values, left, right), max.Query(left, right));
        }
    }

    [Fact]
    public void ConstructorsValidateNullArgumentsAndToStringReportsMetadata()
    {
        Assert.Throws<ArgumentNullException>(() => new SegmentTree<int>(null!, static (a, b) => a + b, 0));
        Assert.Throws<ArgumentNullException>(() => new SegmentTree<int>(new[] { 1 }, null!, 0));
        Assert.Equal("SegmentTree{n=3, identity=0}", SegmentTree<int>.SumTree(new[] { 1, 2, 3 }).ToString());
    }

    private static void AssertAllRanges(int[] values, SegmentTree<int> tree, Func<int[], int, int, int> reference)
    {
        for (var left = 0; left < values.Length; left++)
        {
            for (var right = left; right < values.Length; right++)
            {
                Assert.Equal(reference(values, left, right), tree.Query(left, right));
            }
        }
    }

    private static int BruteSum(int[] values, int left, int right)
    {
        var sum = 0;
        for (var i = left; i <= right; i++)
        {
            sum += values[i];
        }

        return sum;
    }

    private static int BruteMin(int[] values, int left, int right)
    {
        var min = values[left];
        for (var i = left + 1; i <= right; i++)
        {
            min = Math.Min(min, values[i]);
        }

        return min;
    }

    private static int BruteMax(int[] values, int left, int right)
    {
        var max = values[left];
        for (var i = left + 1; i <= right; i++)
        {
            max = Math.Max(max, values[i]);
        }

        return max;
    }

    private static int BruteGcd(int[] values, int left, int right)
    {
        var gcd = Math.Abs(values[left]);
        for (var i = left + 1; i <= right; i++)
        {
            gcd = Gcd(gcd, values[i]);
        }

        return gcd;
    }

    private static int Gcd(int a, int b)
    {
        a = Math.Abs(a);
        b = Math.Abs(b);
        while (b != 0)
        {
            var next = a % b;
            a = b;
            b = next;
        }

        return a;
    }
}
