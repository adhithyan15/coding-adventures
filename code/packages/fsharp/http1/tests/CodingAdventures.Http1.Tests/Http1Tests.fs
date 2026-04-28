namespace CodingAdventures.Http1.Tests

open System
open System.Text
open CodingAdventures.Http1.FSharp
open CodingAdventures.HttpCore.FSharp
open Xunit

type Http1Tests() =
    let bytes (text: string) = Encoding.Latin1.GetBytes(text)

    [<Fact>]
    member _.VersionExists() =
        Assert.Equal("0.1.0", Http1.VERSION)

    [<Fact>]
    member _.ParseSimpleRequest() =
        let parsed = Http1.parseRequestHead (bytes "GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")

        Assert.Equal("GET", parsed.Head.Method)
        Assert.Equal("/", parsed.Head.Target)
        Assert.Equal({ Major = 1us; Minor = 0us }, parsed.Head.Version)
        Assert.Equal("example.com", parsed.Head.Headers[0].Value)
        Assert.Equal(BodyKind.None(), parsed.BodyKind)

    [<Fact>]
    member _.ParsePostRequestWithContentLength() =
        let parsed = Http1.parseRequestHead (bytes "POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello")

        Assert.Equal(44, parsed.BodyOffset)
        Assert.Equal(BodyKind.ContentLength 5, parsed.BodyKind)

    [<Fact>]
    member _.ChunkedTransferEncodingWinsForRequests() =
        let parsed =
            Http1.parseRequestHead (bytes "POST / HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\nContent-Length: 99\r\n\r\n")

        Assert.Equal(BodyKind.Chunked(), parsed.BodyKind)

    [<Fact>]
    member _.ParseResponseHead() =
        let parsed = Http1.parseResponseHead (bytes "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody")

        Assert.Equal(200us, parsed.Head.Status)
        Assert.Equal("OK", parsed.Head.Reason)
        Assert.Equal(BodyKind.ContentLength 4, parsed.BodyKind)

    [<Fact>]
    member _.ResponseWithoutLengthUsesUntilEof() =
        let parsed = Http1.parseResponseHead (bytes "HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n")

        Assert.Equal(BodyKind.UntilEof(), parsed.BodyKind)

    [<Fact>]
    member _.BodylessStatusCodesIgnoreBodyHeaders() =
        let parsed = Http1.parseResponseHead (bytes "HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n")

        Assert.Equal(BodyKind.None(), parsed.BodyKind)

    [<Fact>]
    member _.AcceptsLfOnlyAndPreservesDuplicateHeaders() =
        let parsed = Http1.parseResponseHead (bytes "\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload")

        Assert.Equal<string list>([ "a=1"; "b=2" ], parsed.Head.Headers |> List.map _.Value)

    [<Fact>]
    member _.DecodesHeaderValuesAsLatin1() =
        let parsed = Http1.parseRequestHead (bytes "GET / HTTP/1.1\r\nX-Name: café\r\n\r\n")

        Assert.Equal("café", parsed.Head.Headers[0].Value)

    [<Fact>]
    member _.InvalidInputsRaiseParseExceptions() =
        let assertError thunk =
            Assert.Throws<Http1ParseException>(fun () -> thunk () |> ignore) |> ignore

        assertError (fun () -> Http1.parseRequestHead (bytes "GET / HTTP/1.1\r\nHost: example.com"))
        assertError (fun () -> Http1.parseRequestHead (bytes "GET / too many HTTP/1.1\r\n\r\n"))
        assertError (fun () -> Http1.parseRequestHead (bytes "GET / HTTP/1.1\r\nHost example.com\r\n\r\n"))
        assertError (fun () -> Http1.parseRequestHead (bytes "GET / HTTP/1.1\r\n: nope\r\n\r\n"))
        assertError (fun () -> Http1.parseRequestHead (bytes "GET / NOPE\r\n\r\n"))
        assertError (fun () -> Http1.parseResponseHead (bytes "NOPE 200 OK\r\n\r\n"))
        assertError (fun () -> Http1.parseResponseHead (bytes "HTTP/1.1 NOPE OK\r\n\r\n"))
        assertError (fun () -> Http1.parseResponseHead (bytes "HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n"))
