using CodingAdventures.Tree;

namespace CodingAdventures.Tree.Tests;

public sealed class TreeTests
{
    private static Tree Sample() =>
        new Tree("root")
            .AddChild("root", "child1")
            .AddChild("root", "child2")
            .AddChild("child1", "grandchild");

    [Fact]
    public void ConstructionAndQueriesWork()
    {
        var tree = Sample();

        Assert.Equal("root", tree.Root);
        Assert.Equal(4, tree.Size);
        Assert.Equal(2, tree.Height());
        Assert.Equal("child1", tree.Parent("grandchild"));
        Assert.Null(tree.Parent("root"));
        Assert.Equal(new[] { "child1", "child2" }, tree.Children("root"));
        Assert.Equal(new[] { "child2" }, tree.Siblings("child1"));
        Assert.True(tree.IsLeaf("grandchild"));
        Assert.True(tree.IsRoot("root"));
        Assert.True(tree.HasNode("child2"));
        Assert.Equal("Tree(root=root, size=4, height=2)", tree.ToString());
    }

    [Fact]
    public void TraversalsAndPathsAreDeterministic()
    {
        var tree = Sample();

        Assert.Equal(new[] { "child1", "child2", "grandchild", "root" }, tree.Nodes());
        Assert.Equal(new[] { "child2", "grandchild" }, tree.Leaves());
        Assert.Equal(new[] { "root", "child1", "grandchild", "child2" }, tree.Preorder());
        Assert.Equal(new[] { "grandchild", "child1", "child2", "root" }, tree.Postorder());
        Assert.Equal(new[] { "root", "child1", "child2", "grandchild" }, tree.LevelOrder());
        Assert.Equal(new[] { "root", "child1", "grandchild" }, tree.PathTo("grandchild"));
        Assert.Equal("root", tree.LowestCommonAncestor("grandchild", "child2"));
        Assert.Equal("child1", tree.Lca("child1", "grandchild"));
    }

    [Fact]
    public void SubtreeAndRemoveSubtreeWork()
    {
        var tree = Sample();
        var subtree = tree.Subtree("child1");

        Assert.Equal("child1", subtree.Root);
        Assert.Equal(new[] { "child1", "grandchild" }, subtree.Preorder());

        tree.RemoveSubtree("child1");

        Assert.Equal(new[] { "root", "child2" }, tree.Preorder());
        Assert.False(tree.HasNode("grandchild"));
    }

    [Fact]
    public void ErrorsAreSpecific()
    {
        var tree = Sample();

        var missing = Assert.Throws<TreeException>(() => tree.AddChild("missing", "x"));
        Assert.Equal(TreeErrorKind.NodeNotFound, missing.Kind);
        var duplicate = Assert.Throws<TreeException>(() => tree.AddChild("root", "child1"));
        Assert.Equal(TreeErrorKind.DuplicateNode, duplicate.Kind);
        var rootRemoval = Assert.Throws<TreeException>(() => tree.RemoveSubtree("root"));
        Assert.Equal(TreeErrorKind.RootRemoval, rootRemoval.Kind);
        Assert.Throws<TreeException>(() => tree.Depth("missing"));
        Assert.Throws<ArgumentException>(() => new Tree(""));
    }

    [Fact]
    public void AsciiRenderingIncludesShape()
    {
        var ascii = Sample().ToAscii();

        Assert.Contains("root", ascii);
        Assert.Contains("|-- child1", ascii);
        Assert.Contains("`-- child2", ascii);
    }
}
