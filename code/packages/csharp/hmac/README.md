# CodingAdventures.Hmac.CSharp

HMAC helpers for .NET, implemented directly with package-local hash primitives.

The package provides named HMAC helpers for MD5, SHA-1, SHA-256, and SHA-512, plus a generic RFC 2104 `Compute` helper and constant-time tag verification.

```csharp
using CodingAdventures.Hmac;

byte[] tag = Hmac.HmacSha256("key"u8.ToArray(), "message"u8.ToArray());
string hex = Hmac.HmacSha256Hex("key"u8.ToArray(), "message"u8.ToArray());

bool ok = Hmac.Verify(tag, Hmac.HmacSha256("key"u8.ToArray(), "message"u8.ToArray()));
```
