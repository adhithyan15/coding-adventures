namespace CodingAdventures.Trie.Tests;

public sealed class TrieTests
{
    [Fact]
    public void InsertGetAndOverwrite()
    {
        var trie = new Trie<int>();
        trie.Insert("apple", 1);
        trie.Insert("app", 2);

        Assert.True(trie.TryGetValue("apple", out var apple));
        Assert.Equal(1, apple);
        Assert.Equal(2, trie.Get("app"));
        Assert.Equal(2, trie.Count);

        trie.Insert("apple", 42);
        Assert.Equal(42, trie.Get("apple"));
        Assert.Equal(2, trie.Size);
    }

    [Fact]
    public void MissingKeysAndPrefixesAreDistinct()
    {
        var trie = new Trie<int>();
        trie.Insert("apple", 1);

        Assert.False(trie.Contains("app"));
        Assert.True(trie.StartsWith("app"));
        Assert.False(trie.TryGetValue("banana", out _));
        Assert.False(trie.StartsWith("banana"));
        Assert.Equal(default, trie.Get("apples"));
    }

    [Fact]
    public void EmptyKeyAndNullValueAreSupported()
    {
        var trie = new Trie<string>();
        trie.Insert("", "empty");
        trie.Insert("nullable", null);

        Assert.True(trie.Contains(""));
        Assert.Equal("empty", trie.Get(""));
        Assert.True(trie.TryGetValue("nullable", out var value));
        Assert.Null(value);
        Assert.Equal(2, trie.Count);
    }

    [Fact]
    public void DeleteUnmarksOnlyTheRequestedKey()
    {
        var trie = new Trie<int>();
        trie.Insert("app", 1);
        trie.Insert("apple", 2);
        trie.Insert("apply", 3);

        Assert.True(trie.Delete("apple"));
        Assert.False(trie.Contains("apple"));
        Assert.True(trie.Contains("app"));
        Assert.True(trie.Contains("apply"));
        Assert.False(trie.Delete("apple"));
        Assert.False(trie.Delete("banana"));
        Assert.Equal(2, trie.Count);
    }

    [Fact]
    public void KeysWithPrefixReturnsMatches()
    {
        var trie = new Trie<int>();
        trie.Insert("app", 1);
        trie.Insert("apple", 2);
        trie.Insert("apply", 3);
        trie.Insert("apt", 4);
        trie.Insert("banana", 5);

        var appKeys = trie.KeysWithPrefix("app");
        Assert.Equal(3, appKeys.Count);
        Assert.Contains("app", appKeys);
        Assert.Contains("apple", appKeys);
        Assert.Contains("apply", appKeys);
        Assert.DoesNotContain("apt", appKeys);
        Assert.Empty(trie.KeysWithPrefix("z"));

        var allKeys = trie.Keys();
        Assert.Equal(5, allKeys.Count);
        Assert.Contains("banana", allKeys);
    }

    [Fact]
    public void SizeAndIsEmptyTrackMutations()
    {
        var trie = new Trie<int>();
        Assert.True(trie.IsEmpty);
        trie.Insert("a", 1);
        trie.Insert("b", 2);
        Assert.False(trie.IsEmpty);
        trie.Delete("a");
        trie.Delete("b");
        Assert.True(trie.IsEmpty);
        Assert.Equal(0, trie.Count);
    }

    [Fact]
    public void UnicodeAndLargeDatasetsWork()
    {
        var trie = new Trie<int>();
        trie.Insert("cafe\u0301", 1);
        trie.Insert("cafe\u0301s", 2);
        Assert.True(trie.Contains("cafe\u0301"));
        Assert.Equal(2, trie.KeysWithPrefix("caf").Count);

        for (var i = 0; i < 250; i++)
        {
            trie.Insert($"key{i}", i);
        }

        Assert.Equal(252, trie.Count);
        Assert.Equal(250, trie.KeysWithPrefix("key").Count);
    }

    [Fact]
    public void NullKeysAreRejectedOrTreatedAsAbsent()
    {
        var trie = new Trie<int>();

        Assert.Throws<ArgumentNullException>(() => trie.Insert(null!, 1));
        Assert.Throws<ArgumentNullException>(() => trie.KeysWithPrefix(null!));
        Assert.False(trie.Contains(null!));
        Assert.False(trie.StartsWith(null!));
        Assert.False(trie.Delete(null!));
    }
}
