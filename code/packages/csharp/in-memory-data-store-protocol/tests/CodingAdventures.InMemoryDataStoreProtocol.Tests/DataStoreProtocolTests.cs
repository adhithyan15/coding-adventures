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

    [Fact]
    public void BuildsCommandsFromPartsAndFrames()
    {
        var command = DataStoreProtocol.CommandFromParts(["  set ", "counter", "2"]);
        Assert.Equal("SET", command.Name);
        Assert.Equal(["counter", "2"], command.Args);
        Assert.Equal(["SET", "counter", "2"], DataStoreProtocol.CommandToParts(command));

        var frame = DataStoreProtocol.CommandFrameToResp(["PING"]);
        var value = Assert.IsType<RespArray>(DataStoreProtocol.CommandToRespValue(new DataStoreCommand("PING", [])));

        Assert.Single(frame.Value!);
        Assert.Single(value.Value!);
    }

    [Fact]
    public void HandlesMoreRespValueConversionsAndValidation()
    {
        Assert.Throws<InvalidOperationException>(() => DataStoreProtocol.CommandFromParts([]));

        Assert.Equal("OK", DataStoreProtocol.RespValueToString(Resp.SimpleString("OK")));
        Assert.Equal("ERR boom", DataStoreProtocol.RespValueToString(Resp.ErrorValue("ERR boom")));
        Assert.Equal("42", DataStoreProtocol.RespValueToString(Resp.Integer(42)));
        Assert.Null(DataStoreProtocol.RespValueToString(Resp.Array([])));

        var mixed = Resp.Array([Resp.Integer(1), Resp.BulkString("hello")]);
        var command = DataStoreProtocol.CommandFromResp(mixed);
        Assert.NotNull(command);
        Assert.Equal("1", command!.Name);
        Assert.Equal(["hello"], command.Args);
    }
}
