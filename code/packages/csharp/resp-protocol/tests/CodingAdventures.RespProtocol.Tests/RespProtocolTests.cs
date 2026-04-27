using System.Text;

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

    [Fact]
    public void SupportsNullValuesAndArrayHelpers()
    {
        var bulk = Assert.IsType<RespBulkString>(RespProtocol.Decode("$-1\r\n")!.Value);
        Assert.Null(bulk.Value);

        var array = Assert.IsType<RespArray>(RespProtocol.Decode("*-1\r\n")!.Value);
        Assert.Null(array.Value);

        var combined = RespProtocol.ConcatBytes([RespProtocol.EncodeInteger(1), RespProtocol.EncodeBulkString("a")]);
        Assert.Equal(":1\r\n$1\r\na\r\n", Encoding.UTF8.GetString(combined));
    }

    [Fact]
    public void HandlesPartialAndInvalidFrames()
    {
        Assert.Null(RespProtocol.Decode("$5\r\nhel"));
        Assert.Null(RespProtocol.Decode("*2\r\n$4\r\nPING\r\n$5\r\nhel"));
        Assert.Throws<RespDecodeError>(() => RespProtocol.Decode(":abc\r\n"));
        Assert.Throws<RespDecodeError>(() => RespProtocol.Decode("$-2\r\n"));
        Assert.Throws<RespDecodeError>(() => RespProtocol.Decode("*-2\r\n"));
        Assert.Throws<RespDecodeError>(() => RespProtocol.Decode("$3\r\nabcx\r\n"));
    }

    [Fact]
    public void DecoderSurfacesErrorsAndEmptyQueue()
    {
        var decoder = new RespDecoder();
        decoder.Feed("PING\r\n");
        Assert.True(decoder.HasMessage());
        Assert.IsType<RespArray>(decoder.GetMessage());
        Assert.False(decoder.HasMessage());
        Assert.Throws<RespDecodeError>(() => decoder.GetMessage());

        Assert.Throws<RespDecodeError>(() => decoder.Feed(":oops\r\n"));
    }

    [Fact]
    public void RejectsUnsupportedEncodeTypes()
    {
        Assert.Throws<RespEncodeError>(() => RespProtocol.Encode(new UnknownRespValue()));
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

    private sealed class UnknownRespValue() : RespValue("unknown");
}
