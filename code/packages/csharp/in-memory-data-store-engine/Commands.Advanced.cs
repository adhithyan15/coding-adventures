using CodingAdventures.HashSet;
using CodingAdventures.HyperLogLog;
using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;
using Hll = CodingAdventures.HyperLogLog.HyperLogLog;
using StringSet = CodingAdventures.HashSet.HashSet<string>;

namespace CodingAdventures.InMemoryDataStoreEngine;

public sealed partial class DataStoreEngine
{
    private static (Store, RespValue) CmdLPush(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("LPUSH"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is not null && entry.EntryType != EntryType.List)
        {
            return (store, WrongType());
        }

        List<string> list = entry?.EntryType == EntryType.List ? [.. (List<string>)entry.Value] : [];
        foreach (var value in args.Skip(1))
        {
            list.Insert(0, value);
        }

        return (store.Set(key, DataStoreTypes.ListEntry(list, entry?.ExpiresAt)), Resp.Integer(list.Count));
    }

    private static (Store, RespValue) CmdRPush(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("RPUSH"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is not null && entry.EntryType != EntryType.List)
        {
            return (store, WrongType());
        }

        List<string> list = entry?.EntryType == EntryType.List ? [.. (List<string>)entry.Value] : [];
        list.AddRange(args.Skip(1));
        return (store.Set(key, DataStoreTypes.ListEntry(list, entry?.ExpiresAt)), Resp.Integer(list.Count));
    }

    private static (Store, RespValue) CmdLPop(Store store, IReadOnlyList<string> args) => PopListValue(store, args, true, "LPOP");

    private static (Store, RespValue) CmdRPop(Store store, IReadOnlyList<string> args) => PopListValue(store, args, false, "RPOP");

    private static (Store, RespValue) CmdLLen(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("LLEN"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        return entry.EntryType != EntryType.List ? (store, WrongType()) : (store, Resp.Integer(((List<string>)entry.Value).Count));
    }

    private static (Store, RespValue) CmdLRange(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 3)
        {
            return (store, WrongArgCount("LRANGE"));
        }

        var start = ParseInteger(args[1]);
        var end = ParseInteger(args[2]);
        if (start is null || end is null)
        {
            return (store, InvalidInteger());
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.List)
        {
            return (store, WrongType());
        }

        var list = (List<string>)entry.Value;
        var length = list.Count;
        var normalizedStart = start.Value < 0 ? length + (int)start.Value : (int)start.Value;
        var normalizedEnd = end.Value < 0 ? length + (int)end.Value : (int)end.Value;
        if (normalizedStart < 0 || normalizedEnd < 0 || normalizedStart >= length || normalizedStart > normalizedEnd)
        {
            return (store, Resp.Array([]));
        }

        var values = list.Skip(normalizedStart).Take(normalizedEnd - normalizedStart + 1).Select(Resp.BulkString).Cast<RespValue>().ToList();
        return (store, Resp.Array(values));
    }

    private static (Store, RespValue) CmdLIndex(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("LINDEX"));
        }

        var index = ParseInteger(args[1]);
        if (index is null)
        {
            return (store, InvalidInteger());
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Nil());
        }

        if (entry.EntryType != EntryType.List)
        {
            return (store, WrongType());
        }

        var list = (List<string>)entry.Value;
        var normalized = index.Value < 0 ? list.Count + (int)index.Value : (int)index.Value;
        return normalized < 0 || normalized >= list.Count ? (store, Nil()) : (store, Resp.BulkString(list[normalized]));
    }

    private static (Store, RespValue) CmdSAdd(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("SADD"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is not null && entry.EntryType != EntryType.Set)
        {
            return (store, WrongType());
        }

        var set = entry?.EntryType == EntryType.Set ? ((StringSet)entry.Value).Clone() : new StringSet();
        long added = 0;
        foreach (var member in args.Skip(1))
        {
            if (!set.Has(member))
            {
                added += 1;
            }

            set = set.Add(member);
        }

        return (store.Set(key, DataStoreTypes.SetEntry(set, entry?.ExpiresAt)), Resp.Integer(added));
    }

    private static (Store, RespValue) CmdSRem(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("SREM"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        if (entry.EntryType != EntryType.Set)
        {
            return (store, WrongType());
        }

        var set = ((StringSet)entry.Value).Clone();
        long removed = 0;
        foreach (var member in args.Skip(1))
        {
            if (set.Has(member))
            {
                removed += 1;
                set = set.Remove(member);
            }
        }

        var next = set.IsEmpty() ? store.Delete(key) : store.Set(key, DataStoreTypes.SetEntry(set, entry.ExpiresAt));
        return (next, Resp.Integer(removed));
    }

    private static (Store, RespValue) CmdSIsMember(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("SISMEMBER"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        if (entry.EntryType != EntryType.Set)
        {
            return (store, WrongType());
        }

        return (store, Resp.Integer(((StringSet)entry.Value).Has(args[1]) ? 1 : 0));
    }

    private static (Store, RespValue) CmdSMembers(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("SMEMBERS"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.Set)
        {
            return (store, WrongType());
        }

        var values = ((StringSet)entry.Value).ToList().OrderBy(member => member, StringComparer.Ordinal).Select(Resp.BulkString).Cast<RespValue>().ToList();
        return (store, Resp.Array(values));
    }

    private static (Store, RespValue) CmdSCard(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("SCARD"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        return entry.EntryType != EntryType.Set ? (store, WrongType()) : (store, Resp.Integer(((StringSet)entry.Value).Size));
    }

    private static (Store, RespValue) CmdSUnion(Store store, IReadOnlyList<string> args) => ApplySetOperation(store, args, "SUNION", (current, next, first) => first ? next.Clone() : current.Union(next));

    private static (Store, RespValue) CmdSInter(Store store, IReadOnlyList<string> args) => ApplySetOperation(store, args, "SINTER", (current, next, first) => first ? next.Clone() : current.Intersection(next));

    private static (Store, RespValue) CmdSDiff(Store store, IReadOnlyList<string> args) => ApplySetOperation(store, args, "SDIFF", (current, next, first) => first ? next.Clone() : current.Difference(next));

    private static (Store, RespValue) CmdZAdd(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 3 || args.Count % 2 == 0)
        {
            return (store, WrongArgCount("ZADD"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is not null && entry.EntryType != EntryType.ZSet)
        {
            return (store, WrongType());
        }

        var zset = entry?.EntryType == EntryType.ZSet ? ((SortedSet)entry.Value).Clone() : new SortedSet();
        long added = 0;
        for (var i = 1; i < args.Count; i += 2)
        {
            var score = ParseFloatStrict(args[i]);
            if (score is null)
            {
                return (store, InvalidFloat());
            }

            if (zset.Insert(score.Value, args[i + 1]))
            {
                added += 1;
            }
        }

        return (store.Set(key, DataStoreTypes.ZSetEntry(zset, entry?.ExpiresAt)), Resp.Integer(added));
    }

    private static (Store, RespValue) CmdZRange(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 3 || args.Count > 4)
        {
            return (store, WrongArgCount("ZRANGE"));
        }

        var start = ParseInteger(args[1]);
        var end = ParseInteger(args[2]);
        if (start is null || end is null)
        {
            return (store, InvalidInteger());
        }

        var withScores = args.Count == 4 && args[3].Equals("WITHSCORES", StringComparison.OrdinalIgnoreCase);
        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.ZSet)
        {
            return (store, WrongType());
        }

        return (store, Resp.Array(FlattenZSetValues(((SortedSet)entry.Value).RangeByIndex((int)start.Value, (int)end.Value), withScores)));
    }

    private static (Store, RespValue) CmdZRangeByScore(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 3 || args.Count > 4)
        {
            return (store, WrongArgCount("ZRANGEBYSCORE"));
        }

        var min = ParseFloatStrict(args[1]);
        var max = ParseFloatStrict(args[2]);
        if (min is null || max is null)
        {
            return (store, InvalidFloat());
        }

        var withScores = args.Count == 4 && args[3].Equals("WITHSCORES", StringComparison.OrdinalIgnoreCase);
        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.ZSet)
        {
            return (store, WrongType());
        }

        return (store, Resp.Array(FlattenZSetValues(((SortedSet)entry.Value).RangeByScore(min.Value, max.Value), withScores)));
    }

    private static (Store, RespValue) CmdZRank(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("ZRANK"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Nil());
        }

        if (entry.EntryType != EntryType.ZSet)
        {
            return (store, WrongType());
        }

        var rank = ((SortedSet)entry.Value).Rank(args[1]);
        return rank is null ? (store, Nil()) : (store, Resp.Integer(rank.Value));
    }

    private static (Store, RespValue) CmdZScore(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("ZSCORE"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Nil());
        }

        if (entry.EntryType != EntryType.ZSet)
        {
            return (store, WrongType());
        }

        var score = ((SortedSet)entry.Value).Score(args[1]);
        return score is null ? (store, Nil()) : (store, Resp.BulkString(score.Value.ToString()));
    }

    private static (Store, RespValue) CmdZCard(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("ZCARD"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        return entry.EntryType != EntryType.ZSet ? (store, WrongType()) : (store, Resp.Integer(((SortedSet)entry.Value).Length));
    }

    private static (Store, RespValue) CmdZRem(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("ZREM"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        if (entry.EntryType != EntryType.ZSet)
        {
            return (store, WrongType());
        }

        var zset = ((SortedSet)entry.Value).Clone();
        long removed = 0;
        foreach (var member in args.Skip(1))
        {
            if (zset.Remove(member))
            {
                removed += 1;
            }
        }

        var next = zset.IsEmpty() ? store.Delete(key) : store.Set(key, DataStoreTypes.ZSetEntry(zset, entry.ExpiresAt));
        return (next, Resp.Integer(removed));
    }

    private static (Store, RespValue) CmdPfAdd(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("PFADD"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is not null && entry.EntryType != EntryType.Hll)
        {
            return (store, WrongType());
        }

        var hll = entry?.EntryType == EntryType.Hll ? ((Hll)entry.Value).Clone() : new Hll();
        var before = hll.Clone();
        foreach (var member in args.Skip(1))
        {
            hll.Add(member);
        }

        var changed = before.Equals(hll) ? 0 : 1;
        return (store.Set(key, DataStoreTypes.HllEntry(hll, entry?.ExpiresAt)), Resp.Integer(changed));
    }

    private static (Store, RespValue) CmdPfCount(Store store, IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            return (store, WrongArgCount("PFCOUNT"));
        }

        Hll? aggregate = null;
        foreach (var key in args)
        {
            var entry = store.Get(key);
            if (entry is null)
            {
                continue;
            }

            if (entry.EntryType != EntryType.Hll)
            {
                return (store, WrongType());
            }

            aggregate = aggregate is null ? ((Hll)entry.Value).Clone() : aggregate.Merge((Hll)entry.Value);
        }

        aggregate ??= new Hll();
        return (store, Resp.Integer(aggregate.Count()));
    }

    private static (Store, RespValue) CmdPfMerge(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("PFMERGE"));
        }

        var destination = args[0];
        Hll? merged = null;
        foreach (var key in args.Skip(1))
        {
            var entry = store.Get(key);
            if (entry is null)
            {
                continue;
            }

            if (entry.EntryType != EntryType.Hll)
            {
                return (store, WrongType());
            }

            merged = merged is null ? ((Hll)entry.Value).Clone() : merged.Merge((Hll)entry.Value);
        }

        merged ??= new Hll();
        return (store.Set(destination, DataStoreTypes.HllEntry(merged, store.Get(destination)?.ExpiresAt)), Resp.SimpleString("OK"));
    }

    private static (Store, RespValue) PopListValue(Store store, IReadOnlyList<string> args, bool popLeft, string commandName)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount(commandName));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is null)
        {
            return (store, Nil());
        }

        if (entry.EntryType != EntryType.List)
        {
            return (store, WrongType());
        }

        List<string> list = [.. (List<string>)entry.Value];
        var value = popLeft ? list[0] : list[^1];
        if (popLeft)
        {
            list.RemoveAt(0);
        }
        else
        {
            list.RemoveAt(list.Count - 1);
        }

        var next = list.Count == 0 ? store.Delete(key) : store.Set(key, DataStoreTypes.ListEntry(list, entry.ExpiresAt));
        return (next, Resp.BulkString(value));
    }

    private static (Store, RespValue) ApplySetOperation(Store store, IReadOnlyList<string> args, string commandName, Func<StringSet, StringSet, bool, StringSet> combine)
    {
        if (args.Count == 0)
        {
            return (store, WrongArgCount(commandName));
        }

        var result = new StringSet();
        var first = true;
        foreach (var key in args)
        {
            var entry = store.Get(key);
            var set = entry is null ? new StringSet() : entry.EntryType == EntryType.Set ? (StringSet)entry.Value : null;
            if (set is null)
            {
                return (store, WrongType());
            }

            result = combine(result, set, first);
            first = false;
        }

        var values = result.ToList().OrderBy(member => member, StringComparer.Ordinal).Select(Resp.BulkString).Cast<RespValue>().ToList();
        return (store, Resp.Array(values));
    }

    private static List<RespValue> FlattenZSetValues(IEnumerable<KeyValuePair<string, double>> values, bool withScores)
    {
        var response = new List<RespValue>();
        foreach (var (member, score) in values)
        {
            response.Add(Resp.BulkString(member));
            if (withScores)
            {
                response.Add(Resp.BulkString(score.ToString()));
            }
        }

        return response;
    }
}
