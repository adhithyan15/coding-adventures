namespace CodingAdventures.HttpCore.Tests

open System
open Xunit
open CodingAdventures.HttpCore.FSharp

module HttpCoreTests =
    [<Fact>]
    let ``parses http versions`` () =
        let version = HttpVersion.Parse "HTTP/1.1"
        Assert.Equal(1us, version.Major)
        Assert.Equal(1us, version.Minor)
        Assert.Equal("HTTP/1.1", version.ToString())

        Assert.Throws<FormatException>(fun () -> HttpVersion.Parse "HTP/1.1" |> ignore) |> ignore
        Assert.Throws<FormatException>(fun () -> HttpVersion.Parse "HTTP/one.1" |> ignore) |> ignore

    [<Fact>]
    let ``finds headers and parses content helpers`` () =
        let headers =
            [ { Name = "Content-Length"; Value = "42" }
              { Name = "Content-Type"; Value = "text/html; charset=\"utf-8\"" } ]

        Assert.Equal(Some "42", HttpCore.findHeader headers "content-length")
        Assert.Equal(Some 42, HttpCore.parseContentLength headers)
        Assert.Equal(Some("text/html", Some "utf-8"), HttpCore.parseContentType headers)
        Assert.Equal(None, HttpCore.findHeader headers "missing")
        Assert.Equal(None, HttpCore.parseContentLength [ { Name = "Content-Length"; Value = "forty-two" } ])
        Assert.Equal(None, HttpCore.parseContentType [ { Name = "Content-Type"; Value = "" } ])

    [<Fact>]
    let ``heads delegate to helpers`` () =
        let request =
            { Method = "POST"
              Target = "/submit"
              Version = { Major = 1us; Minor = 1us }
              Headers = [ { Name = "Content-Length"; Value = "5" } ] }

        let response =
            { Version = { Major = 1us; Minor = 0us }
              Status = 200us
              Reason = "OK"
              Headers = [ { Name = "Content-Type"; Value = "application/json" } ] }

        Assert.Equal(Some "5", request.Header "content-length")
        Assert.Equal(Some 5, request.ContentLength())
        Assert.Equal(Some("application/json", None), response.ContentType())
        Assert.Equal(None, response.ContentLength())

    [<Fact>]
    let ``body kind constructors match semantic modes`` () =
        Assert.Equal({ Mode = "none"; Length = None }, BodyKind.None())
        Assert.Equal({ Mode = "content-length"; Length = Some 7 }, BodyKind.ContentLength 7)
        Assert.Equal({ Mode = "until-eof"; Length = None }, BodyKind.UntilEof())
        Assert.Equal({ Mode = "chunked"; Length = None }, BodyKind.Chunked())

    [<Fact>]
    let ``splits paths and matches route patterns`` () =
        Assert.Empty(HttpCore.splitPathSegments "/")
        Assert.Equal<string>([ "hello"; "world" ], HttpCore.splitPathSegments "/hello/world/")

        let pattern = RoutePattern.Parse "/hello/:name"
        let parameters = pattern.MatchPath "/hello/Adhithya"

        Assert.Equal(Some [ ("name", "Adhithya") ], parameters)
        Assert.Equal(None, pattern.MatchPath "/hello")
        Assert.Equal(None, pattern.MatchPath "/goodbye/Adhithya")

    [<Fact>]
    let ``route pattern handles root`` () =
        let pattern = RoutePattern.Parse "/"
        Assert.Empty(pattern.Segments)
        Assert.Equal(Some [], pattern.MatchPath "/")
        Assert.Equal(None, pattern.MatchPath "/extra")
