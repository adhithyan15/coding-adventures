using System.Text;
using CodingAdventures.InMemoryDataStoreEngine;
using CodingAdventures.InMemoryDataStoreProtocol;
using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStore.Tests;

public sealed class InMemoryDataStoreTests
{
    [Fact]
    public void ExecutesRespFramesEndToEnd()
    {
        var store = new InMemoryDataStore();
        var frame = Resp.Array([Resp.BulkString("PING")]);

        var response = Assert.IsType<RespSimpleString>(store.ExecuteFrame(frame));
        Assert.Equal("PONG", response.Value);
    }

    [Fact]
    public void ProcessesMultipleRespCommandsAndEncodesResponses()
    {
        var store = new InMemoryDataStore();
        var first = Resp.Encode(Resp.Array([Resp.BulkString("SET"), Resp.BulkString("counter"), Resp.BulkString("1")]));
        var second = Resp.Encode(Resp.Array([Resp.BulkString("GET"), Resp.BulkString("counter")]));
        var input = new byte[first.Length + second.Length];
        Buffer.BlockCopy(first, 0, input, 0, first.Length);
        Buffer.BlockCopy(second, 0, input, first.Length, second.Length);

        var responses = store.Process(input);
        Assert.Collection(
            responses,
            first =>
            {
                var value = Assert.IsType<RespSimpleString>(first);
                Assert.Equal("OK", value.Value);
            },
            second =>
            {
                var value = Assert.IsType<RespBulkString>(second);
                Assert.Equal("1", value.Value);
            });
    }

    [Fact]
    public void IgnoresBlankRespArrays()
    {
        var store = new InMemoryDataStore();
        Assert.Null(store.ExecuteFrame(Resp.Array([])));
    }

    [Fact]
    public void CanTranslateCommandsBeforeExecution()
    {
        var command = DataStoreProtocol.CommandFromResp(Resp.Array([Resp.BulkString("PING")]));
        Assert.NotNull(command);
        Assert.Equal("PING", command!.Name);
        Assert.Empty(command.Args);
    }

    [Fact]
    public void SupportsExecutionHelpersAndStaticUtilities()
    {
        var store = new InMemoryDataStore(new InMemoryDataStoreOptions { Store = Store.Empty().Set("alpha", DataStoreTypes.StringEntry("1")) });

        AssertBulk(store.Execute(["GET", "alpha"]), "1");
        AssertBulk(store.Execute(new DataStoreCommand("ECHO", ["hello"])), "hello");
        AssertBulk(store.ExecuteCommand(new DataStoreCommand("ECHO", ["again"])), "again");
        AssertBulk(store.ExecuteParts(["ECHO", "parts"]), "parts");

        var frame = InMemoryDataStore.CommandToFrame(new DataStoreCommand("PING", []));
        Assert.Single(frame.Value!);
        Assert.Equal("OK", InMemoryDataStore.FrameToResponseText(InMemoryDataStore.Ok()));
        Assert.Equal("(nil)", InMemoryDataStore.FrameToResponseText(Resp.BulkString(null)));
        Assert.Equal("[array:1]", InMemoryDataStore.FrameToResponseText(Resp.Array([Resp.BulkString("x")])));
        Assert.Equal(string.Empty, InMemoryDataStore.FrameToResponseText(new UnknownRespValue()));

        var bytes = InMemoryDataStore.ConcatBytes([Encoding.UTF8.GetBytes("a"), Encoding.UTF8.GetBytes("b")]);
        Assert.Equal("ab", Encoding.UTF8.GetString(bytes));
        Assert.Equal("+OK\r\n", Encoding.UTF8.GetString(InMemoryDataStore.EncodeRespStream([Resp.SimpleString("OK")])));
    }

    [Fact]
    public void ReturnsErrorsForUnsupportedFramesAndCanHandleResponses()
    {
        var store = new InMemoryDataStore();

        var notArray = Assert.IsType<RespErrorValue>(store.ExecuteFrame(Resp.SimpleString("PING")));
        Assert.Equal("ERR expected RESP array command", notArray.Value);

        var badArray = Assert.IsType<RespErrorValue>(store.ExecuteFrame(Resp.Array([Resp.Array([])])));
        Assert.Equal("ERR expected RESP command array", badArray.Value);

        var bytes = store.Handle("*1\r\n$4\r\nPING\r\n");
        Assert.Equal("+PONG\r\n", Encoding.UTF8.GetString(bytes));
    }

    [Fact]
    public void SupportsResetAndModuleRegistration()
    {
        var engine = new DataStoreEngine();
        var store = new InMemoryDataStore(new InMemoryDataStoreOptions { Engine = engine });
        store.RegisterModule(new TestModule());

        AssertBulk(store.Execute(["MODULE.ECHO", "hello"]), "hello");

        store.Execute(["SET", "counter", "1"]);
        store.Reset(Store.Empty());

        AssertBulk(store.Execute(["GET", "counter"]), null);
    }

    private static void AssertBulk(RespValue value, string? expected)
    {
        var bulk = Assert.IsType<RespBulkString>(value);
        Assert.Equal(expected, bulk.Value);
    }

    private sealed class TestModule : IDataStoreModule
    {
        public void Register(DataStoreEngine engine)
        {
            engine.RegisterCommand("MODULE.ECHO", (state, args) => (state, Resp.BulkString(args[0])));
        }
    }

    private sealed class UnknownRespValue() : RespValue("unknown");
}
