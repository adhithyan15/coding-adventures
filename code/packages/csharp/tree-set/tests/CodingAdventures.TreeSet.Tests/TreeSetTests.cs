using CodingAdventures.TreeSet;

namespace CodingAdventures.TreeSet.Tests;

public sealed class TreeSetTests
{
    [Fact]
    public void ConstructionAndMutationCollapseDuplicates()
    {
        var set = new TreeSet<int>([3, 1, 4, 1, 5]);

        set.Add(9).Add(2).Add(6).Add(5);

        Assert.Equal(7, set.Count);
        Assert.Equal(7, set.Size);
        Assert.False(set.IsEmpty);
        Assert.True(set.Contains(9));
        Assert.True(set.Has(3));
        Assert.False(set.Contains(8));
        Assert.True(set.Remove(4));
        Assert.True(set.Delete(9));
        Assert.False(set.Discard(99));
        Assert.Equal([1, 2, 3, 5, 6], set.ToList());
    }

    [Fact]
    public void MinMaxPredecessorSuccessorAndRankWork()
    {
        var set = new TreeSet<int>([10, 20, 30, 40, 50]);

        Assert.Equal(10, set.Min());
        Assert.Equal(50, set.Max());
        Assert.Equal(10, set.First());
        Assert.Equal(50, set.Last());
        Assert.Null(new TreeSet<string>().Min());
        Assert.Null(new TreeSet<string>(["alpha"]).Predecessor("alpha"));
        Assert.Equal(20, set.Predecessor(25));
        Assert.Equal(30, set.Successor(25));
        Assert.Null(new TreeSet<string>(["alpha"]).Successor("alpha"));
        Assert.Equal(2, set.Rank(30));
        Assert.Equal(1, set.Rank(15));
    }

    [Fact]
    public void ByRankKthAndRangeReturnSortedResults()
    {
        var set = new TreeSet<int>(Enumerable.Range(1, 10));

        Assert.Equal(1, set.ByRank(0));
        Assert.Equal(10, set.ByRank(9));
        var words = new TreeSet<string>(["alpha", "beta"]);
        Assert.Null(words.ByRank(-1));
        Assert.Null(words.ByRank(2));
        Assert.Equal(3, set.KthSmallest(3));
        Assert.Null(words.KthSmallest(0));
        Assert.Equal([3, 4, 5, 6, 7], set.Range(3, 7));
        Assert.Equal([4, 5, 6], set.Range(3, 7, inclusive: false));
        Assert.Empty(set.Range(7, 3));
        Assert.Equal(set.ToList(), set.ToSortedArray());
    }

    [Fact]
    public void SetAlgebraDoesNotMutateInputs()
    {
        var a = new TreeSet<int>([1, 2, 3, 4]);
        var b = new TreeSet<int>([3, 4, 5, 6]);

        Assert.Equal([1, 2, 3, 4, 5, 6], a.Union(b).ToList());
        Assert.Equal([3, 4], a.Intersection(b).ToList());
        Assert.Equal([1, 2], a.Difference(b).ToList());
        Assert.Equal([1, 2, 5, 6], a.SymmetricDifference(b).ToList());
        Assert.Equal([1, 2, 3, 4], a.ToList());
        Assert.Equal([3, 4, 5, 6], b.ToList());
    }

    [Fact]
    public void PredicatesAndEqualityCompareSetContents()
    {
        var small = new TreeSet<int>([2, 3]);
        var large = new TreeSet<int>([1, 2, 3, 4]);
        var disjoint = new TreeSet<int>([8, 9]);
        var same = new TreeSet<int>([3, 2]);

        Assert.True(small.IsSubset(large));
        Assert.True(large.IsSuperset(small));
        Assert.True(small.IsDisjoint(disjoint));
        Assert.False(small.IsDisjoint(large));
        Assert.Equal(small, same);
        Assert.Equal(small.GetHashCode(), same.GetHashCode());
        Assert.NotEqual(small, large);
    }

    [Fact]
    public void IterationAndToStringUseSortedOrder()
    {
        var set = new TreeSet<int>([5, 2, 8, 1, 9, 3]);

        Assert.Equal([1, 2, 3, 5, 8, 9], set.ToArray());
        Assert.Equal("TreeSet([1, 2, 3, 5, 8, 9])", set.ToString());
    }

    [Fact]
    public void StressMatchesFrameworkSortedSet()
    {
        var ours = new TreeSet<int>();
        var reference = new SortedSet<int>();
        var random = new Random(314);

        for (var i = 0; i < 500; i++)
        {
            var key = random.Next(300);
            ours.Add(key);
            reference.Add(key);
        }

        foreach (var key in reference.Take(200).ToList())
        {
            Assert.Equal(reference.Remove(key), ours.Remove(key));
        }

        for (var i = 0; i < 300; i++)
        {
            var key = random.Next(600);
            if (random.Next(2) == 0)
            {
                ours.Add(key);
                reference.Add(key);
            }
            else
            {
                Assert.Equal(reference.Remove(key), ours.Remove(key));
            }
        }

        Assert.Equal(reference, ours.ToList());
        Assert.Equal(reference.Min, ours.Min());
        Assert.Equal(reference.Max, ours.Max());
    }
}
