# CodingAdventures.Sha512.FSharp

SHA-512 helpers for .NET, implemented directly from FIPS 180-4.

The package exposes one-shot hashing plus a streaming-style hasher whose `Digest` calls are non-destructive.

```fsharp
open System.Text
open CodingAdventures.Sha512.FSharp

let digest = Sha512.hash (Encoding.ASCII.GetBytes "abc")
let hex = Sha512.hashHex (Encoding.ASCII.GetBytes "abc")

let hasher = Sha512Hasher()
hasher.Update(Encoding.ASCII.GetBytes "ab").Update(Encoding.ASCII.GetBytes "c") |> ignore
let streamed = hasher.HexDigest()
```
