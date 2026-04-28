using CodingAdventures.ImmutableList;

namespace CodingAdventures.ImmutableList.Tests;

public sealed class ImmutableListTests
{
    [Fact]
    public void EmptyListHasExpectedShape()
    {
        var list = ImmutableList<string>.Empty;

        Assert.True(list.IsEmpty);
        Assert.True(list.Count == 0);
        Assert.Equal(0, list.Length);
        Assert.Null(list.Get(0));
        Assert.False(list.TryGet(0, out string? value));
        Assert.Null(value);
        Assert.Empty(list);
        Assert.Equal("ImmutableList(count=0)", list.ToString());
    }

    [Fact]
    public void PushReturnsNewListAndLeavesOriginalUnchanged()
    {
        var empty = ImmutableList<string>.Empty;
        var one = empty.Push("hello");
        var two = one.Push("world");

        Assert.Empty(empty);
        Assert.Equal(new[] { "hello" }, one.ToArray());
        Assert.Equal(new[] { "hello", "world" }, two.ToList());
        Assert.Equal("world", two[1]);
    }

    [Fact]
    public void FromEnumerableAndFromSlicePreserveOrder()
    {
        var list = ImmutableList<int>.FromEnumerable(new[] { 3, 1, 4 });
        var slice = ImmutableList<int>.FromSlice(new[] { 1, 5, 9 });

        Assert.Equal(new[] { 3, 1, 4 }, list.ToArray());
        Assert.Equal(new[] { 1, 5, 9 }, slice.ToArray());
        Assert.Same(ImmutableList<int>.Empty, ImmutableList<int>.FromEnumerable(Array.Empty<int>()));
        Assert.Throws<ArgumentNullException>(() => ImmutableList<int>.FromEnumerable(null!));
    }

    [Fact]
    public void SetReturnsChangedCopy()
    {
        var list = ImmutableList<string>.FromSlice(new[] { "a", "b", "c" });
        var updated = list.Set(1, "B");

        Assert.Equal(new[] { "a", "b", "c" }, list.ToArray());
        Assert.Equal(new[] { "a", "B", "c" }, updated.ToArray());
        Assert.Throws<ArgumentOutOfRangeException>(() => list.Set(-1, "x"));
        Assert.Throws<ArgumentOutOfRangeException>(() => list.Set(3, "x"));
    }

    [Fact]
    public void PopReturnsRemainderAndRemovedValue()
    {
        var list = ImmutableList<int>.FromSlice(new[] { 1, 2, 3 });
        var (two, removed) = list.Pop();
        var (one, secondRemoved) = two.Pop();
        var (empty, firstRemoved) = one.Pop();

        Assert.Equal(3, removed);
        Assert.Equal(2, secondRemoved);
        Assert.Equal(1, firstRemoved);
        Assert.Equal(new[] { 1, 2 }, two.ToArray());
        Assert.Same(ImmutableList<int>.Empty, empty);
        Assert.Throws<InvalidOperationException>(() => empty.Pop());
    }

    [Fact]
    public void GetTryGetAndIndexerHandleBounds()
    {
        var list = ImmutableList<int>.FromSlice(new[] { 10, 20 });

        Assert.Equal(10, list.Get(0));
        Assert.Equal(0, list.Get(20));
        Assert.True(list.TryGet(1, out var value));
        Assert.Equal(20, value);
        Assert.False(list.TryGet(-1, out value));
        Assert.Equal(0, value);
        Assert.Throws<ArgumentOutOfRangeException>(() => list[-1]);
        Assert.Throws<ArgumentOutOfRangeException>(() => list[2]);
    }

    [Fact]
    public void EnumerationUsesSnapshotOrder()
    {
        var list = new ImmutableList<string>().Push("a").Push("b").Push("c");

        Assert.Equal(new[] { "a", "b", "c" }, list.Select(item => item));
        Assert.Equal("ImmutableList(count=3)", list.ToString());
    }
}
