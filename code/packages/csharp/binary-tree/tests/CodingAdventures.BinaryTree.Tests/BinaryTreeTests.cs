using Xunit;

namespace CodingAdventures.BinaryTree.Tests;

public sealed class BinaryTreeTests
{
    [Fact]
    public void LevelOrderRoundTrips()
    {
        var tree = BinaryTree<string>.FromLevelOrder(["A", "B", "C", "D", "E", "F", "G"]);

        Assert.Equal("A", tree.Root?.Value);
        Assert.Equal(["A", "B", "C", "D", "E", "F", "G"], tree.LevelOrder());
        Assert.Equal(["A", "B", "C", "D", "E", "F", "G"], tree.ToArray());
    }

    [Fact]
    public void ShapeQueriesWork()
    {
        var tree = BinaryTree<string>.FromLevelOrder(["A", "B", null]);

        Assert.False(tree.IsFull());
        Assert.True(tree.IsComplete());
        Assert.False(tree.IsPerfect());
        Assert.Equal(1, tree.Height());
        Assert.Equal(2, tree.Size());
        Assert.Equal("B", tree.LeftChild("A")?.Value);
        Assert.Null(tree.RightChild("A"));
        Assert.Null(tree.Find("missing"));
    }

    [Fact]
    public void TraversalsWork()
    {
        var tree = BinaryTree<string>.FromLevelOrder(["A", "B", "C", "D", null, "E", null]);

        Assert.Equal(["A", "B", "D", "C", "E"], tree.Preorder());
        Assert.Equal(["D", "B", "A", "E", "C"], tree.Inorder());
        Assert.Equal(["D", "B", "E", "C", "A"], tree.Postorder());
        Assert.Equal(["A", "B", "C", "D", "E"], tree.LevelOrder());
        Assert.Equal(["A", "B", "C", "D", null, "E", null], tree.ToArray());
    }

    [Fact]
    public void PerfectAndEmptyTreesWork()
    {
        var perfect = BinaryTree<string>.FromLevelOrder(["A", "B", "C", "D", "E", "F", "G"]);
        Assert.True(perfect.IsFull());
        Assert.True(perfect.IsComplete());
        Assert.True(perfect.IsPerfect());

        var empty = new BinaryTree<string>();
        Assert.Null(empty.Root);
        Assert.True(empty.IsFull());
        Assert.True(empty.IsComplete());
        Assert.True(empty.IsPerfect());
        Assert.Equal(-1, empty.Height());
        Assert.Equal(0, empty.Size());
        Assert.Empty(empty.LevelOrder());
        Assert.Empty(empty.ToArray());
        Assert.Equal(string.Empty, empty.ToAscii());
        Assert.Equal("BinaryTree(root=null, size=0)", empty.ToString());
    }

    [Fact]
    public void ExplicitRootAndStaticHelpersWork()
    {
        var root = new BinaryTreeNode<string>(
            "root",
            new BinaryTreeNode<string>("left"),
            new BinaryTreeNode<string>("right"));
        var tree = BinaryTree<string>.WithRoot(root);

        Assert.Same(root, tree.Root);
        Assert.Equal("root", BinaryTree<string>.Find(root, "root")?.Value);
        Assert.False(BinaryTree<string>.IsFull(new BinaryTreeNode<string>("x", new BinaryTreeNode<string>("l"))));
        Assert.True(BinaryTree<string>.IsComplete(root));
        Assert.True(BinaryTree<string>.IsPerfect(root));
        Assert.Equal(1, BinaryTree<string>.Height(root));
        Assert.Equal(3, BinaryTree<string>.Size(root));
        Assert.Contains("root", tree.ToAscii());

        var singleton = BinaryTree<string>.Singleton("solo");
        Assert.Equal("solo", singleton.Root?.Value);
    }
}
