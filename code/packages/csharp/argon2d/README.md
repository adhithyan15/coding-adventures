# CodingAdventures.Argon2d.CSharp

Pure C# Argon2d password hashing following RFC 9106. The implementation uses
the existing [`blake2b`](../blake2b) package for the initial and
variable-length hashes, then implements Argon2's memory matrix, modified
BLAKE2 round, and data-dependent block selection directly.

Argon2d uses password-derived memory addresses. This maximizes resistance to
time-memory trade-off attacks, but can expose those addresses through side
channels. For general password hashing, RFC 9106 recommends Argon2id instead.

```csharp
using CodingAdventures.Argon2d;

byte[] tag = Argon2d.Derive(
    "password"u8.ToArray(),
    "random-salt"u8.ToArray(),
    timeCost: 3,
    memoryCost: 65_536,
    parallelism: 4,
    tagLength: 32);

string hex = Argon2d.DeriveHex(
    "password"u8.ToArray(),
    "random-salt"u8.ToArray(),
    3,
    65_536,
    4,
    32,
    new Argon2dOptions
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
dotnet test tests/CodingAdventures.Argon2d.Tests/CodingAdventures.Argon2d.Tests.csproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line "/p:Include=[CodingAdventures.Argon2d]*"
```
