# CodingAdventures.Zeroize.FSharp

Helpers for clearing sensitive managed buffers in place.

```fsharp
open CodingAdventures.Zeroize.FSharp

let secret = [| 1uy; 2uy; 3uy |]
Zeroize.zeroizeBytes secret
```
