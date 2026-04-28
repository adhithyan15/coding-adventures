namespace CodingAdventures.HttpCore.Tests;

public sealed class HttpCoreTests
{
    [Fact]
    public void ParsesHttpVersions()
    {
        var version = HttpVersion.Parse("HTTP/1.1");
        Assert.Equal((ushort)1, version.Major);
        Assert.Equal((ushort)1, version.Minor);
        Assert.Equal("HTTP/1.1", version.ToString());

        Assert.Throws<FormatException>(() => HttpVersion.Parse("HTP/1.1"));
        Assert.Throws<FormatException>(() => HttpVersion.Parse("HTTP/one.1"));
    }

    [Fact]
    public void FindsHeadersAndParsesContentHelpers()
    {
        var headers = new[]
        {
            new Header("Content-Length", "42"),
            new Header("Content-Type", "text/html; charset=\"utf-8\""),
        };

        Assert.Equal("42", HttpCore.FindHeader(headers, "content-length"));
        Assert.Equal(42, HttpCore.ParseContentLength(headers));
        Assert.Equal(("text/html", "utf-8"), HttpCore.ParseContentType(headers));
        Assert.Null(HttpCore.FindHeader(headers, "missing"));
        Assert.Null(HttpCore.ParseContentLength([new Header("Content-Length", "forty-two")]));
        Assert.Null(HttpCore.ParseContentType([new Header("Content-Type", "")]));
    }

    [Fact]
    public void HeadsDelegateToHelpers()
    {
        var request = new RequestHead("POST", "/submit", new HttpVersion(1, 1), [new Header("Content-Length", "5")]);
        var response = new ResponseHead(new HttpVersion(1, 0), 200, "OK", [new Header("Content-Type", "application/json")]);

        Assert.Equal("5", request.Header("content-length"));
        Assert.Equal(5, request.ContentLength());
        Assert.Equal(("application/json", null), response.ContentType());
        Assert.Null(response.ContentLength());
    }

    [Fact]
    public void BodyKindConstructorsMatchSemanticModes()
    {
        Assert.Equal(new BodyKind("none"), BodyKind.None());
        Assert.Equal(new BodyKind("content-length", 7), BodyKind.ContentLength(7));
        Assert.Equal(new BodyKind("until-eof"), BodyKind.UntilEof());
        Assert.Equal(new BodyKind("chunked"), BodyKind.Chunked());
    }

    [Fact]
    public void SplitsPathsAndMatchesRoutePatterns()
    {
        Assert.Empty(HttpCore.SplitPathSegments("/"));
        Assert.Equal(["hello", "world"], HttpCore.SplitPathSegments("/hello/world/"));

        var pattern = RoutePattern.Parse("/hello/:name");
        var parameters = pattern.MatchPath("/hello/Adhithya");

        Assert.NotNull(parameters);
        Assert.Equal(("name", "Adhithya"), parameters![0]);
        Assert.Null(pattern.MatchPath("/hello"));
        Assert.Null(pattern.MatchPath("/goodbye/Adhithya"));
    }

    [Fact]
    public void RoutePatternHandlesRoot()
    {
        var pattern = RoutePattern.Parse("/");
        Assert.Empty(pattern.Segments);
        Assert.Empty(pattern.MatchPath("/")!);
        Assert.Null(pattern.MatchPath("/extra"));
    }
}
