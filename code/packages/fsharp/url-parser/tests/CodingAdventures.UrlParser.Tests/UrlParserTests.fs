namespace CodingAdventures.UrlParser.Tests

open Xunit
open CodingAdventures.UrlParser.FSharp

module UrlParserTests =
    let private assertUrlError expected action =
        let mutable matched = false
        try
            action() |> ignore
        with
        | UrlParseException(kind, _) ->
            matched <- true
            Assert.Equal(expected, kind)

        Assert.True(matched, "expected UrlParseException")

    [<Fact>]
    let ``version is exposed`` () =
        Assert.Equal("0.1.0", UrlParser.VERSION)

    [<Fact>]
    let ``parses all hierarchical components`` () =
        let url = Url.Parse "http://user:pass@Example.COM:8080/path/to/page?q=1#frag"

        Assert.Equal("http", url.Scheme)
        Assert.Equal(Some "user:pass", url.Userinfo)
        Assert.Equal(Some "example.com", url.Host)
        Assert.Equal(Some 8080us, url.Port)
        Assert.Equal("/path/to/page", url.Path)
        Assert.Equal(Some "q=1", url.Query)
        Assert.Equal(Some "frag", url.Fragment)
        Assert.Equal("user:pass@example.com:8080", url.Authority())
        Assert.Equal(Some 8080us, url.EffectivePort())

    [<Fact>]
    let ``parses opaque urls and default ports`` () =
        let mailto = UrlParser.parse "mailto:alice@example.com?subject=Hi#top"

        Assert.Equal("mailto", mailto.Scheme)
        Assert.Equal(None, mailto.Host)
        Assert.Equal("alice@example.com", mailto.Path)
        Assert.Equal(Some "subject=Hi", mailto.Query)
        Assert.Equal(Some "top", mailto.Fragment)
        Assert.Equal(None, UrlParser.effectivePort mailto)

        Assert.Equal(Some 80us, (Url.Parse "HTTP://EXAMPLE.COM/Path").EffectivePort())
        Assert.Equal(Some 443us, (Url.Parse "https://example.com").EffectivePort())
        Assert.Equal(Some 21us, (Url.Parse "ftp://example.com").EffectivePort())

    [<Fact>]
    let ``parses ipv6 and edge case authority forms`` () =
        let ipv6 = Url.Parse "http://[::1]:8080/path"
        Assert.Equal(Some "[::1]", ipv6.Host)
        Assert.Equal(Some 8080us, ipv6.Port)
        Assert.Equal("/path", ipv6.Path)

        let queryOnly = Url.Parse "http://example.com?query=1"
        Assert.Equal("/", queryOnly.Path)
        Assert.Equal(Some "query=1", queryOnly.Query)

        let fragmentOnly = Url.Parse "http://example.com#section"
        Assert.Equal("/", fragmentOnly.Path)
        Assert.Equal(Some "section", fragmentOnly.Fragment)

    [<Fact>]
    let ``reports expected error kinds`` () =
        assertUrlError UrlErrorKind.MissingScheme (fun () -> Url.Parse "example.com/path")
        assertUrlError UrlErrorKind.InvalidScheme (fun () -> Url.Parse "1http://example.com")
        assertUrlError UrlErrorKind.InvalidPort (fun () -> Url.Parse "http://example.com:99999")
        assertUrlError UrlErrorKind.InvalidPercentEncoding (fun () -> UrlParser.percentDecode "%GG")
        assertUrlError UrlErrorKind.InvalidPercentEncoding (fun () -> UrlParser.percentDecode "%2")
        assertUrlError UrlErrorKind.InvalidPercentEncoding (fun () -> UrlParser.percentDecode "%FF")

    [<Fact>]
    let ``percent encoding uses utf8 bytes`` () =
        Assert.Equal("hello%20world/a~b", UrlParser.percentEncode "hello world/a~b")
        Assert.Equal("%E6%97%A5", UrlParser.percentEncode "日")
        Assert.Equal("hello world/日", UrlParser.percentDecode "hello%20world/%E6%97%A5")
        Assert.Equal("cafe", UrlParser.percentDecode "cafe")

    [<Theory>]
    [<InlineData("d", "/a/b/d", null, null)>]
    [<InlineData("../d", "/a/d", null, null)>]
    [<InlineData("../../d", "/d", null, null)>]
    [<InlineData("./d", "/a/b/d", null, null)>]
    [<InlineData("d?key=val", "/a/b/d", "key=val", null)>]
    [<InlineData("#frag", "/a/b/c", "x=1", "frag")>]
    [<InlineData("", "/a/b/c", "x=1", null)>]
    let ``resolves relative references`` relative expectedPath expectedQuery expectedFragment =
        let baseUrl = Url.Parse "http://example.com/a/b/c?x=1#old"
        let resolved = baseUrl.Resolve relative

        Assert.Equal("http", resolved.Scheme)
        Assert.Equal(Some "example.com", resolved.Host)
        Assert.Equal(expectedPath, resolved.Path)
        Assert.Equal(Option.ofObj expectedQuery, resolved.Query)
        Assert.Equal(Option.ofObj expectedFragment, resolved.Fragment)

    [<Fact>]
    let ``resolves absolute and scheme relative references`` () =
        let baseUrl = Url.Parse "http://example.com/a/b/c"

        Assert.Equal("/root/d", (baseUrl.Resolve "/root/./x/../d").Path)

        let schemeRelative = baseUrl.Resolve "//other.example/d"
        Assert.Equal("http", schemeRelative.Scheme)
        Assert.Equal(Some "other.example", schemeRelative.Host)
        Assert.Equal("/d", schemeRelative.Path)

        let absolute = UrlParser.resolve "https://new.example/page" baseUrl
        Assert.Equal("https", absolute.Scheme)
        Assert.Equal(Some "new.example", absolute.Host)
        Assert.Equal("/page", absolute.Path)

    [<Fact>]
    let ``serializes urls`` () =
        let full = Url.Parse "http://user:pass@example.com:8080/path?query=1#frag"
        Assert.Equal("http://user:pass@example.com:8080/path?query=1#frag", UrlParser.toUrlString full)
        Assert.Equal(full.ToUrlString(), full.ToString())

        let opaque = Url.Parse "mailto:alice@example.com"
        Assert.Equal("mailto:alice@example.com", opaque.ToUrlString())
