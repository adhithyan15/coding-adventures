namespace CodingAdventures.Heap.Tests;

public sealed class HeapTests
{
    [Fact]
    public void MinHeapPushesAndPopsInAscendingOrder()
    {
        var heap = new MinHeap<int>();
        foreach (var value in new[] { 5, 3, 8, 1, 4 })
        {
            heap.Push(value);
        }

        Assert.Equal(1, heap.Peek());
        Assert.Equal([1, 3, 4, 5, 8], PopAll(heap));
    }

    [Fact]
    public void MaxHeapPushesAndPopsInDescendingOrder()
    {
        var heap = new MaxHeap<int>();
        foreach (var value in new[] { 5, 3, 8, 1, 4 })
        {
            heap.Push(value);
        }

        Assert.Equal(8, heap.Peek());
        Assert.Equal([8, 5, 4, 3, 1], PopAll(heap));
    }

    [Fact]
    public void ThrowsOnEmptyPopAndPeek()
    {
        var heap = new MinHeap<int>();
        Assert.Throws<InvalidOperationException>(() => heap.Pop());
        Assert.Throws<InvalidOperationException>(() => heap.Peek());
    }

    [Fact]
    public void BuildsFromEnumerableAndPreservesAllElements()
    {
        var heap = MinHeap<int>.FromEnumerable([9, 2, 7, 1, 5]);
        Assert.Equal([1, 2, 5, 7, 9], PopAll(heap));
    }

    [Fact]
    public void PureFunctionsCoverSortingAndTopK()
    {
        var values = new[] { 3, 1, 4, 1, 5, 9, 2, 6 };
        Assert.Equal([1, 1, 2, 3, 4, 5, 6, 9], HeapFunctions.HeapSort(values));
        Assert.Equal([9, 6, 5], HeapFunctions.NLargest(values, 3));
        Assert.Equal([1, 1, 2], HeapFunctions.NSmallest(values, 3));
    }

    [Fact]
    public void SupportsCustomComparators()
    {
        Comparator<string> comparator = (left, right) => left.Length.CompareTo(right.Length);
        var heap = MinHeap<string>.FromEnumerable(["aaaa", "bb", "c", "ddd"], comparator);
        Assert.Equal(["c", "bb", "ddd", "aaaa"], PopAll(heap));
    }

    private static List<int> PopAll(MinHeap<int> heap)
    {
        var values = new List<int>();
        while (!heap.IsEmpty())
        {
            values.Add(heap.Pop());
        }

        return values;
    }

    private static List<int> PopAll(MaxHeap<int> heap)
    {
        var values = new List<int>();
        while (!heap.IsEmpty())
        {
            values.Add(heap.Pop());
        }

        return values;
    }

    private static List<string> PopAll(MinHeap<string> heap)
    {
        var values = new List<string>();
        while (!heap.IsEmpty())
        {
            values.Add(heap.Pop());
        }

        return values;
    }
}
