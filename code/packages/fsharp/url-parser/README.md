# CodingAdventures.UrlParser.FSharp

RFC-style URL parser with relative resolution and percent encoding for F#.

```fsharp
open CodingAdventures.UrlParser.FSharp

let url = Url.Parse "https://example.com:8080/path?q=hello#top"

printfn "%s" url.Scheme
printfn "%A" url.Host
printfn "%A" (url.EffectivePort())

let baseUrl = Url.Parse "http://example.com/a/b/c"
let resolved = baseUrl.Resolve "../d"
printfn "%s" (resolved.ToUrlString())

printfn "%s" (UrlParser.percentEncode "hello world")
```

## API

- `Url.Parse input` parses an absolute URL.
- `url.Resolve relative` resolves a relative URL against a base URL.
- `url.EffectivePort()` returns an explicit port or an HTTP/HTTPS/FTP default.
- `url.Authority()` reconstructs `[userinfo@]host[:port]`.
- `url.ToUrlString()` serializes the URL.
- `UrlParser.percentEncode input` and `UrlParser.percentDecode input` handle UTF-8 percent encoding.
