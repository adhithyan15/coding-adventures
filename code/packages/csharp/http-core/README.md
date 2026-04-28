# CodingAdventures.HttpCore.CSharp

Shared HTTP semantic types and helpers for C#.

```csharp
using CodingAdventures.HttpCore;

var version = HttpVersion.Parse("HTTP/1.1");
var head = new RequestHead("GET", "/", version, [new Header("Accept", "*/*")]);
```

Includes request/response heads, headers, body framing modes, content helper
parsers, and simple route pattern matching.
