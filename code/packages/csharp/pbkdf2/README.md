# CodingAdventures.Pbkdf2.CSharp

PBKDF2 key derivation helpers for .NET, backed by `Rfc2898DeriveBytes.Pbkdf2`.

```csharp
using CodingAdventures.Pbkdf2;

byte[] key = Pbkdf2.Pbkdf2HmacSha256("password"u8.ToArray(), "salt"u8.ToArray(), 100_000, 32);
string hex = Pbkdf2.Pbkdf2HmacSha256Hex("password"u8.ToArray(), "salt"u8.ToArray(), 100_000, 32);
```
