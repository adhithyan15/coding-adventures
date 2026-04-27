using System.Collections;

namespace CodingAdventures.SkipList;

public delegate int Comparator<in T>(T left, T right);

public static class SkipListComparators
{
    public static int Default<T>(T left, T right) => Comparer<T>.Default.Compare(left, right);
}

public sealed class SkipList<TKey, TValue> : IEnumerable<TKey>
{
    private readonly Comparator<TKey> _comparator;
    private readonly int _maxLevel;
    private readonly double _probability;
    private readonly List<KeyValuePair<TKey, TValue>> _items = [];

    public SkipList(Comparator<TKey>? comparator = null, int maxLevel = 32, double probability = 0.5)
    {
        _comparator = comparator ?? SkipListComparators.Default;
        _maxLevel = Math.Max(1, double.IsFinite(maxLevel) ? maxLevel : 32);
        _probability = double.IsFinite(probability) && probability > 0 && probability < 1 ? probability : 0.5;
    }

    public static SkipList<TKey, TValue> WithParams(int maxLevel = 32, double probability = 0.5, Comparator<TKey>? comparator = null) =>
        new(comparator, maxLevel, probability);

    public void Insert(TKey key, TValue value)
    {
        var index = FindInsertIndex(key);
        if (index < _items.Count && _comparator(_items[index].Key, key) == 0)
        {
            _items[index] = new KeyValuePair<TKey, TValue>(key, value);
            return;
        }

        _items.Insert(index, new KeyValuePair<TKey, TValue>(key, value));
    }

    public bool Delete(TKey key)
    {
        var index = FindIndex(key);
        if (index < 0)
        {
            return false;
        }

        _items.RemoveAt(index);
        return true;
    }

    public TValue? Search(TKey key)
    {
        var index = FindIndex(key);
        return index < 0 ? default : _items[index].Value;
    }

    public bool Contains(TKey key) => FindIndex(key) >= 0;

    public bool ContainsKey(TKey key) => Contains(key);

    public int? Rank(TKey key)
    {
        var index = FindIndex(key);
        return index < 0 ? null : index;
    }

    public TKey? ByRank(int rank)
    {
        if (rank < 0 || rank >= _items.Count)
        {
            return default;
        }

        return _items[rank].Key;
    }

    public List<KeyValuePair<TKey, TValue>> RangeQuery(TKey low, TKey high, bool inclusive) => Range(low, high, inclusive);

    public List<KeyValuePair<TKey, TValue>> Range(TKey low, TKey high, bool inclusive)
    {
        if (_comparator(low, high) > 0)
        {
            return [];
        }

        bool Lower(TKey value) => inclusive ? _comparator(value, low) >= 0 : _comparator(value, low) > 0;
        bool Upper(TKey value) => inclusive ? _comparator(value, high) <= 0 : _comparator(value, high) < 0;
        return _items.Where(entry => Lower(entry.Key) && Upper(entry.Key)).ToList();
    }

    public List<TKey> ToList() => _items.Select(entry => entry.Key).ToList();

    public List<KeyValuePair<TKey, TValue>> EntriesList() => [.. _items];

    public List<KeyValuePair<TKey, TValue>> Entries() => EntriesList();

    public TKey? Min() => _items.Count == 0 ? default : _items[0].Key;

    public TKey? Max() => _items.Count == 0 ? default : _items[^1].Key;

    public int Len() => _items.Count;

    public int Size() => Len();

    public bool IsEmpty() => _items.Count == 0;

    public int MaxLevel() => _maxLevel;

    public double Probability() => _probability;

    public int CurrentMax() => EstimatedCurrentMax();

    public IEnumerator<TKey> GetEnumerator() => ToList().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    private int FindIndex(TKey key)
    {
        var low = 0;
        var high = _items.Count - 1;
        while (low <= high)
        {
            var mid = (low + high) / 2;
            var cmp = _comparator(_items[mid].Key, key);
            if (cmp == 0)
            {
                return mid;
            }

            if (cmp < 0)
            {
                low = mid + 1;
            }
            else
            {
                high = mid - 1;
            }
        }

        return -1;
    }

    private int FindInsertIndex(TKey key)
    {
        var low = 0;
        var high = _items.Count;
        while (low < high)
        {
            var mid = (low + high) / 2;
            if (_comparator(_items[mid].Key, key) < 0)
            {
                low = mid + 1;
            }
            else
            {
                high = mid;
            }
        }

        return low;
    }

    private int EstimatedCurrentMax()
    {
        if (_items.Count == 0)
        {
            return 1;
        }

        var levels = (int)Math.Ceiling(Math.Log(_items.Count) / Math.Log(1 / _probability));
        return Math.Min(_maxLevel, Math.Max(1, levels));
    }
}
