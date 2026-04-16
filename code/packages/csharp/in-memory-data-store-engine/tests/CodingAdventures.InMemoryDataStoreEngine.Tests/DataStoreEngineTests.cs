using CodingAdventures.InMemoryDataStoreProtocol;
using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStoreEngine.Tests;

public sealed class DataStoreEngineTests
{
    [Fact]
    public void ExecutesStringCommands()
    {
        var engine = new DataStoreEngine();

        AssertSimple(engine.Execute(["PING"]), "PONG");
        AssertSimple(engine.Execute(["SET", "counter", "1"]), "OK");
        AssertBulk(engine.Execute(["GET", "counter"]), "1");
        AssertInteger(engine.Execute(["INCR", "counter"]), 2);
    }

    [Fact]
    public void HandlesHashesSetsListsSortedSetsAndHlls()
    {
        var engine = new DataStoreEngine();

        AssertInteger(engine.Execute(["HSET", "hash", "field", "value"]), 1);
        AssertInteger(engine.Execute(["SADD", "set", "a", "b"]), 2);
        AssertInteger(engine.Execute(["LPUSH", "list", "a", "b"]), 2);
        AssertInteger(engine.Execute(["ZADD", "zset", "1", "alice", "2", "bob"]), 2);
        AssertInteger(engine.Execute(["PFADD", "hll", "alice", "bob"]), 1);
    }

    [Fact]
    public void SupportsKeyspaceAndTtlCommands()
    {
        var engine = new DataStoreEngine();
        engine.Execute(["SET", "temp", "1"]);

        AssertInteger(engine.Execute(["EXPIRE", "temp", "10"]), 1);
        Assert.IsType<RespInteger>(engine.Execute(["TTL", "temp"]));
        AssertInteger(engine.Execute(["DBSIZE"]), 1);
        AssertSimple(engine.Execute(["FLUSHDB"]), "OK");
    }

    [Fact]
    public void ExposesCurrentStoreSnapshot()
    {
        var engine = new DataStoreEngine(Store.Empty());
        engine.Reset(Store.Empty().Set("alpha", DataStoreTypes.StringEntry("1")));

        Assert.True(engine.Store.Exists("alpha"));
    }

    [Fact]
    public void ReturnsErrorsForUnknownCommands()
    {
        var engine = new DataStoreEngine();
        AssertError(engine.Execute(["NOPE"]), "ERR unknown command 'NOPE'");
    }

    [Fact]
    public void KeepsCommandArraysUsableForRespIntegration()
    {
        var engine = new DataStoreEngine();
        AssertBulk(engine.Execute(["ECHO", "hello"]), "hello");
    }

    [Fact]
    public void SupportsExpandedStringAndKeyspaceCommands()
    {
        var engine = new DataStoreEngine();

        AssertSimple(engine.Execute(["SET", "alpha", "1", "NX"]), "OK");
        AssertBulk(engine.Execute(["SET", "alpha", "2", "NX"]), null);
        AssertSimple(engine.Execute(["SET", "alpha", "3", "XX"]), "OK");
        AssertInteger(engine.Execute(["APPEND", "alpha", "4"]), 2);
        AssertBulk(engine.Execute(["GET", "alpha"]), "34");
        AssertInteger(engine.Execute(["DECR", "alpha"]), 33);
        AssertInteger(engine.Execute(["INCRBY", "alpha", "7"]), 40);
        AssertInteger(engine.Execute(["DECRBY", "alpha", "5"]), 35);
        AssertInteger(engine.Execute(["EXISTS", "alpha", "missing"]), 1);
        AssertSimple(engine.Execute(["TYPE", "alpha"]), "string");
        AssertSimple(engine.Execute(["RENAME", "alpha", "beta"]), "OK");
        AssertBulk(engine.Execute(["GET", "beta"]), "35");
        AssertInteger(engine.Execute(["DEL", "beta", "missing"]), 1);
        AssertBulk(engine.Execute(["GET", "beta"]), null);
    }

    [Fact]
    public void SupportsCollectionCommandsAcrossDataTypes()
    {
        var engine = new DataStoreEngine();

        AssertInteger(engine.Execute(["HSET", "hash", "one", "1", "two", "2"]), 2);
        AssertBulk(engine.Execute(["HGET", "hash", "one"]), "1");
        AssertInteger(engine.Execute(["HLEN", "hash"]), 2);
        AssertInteger(engine.Execute(["HEXISTS", "hash", "two"]), 1);
        AssertArray(engine.Execute(["HKEYS", "hash"]), "one", "two");
        AssertArray(engine.Execute(["HVALS", "hash"]), "1", "2");
        AssertArray(engine.Execute(["HGETALL", "hash"]), "one", "1", "two", "2");
        AssertInteger(engine.Execute(["HDEL", "hash", "one"]), 1);

        AssertInteger(engine.Execute(["LPUSH", "list", "b", "a"]), 2);
        AssertInteger(engine.Execute(["RPUSH", "list", "c"]), 3);
        AssertInteger(engine.Execute(["LLEN", "list"]), 3);
        AssertArray(engine.Execute(["LRANGE", "list", "0", "-1"]), "a", "b", "c");
        AssertBulk(engine.Execute(["LINDEX", "list", "-1"]), "c");
        AssertBulk(engine.Execute(["LPOP", "list"]), "a");
        AssertBulk(engine.Execute(["RPOP", "list"]), "c");

        AssertInteger(engine.Execute(["SADD", "set-a", "x", "y"]), 2);
        AssertInteger(engine.Execute(["SADD", "set-b", "y", "z"]), 2);
        AssertInteger(engine.Execute(["SISMEMBER", "set-a", "x"]), 1);
        AssertInteger(engine.Execute(["SCARD", "set-a"]), 2);
        AssertArray(engine.Execute(["SMEMBERS", "set-a"]), "x", "y");
        AssertArray(engine.Execute(["SUNION", "set-a", "set-b"]), "x", "y", "z");
        AssertArray(engine.Execute(["SINTER", "set-a", "set-b"]), "y");
        AssertArray(engine.Execute(["SDIFF", "set-a", "set-b"]), "x");
        AssertInteger(engine.Execute(["SREM", "set-a", "x"]), 1);

        AssertInteger(engine.Execute(["ZADD", "scores", "1", "alice", "2", "bob", "1.5", "cara"]), 3);
        AssertArray(engine.Execute(["ZRANGE", "scores", "0", "-1"]), "alice", "cara", "bob");
        AssertArray(engine.Execute(["ZRANGE", "scores", "0", "-1", "WITHSCORES"]), "alice", "1", "cara", "1.5", "bob", "2");
        AssertArray(engine.Execute(["ZRANGEBYSCORE", "scores", "1", "1.5"]), "alice", "cara");
        AssertInteger(engine.Execute(["ZRANK", "scores", "cara"]), 1);
        AssertBulk(engine.Execute(["ZSCORE", "scores", "bob"]), "2");
        AssertInteger(engine.Execute(["ZCARD", "scores"]), 3);
        AssertInteger(engine.Execute(["ZREM", "scores", "alice"]), 1);

        AssertInteger(engine.Execute(["PFADD", "hll-a", "alpha", "beta"]), 1);
        AssertInteger(engine.Execute(["PFADD", "hll-b", "beta", "gamma"]), 1);
        Assert.True(((RespInteger)engine.Execute(["PFCOUNT", "hll-a", "hll-b"])).Value >= 2);
        AssertSimple(engine.Execute(["PFMERGE", "hll-merged", "hll-a", "hll-b"]), "OK");
        Assert.True(((RespInteger)engine.Execute(["PFCOUNT", "hll-merged"])).Value >= 2);
    }

    [Fact]
    public void SupportsExpiryAdminAndCommandHelpers()
    {
        var engine = new DataStoreEngine();

        Assert.True(DataStoreEngine.IsMutatingCommand("set"));
        Assert.False(DataStoreEngine.IsMutatingCommand("get"));
        AssertError(engine.Execute([]), "ERR empty command");

        AssertSimple(engine.Execute(["SET", "temp", "1"]), "OK");
        AssertInteger(engine.Execute(["EXPIRE", "temp", "10"]), 1);
        Assert.True(((RespInteger)engine.Execute(["TTL", "temp"])).Value >= 0);
        Assert.True(((RespInteger)engine.Execute(["PTTL", "temp"])).Value >= 0);
        AssertInteger(engine.Execute(["PERSIST", "temp"]), 1);
        AssertInteger(engine.Execute(["TTL", "temp"]), -1);
        AssertInteger(engine.Execute(["EXPIREAT", "temp", "0"]), 1);
        AssertInteger(engine.Execute(["TTL", "temp"]), -2);

        AssertSimple(engine.Execute(["SET", "app:1", "one"]), "OK");
        AssertSimple(engine.Execute(["SET", "app:2", "two"]), "OK");
        AssertArray(engine.Execute(["KEYS", "app:*"]), "app:1", "app:2");
        AssertSimple(engine.Execute(["SELECT", "1"]), "OK");
        AssertInteger(engine.Execute(["DBSIZE"]), 0);
        Assert.Contains("active_db:1", Assert.IsType<RespBulkString>(engine.Execute(["INFO"])).Value);
        AssertSimple(engine.Execute(["FLUSHALL"]), "OK");
    }

    [Fact]
    public void ExposesStoreAndTypeHelpers()
    {
        var sorted = new SortedSet();
        Assert.True(sorted.IsEmpty());
        Assert.True(sorted.Insert(1.0, "alice"));
        Assert.False(sorted.Insert(2.0, "alice"));
        Assert.True(sorted.Contains("alice"));
        Assert.Equal(2.0, sorted.Score("alice"));
        Assert.Equal(0, sorted.Rank("alice"));
        Assert.Single(sorted.RangeByIndex(0, 0));
        Assert.Single(sorted.RangeByScore(1.5, 2.5));
        Assert.True(sorted.Remove("alice"));
        Assert.False(sorted.Remove("alice"));
        Assert.Throws<InvalidOperationException>(() => sorted.Insert(double.NaN, "bad"));
        Assert.Throws<InvalidOperationException>(() => sorted.RangeByScore(double.NaN, 1));

        var db = Database.Empty()
            .Set("match:1", DataStoreTypes.StringEntry("one"))
            .Set("gone", DataStoreTypes.StringEntry("x", Database.CurrentTimeMs() - 1))
            .Set("future", DataStoreTypes.StringEntry("y", Database.CurrentTimeMs() + 60_000));

        Assert.True(db.Exists("match:1"));
        Assert.Null(db.Get("gone"));
        Assert.Equal(EntryType.String, db.TypeOf("match:1"));
        Assert.Equal(["match:1"], db.Keys("match:?"));
        Assert.Equal(2, db.DbSize());
        Assert.Null(db.ExpireLazy("future").Get("gone"));
        Assert.False(db.ActiveExpire().Exists("gone"));
        Assert.Empty(db.Clear().Keys("*"));

        var store = Store.Empty(2)
            .Set("alpha", DataStoreTypes.StringEntry("1"))
            .Select(1)
            .Set("beta", DataStoreTypes.ListEntry(["a"]))
            .Select(0);

        Assert.Equal(0, store.ActiveDb);
        Assert.True(store.Exists("alpha"));
        Assert.Equal(1, store.Select(1).DbSize());
        Assert.Equal(EntryType.List, store.Select(1).TypeOf("beta"));
        Assert.Empty(store.FlushDb().Keys("*"));
        Assert.Equal(0, store.FlushAll().DbSize());
        Assert.Equal(0, Store.Empty(0).ActiveDb);
        Assert.True(DataStoreTypes.CloneEntry(DataStoreTypes.ZSetEntry(new SortedSet())).Value is SortedSet);
        Assert.Equal("set", DataStoreTypes.EntryTypeName(EntryType.Set));
        Assert.True(DataStoreTypes.CompareExpiry((1, "a"), (1, "b")) < 0);
        Assert.Equal((1L, "a"), DataStoreTypes.CreateExpiryHeap([(2L, "b"), (1L, "a")]).Pop());
    }

    [Fact]
    public void SupportsCustomCommandRegistrationAndModules()
    {
        var engine = new DataStoreEngine();
        engine.RegisterCommand("HELLO", (store, args) => (store, Resp.BulkString(string.Join(",", args))));
        engine.RegisterModule(new TestModule());

        AssertBulk(engine.Execute(new DataStoreCommand("HELLO", ["a", "b"])), "a,b");
        AssertBulk(engine.ExecuteCommand(new DataStoreCommand("MODULE.PING", ["ok"])), "ok");
        AssertBulk(engine.ExecuteParts(["MODULE.PING", "again"]), "again");
    }

    [Fact]
    public void ReturnsValidationErrorsForBadInputAndWrongTypes()
    {
        var engine = new DataStoreEngine();
        engine.Execute(["SET", "string", "value"]);
        engine.Execute(["SADD", "set", "a"]);

        AssertError(engine.Execute(["PING", "a", "b"]), "ERR wrong number of arguments for 'PING'");
        AssertError(engine.Execute(["SET", "a", "1", "NX", "XX"]), "ERR syntax error");
        AssertError(engine.Execute(["SET", "a", "1", "EX", "oops"]), "ERR value is not an integer or out of range");
        AssertError(engine.Execute(["GET"]), "ERR wrong number of arguments for 'GET'");
        AssertError(engine.Execute(["RENAME", "missing", "next"]), "ERR no such key");
        AssertError(engine.Execute(["HSET", "hash", "field"]), "ERR wrong number of arguments for 'HSET'");
        AssertError(engine.Execute(["HGET", "string", "field"]), "WRONGTYPE Operation against a key holding the wrong kind of value");
        AssertError(engine.Execute(["LRANGE", "missing", "x", "1"]), "ERR value is not an integer or out of range");
        AssertError(engine.Execute(["ZADD", "scores", "nan", "alice"]), "ERR value is not a valid float");
        AssertError(engine.Execute(["ZRANGE", "scores", "x", "1"]), "ERR value is not an integer or out of range");
        AssertError(engine.Execute(["ZRANGEBYSCORE", "scores", "x", "1"]), "ERR value is not a valid float");
        AssertError(engine.Execute(["PFCOUNT"]), "ERR wrong number of arguments for 'PFCOUNT'");
        AssertError(engine.Execute(["EXPIRE", "a"]), "ERR wrong number of arguments for 'EXPIRE'");
        AssertError(engine.Execute(["EXPIREAT", "a", "oops"]), "ERR value is not an integer or out of range");
        AssertError(engine.Execute(["TTL", "a", "b"]), "ERR wrong number of arguments for 'TTL'");
        AssertError(engine.Execute(["PTTL", "a", "b"]), "ERR wrong number of arguments for 'PTTL'");
        AssertError(engine.Execute(["SELECT", "oops"]), "ERR value is not an integer or out of range");
        AssertError(engine.Execute(["SELECT", "99"]), "ERR DB index out of range");
        AssertError(engine.Execute(["FLUSHDB", "extra"]), "ERR wrong number of arguments for 'FLUSHDB'");
        AssertError(engine.Execute(["FLUSHALL", "extra"]), "ERR wrong number of arguments for 'FLUSHALL'");
        AssertError(engine.Execute(["DBSIZE", "extra"]), "ERR wrong number of arguments for 'DBSIZE'");
        AssertError(engine.Execute(["INFO", "extra"]), "ERR wrong number of arguments for 'INFO'");
        AssertError(engine.Execute(["KEYS"]), "ERR wrong number of arguments for 'KEYS'");

        var expiredEverywhere = Store.Empty(2)
            .Set("soon", DataStoreTypes.StringEntry("1", Database.CurrentTimeMs() - 1))
            .Select(1)
            .Set("later", DataStoreTypes.StringEntry("2", Database.CurrentTimeMs() - 1))
            .ActiveExpireAll();

        Assert.Equal(0, expiredEverywhere.WithActiveDb(0).DbSize());
        Assert.Equal(0, expiredEverywhere.WithActiveDb(1).DbSize());
    }

    private static void AssertSimple(RespValue value, string expected)
    {
        var actual = Assert.IsType<RespSimpleString>(value);
        Assert.Equal(expected, actual.Value);
    }

    private static void AssertBulk(RespValue value, string? expected)
    {
        var actual = Assert.IsType<RespBulkString>(value);
        Assert.Equal(expected, actual.Value);
    }

    private static void AssertInteger(RespValue value, long expected)
    {
        var actual = Assert.IsType<RespInteger>(value);
        Assert.Equal(expected, actual.Value);
    }

    private static void AssertError(RespValue value, string expected)
    {
        var actual = Assert.IsType<RespErrorValue>(value);
        Assert.Equal(expected, actual.Value);
    }

    private static void AssertArray(RespValue value, params string[] expected)
    {
        var actual = Assert.IsType<RespArray>(value);
        var members = actual.Value!
            .Select(item => Assert.IsType<RespBulkString>(item).Value)
            .ToArray();
        Assert.Equal(expected, members);
    }

    private sealed class TestModule : IDataStoreModule
    {
        public void Register(DataStoreEngine engine)
        {
            engine.RegisterCommand("MODULE.PING", (store, args) => (store, Resp.BulkString(args[0])));
        }
    }
}
