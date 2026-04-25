using CodingAdventures.BTree;

namespace CodingAdventures.BTree.Tests;

public sealed class BTreeTests
{
    [Fact]
    public void ConstructorRejectsInvalidMinimumDegree()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BTree<int, string>(1));

        var tree = new BTree<int, string>();
        Assert.True(tree.IsEmpty);
        Assert.Equal(0, tree.Height());
        Assert.True(tree.IsValid());
        Assert.Equal(2, tree.MinimumDegree);
    }

    [Fact]
    public void InsertSearchAndUpdateKeepSizeStable()
    {
        var tree = new BTree<int, string>(3);
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
    public void SequentialInsertsSplitRootAndTraverseSorted()
    {
        var tree = new BTree<int, string>(2);
        for (var i = 0; i < 100; i++)
        {
            tree.Insert(i, $"v{i}");
            Assert.True(tree.IsValid());
        }

        Assert.Equal(100, tree.Count);
        Assert.True(tree.Height() > 0);
        Assert.Equal(Enumerable.Range(0, 100), tree.InOrder().Select(entry => entry.Key));
    }

    [Fact]
    public void DeleteHandlesLeafBorrowMergeAndRootShrink()
    {
        var tree = new BTree<int, string>(2);
        for (var i = 1; i <= 25; i++)
        {
            tree.Insert(i, $"v{i}");
        }

        var deletionOrder = new[] { 7, 12, 1, 25, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 2, 3, 4, 5, 6, 8, 9, 10 };
        foreach (var key in deletionOrder)
        {
            tree.Delete(key);
            Assert.False(tree.Contains(key));
            Assert.True(tree.IsValid());
        }

        Assert.Equal(1, tree.Count);
        Assert.Equal(11, tree.MinKey());
        Assert.Equal(11, tree.MaxKey());
        Assert.Equal(0, tree.Height());
    }

    [Fact]
    public void DeleteMissingKeyThrows()
    {
        var tree = new BTree<int, string>();
        tree.Insert(10, "ten");

        Assert.Throws<KeyNotFoundException>(() => tree.Delete(99));
        Assert.True(tree.Contains(10));
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void MinMaxAndRangeQueryWorkAcrossLevels()
    {
        var tree = new BTree<int, string>(3);
        for (var i = 1; i <= 50; i++)
        {
            tree.Insert(i, $"v{i}");
        }

        Assert.Equal(1, tree.MinKey());
        Assert.Equal(50, tree.MaxKey());
        Assert.Equal(Enumerable.Range(10, 11), tree.RangeQuery(10, 20).Select(entry => entry.Key));
        Assert.Empty(tree.RangeQuery(60, 70));
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void EmptyMinMaxThrowAndRangeIsEmpty()
    {
        var tree = new BTree<int, string>();

        Assert.Throws<InvalidOperationException>(() => tree.MinKey());
        Assert.Throws<InvalidOperationException>(() => tree.MaxKey());
        Assert.Empty(tree.RangeQuery(1, 10));
        Assert.Empty(tree.InOrder());
        Assert.Equal("BTree(t=2, size=0, height=0)", tree.ToString());
    }

    [Fact]
    public void StressMatchesSortedDictionary()
    {
        var tree = new BTree<int, string>(3);
        var reference = new SortedDictionary<int, string>();
        var random = new Random(1234);
        var keys = Enumerable.Range(0, 400).OrderBy(_ => random.Next()).ToList();

        foreach (var key in keys)
        {
            var value = $"v{key}";
            tree.Insert(key, value);
            reference[key] = value;
        }

        foreach (var key in keys)
        {
            Assert.Equal(reference[key], tree.Search(key));
        }

        foreach (var key in keys.Take(175))
        {
            tree.Delete(key);
            reference.Remove(key);
        }

        for (var i = 0; i < 100; i++)
        {
            var key = random.Next(600);
            if (random.Next(2) == 0)
            {
                tree.Insert(key, $"v{key}");
                reference[key] = $"v{key}";
            }
            else if (reference.Remove(key))
            {
                tree.Delete(key);
            }
        }

        Assert.Equal(reference.Count, tree.Count);
        Assert.True(tree.IsValid());
        Assert.Equal(reference.Keys, tree.InOrder().Select(entry => entry.Key));
    }
}
