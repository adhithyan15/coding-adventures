# CodingAdventures.Hkdf.FSharp

HKDF extract-and-expand helpers for .NET, implemented directly with package-local HMAC and hash primitives.

```fsharp
open CodingAdventures.Hkdf.FSharp

let okm =
    Hkdf.deriveSha256 "salt"B "input keying material"B "context"B 32
```
