# CodingAdventures.Sha512.CSharp

SHA-512 helpers for .NET, implemented directly from FIPS 180-4.

The package exposes one-shot hashing plus a streaming-style hasher whose `Digest` calls are non-destructive.

```csharp
using CodingAdventures.Sha512;

byte[] digest = Sha512.Hash("abc"u8.ToArray());
string hex = Sha512.HashHex("abc"u8.ToArray());

var hasher = new Sha512Hasher();
hasher.Update("ab"u8.ToArray()).Update("c"u8.ToArray());
string streamed = hasher.HexDigest();
```
