namespace CodingAdventures.HashMap.Tests;

public sealed class HashMapTests
{
    [Fact]
    public void CreatesReadsUpdatesAndDeletesImmutably()
    {
        var empty = new HashMap<string, int>();
        var filled = empty.Set("a", 1).Set("b", 2);

        Assert.Equal(0, empty.Size);
        Assert.Equal(2, filled.Size);
        Assert.Equal(1, filled.Get("a"));
        Assert.True(filled.Has("b"));

        var updated = filled.Set("a", 3);
        Assert.Equal(3, updated.Get("a"));
        Assert.Equal(1, filled.Get("a"));

        var removed = updated.Delete("a");
        Assert.False(removed.Has("a"));
        Assert.True(updated.Has("a"));
    }

    [Fact]
    public void EnumeratesKeysValuesAndEntries()
    {
        var map = HashMap<string, int>.FromEntries(
        [
            new KeyValuePair<string, int>("x", 1),
            new KeyValuePair<string, int>("y", 2)
        ]);

        Assert.Equal(["x", "y"], map.Keys());
        Assert.Equal([1, 2], map.Values());
        Assert.Equal(
        [
            new KeyValuePair<string, int>("x", 1),
            new KeyValuePair<string, int>("y", 2)
        ],
        map.Entries());
    }
}
