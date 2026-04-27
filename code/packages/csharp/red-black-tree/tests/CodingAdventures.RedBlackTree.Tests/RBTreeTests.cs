using CodingAdventures.RedBlackTree;

namespace CodingAdventures.RedBlackTree.Tests;

public sealed class RBTreeTests
{
    [Fact]
    public void EmptyTreeReportsMetadataAndMissingValues()
    {
        var tree = RBTree.Empty();

        Assert.True(tree.IsValidRB());
        Assert.True(tree.IsEmpty);
        Assert.Equal(0, tree.Size());
        Assert.Equal(0, tree.Height());
        Assert.Equal(0, tree.BlackHeight());
        Assert.False(tree.Contains(42));
        Assert.Null(tree.Min());
        Assert.Null(tree.Max());
        Assert.Empty(tree.ToSortedList());
        Assert.Throws<InvalidOperationException>(() => tree.KthSmallest(1));
    }

    [Fact]
    public void SingleInsertCreatesBlackRoot()
    {
        var tree = Build(10);

        Assert.True(tree.IsValidRB());
        Assert.Equal(Color.Black, tree.GetRoot()!.Color);
        Assert.Equal(tree.Root, tree.GetRoot());
        Assert.Equal(1, tree.Size());
        Assert.True(tree.Contains(10));
        Assert.False(tree.Contains(11));
        Assert.Equal(10, tree.Min());
        Assert.Equal(10, tree.Max());
    }

    [Fact]
    public void InsertSequencesStayValidAndSorted()
    {
        var ascending = RBTree.Empty();
        for (var i = 1; i <= 40; i++)
        {
            ascending = ascending.Insert(i);
            Assert.True(ascending.IsValidRB(), $"after inserting {i}");
        }

        Assert.Equal(Enumerable.Range(1, 40), ascending.ToSortedList());

        var descending = RBTree.Empty();
        for (var i = 40; i >= 1; i--)
        {
            descending = descending.Insert(i);
            Assert.True(descending.IsValidRB(), $"after inserting {i}");
        }

        Assert.Equal(Enumerable.Range(1, 40), descending.ToSortedList());
    }

    [Fact]
    public void DuplicatesAreIgnored()
    {
        var tree = Build(5, 5, 5, 3, 3, 7);

        Assert.True(tree.IsValidRB());
        Assert.Equal(3, tree.Size());
        Assert.Equal(new[] { 3, 5, 7 }, tree.ToSortedList());
    }

    [Fact]
    public void SearchMinMaxPredecessorSuccessorWork()
    {
        var tree = Build(10, 5, 15, 3, 7, 12, 20);

        Assert.True(tree.Contains(10));
        Assert.True(tree.Contains(20));
        Assert.False(tree.Contains(11));
        Assert.Equal(3, tree.Min());
        Assert.Equal(20, tree.Max());
        Assert.Equal(10, tree.Predecessor(12));
        Assert.Equal(7, tree.Predecessor(10));
        Assert.Null(tree.Predecessor(3));
        Assert.Equal(12, tree.Successor(10));
        Assert.Equal(10, tree.Successor(7));
        Assert.Null(tree.Successor(20));
    }

    [Fact]
    public void KthSmallestUsesSortedOrder()
    {
        var tree = Build(5, 3, 8, 1, 9, 4);

        Assert.Equal(1, tree.KthSmallest(1));
        Assert.Equal(3, tree.KthSmallest(2));
        Assert.Equal(4, tree.KthSmallest(3));
        Assert.Equal(5, tree.KthSmallest(4));
        Assert.Equal(8, tree.KthSmallest(5));
        Assert.Equal(9, tree.KthSmallest(6));
        Assert.Throws<InvalidOperationException>(() => tree.KthSmallest(0));
        Assert.Throws<InvalidOperationException>(() => tree.KthSmallest(7));
    }

    [Fact]
    public void DeleteCasesPreserveInvariants()
    {
        var absent = Build(5, 3, 7).Delete(99);
        Assert.True(absent.IsValidRB());
        Assert.Equal(new[] { 3, 5, 7 }, absent.ToSortedList());

        Assert.True(Build(42).Delete(42).IsEmpty);

        var rootDeleted = Build(5, 3).Delete(5);
        Assert.True(rootDeleted.IsValidRB());
        Assert.Equal(new[] { 3 }, rootDeleted.ToSortedList());

        var leafDeleted = Build(5, 3, 7).Delete(3);
        Assert.True(leafDeleted.IsValidRB());
        Assert.Equal(new[] { 5, 7 }, leafDeleted.ToSortedList());

        var internalDeleted = Build(10, 5, 15, 3, 7, 12, 20).Delete(5);
        Assert.True(internalDeleted.IsValidRB());
        Assert.Equal(new[] { 3, 7, 10, 12, 15, 20 }, internalDeleted.ToSortedList());
    }

    [Fact]
    public void DeleteAllElementsInSeveralOrders()
    {
        var values = new[] { 10, 5, 15, 3, 7, 12, 20 };
        var tree = Build(values);

        foreach (var value in values)
        {
            tree = tree.Delete(value);
            Assert.True(tree.IsValidRB(), $"after deleting {value}");
            Assert.False(tree.Contains(value));
        }

        Assert.True(tree.IsEmpty);

        var minOrder = Build(1, 2, 3, 4, 5);
        for (var i = 1; i <= 5; i++)
        {
            minOrder = minOrder.Delete(i);
            Assert.True(minOrder.IsValidRB());
        }

        var maxOrder = Build(1, 2, 3, 4, 5);
        for (var i = 5; i >= 1; i--)
        {
            maxOrder = maxOrder.Delete(i);
            Assert.True(maxOrder.IsValidRB());
        }
    }

    [Fact]
    public void ImmutabilityKeepsOldTreeUnchanged()
    {
        var original = Build(5, 3, 7);
        var modified = original.Insert(1).Insert(9).Delete(3);

        Assert.Equal(new[] { 3, 5, 7 }, original.ToSortedList());
        Assert.Equal(new[] { 1, 5, 7, 9 }, modified.ToSortedList());
        Assert.True(modified.IsValidRB());
    }

    [Fact]
    public void HeightBlackHeightAndToStringAreConsistent()
    {
        var tree = Build(Enumerable.Range(1, 100).ToArray());
        var maxAllowedHeight = (int)(2 * Math.Ceiling(Math.Log2(101)));

        Assert.True(tree.IsValidRB());
        Assert.True(tree.Height() <= maxAllowedHeight);
        Assert.True(tree.BlackHeight() > 0);
        Assert.Equal($"RBTree{{size={tree.Size()}, height={tree.Height()}, blackHeight={tree.BlackHeight()}}}", tree.ToString());
    }

    [Fact]
    public void RandomStressMatchesSortedSet()
    {
        var random = new Random(42);
        var tree = RBTree.Empty();
        var reference = new SortedSet<int>();

        for (var i = 0; i < 200; i++)
        {
            var value = random.Next(100);
            tree = tree.Insert(value);
            reference.Add(value);
            Assert.True(tree.IsValidRB());
            Assert.Equal(reference, tree.ToSortedList());
        }

        foreach (var value in reference.OrderBy(_ => random.Next()).ToArray())
        {
            tree = tree.Delete(value);
            reference.Remove(value);
            Assert.True(tree.IsValidRB());
            Assert.Equal(reference, tree.ToSortedList());
        }

        Assert.True(tree.IsEmpty);
    }

    [Fact]
    public void NodeRecordsExposeColorAndIsRed()
    {
        var red = new RBTree.Node(5, Color.Red);
        var black = red with { Color = Color.Black };

        Assert.True(red.IsRed);
        Assert.False(black.IsRed);
        Assert.Equal(5, red.Value);
    }

    private static RBTree Build(params int[] values)
    {
        var tree = RBTree.Empty();
        foreach (var value in values)
        {
            tree = tree.Insert(value);
        }

        return tree;
    }
}
