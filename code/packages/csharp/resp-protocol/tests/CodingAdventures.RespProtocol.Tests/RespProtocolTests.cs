namespace CodingAdventures.RespProtocol.Tests;

public sealed class RespProtocolTests
{
    [Fact]
    public void EncodesAndDecodesResp2Values()
    {
        AssertSimpleString(RespProtocol.Decode(RespProtocol.Encode(RespProtocol.SimpleString("OK"))), "OK");
        AssertError(RespProtocol.Decode(RespProtocol.Encode(RespProtocol.ErrorValue("ERR boom"))), "ERR boom");
        AssertInteger(RespProtocol.Decode(RespProtocol.Encode(RespProtocol.Integer(42))), 42);
        AssertBulkString(RespProtocol.Decode(RespProtocol.Encode(RespProtocol.BulkString("foo"))), "foo");
    }

    [Fact]
    public void RoundTripsCommandArrays()
    {
        var frame = RespProtocol.Array(
        [
            RespProtocol.BulkString("SET"),
            RespProtocol.BulkString("counter"),
            RespProtocol.BulkString("1")
        ]);

        var result = RespProtocol.Decode(RespProtocol.Encode(frame));
        Assert.NotNull(result);
        var decoded = Assert.IsType<RespArray>(result!.Value);
        Assert.NotNull(decoded.Value);
        Assert.Equal(3, decoded.Value!.Count);
    }

    [Fact]
    public void DecodesInlineCommandsAndBlankLines()
    {
        var ping = RespProtocol.Decode("PING\r\n");
        var blank = RespProtocol.Decode("   \r\n");

        Assert.NotNull(ping);
        Assert.NotNull(blank);
        Assert.Equal(6, ping!.Consumed);
        Assert.Equal(5, blank!.Consumed);
    }

    [Fact]
    public void SupportsIncrementalBuffering()
    {
        var decoder = new RespDecoder();
        decoder.Feed("*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n");
        Assert.True(decoder.HasMessage());
        var message = Assert.IsType<RespArray>(decoder.GetMessage());
        Assert.NotNull(message.Value);
        Assert.Equal(2, message.Value!.Count);
    }

    [Fact]
    public void DecodesMultipleConcatenatedMessages()
    {
        var bytes = RespProtocol.Encode(RespProtocol.SimpleString("OK"));
        var combined = new byte[bytes.Length * 2];
        Buffer.BlockCopy(bytes, 0, combined, 0, bytes.Length);
        Buffer.BlockCopy(bytes, 0, combined, bytes.Length, bytes.Length);

        var result = RespProtocol.DecodeAll(combined);
        Assert.Equal(2, result.Values.Count);
        Assert.Equal(combined.Length, result.Consumed);
    }

    private static void AssertSimpleString(RespDecodeResult? result, string expected)
    {
        Assert.NotNull(result);
        var value = Assert.IsType<RespSimpleString>(result!.Value);
        Assert.Equal(expected, value.Value);
    }

    private static void AssertError(RespDecodeResult? result, string expected)
    {
        Assert.NotNull(result);
        var value = Assert.IsType<RespErrorValue>(result!.Value);
        Assert.Equal(expected, value.Value);
    }

    private static void AssertInteger(RespDecodeResult? result, long expected)
    {
        Assert.NotNull(result);
        var value = Assert.IsType<RespInteger>(result!.Value);
        Assert.Equal(expected, value.Value);
    }

    private static void AssertBulkString(RespDecodeResult? result, string expected)
    {
        Assert.NotNull(result);
        var value = Assert.IsType<RespBulkString>(result!.Value);
        Assert.Equal(expected, value.Value);
    }
}
