using System.Text;
using CodingAdventures.HttpCore;

namespace CodingAdventures.Http1.Tests;

public sealed class Http1Tests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Http1.Version);
    }

    [Fact]
    public void ParseSimpleRequest()
    {
        var parsed = Http1.ParseRequestHead("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n"u8);

        Assert.Equal("GET", parsed.Head.Method);
        Assert.Equal("/", parsed.Head.Target);
        Assert.Equal(new HttpVersion(1, 0), parsed.Head.Version);
        Assert.Equal("example.com", parsed.Head.Headers[0].Value);
        Assert.Equal(BodyKind.None(), parsed.BodyKind);
    }

    [Fact]
    public void ParsePostRequestWithContentLength()
    {
        var parsed = Http1.ParseRequestHead("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello"u8);

        Assert.Equal(44, parsed.BodyOffset);
        Assert.Equal(BodyKind.ContentLength(5), parsed.BodyKind);
    }

    [Fact]
    public void ChunkedTransferEncodingWinsForRequests()
    {
        var parsed = Http1.ParseRequestHead("POST / HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\nContent-Length: 99\r\n\r\n"u8);

        Assert.Equal(BodyKind.Chunked(), parsed.BodyKind);
    }

    [Fact]
    public void ParseResponseHead()
    {
        var parsed = Http1.ParseResponseHead("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody"u8);

        Assert.Equal(200, parsed.Head.Status);
        Assert.Equal("OK", parsed.Head.Reason);
        Assert.Equal(BodyKind.ContentLength(4), parsed.BodyKind);
    }

    [Fact]
    public void ResponseWithoutLengthUsesUntilEof()
    {
        var parsed = Http1.ParseResponseHead("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n"u8);

        Assert.Equal(BodyKind.UntilEof(), parsed.BodyKind);
    }

    [Fact]
    public void BodylessStatusCodesIgnoreBodyHeaders()
    {
        var parsed = Http1.ParseResponseHead("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n"u8);

        Assert.Equal(BodyKind.None(), parsed.BodyKind);
    }

    [Fact]
    public void AcceptsLfOnlyAndPreservesDuplicateHeaders()
    {
        var parsed = Http1.ParseResponseHead("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload"u8);

        Assert.Equal(["a=1", "b=2"], parsed.Head.Headers.Select(header => header.Value));
    }

    [Fact]
    public void DecodesHeaderValuesAsLatin1()
    {
        var bytes = Encoding.Latin1.GetBytes("GET / HTTP/1.1\r\nX-Name: café\r\n\r\n");
        var parsed = Http1.ParseRequestHead(bytes);

        Assert.Equal("café", parsed.Head.Headers[0].Value);
    }

    [Fact]
    public void InvalidInputsRaiseParseExceptions()
    {
        AssertError("IncompleteHead", () => Http1.ParseRequestHead("GET / HTTP/1.1\r\nHost: example.com"u8));
        AssertError("InvalidStartLine", () => Http1.ParseRequestHead("GET / too many HTTP/1.1\r\n\r\n"u8));
        AssertError("InvalidHeader", () => Http1.ParseRequestHead("GET / HTTP/1.1\r\nHost example.com\r\n\r\n"u8));
        AssertError("InvalidHeader", () => Http1.ParseRequestHead("GET / HTTP/1.1\r\n: nope\r\n\r\n"u8));
        AssertError("InvalidVersion", () => Http1.ParseRequestHead("GET / NOPE\r\n\r\n"u8));
        AssertError("InvalidVersion", () => Http1.ParseResponseHead("NOPE 200 OK\r\n\r\n"u8));
        AssertError("InvalidStatus", () => Http1.ParseResponseHead("HTTP/1.1 NOPE OK\r\n\r\n"u8));
        AssertError("InvalidContentLength", () => Http1.ParseResponseHead("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n"u8));
    }

    private static void AssertError(string kind, Action action)
    {
        var error = Assert.Throws<Http1ParseException>(action);
        Assert.Equal(kind, error.Kind);
    }
}
