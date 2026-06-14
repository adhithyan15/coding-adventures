# CodingAdventures.Blake2b.FSharp

BLAKE2b helpers for .NET, implemented directly from RFC 7693.

The package exposes one-shot hashing, keyed MAC mode, salt and personalization
parameters, plus a streaming-style hasher whose `Digest` calls are
non-destructive.

```fsharp
open CodingAdventures.Blake2b.FSharp

let digest = Blake2b.hash "abc"B
let hex = Blake2b.hashHex "abc"B

let keyed =
    Blake2bOptions.Default
        .WithDigestSize(32)
        .WithKey("shared secret"B)

let tag = Blake2b.hashWithOptions keyed "message"B

let hasher = Blake2bHasher(Blake2bOptions.Default.WithDigestSize(32))
hasher.Update("partial "B).Update("payload"B) |> ignore
let streamed = hasher.HexDigest()
```
