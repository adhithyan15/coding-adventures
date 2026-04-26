# CodingAdventures.Csprng.FSharp

Cryptographically secure random byte and integer helpers for .NET.

```fsharp
open CodingAdventures.Csprng.FSharp

let nonce = Csprng.randomBytes 24
let id = Csprng.randomUInt32 ()
```
