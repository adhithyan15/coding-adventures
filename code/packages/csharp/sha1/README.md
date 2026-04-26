# CodingAdventures.Sha1.CSharp

SHA-1 helpers for .NET, backed by `System.Security.Cryptography.SHA1`.

SHA-1 is included for compatibility and learning parity with the repository's hash-function packages. New security-sensitive systems should prefer SHA-256 or stronger hashes.

```csharp
using CodingAdventures.Sha1;

byte[] digest = Sha1.Hash("abc"u8.ToArray());
string hex = Sha1.HashHex("abc"u8.ToArray());

var hasher = new Sha1Hasher();
hasher.Update("ab"u8.ToArray()).Update("c"u8.ToArray());
string streamed = hasher.HexDigest();
```
