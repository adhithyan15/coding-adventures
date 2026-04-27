using System.Collections;

namespace CodingAdventures.HashMap;

public sealed class HashMap<TKey, TValue> : IReadOnlyCollection<KeyValuePair<TKey, TValue>>
    where TKey : notnull
{
    private readonly Dictionary<TKey, TValue> _map;

    public HashMap()
    {
        _map = new Dictionary<TKey, TValue>();
    }

    public HashMap(IEnumerable<KeyValuePair<TKey, TValue>> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);
        _map = new Dictionary<TKey, TValue>();
        foreach (var entry in entries)
        {
            _map[entry.Key] = entry.Value;
        }
    }

    private HashMap(Dictionary<TKey, TValue> map)
    {
        _map = map;
    }

    public int Count => _map.Count;
    public int Size => _map.Count;

    public static HashMap<TKey, TValue> FromEntries(IEnumerable<KeyValuePair<TKey, TValue>> entries) => new(entries);

    public HashMap<TKey, TValue> Clone() => new(new Dictionary<TKey, TValue>(_map));

    public TValue? Get(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return _map.TryGetValue(key, out var value) ? value : default;
    }

    public bool Has(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return _map.ContainsKey(key);
    }

    public HashMap<TKey, TValue> Set(TKey key, TValue value)
    {
        ArgumentNullException.ThrowIfNull(key);
        var next = new Dictionary<TKey, TValue>(_map)
        {
            [key] = value
        };
        return new HashMap<TKey, TValue>(next);
    }

    public HashMap<TKey, TValue> Delete(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);
        var next = new Dictionary<TKey, TValue>(_map);
        next.Remove(key);
        return new HashMap<TKey, TValue>(next);
    }

    public HashMap<TKey, TValue> Clear() => new();

    public List<TKey> Keys() => [.. _map.Keys];

    public List<TValue> Values() => [.. _map.Values];

    public List<KeyValuePair<TKey, TValue>> Entries() => [.. _map];

    public Dictionary<TKey, TValue> ToDictionary() => new(_map);

    public IEnumerator<KeyValuePair<TKey, TValue>> GetEnumerator() => _map.GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}
