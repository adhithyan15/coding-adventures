namespace CodingAdventures.Heap;

public delegate int Comparator<in T>(T left, T right);

public static class HeapComparators
{
    public static int Default<T>(T left, T right) => Comparer<T>.Default.Compare(left, right);
}

public abstract class Heap<T>
{
    private readonly List<T> _data = [];

    protected Heap(Comparator<T>? comparator = null)
    {
        Comparator = comparator ?? HeapComparators.Default;
    }

    protected Comparator<T> Comparator { get; }

    protected abstract bool HigherPriority(T left, T right);

    public int Size => _data.Count;

    public void Push(T value)
    {
        _data.Add(value);
        SiftUp(_data.Count - 1);
    }

    public T Pop()
    {
        if (_data.Count == 0)
        {
            throw new InvalidOperationException("pop from an empty heap");
        }

        var root = _data[0];
        var last = _data[^1];
        _data.RemoveAt(_data.Count - 1);
        if (_data.Count > 0)
        {
            _data[0] = last;
            SiftDown(0);
        }

        return root;
    }

    public T Peek()
    {
        if (_data.Count == 0)
        {
            throw new InvalidOperationException("peek at an empty heap");
        }

        return _data[0];
    }

    public bool IsEmpty() => _data.Count == 0;

    public List<T> ToArray() => [.. _data];

    public override string ToString()
    {
        var root = _data.Count == 0 ? "empty" : _data[0]?.ToString() ?? "null";
        return $"{GetType().Name}(size={Size}, root={root})";
    }

    protected void BuildFromEnumerable(IEnumerable<T> items)
    {
        _data.Clear();
        _data.AddRange(items);
        for (var i = (_data.Count - 2) / 2; i >= 0; i--)
        {
            SiftDown(i);
            if (i == 0)
            {
                break;
            }
        }
    }

    private void SiftUp(int index)
    {
        var currentIndex = index;
        while (currentIndex > 0)
        {
            var parentIndex = (currentIndex - 1) / 2;
            if (!HigherPriority(_data[currentIndex], _data[parentIndex]))
            {
                break;
            }

            (_data[currentIndex], _data[parentIndex]) = (_data[parentIndex], _data[currentIndex]);
            currentIndex = parentIndex;
        }
    }

    private void SiftDown(int index)
    {
        var currentIndex = index;
        while (true)
        {
            var best = currentIndex;
            var left = (2 * currentIndex) + 1;
            var right = left + 1;

            if (left < _data.Count && HigherPriority(_data[left], _data[best]))
            {
                best = left;
            }

            if (right < _data.Count && HigherPriority(_data[right], _data[best]))
            {
                best = right;
            }

            if (best == currentIndex)
            {
                return;
            }

            (_data[currentIndex], _data[best]) = (_data[best], _data[currentIndex]);
            currentIndex = best;
        }
    }
}

public sealed class MinHeap<T> : Heap<T>
{
    public MinHeap(Comparator<T>? comparator = null)
        : base(comparator)
    {
    }

    public static MinHeap<T> FromEnumerable(IEnumerable<T> items, Comparator<T>? comparator = null)
    {
        var heap = new MinHeap<T>(comparator);
        heap.BuildFromEnumerable(items);
        return heap;
    }

    protected override bool HigherPriority(T left, T right) => Comparator(left, right) < 0;
}

public sealed class MaxHeap<T> : Heap<T>
{
    public MaxHeap(Comparator<T>? comparator = null)
        : base(comparator)
    {
    }

    public static MaxHeap<T> FromEnumerable(IEnumerable<T> items, Comparator<T>? comparator = null)
    {
        var heap = new MaxHeap<T>(comparator);
        heap.BuildFromEnumerable(items);
        return heap;
    }

    protected override bool HigherPriority(T left, T right) => Comparator(left, right) > 0;
}

public static class HeapFunctions
{
    public static List<T> Heapify<T>(IEnumerable<T> values, Comparator<T>? comparator = null) =>
        MinHeap<T>.FromEnumerable(values, comparator).ToArray();

    public static List<T> HeapSort<T>(IEnumerable<T> values, Comparator<T>? comparator = null)
    {
        var heap = MinHeap<T>.FromEnumerable(values, comparator);
        var result = new List<T>();
        while (!heap.IsEmpty())
        {
            result.Add(heap.Pop());
        }

        return result;
    }

    public static List<T> NLargest<T>(IEnumerable<T> iterable, int count, Comparator<T>? comparator = null)
    {
        if (count <= 0)
        {
            return [];
        }

        comparator ??= HeapComparators.Default;
        var items = iterable.ToList();
        if (count >= items.Count)
        {
            return [.. items.OrderByDescending(value => value, Comparer<T>.Create((left, right) => comparator(right, left)))];
        }

        var heap = MinHeap<T>.FromEnumerable(items.Take(count), comparator);
        foreach (var value in items.Skip(count))
        {
            if (comparator(value, heap.Peek()) > 0)
            {
                heap.Pop();
                heap.Push(value);
            }
        }

        var result = new List<T>();
        while (!heap.IsEmpty())
        {
            result.Add(heap.Pop());
        }

        result.Sort((left, right) => comparator(right, left));
        return result;
    }

    public static List<T> NSmallest<T>(IEnumerable<T> iterable, int count, Comparator<T>? comparator = null)
    {
        if (count <= 0)
        {
            return [];
        }

        comparator ??= HeapComparators.Default;
        var items = iterable.ToList();
        if (count >= items.Count)
        {
            var copy = items.ToList();
            copy.Sort((left, right) => comparator(left, right));
            return copy;
        }

        var heap = MaxHeap<T>.FromEnumerable(items.Take(count), comparator);
        foreach (var value in items.Skip(count))
        {
            if (comparator(value, heap.Peek()) < 0)
            {
                heap.Pop();
                heap.Push(value);
            }
        }

        var result = new List<T>();
        while (!heap.IsEmpty())
        {
            result.Add(heap.Pop());
        }

        result.Sort((left, right) => comparator(left, right));
        return result;
    }
}
