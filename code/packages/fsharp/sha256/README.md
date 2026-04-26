# CodingAdventures.Sha256.FSharp

SHA-256 helpers for .NET, backed by `System.Security.Cryptography.SHA256`.

The package exposes one-shot hashing plus a streaming-style hasher whose `Digest` calls are non-destructive.

```fsharp
open System.Text
open CodingAdventures.Sha256.FSharp

let digest = Sha256.hash (Encoding.ASCII.GetBytes "abc")
let hex = Sha256.hashHex (Encoding.ASCII.GetBytes "abc")

let hasher = Sha256Hasher()
hasher.Update(Encoding.ASCII.GetBytes "ab").Update(Encoding.ASCII.GetBytes "c") |> ignore
let streamed = hasher.HexDigest()
```
