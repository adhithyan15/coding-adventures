using CodingAdventures.Treap;

namespace CodingAdventures.Treap.Tests;

public sealed class TreapTests
{
    private static Treap Build(params int[] keys)
    {
        var treap = Treap.WithSeed(42);
        foreach (var key in keys)
        {
            treap = treap.Insert(key);
        }

        return treap;
    }

    [Fact]
    public void EmptyAndSingleTreapsExposeMetrics()
    {
        var empty = Treap.WithSeed(1);

        Assert.True(empty.IsValidTreap());
        Assert.True(empty.IsEmpty);
        Assert.Equal(0, empty.Size);
        Assert.Equal(0, empty.Height);
        Assert.Null(empty.Min());
        Assert.Null(empty.Max());
        Assert.Throws<ArgumentOutOfRangeException>(() => empty.KthSmallest(1));

        var single = empty.Insert(10);
        Assert.False(single.IsEmpty);
        Assert.Equal(1, single.Size);
        Assert.Equal(1, single.Height);
        Assert.Equal(10, single.Root?.Key);
        Assert.Equal(10, single.Min());
        Assert.Equal(10, single.Max());
        Assert.True(single.IsValidTreap());
    }

    [Fact]
    public void ExplicitPrioritiesShapeTreapDeterministically()
    {
        var treap = Treap.WithSeed(0)
            .InsertWithPriority(5, 0.91)
            .InsertWithPriority(3, 0.53)
            .InsertWithPriority(7, 0.75)
            .InsertWithPriority(1, 0.22)
            .InsertWithPriority(4, 0.68);

        Assert.True(treap.IsValidTreap());
        Assert.Equal(5, treap.Root?.Key);
        Assert.Equal(0.91, treap.Root?.Priority);
        Assert.Equal([1, 3, 4, 5, 7], treap.ToSortedList());
    }

    [Fact]
    public void InsertIgnoresDuplicatesAndPreservesOriginal()
    {
        var original = Build(5, 3, 7);
        var modified = original.Insert(1).Insert(9).Insert(5);

        Assert.Equal([3, 5, 7], original.ToSortedList());
        Assert.Equal([1, 3, 5, 7, 9], modified.ToSortedList());
        Assert.Equal(5, modified.Size);
        Assert.True(modified.IsValidTreap());
    }

    [Fact]
    public void ContainsMinMaxPredecessorSuccessorAndKthWork()
    {
        var treap = Build(10, 5, 15, 3, 7, 12, 20);

        Assert.True(treap.Contains(10));
        Assert.False(treap.Contains(11));
        Assert.Equal(3, treap.Min());
        Assert.Equal(20, treap.Max());
        Assert.Null(treap.Predecessor(3));
        Assert.Equal(7, treap.Predecessor(10));
        Assert.Equal(12, treap.Successor(10));
        Assert.Null(treap.Successor(20));
        Assert.Equal(3, treap.KthSmallest(1));
        Assert.Equal(10, treap.KthSmallest(4));
        Assert.Throws<ArgumentOutOfRangeException>(() => treap.KthSmallest(0));
    }

    [Fact]
    public void SplitAndMergePartitionAndReconstruct()
    {
        var original = Build(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        var parts = original.Split(5);
        var left = Treap.FromRoot(parts.Left);
        var right = Treap.FromRoot(parts.Right);

        Assert.Equal([1, 2, 3, 4, 5], left.ToSortedList());
        Assert.Equal([6, 7, 8, 9, 10], right.ToSortedList());

        var merged = Treap.Merge(left, right);
        Assert.True(merged.IsValidTreap());
        Assert.Equal(original.ToSortedList(), merged.ToSortedList());
    }

    [Fact]
    public void DeleteHandlesAbsentLeafRootAndAllKeys()
    {
        var treap = Treap.WithSeed(0)
            .InsertWithPriority(5, 0.9)
            .InsertWithPriority(3, 0.5)
            .InsertWithPriority(7, 0.6);

        Assert.Same(treap, treap.Delete(99));
        var withoutRoot = treap.Delete(5);
        Assert.True(withoutRoot.IsValidTreap());
        Assert.Equal([3, 7], withoutRoot.ToSortedList());

        var all = Build(10, 5, 15, 3, 7, 12, 20);
        foreach (var key in all.ToSortedList())
        {
            all = all.Delete(key);
            Assert.True(all.IsValidTreap());
            Assert.False(all.Contains(key));
        }

        Assert.True(all.IsEmpty);
    }

    [Fact]
    public void RandomStressPreservesSortedSetContents()
    {
        var random = new Random(7);
        var treap = Treap.WithSeed(55);
        var reference = new SortedSet<int>();

        for (var i = 0; i < 200; i++)
        {
            var key = random.Next(100);
            treap = treap.Insert(key);
            reference.Add(key);
            Assert.True(treap.IsValidTreap());
        }

        foreach (var key in reference.OrderBy(_ => random.Next()).ToList())
        {
            treap = treap.Delete(key);
            Assert.True(treap.IsValidTreap());
        }

        Assert.True(treap.IsEmpty);
    }
}
