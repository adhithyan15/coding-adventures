using CodingAdventures.SuffixTree;

namespace CodingAdventures.SuffixTree.Tests;

public sealed class SuffixTreeTests
{
    [Fact]
    public void BuildSearchAndCountWork()
    {
        var tree = SuffixTree.Build("banana");

        Assert.Equal("banana", tree.Text);
        Assert.Equal(new[] { 1, 3 }, tree.Search("ana"));
        Assert.Equal(new[] { 0, 1, 2, 3, 4, 5, 6 }, tree.Search(""));
        Assert.Empty(tree.Search("band"));
        Assert.Equal(2, tree.CountOccurrences("ana"));
        Assert.Equal(7, tree.NodeCount());
    }

    [Fact]
    public void StaticHelpersDelegateToTree()
    {
        var tree = SuffixTree.BuildUkkonen("banana");

        Assert.Equal(new[] { 2, 4 }, SuffixTree.Search(tree, "n"));
        Assert.Equal(1, SuffixTree.CountOccurrences(tree, "nan"));
        Assert.Equal(7, SuffixTree.NodeCount(tree));
        Assert.Throws<ArgumentNullException>(() => SuffixTree.Build(null!));
        Assert.Throws<ArgumentNullException>(() => tree.Search(null!));
        Assert.Throws<ArgumentNullException>(() => SuffixTree.Search(null!, "a"));
    }

    [Fact]
    public void SuffixAndRepeatedSubstringHelpersWork()
    {
        var tree = SuffixTree.Build("banana");

        Assert.Equal("ana", tree.LongestRepeatedSubstring());
        Assert.Equal("ana", SuffixTree.LongestRepeatedSubstring(tree));
        Assert.Equal("banana", tree.AllSuffixes()[0]);
        Assert.Equal("a", SuffixTree.AllSuffixes(tree)[^1]);
        Assert.Throws<ArgumentNullException>(() => SuffixTree.AllSuffixes(null!));
        Assert.Throws<ArgumentNullException>(() => SuffixTree.LongestRepeatedSubstring(null!));
    }

    [Fact]
    public void LongestCommonSubstringHandlesEdges()
    {
        Assert.Equal("abxa", SuffixTree.LongestCommonSubstring("xabxac", "abcabxabcd"));
        Assert.Equal(string.Empty, SuffixTree.LongestCommonSubstring("", "abc"));
        Assert.Equal(string.Empty, SuffixTree.LongestCommonSubstring("abc", ""));
        Assert.Throws<ArgumentNullException>(() => SuffixTree.LongestCommonSubstring(null!, "abc"));
        Assert.Throws<ArgumentNullException>(() => SuffixTree.LongestCommonSubstring("abc", null!));
    }
}
