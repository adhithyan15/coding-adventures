# CodingAdventures.Hmac.FSharp

HMAC helpers for .NET, implemented directly with package-local hash primitives.

The package provides named HMAC helpers for MD5, SHA-1, SHA-256, and SHA-512, plus a generic RFC 2104 `compute` helper and constant-time tag verification.

```fsharp
open CodingAdventures.Hmac.FSharp

let tag = Hmac.hmacSha256 "key"B "message"B
let hex = Hmac.hmacSha256Hex "key"B "message"B

let ok = Hmac.verify tag (Hmac.hmacSha256 "key"B "message"B)
```
