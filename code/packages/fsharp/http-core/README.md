# CodingAdventures.HttpCore.FSharp

Shared HTTP semantic types and helpers for F#.

```fsharp
open CodingAdventures.HttpCore.FSharp

let version = HttpVersion.Parse "HTTP/1.1"
let head = { Method = "GET"; Target = "/"; Version = version; Headers = [ { Name = "Accept"; Value = "*/*" } ] }
```

Includes request/response heads, headers, body framing modes, content helper
parsers, and simple route pattern matching.
