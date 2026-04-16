using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStoreProtocol.Tests;

public sealed class DataStoreProtocolTests
{
    [Fact]
    public void ParsesRespCommandArrays()
    {
        var command = DataStoreProtocol.CommandFromResp(
            Resp.Array(
            [
                Resp.BulkString("SET"),
                Resp.BulkString("counter"),
                Resp.BulkString("1")
            ]));

        Assert.NotNull(command);
        Assert.Equal("SET", command!.Name);
        Assert.Equal(["counter", "1"], command.Args);
    }

    [Fact]
    public void RejectsEmptyOrUnsupportedFrames()
    {
        Assert.Null(DataStoreProtocol.CommandFromResp(Resp.Array([])));
        Assert.Null(DataStoreProtocol.CommandFromResp(Resp.SimpleString("OK")));
    }

    [Fact]
    public void RoundTripsCommandsBackToResp()
    {
        var command = new DataStoreCommand("PING", []);
        var frame = DataStoreProtocol.CommandToResp(command);

        Assert.NotNull(frame.Value);
        var element = Assert.IsType<RespBulkString>(Assert.Single(frame.Value!));
        Assert.Equal("PING", element.Value);
    }

    [Fact]
    public void NormalizesCommandNames()
    {
        Assert.Equal("PING", DataStoreProtocol.CommandName(["  ping  "]));
    }

    [Fact]
    public void ConvertsRespValuesToStringsWhenPossible()
    {
        Assert.Equal("hello", DataStoreProtocol.RespValueToString(Resp.BulkString("hello")));
    }
}
