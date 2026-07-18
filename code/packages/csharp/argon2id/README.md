# CodingAdventures.Argon2id.CSharp

Pure C# Argon2id password hashing following RFC 9106. The implementation uses
the existing [`blake2b`](../blake2b) package for the initial and
variable-length hashes, then implements Argon2's memory matrix, modified
BLAKE2 round, and hybrid address selection directly.

Argon2id uses data-independent addresses for the first half of its first pass,
then switches to password-derived addresses. This balances side-channel
resistance with protection against time-memory trade-off attacks, and RFC 9106
recommends Argon2id for general password hashing.

```csharp
using CodingAdventures.Argon2id;

byte[] tag = Argon2id.Derive(
    "password"u8.ToArray(),
    "random-salt"u8.ToArray(),
    timeCost: 3,
    memoryCost: 65_536,
    parallelism: 4,
    tagLength: 32);

string hex = Argon2id.DeriveHex(
    "password"u8.ToArray(),
    "random-salt"u8.ToArray(),
    3,
    65_536,
    4,
    32,
    new Argon2idOptions
    {
        Key = "server-secret"u8.ToArray(),
        AssociatedData = "account-v1"u8.ToArray(),
    });
```

`memoryCost` is measured in 1 KiB blocks and must be at least eight times
`parallelism`. Only Argon2 version 1.3 (`0x13`) is supported. See
[`KD03-argon2.md`](../../../specs/KD03-argon2.md) for the shared algorithm
walk-through.

Run the test and coverage gate with:

```bash
dotnet test tests/CodingAdventures.Argon2id.Tests/CodingAdventures.Argon2id.Tests.csproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line "/p:Include=[CodingAdventures.Argon2id]*"
```
