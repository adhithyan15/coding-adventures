using CodingAdventures.HashMap;
using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStoreEngine;

public sealed partial class DataStoreEngine
{
    partial void InstallDefaultCommands()
    {
        RegisterCommand("PING", CmdPing);
        RegisterCommand("ECHO", CmdEcho);
        RegisterCommand("SET", CmdSet);
        RegisterCommand("GET", CmdGet);
        RegisterCommand("DEL", CmdDel);
        RegisterCommand("EXISTS", CmdExists);
        RegisterCommand("TYPE", CmdType);
        RegisterCommand("RENAME", CmdRename);
        RegisterCommand("INCR", CmdIncr);
        RegisterCommand("DECR", CmdDecr);
        RegisterCommand("INCRBY", CmdIncrBy);
        RegisterCommand("DECRBY", CmdDecrBy);
        RegisterCommand("APPEND", CmdAppend);
        RegisterCommand("HSET", CmdHSet);
        RegisterCommand("HGET", CmdHGet);
        RegisterCommand("HDEL", CmdHDel);
        RegisterCommand("HGETALL", CmdHGetAll);
        RegisterCommand("HLEN", CmdHLen);
        RegisterCommand("HEXISTS", CmdHExists);
        RegisterCommand("HKEYS", CmdHKeys);
        RegisterCommand("HVALS", CmdHVals);
        RegisterCommand("LPUSH", CmdLPush);
        RegisterCommand("RPUSH", CmdRPush);
        RegisterCommand("LPOP", CmdLPop);
        RegisterCommand("RPOP", CmdRPop);
        RegisterCommand("LLEN", CmdLLen);
        RegisterCommand("LRANGE", CmdLRange);
        RegisterCommand("LINDEX", CmdLIndex);
        RegisterCommand("SADD", CmdSAdd);
        RegisterCommand("SREM", CmdSRem);
        RegisterCommand("SISMEMBER", CmdSIsMember);
        RegisterCommand("SMEMBERS", CmdSMembers);
        RegisterCommand("SCARD", CmdSCard);
        RegisterCommand("SUNION", CmdSUnion);
        RegisterCommand("SINTER", CmdSInter);
        RegisterCommand("SDIFF", CmdSDiff);
        RegisterCommand("ZADD", CmdZAdd);
        RegisterCommand("ZRANGE", CmdZRange);
        RegisterCommand("ZRANGEBYSCORE", CmdZRangeByScore);
        RegisterCommand("ZRANK", CmdZRank);
        RegisterCommand("ZSCORE", CmdZScore);
        RegisterCommand("ZCARD", CmdZCard);
        RegisterCommand("ZREM", CmdZRem);
        RegisterCommand("PFADD", CmdPfAdd);
        RegisterCommand("PFCOUNT", CmdPfCount);
        RegisterCommand("PFMERGE", CmdPfMerge);
        RegisterCommand("EXPIRE", CmdExpire);
        RegisterCommand("EXPIREAT", CmdExpireAt);
        RegisterCommand("TTL", CmdTtl);
        RegisterCommand("PTTL", CmdPTtl);
        RegisterCommand("PERSIST", CmdPersist);
        RegisterCommand("SELECT", CmdSelect);
        RegisterCommand("FLUSHDB", CmdFlushDb);
        RegisterCommand("FLUSHALL", CmdFlushAll);
        RegisterCommand("DBSIZE", CmdDbSize);
        RegisterCommand("INFO", CmdInfo);
        RegisterCommand("KEYS", CmdKeys);
    }

    private static (Store, RespValue) CmdPing(Store store, IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            return (store, Resp.SimpleString("PONG"));
        }

        return args.Count == 1
            ? (store, Resp.BulkString(args[0]))
            : (store, WrongArgCount("PING"));
    }

    private static (Store, RespValue) CmdEcho(Store store, IReadOnlyList<string> args) =>
        args.Count != 1 ? (store, WrongArgCount("ECHO")) : (store, Resp.BulkString(args[0]));

    private static (Store, RespValue) CmdSet(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("SET"));
        }

        var key = args[0];
        var value = args[1];
        long? expiresAt = null;
        var nx = false;
        var xx = false;

        for (var i = 2; i < args.Count;)
        {
            var option = args[i].ToUpperInvariant();
            if (option == "EX" && i + 1 < args.Count)
            {
                var seconds = ParseInteger(args[i + 1]);
                if (seconds is null)
                {
                    return (store, InvalidInteger());
                }

                expiresAt = ExpirationFromSeconds(seconds.Value);
                i += 2;
            }
            else if (option == "PX" && i + 1 < args.Count)
            {
                var millis = ParseInteger(args[i + 1]);
                if (millis is null)
                {
                    return (store, InvalidInteger());
                }

                expiresAt = ExpirationFromMillis(millis.Value);
                i += 2;
            }
            else if (option == "NX")
            {
                nx = true;
                i += 1;
            }
            else if (option == "XX")
            {
                xx = true;
                i += 1;
            }
            else
            {
                return (store, SyntaxError());
            }
        }

        if (nx && xx)
        {
            return (store, SyntaxError());
        }

        var exists = store.Get(key) is not null;
        if ((nx && exists) || (xx && !exists))
        {
            return (store, Nil());
        }

        return (store.Set(key, DataStoreTypes.StringEntry(value, expiresAt)), Resp.SimpleString("OK"));
    }

    private static (Store, RespValue) CmdGet(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("GET"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Nil());
        }

        return entry.EntryType != EntryType.String
            ? (store, WrongType())
            : (store, Resp.BulkString((string)entry.Value));
    }

    private static (Store, RespValue) CmdDel(Store store, IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            return (store, WrongArgCount("DEL"));
        }

        long removed = 0;
        var next = store;
        foreach (var key in args)
        {
            if (next.Get(key) is not null)
            {
                removed += 1;
                next = next.Delete(key);
            }
        }

        return (next, Resp.Integer(removed));
    }

    private static (Store, RespValue) CmdExists(Store store, IReadOnlyList<string> args)
    {
        if (args.Count == 0)
        {
            return (store, WrongArgCount("EXISTS"));
        }

        return (store, Resp.Integer(args.Count(key => store.Get(key) is not null)));
    }

    private static (Store, RespValue) CmdType(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("TYPE"));
        }

        return (store, Resp.SimpleString(store.TypeOf(args[0]) is EntryType type ? DataStoreTypes.EntryTypeName(type) : "none"));
    }

    private static (Store, RespValue) CmdRename(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("RENAME"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.ErrorValue("ERR no such key"));
        }

        return (store.Delete(args[0]).Set(args[1], DataStoreTypes.CloneEntry(entry)), Resp.SimpleString("OK"));
    }

    private static (Store, RespValue) CmdIncr(Store store, IReadOnlyList<string> args) =>
        args.Count != 1 ? (store, WrongArgCount("INCR")) : AdjustInteger(store, args[0], 1);

    private static (Store, RespValue) CmdDecr(Store store, IReadOnlyList<string> args) =>
        args.Count != 1 ? (store, WrongArgCount("DECR")) : AdjustInteger(store, args[0], -1);

    private static (Store, RespValue) CmdIncrBy(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("INCRBY"));
        }

        var delta = ParseInteger(args[1]);
        return delta is null ? (store, InvalidInteger()) : AdjustInteger(store, args[0], delta.Value);
    }

    private static (Store, RespValue) CmdDecrBy(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("DECRBY"));
        }

        var delta = ParseInteger(args[1]);
        return delta is null ? (store, InvalidInteger()) : AdjustInteger(store, args[0], -delta.Value);
    }

    private static (Store, RespValue) CmdAppend(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("APPEND"));
        }

        var key = args[0];
        var value = args[1];
        var entry = store.Get(key);
        var current = string.Empty;
        long? expiresAt = null;
        if (entry is not null)
        {
            if (entry.EntryType != EntryType.String)
            {
                return (store, WrongType());
            }

            current = (string)entry.Value;
            expiresAt = entry.ExpiresAt;
        }

        var next = current + value;
        return (store.Set(key, DataStoreTypes.StringEntry(next, expiresAt)), Resp.Integer(next.Length));
    }

    private static (Store, RespValue) CmdHSet(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 3 || args.Count % 2 == 0)
        {
            return (store, WrongArgCount("HSET"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is not null && entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        var map = entry?.EntryType == EntryType.Hash ? ((HashMap<string, string>)entry.Value).Clone() : new HashMap<string, string>();
        var expiresAt = entry?.ExpiresAt;
        long added = 0;
        for (var i = 1; i < args.Count; i += 2)
        {
            var field = args[i];
            var value = args[i + 1];
            if (!map.Has(field))
            {
                added += 1;
            }

            map = map.Set(field, value);
        }

        return (store.Set(key, DataStoreTypes.HashEntry(map, expiresAt)), Resp.Integer(added));
    }

    private static (Store, RespValue) CmdHGet(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("HGET"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Nil());
        }

        if (entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        var map = (HashMap<string, string>)entry.Value;
        return (store, map.Has(args[1]) ? Resp.BulkString(map.Get(args[1])) : Nil());
    }

    private static (Store, RespValue) CmdHDel(Store store, IReadOnlyList<string> args)
    {
        if (args.Count < 2)
        {
            return (store, WrongArgCount("HDEL"));
        }

        var key = args[0];
        var entry = store.Get(key);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        if (entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        var map = ((HashMap<string, string>)entry.Value).Clone();
        long removed = 0;
        foreach (var field in args.Skip(1))
        {
            if (map.Has(field))
            {
                removed += 1;
                map = map.Delete(field);
            }
        }

        var next = map.Size == 0 ? store.Delete(key) : store.Set(key, DataStoreTypes.HashEntry(map, entry.ExpiresAt));
        return (next, Resp.Integer(removed));
    }

    private static (Store, RespValue) CmdHGetAll(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("HGETALL"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        var values = ((HashMap<string, string>)entry.Value)
            .Entries()
            .OrderBy(pair => pair.Key, StringComparer.Ordinal)
            .SelectMany(pair => new RespValue[] { Resp.BulkString(pair.Key), Resp.BulkString(pair.Value) })
            .ToList();

        return (store, Resp.Array(values));
    }

    private static (Store, RespValue) CmdHLen(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("HLEN"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        return entry.EntryType != EntryType.Hash
            ? (store, WrongType())
            : (store, Resp.Integer(((HashMap<string, string>)entry.Value).Size));
    }

    private static (Store, RespValue) CmdHExists(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("HEXISTS"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        if (entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        return (store, Resp.Integer(((HashMap<string, string>)entry.Value).Has(args[1]) ? 1 : 0));
    }

    private static (Store, RespValue) CmdHKeys(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("HKEYS"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        var values = ((HashMap<string, string>)entry.Value)
            .Keys()
            .OrderBy(field => field, StringComparer.Ordinal)
            .Select(Resp.BulkString)
            .Cast<RespValue>()
            .ToList();

        return (store, Resp.Array(values));
    }

    private static (Store, RespValue) CmdHVals(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("HVALS"));
        }

        var entry = store.Get(args[0]);
        if (entry is null)
        {
            return (store, Resp.Array([]));
        }

        if (entry.EntryType != EntryType.Hash)
        {
            return (store, WrongType());
        }

        var values = ((HashMap<string, string>)entry.Value)
            .Entries()
            .OrderBy(pair => pair.Key, StringComparer.Ordinal)
            .Select(pair => (RespValue)Resp.BulkString(pair.Value))
            .ToList();

        return (store, Resp.Array(values));
    }

    private static (Store, RespValue) AdjustInteger(Store store, string key, long delta)
    {
        var entry = store.Get(key);
        long current = 0;
        long? expiresAt = null;

        if (entry is not null)
        {
            if (entry.EntryType != EntryType.String)
            {
                return (store, WrongType());
            }

            var parsed = ParseInteger((string)entry.Value);
            if (parsed is null)
            {
                return (store, InvalidInteger());
            }

            current = parsed.Value;
            expiresAt = entry.ExpiresAt;
        }

        try
        {
            var next = checked(current + delta);
            return (store.Set(key, DataStoreTypes.StringEntry(next.ToString(), expiresAt)), Resp.Integer(next));
        }
        catch (OverflowException)
        {
            return (store, Resp.ErrorValue("ERR increment or decrement would overflow"));
        }
    }
}
