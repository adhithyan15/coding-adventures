# CodingAdventures.HashFunctions.FSharp

Non-cryptographic hash functions for .NET.

This package mirrors the repository hash-functions surface with FNV-1a, DJB2, polynomial rolling, MurmurHash3, SipHash-2-4, and small analysis helpers.

```fsharp
open CodingAdventures.HashFunctions.FSharp

let fnv = HashFunctions.fnv1a32 "hello"
let djb = HashFunctions.djb2 "abc"
let murmur = HashFunctions.murmur3_32 "abc"

let chi2 =
    HashFunctions.distributionTest
        (fun bytes -> HashFunctions.fnv1a64Bytes bytes)
        inputs
        16
```
