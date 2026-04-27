using System.Collections;

namespace CodingAdventures.TreeSet;

/// <summary>
/// Sorted set with range queries, order helpers, and set algebra.
/// </summary>
public sealed class TreeSet<T> : IReadOnlyCollection<T>, IEquatable<TreeSet<T>>
    where T : IComparable<T>
{
    private readonly SortedSet<T> _values;

    public TreeSet()
    {
        _values = new SortedSet<T>();
    }

    public TreeSet(IEnumerable<T> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        _values = new SortedSet<T>(values);
    }

    public TreeSet(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        _values = new SortedSet<T>(other._values);
    }

    public int Count => _values.Count;

    public int Size => _values.Count;

    public bool IsEmpty => _values.Count == 0;

    public TreeSet<T> Add(T value)
    {
        ArgumentNullException.ThrowIfNull(value);
        _values.Add(value);
        return this;
    }

    public bool Remove(T value) => value is not null && _values.Remove(value);

    public bool Delete(T value) => Remove(value);

    public bool Discard(T value) => Remove(value);

    public bool Contains(T value) => value is not null && _values.Contains(value);

    public bool Has(T value) => Contains(value);

    public T? Min() => _values.Count == 0 ? default : _values.Min;

    public T? Max() => _values.Count == 0 ? default : _values.Max;

    public T? First() => Min();

    public T? Last() => Max();

    public T? Predecessor(T value)
    {
        if (value is null)
        {
            return default;
        }

        T? candidate = default;
        foreach (var current in _values)
        {
            if (current.CompareTo(value) >= 0)
            {
                break;
            }

            candidate = current;
        }

        return candidate;
    }

    public T? Successor(T value)
    {
        if (value is null)
        {
            return default;
        }

        foreach (var current in _values)
        {
            if (current.CompareTo(value) > 0)
            {
                return current;
            }
        }

        return default;
    }

    public int Rank(T value)
    {
        if (value is null)
        {
            return 0;
        }

        var count = 0;
        foreach (var current in _values)
        {
            if (current.CompareTo(value) >= 0)
            {
                break;
            }

            count++;
        }

        return count;
    }

    public T? ByRank(int rank)
    {
        if (rank < 0 || rank >= _values.Count)
        {
            return default;
        }

        var index = 0;
        foreach (var value in _values)
        {
            if (index == rank)
            {
                return value;
            }

            index++;
        }

        return default;
    }

    public T? KthSmallest(int k) => k <= 0 ? default : ByRank(k - 1);

    public List<T> Range(T low, T high, bool inclusive = true)
    {
        ArgumentNullException.ThrowIfNull(low);
        ArgumentNullException.ThrowIfNull(high);

        if (low.CompareTo(high) > 0)
        {
            return [];
        }

        var result = new List<T>();
        foreach (var value in _values)
        {
            var lower = value.CompareTo(low);
            var upper = value.CompareTo(high);
            var inRange = inclusive ? lower >= 0 && upper <= 0 : lower > 0 && upper < 0;
            if (inRange)
            {
                result.Add(value);
            }
            else if (upper > 0)
            {
                break;
            }
        }

        return result;
    }

    public List<T> ToList() => [.. _values];

    public List<T> ToSortedArray() => ToList();

    public TreeSet<T> Union(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        var result = new TreeSet<T>(this);
        result._values.UnionWith(other._values);
        return result;
    }

    public TreeSet<T> Intersection(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        var result = new TreeSet<T>(this);
        result._values.IntersectWith(other._values);
        return result;
    }

    public TreeSet<T> Difference(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        var result = new TreeSet<T>(this);
        result._values.ExceptWith(other._values);
        return result;
    }

    public TreeSet<T> SymmetricDifference(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        var result = new TreeSet<T>(this);
        result._values.SymmetricExceptWith(other._values);
        return result;
    }

    public bool IsSubset(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return _values.IsSubsetOf(other._values);
    }

    public bool IsSuperset(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return _values.IsSupersetOf(other._values);
    }

    public bool IsDisjoint(TreeSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return !_values.Overlaps(other._values);
    }

    public bool Equals(TreeSet<T>? other) => other is not null && _values.SetEquals(other._values);

    public override bool Equals(object? obj) => obj is TreeSet<T> other && Equals(other);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        foreach (var value in _values)
        {
            hash.Add(value);
        }

        return hash.ToHashCode();
    }

    public IEnumerator<T> GetEnumerator() => _values.GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString() => $"TreeSet([{string.Join(", ", _values)}])";
}
