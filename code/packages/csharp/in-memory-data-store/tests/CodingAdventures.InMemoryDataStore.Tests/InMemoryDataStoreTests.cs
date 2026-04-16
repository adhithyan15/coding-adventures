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
}
