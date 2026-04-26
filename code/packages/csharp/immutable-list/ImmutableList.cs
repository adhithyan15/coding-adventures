using System.Collections;
using System.Diagnostics.CodeAnalysis;

namespace CodingAdventures.ImmutableList;

/// <summary>
/// A persistent list where updates return new list instances and leave prior versions unchanged.
/// </summary>
public sealed class ImmutableList<T> : IReadOnlyList<T>
{
    private static readonly T[] EmptyItems = [];
    private readonly T[] _items;

    public ImmutableList()
        : this(EmptyItems)
    {
    }

    private ImmutableList(T[] items)
    {
        _items = items;
    }

    public static ImmutableList<T> Empty { get; } = new(EmptyItems);

    public static ImmutableList<T> FromEnumerable(IEnumerable<T> items)
    {
        ArgumentNullException.ThrowIfNull(items);

        var array = items.ToArray();
        return array.Length == 0 ? Empty : new ImmutableList<T>(array);
    }

    public static ImmutableList<T> FromSlice(IEnumerable<T> items) => FromEnumerable(items);

    public int Count => _items.Length;

    public int Length => _items.Length;

    public bool IsEmpty => _items.Length == 0;

    public T this[int index]
    {
        get
        {
            EnsureIndex(index);
            return _items[index];
        }
    }

    [return: MaybeNull]
    public T Get(int index) => IsIndexInRange(index) ? _items[index] : default;

    public bool TryGet(int index, [MaybeNullWhen(false)] out T value)
    {
        if (IsIndexInRange(index))
        {
            value = _items[index];
            return true;
        }

        value = default;
        return false;
    }

    public ImmutableList<T> Push(T value)
    {
        var next = new T[_items.Length + 1];
        Array.Copy(_items, next, _items.Length);
        next[^1] = value;
        return new ImmutableList<T>(next);
    }

    public ImmutableList<T> Set(int index, T value)
    {
        EnsureIndex(index);

        var next = ToArray();
        next[index] = value;
        return new ImmutableList<T>(next);
    }

    public (ImmutableList<T> List, T Value) Pop()
    {
        if (_items.Length == 0)
        {
            throw new InvalidOperationException("Cannot pop from an empty list.");
        }

        var value = _items[^1];
        if (_items.Length == 1)
        {
            return (Empty, value);
        }

        var next = new T[_items.Length - 1];
        Array.Copy(_items, next, next.Length);
        return (new ImmutableList<T>(next), value);
    }

    public T[] ToArray()
    {
        var copy = new T[_items.Length];
        Array.Copy(_items, copy, _items.Length);
        return copy;
    }

    public List<T> ToList() => new(_items);

    public IEnumerator<T> GetEnumerator() => ((IEnumerable<T>)_items).GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString() => $"ImmutableList(count={_items.Length})";

    private void EnsureIndex(int index)
    {
        if (!IsIndexInRange(index))
        {
            throw new ArgumentOutOfRangeException(nameof(index), "Index is outside the bounds of the list.");
        }
    }

    private bool IsIndexInRange(int index) => index >= 0 && index < _items.Length;
}
