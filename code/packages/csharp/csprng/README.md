# CodingAdventures.Csprng.CSharp

Cryptographically secure random byte and integer helpers for .NET.

This package is the repository's explicit platform entropy boundary. It reads
from the operating system CSPRNG through `RandomNumberGenerator`; sibling
packages should depend on this package instead of opening their own direct
platform-randomness dependency.

```csharp
using CodingAdventures.Csprng;

byte[] nonce = Csprng.RandomBytes(24);
uint id = Csprng.RandomUInt32();
```
