# CodingAdventures.Zeroize.CSharp

Helpers for clearing sensitive managed buffers in place.

```csharp
using CodingAdventures.Zeroize;

byte[] secret = [1, 2, 3, 4];
Zeroize.ZeroizeBytes(secret);

using var owned = new ZeroizingBuffer([1, 2, 3, 4]);
```
