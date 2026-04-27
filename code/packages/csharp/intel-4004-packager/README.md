# CodingAdventures.Intel4004Packager.CSharp

Intel HEX ROM image encoder and decoder for Intel 4004 tooling.

```csharp
using CodingAdventures.Intel4004Packager;

var hex = Intel4004Packager.EncodeHex([0xD5, 0x01]);
var decoded = Intel4004Packager.DecodeHex(hex);
```

The encoder emits 16-byte data records plus the required EOF record. The
decoder verifies record checksums and reconstructs the binary image.
