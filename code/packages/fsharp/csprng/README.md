# CodingAdventures.Csprng.FSharp

Cryptographically secure random byte and integer helpers for .NET.

This package is the repository's explicit platform entropy boundary. It reads
from the operating system CSPRNG through `RandomNumberGenerator`; sibling
packages should depend on this package instead of opening their own direct
platform-randomness dependency.

```fsharp
open CodingAdventures.Csprng.FSharp

let nonce = Csprng.randomBytes 24
let id = Csprng.randomUInt32 ()
```
