using CodingAdventures.UrlParser;

namespace CodingAdventures.UrlParser.Tests;

public sealed class UrlParserTests
{
    [Fact]
    public void VersionIsExposed()
    {
        Assert.Equal("0.1.0", UrlParser.Version);
    }

    [Fact]
    public void ParsesAllHierarchicalComponents()
    {
        var url = Url.Parse("http://user:pass@Example.COM:8080/path/to/page?q=1#frag");

        Assert.Equal("http", url.Scheme);
        Assert.Equal("user:pass", url.Userinfo);
        Assert.Equal("example.com", url.Host);
        Assert.Equal((ushort)8080, url.Port);
        Assert.Equal("/path/to/page", url.Path);
        Assert.Equal("q=1", url.Query);
        Assert.Equal("frag", url.Fragment);
        Assert.Equal("user:pass@example.com:8080", url.Authority());
        Assert.Equal((ushort)8080, url.EffectivePort());
    }

    [Fact]
    public void ParsesOpaqueUrlsAndDefaultPorts()
    {
        var mailto = Url.Parse("mailto:alice@example.com?subject=Hi#top");
        Assert.Equal("mailto", mailto.Scheme);
        Assert.Null(mailto.Host);
        Assert.Equal("alice@example.com", mailto.Path);
        Assert.Equal("subject=Hi", mailto.Query);
        Assert.Equal("top", mailto.Fragment);
        Assert.Null(mailto.EffectivePort());

        Assert.Equal((ushort)80, Url.Parse("HTTP://EXAMPLE.COM/Path").EffectivePort());
        Assert.Equal((ushort)443, Url.Parse("https://example.com").EffectivePort());
        Assert.Equal((ushort)21, Url.Parse("ftp://example.com").EffectivePort());
    }

    [Fact]
    public void ParsesIpv6AndEdgeCaseAuthorityForms()
    {
        var ipv6 = Url.Parse("http://[::1]:8080/path");
        Assert.Equal("[::1]", ipv6.Host);
        Assert.Equal((ushort)8080, ipv6.Port);
        Assert.Equal("/path", ipv6.Path);

        var queryOnly = Url.Parse("http://example.com?query=1");
        Assert.Equal("/", queryOnly.Path);
        Assert.Equal("query=1", queryOnly.Query);

        var fragmentOnly = Url.Parse("http://example.com#section");
        Assert.Equal("/", fragmentOnly.Path);
        Assert.Equal("section", fragmentOnly.Fragment);
    }

    [Fact]
    public void ReportsExpectedErrorKinds()
    {
        AssertError(UrlErrorKind.MissingScheme, () => Url.Parse("example.com/path"));
        AssertError(UrlErrorKind.InvalidScheme, () => Url.Parse("1http://example.com"));
        AssertError(UrlErrorKind.InvalidPort, () => Url.Parse("http://example.com:99999"));
        AssertError(UrlErrorKind.InvalidPercentEncoding, () => UrlParser.PercentDecode("%GG"));
        AssertError(UrlErrorKind.InvalidPercentEncoding, () => UrlParser.PercentDecode("%2"));
        AssertError(UrlErrorKind.InvalidPercentEncoding, () => UrlParser.PercentDecode("%FF"));
    }

    [Fact]
    public void PercentEncodingUsesUtf8Bytes()
    {
        Assert.Equal("hello%20world/a~b", UrlParser.PercentEncode("hello world/a~b"));
        Assert.Equal("%E6%97%A5", UrlParser.PercentEncode("日"));
        Assert.Equal("hello world/日", UrlParser.PercentDecode("hello%20world/%E6%97%A5"));
        Assert.Equal("cafe", UrlParser.PercentDecode("cafe"));
    }

    [Theory]
    [InlineData("d", "/a/b/d", null, null)]
    [InlineData("../d", "/a/d", null, null)]
    [InlineData("../../d", "/d", null, null)]
    [InlineData("./d", "/a/b/d", null, null)]
    [InlineData("d?key=val", "/a/b/d", "key=val", null)]
    [InlineData("#frag", "/a/b/c", "x=1", "frag")]
    [InlineData("", "/a/b/c", "x=1", null)]
    public void ResolvesRelativeReferences(string relative, string expectedPath, string? expectedQuery, string? expectedFragment)
    {
        var baseUrl = Url.Parse("http://example.com/a/b/c?x=1#old");
        var resolved = baseUrl.Resolve(relative);

        Assert.Equal("http", resolved.Scheme);
        Assert.Equal("example.com", resolved.Host);
        Assert.Equal(expectedPath, resolved.Path);
        Assert.Equal(expectedQuery, resolved.Query);
        Assert.Equal(expectedFragment, resolved.Fragment);
    }

    [Fact]
    public void ResolvesAbsoluteAndSchemeRelativeReferences()
    {
        var baseUrl = Url.Parse("http://example.com/a/b/c");

        Assert.Equal("/root/d", baseUrl.Resolve("/root/./x/../d").Path);

        var schemeRelative = baseUrl.Resolve("//other.example/d");
        Assert.Equal("http", schemeRelative.Scheme);
        Assert.Equal("other.example", schemeRelative.Host);
        Assert.Equal("/d", schemeRelative.Path);

        var absolute = baseUrl.Resolve("https://new.example/page");
        Assert.Equal("https", absolute.Scheme);
        Assert.Equal("new.example", absolute.Host);
        Assert.Equal("/page", absolute.Path);
    }

    [Fact]
    public void SerializesUrls()
    {
        var full = Url.Parse("http://user:pass@example.com:8080/path?query=1#frag");
        Assert.Equal("http://user:pass@example.com:8080/path?query=1#frag", full.ToUrlString());
        Assert.Equal(full.ToUrlString(), full.ToString());

        var opaque = Url.Parse("mailto:alice@example.com");
        Assert.Equal("mailto:alice@example.com", opaque.ToUrlString());
    }

    private static void AssertError(UrlErrorKind kind, Action action)
    {
        var ex = Assert.Throws<UrlParseException>(action);
        Assert.Equal(kind, ex.Kind);
    }
}
