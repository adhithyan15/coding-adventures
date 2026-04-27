# CodingAdventures.Sha256.CSharp

SHA-256 helpers for .NET, implemented directly from FIPS 180-4.

The package exposes one-shot hashing plus a streaming-style hasher whose `Digest` calls are non-destructive.

```csharp
using CodingAdventures.Sha256;

byte[] digest = Sha256.Hash("abc"u8.ToArray());
string hex = Sha256.HashHex("abc"u8.ToArray());

var hasher = new Sha256Hasher();
hasher.Update("ab"u8.ToArray()).Update("c"u8.ToArray());
string streamed = hasher.HexDigest();
```
