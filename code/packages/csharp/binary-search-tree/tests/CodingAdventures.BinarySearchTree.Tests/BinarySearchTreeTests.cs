namespace CodingAdventures.BinarySearchTree.Tests;

public sealed class BinarySearchTreeTests
{
    [Fact]
    public void InsertSearchAndDeleteWorkImmutably()
    {
        var tree = Populated();

        Assert.Equal([1, 3, 5, 7, 8], tree.ToSortedArray());
        Assert.Equal(5, tree.Size());
        Assert.True(tree.Contains(7));
        Assert.Equal(7, tree.Search(7)?.Value);
        Assert.Equal(1, tree.MinValue());
        Assert.Equal(8, tree.MaxValue());
        Assert.Equal(3, tree.Predecessor(5));
        Assert.Equal(7, tree.Successor(5));
        Assert.Equal(2, tree.Rank(4));
        Assert.Equal(7, tree.KthSmallest(4));

        var deleted = tree.Delete(5);

        Assert.False(deleted.Contains(5));
        Assert.True(deleted.IsValid());
        Assert.True(tree.Contains(5));
    }

    [Fact]
    public void FromSortedArrayBuildsBalancedTree()
    {
        var tree = BinarySearchTree<int>.FromSortedArray([1, 2, 3, 4, 5, 6, 7]);

        Assert.Equal([1, 2, 3, 4, 5, 6, 7], tree.ToSortedArray());
        Assert.Equal(2, tree.Height());
        Assert.Equal(7, tree.Size());
        Assert.True(tree.IsValid());
        Assert.Equal([1, 2, 3, 4, 5, 6, 7], tree.ToList());
        Assert.Equal(7, tree.Count);
    }

    [Fact]
    public void EmptyTreeAndBoundaryQueriesReturnDefaults()
    {
        var tree = BinarySearchTree<int>.Empty();

        Assert.Null(tree.Search(1));
        Assert.Equal(0, tree.MinValue());
        Assert.Equal(0, tree.MaxValue());
        Assert.Equal(0, tree.Predecessor(1));
        Assert.Equal(0, tree.Successor(1));
        Assert.Equal(0, tree.KthSmallest(0));
        Assert.Equal(0, tree.KthSmallest(1));
        Assert.Equal(0, tree.Rank(1));
        Assert.Equal(-1, tree.Height());
        Assert.Equal(0, tree.Size());
        Assert.Equal("BinarySearchTree(root=null, size=0)", tree.ToString());
    }

    [Fact]
    public void DuplicateAndSingleChildDeletePreserveShape()
    {
        var tree = BinarySearchTree<int>.FromSortedArray([2, 4, 6, 8]);

        Assert.Equal(6, tree.Root?.Value);
        var duplicate = tree.Insert(4);

        Assert.Equal(tree.ToSortedArray(), duplicate.ToSortedArray());
        Assert.Equal([4, 6, 8], tree.Delete(2).ToSortedArray());
    }

    [Fact]
    public void StaticNodeHelpersCoverRawNodeComposition()
    {
        var root = BinarySearchTree<int>.InsertNode(null, 5);
        root = BinarySearchTree<int>.InsertNode(root, 2);
        root = BinarySearchTree<int>.InsertNode(root, 9);

        Assert.Equal(2, BinarySearchTree<int>.MinValue(root));
        Assert.Equal(9, BinarySearchTree<int>.MaxValue(root));
        Assert.Equal(5, BinarySearchTree<int>.KthSmallest(root, 2));
        Assert.Equal(1, BinarySearchTree<int>.Rank(root, 5));
        Assert.Equal(1, BinarySearchTree<int>.Height(root));
        Assert.True(BinarySearchTree<int>.IsValid(root));
        Assert.Same(root, BinarySearchTree<int>.InsertNode(root, 5));
        Assert.Null(BinarySearchTree<int>.DeleteNode(null, 5));
    }

    [Fact]
    public void ValidationCatchesBadOrderingAndStaleSizes()
    {
        var badOrder = new BinarySearchTree<int>(new BstNode<int>(5, left: new BstNode<int>(6)));
        var badSize = new BinarySearchTree<int>(new BstNode<int>(5, left: new BstNode<int>(3), size: 99));

        Assert.False(badOrder.IsValid());
        Assert.False(badSize.IsValid());
    }

    [Fact]
    public void HandlesReferenceComparableValues()
    {
        var tree = BinarySearchTree<string>.Empty()
            .Insert("delta")
            .Insert("alpha")
            .Insert("gamma");

        Assert.Equal(["alpha", "delta", "gamma"], tree.ToSortedArray());
        Assert.Equal("BinarySearchTree(root=delta, size=3)", tree.ToString());
        Assert.Equal("gamma", tree.Successor("delta"));
        Assert.Null(tree.Predecessor("alpha"));
    }

    private static BinarySearchTree<int> Populated()
    {
        var tree = BinarySearchTree<int>.Empty();
        foreach (var value in new[] { 5, 1, 8, 3, 7 })
        {
            tree = tree.Insert(value);
        }

        return tree;
    }
}
