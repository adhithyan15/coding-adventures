# CodingAdventures.Hkdf.CSharp

HKDF extract-and-expand helpers for .NET.

```csharp
using CodingAdventures.Hkdf;

byte[] okm = Hkdf.DeriveSha256(
    salt: "salt"u8.ToArray(),
    ikm: "input keying material"u8.ToArray(),
    info: "context"u8.ToArray(),
    length: 32);
```
