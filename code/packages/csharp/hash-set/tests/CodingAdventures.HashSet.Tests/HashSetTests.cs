namespace CodingAdventures.HashSet.Tests;

public sealed class HashSetTests
{
    [Fact]
    public void TracksMembershipAndClonesImmutably()
    {
        var baseSet = HashSet<string>.FromList(["alpha", "beta"]);
        var next = baseSet.Add("gamma");

        Assert.False(baseSet.Has("gamma"));
        Assert.True(next.Has("gamma"));
        Assert.Equal(2, baseSet.Size);
        Assert.Equal(3, next.Size);
    }

    [Fact]
    public void RemovesEntriesWithoutMutatingOriginal()
    {
        var baseSet = HashSet<string>.FromList(["alpha", "beta"]);
        var next = baseSet.Remove("alpha");

        Assert.True(baseSet.Has("alpha"));
        Assert.False(next.Has("alpha"));
    }

    [Fact]
    public void SupportsSetAlgebra()
    {
        var left = HashSet<string>.FromList(["alpha", "beta", "gamma"]);
        var right = HashSet<string>.FromList(["beta", "delta"]);

        Assert.Equal(["alpha", "beta", "delta", "gamma"], left.Union(right).ToList().OrderBy(value => value).ToList());
        Assert.Equal(["beta"], left.Intersection(right).ToList());
        Assert.Equal(["alpha", "gamma"], left.Difference(right).ToList().OrderBy(value => value).ToList());
        Assert.Equal(["alpha", "delta", "gamma"], left.SymmetricDifference(right).ToList().OrderBy(value => value).ToList());
    }

    [Fact]
    public void SupportsRelationHelpers()
    {
        var left = HashSet<string>.FromList(["alpha", "beta"]);
        var right = HashSet<string>.FromList(["alpha", "beta", "gamma"]);
        var disjoint = HashSet<string>.FromList(["delta"]);

        Assert.True(left.IsSubset(right));
        Assert.True(right.IsSuperset(left));
        Assert.True(left.IsDisjoint(disjoint));
        Assert.True(left.Equals(HashSet<string>.FromList(["beta", "alpha"])));
    }

    [Fact]
    public void ExposesUtilityMembersAndEqualityContracts()
    {
        var empty = new HashSet<string>();
        Assert.True(empty.IsEmpty());
        Assert.Equal(0, empty.Len());

        var set = empty.Add("alpha").Add("beta");
        var clone = set.Clone();
        var discarded = set.Discard("missing").Discard("alpha");

        Assert.True(set.Contains("alpha"));
        Assert.Equal(["alpha", "beta"], set.OrderBy(value => value).ToList());
        Assert.Equal(["alpha", "beta"], clone.ToList().OrderBy(value => value).ToList());
        Assert.Equal(["beta"], discarded.ToList());
        Assert.False(discarded.IsEmpty());

        Assert.True(set.Equals((object)HashSet<string>.FromList(["beta", "alpha"])));
        Assert.False(set.Equals((object?)null));
        Assert.Equal(set.GetHashCode(), HashSet<string>.FromList(["alpha", "beta"]).GetHashCode());
    }
}
