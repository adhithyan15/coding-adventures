# CodingAdventures.Pbkdf2.FSharp

PBKDF2 key derivation helpers for .NET, implemented directly with package-local HMAC and hash primitives.

```fsharp
open CodingAdventures.Pbkdf2.FSharp

let key = Pbkdf2.pbkdf2HmacSha256 "password"B "salt"B 100_000 32
let hex = Pbkdf2.pbkdf2HmacSha256Hex "password"B "salt"B 100_000 32
```
