using CodingAdventures.RadixTree;

namespace CodingAdventures.RadixTree.Tests;

public sealed class RadixTreeTests
{
    [Fact]
    public void InsertAndSearchCoverSplitCases()
    {
        var tree = new RadixTree<int>();

        tree.Insert("application", 1);
        tree.Insert("apple", 2);
        tree.Insert("app", 3);
        tree.Insert("apt", 4);

        Assert.Equal(1, tree.Search("application"));
        Assert.Equal(2, tree.Search("apple"));
        Assert.Equal(3, tree.Search("app"));
        Assert.Equal(4, tree.Search("apt"));
        Assert.Equal(default, tree.Search("appl"));
        Assert.Equal(4, tree.Count);
    }

    [Fact]
    public void UpdatingExistingKeyDoesNotChangeCount()
    {
        var tree = new RadixTree<string>();

        tree.Insert("alpha", "first");
        tree.Put("alpha", "second");

        Assert.True(tree.ContainsKey("alpha"));
        Assert.Equal("second", tree.Get("alpha"));
        Assert.Single(tree);
    }

    [Fact]
    public void DeletePrunesAndMerges()
    {
        var tree = new RadixTree<int>();
        tree.Insert("app", 1);
        tree.Insert("apple", 2);

        Assert.Equal(3, tree.NodeCount());
        Assert.True(tree.Delete("app"));
        Assert.False(tree.ContainsKey("app"));
        Assert.Equal(2, tree.Search("apple"));
        Assert.Equal(2, tree.NodeCount());
        Assert.False(tree.Delete("app"));
    }

    [Fact]
    public void SupportsPrefixQueriesAndMatches()
    {
        var tree = new RadixTree<int>();
        tree.Insert("search", 1);
        tree.Insert("searcher", 2);
        tree.Insert("searching", 3);
        tree.Insert("banana", 4);

        Assert.True(tree.StartsWith("sear"));
        Assert.False(tree.StartsWith("seek"));
        Assert.Equal(["search", "searcher", "searching"], tree.WordsWithPrefix("search"));
        Assert.Equal("search", tree.LongestPrefixMatch("search-party"));
        Assert.Null(tree.LongestPrefixMatch("xyz"));
    }

    [Fact]
    public void SupportsEmptyStringAndSortedKeys()
    {
        var tree = new RadixTree<int>();
        tree.Insert("", 1);
        tree.Insert("banana", 2);
        tree.Insert("apple", 3);
        tree.Insert("apricot", 4);
        tree.Insert("app", 5);

        Assert.Equal(1, tree.Search(""));
        Assert.Equal("", tree.LongestPrefixMatch("xyz"));
        Assert.True(tree.Delete(""));
        Assert.Equal(default, tree.Search(""));
        Assert.Equal(["app", "apple", "apricot", "banana"], tree.Keys());
        Assert.Equal([5, 3, 4, 2], tree.Values());
        Assert.Equal(4, tree.Size);
        Assert.False(tree.IsEmpty);
    }

    [Fact]
    public void ConstructorAndDictionaryExportKeepSortedKeys()
    {
        var tree = new RadixTree<int>(
        [
            new KeyValuePair<string, int>("delta", 4),
            new KeyValuePair<string, int>("alpha", 1),
            new KeyValuePair<string, int>("charlie", 3),
        ]);

        Assert.Equal(["alpha", "charlie", "delta"], tree.ToDictionary().Keys);
        Assert.Equal("RadixTree(3 keys: [alpha=1, charlie=3, delta=4])", tree.ToString());
    }
}
