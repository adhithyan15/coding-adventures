using System.Collections;
using CodingAdventures.HashMap;

namespace CodingAdventures.HashSet;

public sealed class HashSet<T> : IReadOnlyCollection<T>, IEquatable<HashSet<T>>
    where T : notnull
{
    private readonly HashMap<T, bool> _map;

    public HashSet()
    {
        _map = new HashMap<T, bool>();
    }

    public HashSet(IEnumerable<T> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);
        var map = new HashMap<T, bool>();
        foreach (var entry in entries)
        {
            map = map.Set(entry, true);
        }

        _map = map;
    }

    private HashSet(HashMap<T, bool> map)
    {
        _map = map;
    }

    public int Count => _map.Size;
    public int Size => _map.Size;

    public static HashSet<T> FromList(IEnumerable<T> entries) => new(entries);

    public HashSet<T> Clone() => new(_map.Clone());

    public HashSet<T> Add(T value) => new(_map.Set(value, true));

    public HashSet<T> Remove(T value) => new(_map.Delete(value));

    public HashSet<T> Discard(T value) => Remove(value);

    public bool Has(T value) => _map.Has(value);

    public bool Contains(T value) => Has(value);

    public int Len() => Size;

    public bool IsEmpty() => Size == 0;

    public List<T> ToList() => _map.Keys();

    public HashSet<T> Union(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return FromList(ToList().Concat(other.ToList()));
    }

    public HashSet<T> Intersection(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return FromList(ToList().Where(other.Has));
    }

    public HashSet<T> Difference(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return FromList(ToList().Where(value => !other.Has(value)));
    }

    public HashSet<T> SymmetricDifference(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        var left = ToList().Where(value => !other.Has(value));
        var right = other.ToList().Where(value => !Has(value));
        return FromList(left.Concat(right));
    }

    public bool IsSubset(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return this.All(other.Has);
    }

    public bool IsSuperset(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return other.IsSubset(this);
    }

    public bool IsDisjoint(HashSet<T> other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return this.All(value => !other.Has(value));
    }

    public bool Equals(HashSet<T>? other) => other is not null && Size == other.Size && IsSubset(other);

    public override bool Equals(object? obj) => obj is HashSet<T> other && Equals(other);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        foreach (var value in ToList())
        {
            hash.Add(value);
        }

        return hash.ToHashCode();
    }

    public IEnumerator<T> GetEnumerator() => ToList().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}
