namespace CodingAdventures.BloomFilter.Tests;

public sealed class BloomFilterTests
{
    [Fact]
    public void ConstructorSizesFilter()
    {
        var filter = new BloomFilter<string>(1000, 0.01);

        Assert.True(filter.BitCount > 8000);
        Assert.True(filter.HashCount >= 5);
        Assert.Equal(0, filter.BitsSet);
        Assert.Equal(0, filter.Count);
        Assert.True(filter.SizeBytes * 8 >= filter.BitCount);
    }

    [Fact]
    public void ExplicitConstructorSetsParameters()
    {
        var filter = new BloomFilter<string>(1000, 5, explicitParameters: true);

        Assert.Equal(1000, filter.BitCount);
        Assert.Equal(5, filter.HashCount);
        Assert.False(filter.IsOverCapacity);
    }

    [Fact]
    public void AddedElementsAreAlwaysFound()
    {
        var filter = new BloomFilter<string>(1000, 0.01);
        var words = new[] { "apple", "banana", "cherry", "date", "elderberry" };

        foreach (var word in words)
        {
            filter.Add(word);
        }

        foreach (var word in words)
        {
            Assert.True(filter.Contains(word));
        }

        Assert.Equal(words.Length, filter.Size);
        Assert.True(filter.BitsSet > 0);
        Assert.InRange(filter.FillRatio, 0.0, 1.0);
        Assert.True(filter.EstimatedFalsePositiveRate > 0.0);
    }

    [Fact]
    public void EmptyFilterRejectsMissingItem()
    {
        var filter = new BloomFilter<string>(1000, 0.01);

        Assert.False(filter.Contains("not-added"));
        Assert.Equal(0.0, filter.FillRatio);
        Assert.Equal(0.0, filter.EstimatedFalsePositiveRate);
    }

    [Fact]
    public void WorksWithNonStringValues()
    {
        var filter = new BloomFilter<int>(1000, 0.01);
        filter.Add(42);
        filter.Add(100);

        Assert.True(filter.Contains(42));
        Assert.True(filter.Contains(100));
    }

    [Fact]
    public void OverCapacityTracksAutoSizedFiltersOnly()
    {
        var auto = new BloomFilter<string>(2, 0.1);
        auto.Add("a");
        auto.Add("b");
        Assert.False(auto.IsOverCapacity);
        auto.Add("c");
        Assert.True(auto.IsOverCapacity);

        var explicitFilter = new BloomFilter<string>(16, 2, true);
        for (var i = 0; i < 20; i++)
        {
            explicitFilter.Add(i.ToString());
        }

        Assert.False(explicitFilter.IsOverCapacity);
    }

    [Fact]
    public void UtilityFunctionsMatchExpectedShape()
    {
        var m100 = BloomFilter<string>.OptimalM(100, 0.01);
        var m1000 = BloomFilter<string>.OptimalM(1000, 0.01);
        Assert.True(m1000 > m100);
        Assert.True(BloomFilter<string>.OptimalM(1000, 0.001) > m1000);
        Assert.True(BloomFilter<string>.OptimalK(100, 1000) >= 1);
        Assert.True(BloomFilter<string>.CapacityForMemory(1_000_000, 0.01) > 0);
    }

    [Fact]
    public void InvalidArgumentsAreRejected()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BloomFilter<string>(0, 0.01));
        Assert.Throws<ArgumentOutOfRangeException>(() => new BloomFilter<string>(100, 0.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new BloomFilter<string>(0, 5, true));
        Assert.Throws<ArgumentOutOfRangeException>(() => new BloomFilter<string>(100, 0, true));
        Assert.Throws<ArgumentOutOfRangeException>(() => BloomFilter<string>.OptimalM(0, 0.01));
        Assert.Throws<ArgumentOutOfRangeException>(() => BloomFilter<string>.OptimalM(100, 1.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => BloomFilter<string>.OptimalK(100, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => BloomFilter<string>.CapacityForMemory(0, 0.01));
        Assert.Throws<ArgumentNullException>(() => new BloomFilter<string>(10, 0.1).Add(null!));
    }

    [Fact]
    public void ToStringIncludesKeyFields()
    {
        var filter = new BloomFilter<string>(100, 0.01);
        filter.Add("a");

        var text = filter.ToString();
        Assert.Contains("BloomFilter", text);
        Assert.Contains("m=", text);
        Assert.Contains("k=", text);
        Assert.Contains("~fp=", text);
    }
}
