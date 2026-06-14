# CodingAdventures.Blake2b.CSharp

BLAKE2b helpers for .NET, implemented directly from RFC 7693.

The package exposes one-shot hashing, keyed MAC mode, salt and personalization
parameters, plus a streaming-style hasher whose `Digest` calls are
non-destructive.

```csharp
using CodingAdventures.Blake2b;

byte[] digest = Blake2b.Hash("abc"u8.ToArray());
string hex = Blake2b.HashHex("abc"u8.ToArray());

var keyed = Blake2bOptions.Default
    .WithDigestSize(32)
    .WithKey("shared secret"u8.ToArray());

byte[] tag = Blake2b.Hash("message"u8.ToArray(), keyed);

var hasher = new Blake2bHasher(Blake2bOptions.Default.WithDigestSize(32));
hasher.Update("partial "u8.ToArray()).Update("payload"u8.ToArray());
string streamed = hasher.HexDigest();
```
