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

    private static void AssertSimple(RespValue value, string expected)
    {
        var actual = Assert.IsType<RespSimpleString>(value);
        Assert.Equal(expected, actual.Value);
    }

    private static void AssertBulk(RespValue value, string expected)
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
}
