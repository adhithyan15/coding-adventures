using CodingAdventures.BPlusTree;

namespace CodingAdventures.BPlusTree.Tests;

public sealed class BPlusTreeTests
{
    [Fact]
    public void ConstructorRejectsInvalidMinimumDegree()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BPlusTree<int, string>(1));

        var tree = new BPlusTree<int, string>();
        Assert.Equal(2, tree.MinimumDegree);
        Assert.Equal(0, tree.Count);
        Assert.Equal(0, tree.Size);
        Assert.True(tree.IsEmpty);
        Assert.Equal(0, tree.Height());
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void InsertSearchAndUpdateKeepSizeStable()
    {
        var tree = new BPlusTree<int, string>(3);
        foreach (var key in new[] { 10, 5, 20, 1, 15, 30 })
        {
            tree.Insert(key, $"v{key}");
        }

        tree.Insert(15, "updated");

        Assert.Equal(6, tree.Count);
        Assert.True(tree.Contains(20));
        Assert.False(tree.Contains(99));
        Assert.Equal("updated", tree.Search(15));
        Assert.Null(tree.Search(99));
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void ContainsHandlesNullValues()
    {
        var tree = new BPlusTree<int, string?>();

        tree.Insert(42, null);

        Assert.True(tree.Contains(42));
        Assert.Null(tree.Search(42));
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void NullKeysAreRejected()
    {
        var tree = new BPlusTree<string, int>();

        Assert.Throws<ArgumentNullException>(() => tree.Insert(null!, 1));
        Assert.Throws<ArgumentNullException>(() => tree.Search(null!));
        Assert.Throws<ArgumentNullException>(() => tree.Contains(null!));
        Assert.Throws<ArgumentNullException>(() => tree.Delete(null!));
        Assert.Throws<ArgumentNullException>(() => tree.RangeScan(null!, "z"));
        Assert.Throws<ArgumentNullException>(() => tree.RangeScan("a", null!));
    }

    [Fact]
    public void SequentialInsertsSplitLeavesAndTraverseSorted()
    {
        var tree = new BPlusTree<int, string>(2);
        for (var key = 1; key <= 3; key++)
        {
            tree.Insert(key, $"v{key}");
        }

        Assert.Equal(0, tree.Height());

        for (var key = 4; key <= 50; key++)
        {
            tree.Insert(key, $"v{key}");
            Assert.True(tree.IsValid());
        }

        Assert.True(tree.Height() >= 2);
        Assert.Equal(Enumerable.Range(1, 50), tree.FullScan().Select(entry => entry.Key));
        Assert.Equal(tree.FullScan().Select(entry => entry.Key), tree.Select(entry => entry.Key));
    }

    [Fact]
    public void RangeScanIsInclusiveAndRejectsInvertedBounds()
    {
        var tree = new BPlusTree<int, string>(2);
        foreach (var key in new[] { 9, 3, 7, 1, 5, 2, 8, 4, 6, 10 })
        {
            tree.Insert(key, $"v{key}");
        }

        Assert.Equal(new[] { 3, 4, 5, 6, 7 }, tree.RangeScan(3, 7).Select(entry => entry.Key));
        Assert.Equal(new[] { 3, 4, 5, 6, 7 }, tree.RangeQuery(3, 7).Select(entry => entry.Key));
        Assert.Empty(tree.RangeScan(11, 20));
        Assert.Throws<ArgumentException>(() => tree.RangeScan(7, 3));
    }

    [Fact]
    public void MinMaxAndEmptyEdgesBehavePredictably()
    {
        var tree = new BPlusTree<int, string>();

        Assert.Throws<InvalidOperationException>(() => tree.MinKey());
        Assert.Throws<InvalidOperationException>(() => tree.MaxKey());
        Assert.Empty(tree.FullScan());
        Assert.Empty(tree.InOrder());
        Assert.Empty(tree.RangeScan(1, 10));
        Assert.Equal("BPlusTree(t=2, size=0, height=0)", tree.ToString());

        tree.Insert(20, "twenty");
        tree.Insert(10, "ten");
        tree.Insert(30, "thirty");

        Assert.Equal(10, tree.MinKey());
        Assert.Equal(30, tree.MaxKey());
        Assert.Equal("BPlusTree(t=2, size=3, height=0)", tree.ToString());
    }

    [Fact]
    public void DeleteRemovesKeysAndMissingDeleteIsNoOp()
    {
        var tree = new BPlusTree<int, string>(2);
        for (var key = 1; key <= 25; key++)
        {
            tree.Insert(key, $"v{key}");
        }

        foreach (var key in new[] { 7, 12, 1, 25, 13, 14, 15, 16, 17, 18 })
        {
            tree.Delete(key);
            Assert.False(tree.Contains(key));
            Assert.True(tree.IsValid());
        }

        tree.Delete(99);

        Assert.Equal(15, tree.Count);
        Assert.Equal(2, tree.MinKey());
        Assert.Equal(24, tree.MaxKey());
        Assert.True(tree.Height() > 0);
    }

    [Fact]
    public void MultipleMinimumDegreesStayValid()
    {
        foreach (var degree in new[] { 2, 3, 5, 8 })
        {
            var tree = new BPlusTree<int, string>(degree);
            for (var key = 100; key >= 1; key--)
            {
                tree.Insert(key, $"v{key}");
            }

            Assert.True(tree.IsValid());
            Assert.Equal(Enumerable.Range(1, 100), tree.FullScan().Select(entry => entry.Key));
        }
    }

    [Fact]
    public void StressMatchesSortedDictionary()
    {
        var tree = new BPlusTree<int, string?>(3);
        var reference = new SortedDictionary<int, string?>();
        var random = new Random(1234);

        for (var step = 0; step < 500; step++)
        {
            var key = random.Next(200);
            if (random.Next(4) == 0)
            {
                tree.Delete(key);
                reference.Remove(key);
            }
            else
            {
                var value = random.Next(5) == 0 ? null : $"v{step}";
                tree.Insert(key, value);
                reference[key] = value;
            }

            Assert.Equal(reference.Count, tree.Count);
            Assert.True(tree.IsValid());
            Assert.Equal(reference.Keys, tree.FullScan().Select(entry => entry.Key));
            foreach (var entry in reference)
            {
                Assert.True(tree.Contains(entry.Key));
                Assert.Equal(entry.Value, tree.Search(entry.Key));
            }
        }
    }
}
