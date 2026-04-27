namespace CodingAdventures.SkipList.Tests;

public sealed class SkipListTests
{
    [Fact]
    public void InsertsSearchesAndDeletesInSortedOrder()
    {
        var list = new SkipList<int, string>();
        list.Insert(20, "b");
        list.Insert(10, "a");
        list.Insert(30, "c");

        Assert.Equal([10, 20, 30], list.ToList());
        Assert.Equal("b", list.Search(20));
        Assert.True(list.Delete(20));
        Assert.Null(list.Search(20));
        Assert.Equal([10, 30], list.ToList());
    }

    [Fact]
    public void ComputesRankAndByRankConsistently()
    {
        var list = new SkipList<int, string>();
        foreach (var key in new[] { 50, 10, 30, 20 })
        {
            list.Insert(key, key.ToString());
        }

        Assert.Equal(0, list.Rank(10));
        Assert.Equal(1, list.Rank(20));
        Assert.Equal(30, list.ByRank(2));

        var stringList = new SkipList<string, int>();
        stringList.Insert("alpha", 1);
        Assert.Null(stringList.ByRank(10));
    }

    [Fact]
    public void ReturnsBoundedRanges()
    {
        var list = new SkipList<int, string>();
        foreach (var key in new[] { 10, 20, 30, 40, 50 })
        {
            list.Insert(key, key.ToString());
        }

        Assert.Equal(
        [
            new KeyValuePair<int, string>(20, "20"),
            new KeyValuePair<int, string>(30, "30"),
            new KeyValuePair<int, string>(40, "40")
        ],
        list.Range(15, 45, inclusive: true));

        Assert.Equal(
        [
            new KeyValuePair<int, string>(20, "20"),
            new KeyValuePair<int, string>(30, "30")
        ],
        list.Range(10, 40, inclusive: false));
    }

    [Fact]
    public void SupportsCustomComparatorsForCompositeKeys()
    {
        Comparator<(int Score, string Member)> comparator = (left, right) =>
        {
            var byScore = left.Score.CompareTo(right.Score);
            return byScore != 0 ? byScore : StringComparer.Ordinal.Compare(left.Member, right.Member);
        };

        var list = new SkipList<(int Score, string Member), string>(comparator);
        list.Insert((10, "b"), "b");
        list.Insert((10, "a"), "a");
        list.Insert((5, "z"), "z");

        Assert.Equal([(5, "z"), (10, "a"), (10, "b")], list.ToList());
    }

    [Fact]
    public void ExposesHelpersAndBoundaryBehaviors()
    {
        var list = SkipList<int, string>.WithParams(maxLevel: -1, probability: 2);
        var emptyStrings = new SkipList<string, int>();
        Assert.True(list.IsEmpty());
        Assert.Equal(1, list.MaxLevel());
        Assert.Equal(0.5, list.Probability());
        Assert.Equal(1, list.CurrentMax());
        Assert.Null(emptyStrings.Min());
        Assert.Null(emptyStrings.Max());
        Assert.Empty(list.RangeQuery(10, 1, inclusive: true));

        list.Insert(1, "one");
        list.Insert(2, "two");
        list.Insert(2, "two-updated");
        list.Insert(3, "three");

        Assert.True(list.Contains(2));
        Assert.True(list.ContainsKey(3));
        Assert.False(list.Contains(99));
        Assert.Equal("two-updated", list.Search(2));
        Assert.Equal(3, list.Len());
        Assert.Equal(3, list.Size());
        Assert.Equal(1, list.Min());
        Assert.Equal(3, list.Max());
        Assert.Equal(1, list.ByRank(0));
        Assert.Null(list.Rank(99));
        Assert.True(list.CurrentMax() >= 1);
        Assert.Equal(
            [
                new KeyValuePair<int, string>(1, "one"),
                new KeyValuePair<int, string>(2, "two-updated"),
                new KeyValuePair<int, string>(3, "three")
            ],
            list.Entries());
        Assert.True(list.Delete(3));
        Assert.False(list.Delete(3));
    }
}
