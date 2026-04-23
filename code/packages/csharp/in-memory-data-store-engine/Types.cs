using CodingAdventures.HashMap;
using CodingAdventures.HashSet;
using CodingAdventures.Heap;
using CodingAdventures.HyperLogLog;
using CodingAdventures.SkipList;

namespace CodingAdventures.InMemoryDataStoreEngine;

public enum EntryType
{
    String,
    Hash,
    List,
    Set,
    ZSet,
    Hll
}

public sealed record Entry(EntryType EntryType, object Value, long? ExpiresAt = null);

public sealed record SortedEntry(double Score, string Member);

public sealed class SortedSet
{
    private static readonly CodingAdventures.SkipList.Comparator<SortedEntry> Comparator = CompareEntries;

    private HashMap<string, double> _members = new();
    private SkipList<SortedEntry, byte> _ordering = new(Comparator);

    public int Length => _members.Size;

    public bool IsEmpty() => Length == 0;

    public SortedSet Clone()
    {
        var clone = new SortedSet();
        foreach (var (member, score) in OrderedEntries())
        {
            clone.Insert(score, member);
        }

        return clone;
    }

    public bool Contains(string member) => _members.Has(member);

    public double? Score(string member) => _members.Has(member) ? _members.Get(member) : null;

    public bool Insert(double score, string member)
    {
        if (double.IsNaN(score))
        {
            throw new InvalidOperationException("sorted set score cannot be NaN");
        }

        var isNew = !_members.Has(member);
        if (_members.Has(member))
        {
            var existing = _members.Get(member)!;
            _ordering.Delete(new SortedEntry(existing, member));
        }

        _members = _members.Set(member, score);
        _ordering.Insert(new SortedEntry(score, member), 0);
        return isNew;
    }

    public bool Remove(string member)
    {
        if (!_members.Has(member))
        {
            return false;
        }

        var score = _members.Get(member)!;
        _ordering.Delete(new SortedEntry(score, member));
        _members = _members.Delete(member);
        return true;
    }

    public int? Rank(string member)
    {
        var index = 0;
        foreach (var entry in _ordering)
        {
            if (entry.Member == member)
            {
                return index;
            }

            index += 1;
        }

        return null;
    }

    public List<KeyValuePair<string, double>> OrderedEntries() =>
        _ordering.EntriesList().Select(entry => new KeyValuePair<string, double>(entry.Key.Member, entry.Key.Score)).ToList();

    public List<KeyValuePair<string, double>> RangeByIndex(int start, int end)
    {
        var entries = OrderedEntries();
        if (entries.Count == 0)
        {
            return [];
        }

        var length = entries.Count;
        var normalizedStart = start < 0 ? length + start : start;
        var normalizedEnd = end < 0 ? length + end : end;

        if (normalizedStart < 0 || normalizedEnd < 0 || normalizedStart >= length || normalizedStart > normalizedEnd)
        {
            return [];
        }

        var count = normalizedEnd - normalizedStart + 1;
        return entries.Skip(normalizedStart).Take(count).ToList();
    }

    public List<KeyValuePair<string, double>> RangeByScore(double min, double max)
    {
        if (double.IsNaN(min) || double.IsNaN(max))
        {
            throw new InvalidOperationException("sorted set score cannot be NaN");
        }

        return OrderedEntries().Where(entry => entry.Value >= min && entry.Value <= max).ToList();
    }

    private static int CompareEntries(SortedEntry left, SortedEntry right)
    {
        var scoreComparison = left.Score.CompareTo(right.Score);
        return scoreComparison != 0 ? scoreComparison : string.CompareOrdinal(left.Member, right.Member);
    }
}

public static class DataStoreTypes
{
    public static Entry StringEntry(string value, long? expiresAt = null) => new(EntryType.String, value, expiresAt);

    public static Entry HashEntry(HashMap<string, string> value, long? expiresAt = null) => new(EntryType.Hash, value, expiresAt);

    public static Entry ListEntry(List<string> value, long? expiresAt = null) => new(EntryType.List, value, expiresAt);

    public static Entry SetEntry(CodingAdventures.HashSet.HashSet<string> value, long? expiresAt = null) => new(EntryType.Set, value, expiresAt);

    public static Entry ZSetEntry(SortedSet value, long? expiresAt = null) => new(EntryType.ZSet, value, expiresAt);

    public static Entry HllEntry(HyperLogLog.HyperLogLog value, long? expiresAt = null) => new(EntryType.Hll, value, expiresAt);

    public static Entry CloneEntry(Entry entry) => entry.EntryType switch
    {
        EntryType.String => StringEntry((string)entry.Value, entry.ExpiresAt),
        EntryType.Hash => HashEntry(((HashMap<string, string>)entry.Value).Clone(), entry.ExpiresAt),
        EntryType.List => ListEntry([.. (List<string>)entry.Value], entry.ExpiresAt),
        EntryType.Set => SetEntry(((CodingAdventures.HashSet.HashSet<string>)entry.Value).Clone(), entry.ExpiresAt),
        EntryType.ZSet => ZSetEntry(((SortedSet)entry.Value).Clone(), entry.ExpiresAt),
        EntryType.Hll => HllEntry(((HyperLogLog.HyperLogLog)entry.Value).Clone(), entry.ExpiresAt),
        _ => throw new InvalidOperationException($"Unsupported entry type {entry.EntryType}")
    };

    public static string EntryTypeName(EntryType type) => type switch
    {
        EntryType.String => "string",
        EntryType.Hash => "hash",
        EntryType.List => "list",
        EntryType.Set => "set",
        EntryType.ZSet => "zset",
        EntryType.Hll => "hll",
        _ => "none"
    };

    public static MinHeap<(long ExpiresAt, string Key)> CreateExpiryHeap(IEnumerable<(long ExpiresAt, string Key)>? entries = null) =>
        MinHeap<(long ExpiresAt, string Key)>.FromEnumerable(entries ?? [], CompareExpiry);

    public static int CompareExpiry((long ExpiresAt, string Key) left, (long ExpiresAt, string Key) right)
    {
        var comparison = left.ExpiresAt.CompareTo(right.ExpiresAt);
        return comparison != 0 ? comparison : string.CompareOrdinal(left.Key, right.Key);
    }
}
