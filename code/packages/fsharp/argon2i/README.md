# CodingAdventures.Argon2i.FSharp

Pure F# Argon2i password hashing following RFC 9106. The implementation uses
the existing [`blake2b`](../blake2b) package for the initial and
variable-length hashes, then implements Argon2's memory matrix, modified
BLAKE2 round, and data-independent address stream directly.

Argon2i keeps its memory access pattern independent of password-derived data,
which protects against side-channel observation. For general password hashing,
RFC 9106 recommends Argon2id instead.

```fsharp
open CodingAdventures.Argon2i.FSharp

let tag =
    Argon2i.deriveDefault
        "password"B
        "random-salt"B
        3
        65_536
        4
        32

let hex =
    Argon2i.deriveHex
        "password"B
        "random-salt"B
        3
        65_536
        4
        32
        { Argon2iOptions.Default with
            Key = "server-secret"B
            AssociatedData = "account-v1"B }
```

`memoryCost` is measured in 1 KiB blocks and must be at least eight times
`parallelism`. Only Argon2 version 1.3 (`0x13`) is supported. See
[`KD03-argon2.md`](../../../specs/KD03-argon2.md) for the shared algorithm
walk-through.

Run the test and coverage gate with:

```bash
dotnet test tests/CodingAdventures.Argon2i.Tests/CodingAdventures.Argon2i.Tests.fsproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line "/p:Include=[CodingAdventures.Argon2i.FSharp]*"
```
