---
category: Swift
---

# Foundation JSONDecoder silently accepts duplicate object member names

`JSONDecoder` can decode an object with the same member twice without reporting
an error, and keyed-container `allKeys` exposes only one name. For strict
replay, scan the bounded raw JSON structure first, decode each quoted key with
Foundation so escaped aliases compare equal, and reject repeats within that
object. Keep `JSONDecoder` for the full report so signed `Int64` number tokens
stay exact; `JSONSerialization` would route numeric data through a less precise
representation.
