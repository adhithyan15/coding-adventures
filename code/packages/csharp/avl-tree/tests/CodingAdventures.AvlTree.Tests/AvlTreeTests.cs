namespace CodingAdventures.AvlTree.Tests;

public sealed class AvlTreeTests
{
    [Fact]
    public void RotationsRebalanceTheTree()
    {
        var tree = AvlTree<int>.FromValues([10, 20, 30]);

        Assert.Equal([10, 20, 30], tree.ToSortedArray());
        Assert.Equal(20, tree.Root?.Value);
        Assert.True(tree.IsValidBst());
        Assert.True(tree.IsValidAvl());
        Assert.Equal(1, tree.Height());
        Assert.Equal(3, tree.Size());
        Assert.Equal(0, tree.BalanceFactor(tree.Root));
        Assert.Equal(3, tree.Count);

        tree = AvlTree<int>.FromValues([30, 20, 10]);
        Assert.Equal(20, tree.Root?.Value);
        Assert.True(tree.IsValidAvl());
    }

    [Fact]
    public void SearchAndOrderStatisticsWork()
    {
        var tree = AvlTree<int>.FromValues([40, 20, 60, 10, 30, 50, 70]);

        Assert.Equal(20, tree.Search(20)?.Value);
        Assert.True(tree.Contains(50));
        Assert.Equal(10, tree.MinValue());
        Assert.Equal(70, tree.MaxValue());
        Assert.Equal(30, tree.Predecessor(40));
        Assert.Equal(50, tree.Successor(40));
        Assert.Equal(40, tree.KthSmallest(4));
        Assert.Equal(3, tree.Rank(35));
        Assert.Equal([10, 20, 30, 40, 50, 60, 70], tree.ToList());

        var deleted = tree.Delete(20);

        Assert.False(deleted.Contains(20));
        Assert.True(deleted.IsValidAvl());
        Assert.True(tree.Contains(20));
    }

    [Fact]
    public void EdgeCasesAndDuplicatesReturnDefaults()
    {
        var tree = AvlTree<int>.Empty();

        Assert.Null(tree.Search(1));
        Assert.Equal(0, tree.MinValue());
        Assert.Equal(0, tree.MaxValue());
        Assert.Equal(0, tree.Predecessor(1));
        Assert.Equal(0, tree.Successor(1));
        Assert.Equal(0, tree.KthSmallest(0));
        Assert.Equal(0, tree.Rank(1));
        Assert.Equal(0, tree.BalanceFactor(null));
        Assert.Equal(-1, tree.Height());
        Assert.Equal(0, tree.Size());
        Assert.Equal("AvlTree(root=null, size=0, height=-1)", tree.ToString());

        tree = AvlTree<int>.FromValues([30, 20, 40, 10, 25, 35, 50]);
        var duplicate = tree.Insert(25);
        Assert.Equal(tree.ToSortedArray(), duplicate.ToSortedArray());
        Assert.Equal(tree.ToSortedArray(), tree.Delete(999).ToSortedArray());

        var single = new AvlTree<int>(new AvlNode<int>(5));
        Assert.Equal(5, single.Root?.Value);
        Assert.Equal(0, single.Height());
        Assert.Equal([2], AvlTree<int>.FromValues([1, 2]).Delete(1).ToSortedArray());
        Assert.Equal([1], AvlTree<int>.FromValues([2, 1]).Delete(2).ToSortedArray());
    }

    [Fact]
    public void DoubleRotationsAndValidationFailuresAreCovered()
    {
        var leftRight = AvlTree<int>.FromValues([30, 10, 20]);
        var rightLeft = AvlTree<int>.FromValues([10, 30, 20]);

        Assert.Equal(20, leftRight.Root?.Value);
        Assert.Equal(20, rightLeft.Root?.Value);
        Assert.True(leftRight.IsValidAvl());
        Assert.True(rightLeft.IsValidAvl());

        var badOrder = new AvlTree<int>(new AvlNode<int>(5, left: new AvlNode<int>(6), height: 1, size: 2));
        var badRightOrder = new AvlTree<int>(new AvlNode<int>(5, right: new AvlNode<int>(4), height: 1, size: 2));
        var badHeight = new AvlTree<int>(new AvlNode<int>(5, left: new AvlNode<int>(3), height: 99, size: 2));

        Assert.False(badOrder.IsValidBst());
        Assert.False(badOrder.IsValidAvl());
        Assert.False(badRightOrder.IsValidBst());
        Assert.False(badRightOrder.IsValidAvl());
        Assert.False(badHeight.IsValidAvl());
    }

    [Fact]
    public void DeleteWithNestedSuccessorAndStaticHelpersWork()
    {
        var tree = AvlTree<int>.FromValues([5, 3, 8, 7, 9, 6]);

        var deleted = tree.Delete(5);
        Assert.Equal([3, 6, 7, 8, 9], deleted.ToSortedArray());
        Assert.True(deleted.IsValidAvl());
        Assert.Equal(3, tree.KthSmallest(1));
        Assert.Equal(9, tree.KthSmallest(6));
        Assert.Equal(1, tree.Rank(5));

        AvlNode<int>? root = null;
        root = AvlTree<int>.InsertNode(root, 2);
        root = AvlTree<int>.InsertNode(root, 1);
        root = AvlTree<int>.InsertNode(root, 3);

        Assert.Equal(2, AvlTree<int>.SearchNode(root, 2)?.Value);
        Assert.Equal(1, AvlTree<int>.MinValue(root));
        Assert.Equal(3, AvlTree<int>.MaxValue(root));
        Assert.Equal(2, AvlTree<int>.KthSmallest(root, 2));
        Assert.Equal(1, AvlTree<int>.Rank(root, 2));
        Assert.Equal(0, AvlTree<int>.BalanceFactorNode(root));
        Assert.True(AvlTree<int>.IsValidBst(root));
        Assert.True(AvlTree<int>.IsValidAvl(root));
        Assert.Null(AvlTree<int>.DeleteNode(null, 1));
    }

    [Fact]
    public void ReferenceComparableValuesRetainSortedOrder()
    {
        var tree = AvlTree<string>.FromValues(["delta", "alpha", "gamma"]);

        Assert.Equal(["alpha", "delta", "gamma"], tree.ToSortedArray());
        Assert.Equal("AvlTree(root=delta, size=3, height=1)", tree.ToString());
        Assert.Equal("gamma", tree.Successor("delta"));
        Assert.Null(tree.Predecessor("alpha"));
    }
}
