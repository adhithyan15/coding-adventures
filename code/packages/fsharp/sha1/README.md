# CodingAdventures.Sha1.FSharp

SHA-1 helpers for .NET, implemented directly from FIPS 180-4.

SHA-1 is included for compatibility and learning parity with the repository's hash-function packages. New security-sensitive systems should prefer SHA-256 or stronger hashes.

```fsharp
open System.Text
open CodingAdventures.Sha1.FSharp

let digest = Sha1.hash (Encoding.ASCII.GetBytes "abc")
let hex = Sha1.hashHex (Encoding.ASCII.GetBytes "abc")

let hasher = Sha1Hasher()
hasher.Update(Encoding.ASCII.GetBytes "ab").Update(Encoding.ASCII.GetBytes "c") |> ignore
let streamed = hasher.HexDigest()
```
