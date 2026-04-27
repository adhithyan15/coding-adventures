using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStoreEngine;

public sealed partial class DataStoreEngine
{
    private static (Store, RespValue) CmdExpire(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("EXPIRE"));
        }

        var seconds = ParseInteger(args[1]);
        return seconds is null ? (store, InvalidInteger()) : SetExpiration(store, args[0], ExpirationFromSeconds(seconds.Value));
    }

    private static (Store, RespValue) CmdExpireAt(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 2)
        {
            return (store, WrongArgCount("EXPIREAT"));
        }

        var timestamp = ParseInteger(args[1]);
        return timestamp is null ? (store, InvalidInteger()) : SetExpiration(store, args[0], ExpirationFromSeconds(timestamp.Value - UnixNowSeconds()));
    }

    private static (Store, RespValue) CmdTtl(Store store, IReadOnlyList<string> args) =>
        args.Count != 1 ? (store, WrongArgCount("TTL")) : TtlLike(store, args[0], false);

    private static (Store, RespValue) CmdPTtl(Store store, IReadOnlyList<string> args) =>
        args.Count != 1 ? (store, WrongArgCount("PTTL")) : TtlLike(store, args[0], true);

    private static (Store, RespValue) CmdPersist(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("PERSIST"));
        }

        var entry = store.Get(args[0]);
        if (entry is null || entry.ExpiresAt is null)
        {
            return (store, Resp.Integer(0));
        }

        return (store.Set(args[0], DataStoreTypes.CloneEntry(entry with { ExpiresAt = null })), Resp.Integer(1));
    }

    private static (Store, RespValue) CmdSelect(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("SELECT"));
        }

        var index = ParseInteger(args[0]);
        if (index is null)
        {
            return (store, InvalidInteger());
        }

        if (index.Value < 0 || index.Value >= store.Databases.Count)
        {
            return (store, Resp.ErrorValue("ERR DB index out of range"));
        }

        return (store.Select((int)index.Value), Resp.SimpleString("OK"));
    }

    private static (Store, RespValue) CmdFlushDb(Store store, IReadOnlyList<string> args) =>
        args.Count != 0 ? (store, WrongArgCount("FLUSHDB")) : (store.FlushDb(), Resp.SimpleString("OK"));

    private static (Store, RespValue) CmdFlushAll(Store store, IReadOnlyList<string> args) =>
        args.Count != 0 ? (store, WrongArgCount("FLUSHALL")) : (store.FlushAll(), Resp.SimpleString("OK"));

    private static (Store, RespValue) CmdDbSize(Store store, IReadOnlyList<string> args) =>
        args.Count != 0 ? (store, WrongArgCount("DBSIZE")) : (store, Resp.Integer(store.DbSize()));

    private static (Store, RespValue) CmdInfo(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 0)
        {
            return (store, WrongArgCount("INFO"));
        }

        var info = $"# Server\r\nin_memory_data_store_version:0.1.0\r\nactive_db:{store.ActiveDb}\r\ndbsize:{store.DbSize()}\r\n";
        return (store, Resp.BulkString(info));
    }

    private static (Store, RespValue) CmdKeys(Store store, IReadOnlyList<string> args)
    {
        if (args.Count != 1)
        {
            return (store, WrongArgCount("KEYS"));
        }

        var values = store.Keys(args[0]).Select(Resp.BulkString).Cast<RespValue>().ToList();
        return (store, Resp.Array(values));
    }

    private static (Store, RespValue) TtlLike(Store store, string key, bool milliseconds)
    {
        var entry = store.Get(key);
        if (entry is null)
        {
            return (store, Resp.Integer(-2));
        }

        if (entry.ExpiresAt is null)
        {
            return (store, Resp.Integer(-1));
        }

        var now = Database.CurrentTimeMs();
        if (now >= entry.ExpiresAt.Value)
        {
            return (store.Delete(key), Resp.Integer(-2));
        }

        var remaining = entry.ExpiresAt.Value - now;
        return (store, Resp.Integer(milliseconds ? remaining : remaining / 1000));
    }

    private static (Store, RespValue) SetExpiration(Store store, string key, long expiresAt)
    {
        var entry = store.Get(key);
        if (entry is null)
        {
            return (store, Resp.Integer(0));
        }

        if (expiresAt <= Database.CurrentTimeMs())
        {
            return (store.Delete(key), Resp.Integer(1));
        }

        return (store.Set(key, DataStoreTypes.CloneEntry(entry with { ExpiresAt = expiresAt })), Resp.Integer(1));
    }

    private static long? ParseInteger(string value) => long.TryParse(value, out var parsed) ? parsed : null;

    private static double? ParseFloatStrict(string value)
    {
        if (!double.TryParse(value, out var parsed) || double.IsNaN(parsed) || double.IsInfinity(parsed))
        {
            return null;
        }

        return parsed;
    }

    private static long ExpirationFromSeconds(long seconds) => Database.CurrentTimeMs() + (seconds * 1_000);

    private static long ExpirationFromMillis(long millis) => Database.CurrentTimeMs() + millis;

    private static long UnixNowSeconds() => Database.CurrentTimeMs() / 1_000;

    private static RespErrorValue WrongArgCount(string command) => Resp.ErrorValue($"ERR wrong number of arguments for '{command}'");

    private static RespErrorValue InvalidInteger() => Resp.ErrorValue("ERR value is not an integer or out of range");

    private static RespErrorValue InvalidFloat() => Resp.ErrorValue("ERR value is not a valid float");

    private static RespErrorValue SyntaxError() => Resp.ErrorValue("ERR syntax error");

    private static RespBulkString Nil() => Resp.BulkString(null);

    private static RespErrorValue WrongType() => Resp.ErrorValue("WRONGTYPE Operation against a key holding the wrong kind of value");
}
