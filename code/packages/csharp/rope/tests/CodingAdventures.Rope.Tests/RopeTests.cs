using CodingAdventures.Rope;

namespace CodingAdventures.Rope.Tests;

public sealed class RopeTests
{
    [Fact]
    public void EmptyAndFromStringExposeLength()
    {
        var empty = Rope.Empty();
        var rope = Rope.FromString("hello");

        Assert.True(empty.IsEmpty);
        Assert.Equal(0, empty.Length);
        Assert.Equal(0, empty.Count);
        Assert.Equal(string.Empty, empty.ToString());
        Assert.True(empty.IsBalanced());
        Assert.Equal(0, empty.Depth());
        Assert.Equal("hello", Rope.RopeFromString("hello").ToString());
        Assert.Equal(5, rope.Length);
        Assert.Throws<ArgumentNullException>(() => Rope.FromString(null!));
    }

    [Fact]
    public void ConcatSplitAndIndexWork()
    {
        var rope = Rope.Concat(Rope.FromString("hello"), Rope.FromString(" world"));

        Assert.Equal(11, rope.Length);
        Assert.Equal("hello world", rope.ToString());
        Assert.Equal('e', rope.Index(1));
        Assert.Null(rope.Index(-1));
        Assert.Null(rope.Index(11));

        var (left, right) = rope.Split(5);
        Assert.Equal("hello", left.ToString());
        Assert.Equal(" world", right.ToString());
    }

    [Fact]
    public void InstanceConcatPreservesEmptyIdentities()
    {
        var left = Rope.Empty().Concat(Rope.FromString("a"));
        var right = Rope.FromString("b").Concat(Rope.Empty());

        Assert.Equal("a", left.ToString());
        Assert.Equal("b", right.ToString());
        Assert.Throws<ArgumentNullException>(() => Rope.Concat(null!, Rope.Empty()));
        Assert.Throws<ArgumentNullException>(() => Rope.Concat(Rope.Empty(), null!));
    }

    [Fact]
    public void InsertDeleteAndSubstringClampLikeRust()
    {
        var rope = Rope.FromString("ace").Insert(1, "b").Insert(3, "d");

        Assert.Equal("abcde", rope.ToString());
        Assert.Equal("bcd", rope.Substring(1, 4));
        Assert.Equal(string.Empty, rope.Substring(4, 1));
        Assert.Equal("abcde", rope.Substring(-20, 20));
        Assert.Equal("ade", rope.Delete(1, 2).ToString());
        Assert.Equal("abcde!", rope.Insert(99, "!").ToString());
        Assert.Equal("bcde", rope.Delete(-10, 1).ToString());
        Assert.Throws<ArgumentNullException>(() => rope.Insert(0, null!));
        Assert.Throws<ArgumentOutOfRangeException>(() => rope.Delete(0, -1));
    }

    [Fact]
    public void RebalanceProducesBalancedTreeWithSameText()
    {
        var rope = Rope.FromString("a")
            .Concat(Rope.FromString("b"))
            .Concat(Rope.FromString("c"))
            .Concat(Rope.FromString("d"))
            .Concat(Rope.FromString("e"))
            .Concat(Rope.FromString("f"));

        Assert.False(rope.IsBalanced());

        var balanced = rope.Rebalance();

        Assert.Equal("abcdef", balanced.ToString());
        Assert.True(balanced.IsBalanced());
        Assert.True(balanced.Depth() <= 3);
    }
}
