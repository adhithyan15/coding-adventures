# CodingAdventures.UrlParser.CSharp

RFC-style URL parser with relative resolution and percent encoding for C#.

```csharp
using CodingAdventures.UrlParser;

var url = Url.Parse("https://example.com:8080/path?q=hello#top");

Console.WriteLine(url.Scheme);          // https
Console.WriteLine(url.Host);            // example.com
Console.WriteLine(url.EffectivePort()); // 8080

var baseUrl = Url.Parse("http://example.com/a/b/c");
var resolved = baseUrl.Resolve("../d");
Console.WriteLine(resolved.ToUrlString()); // http://example.com/a/d

Console.WriteLine(UrlParser.PercentEncode("hello world")); // hello%20world
```

## API

- `Url.Parse(input)` parses an absolute URL.
- `Url.Resolve(relative)` resolves a relative URL against a base URL.
- `Url.EffectivePort()` returns an explicit port or an HTTP/HTTPS/FTP default.
- `Url.Authority()` reconstructs `[userinfo@]host[:port]`.
- `Url.ToUrlString()` serializes the URL.
- `UrlParser.PercentEncode(input)` and `UrlParser.PercentDecode(input)` handle UTF-8 percent encoding.
