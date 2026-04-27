# CodingAdventures.Intel4004Packager.FSharp

Intel HEX ROM image encoder and decoder for Intel 4004 tooling.

```fsharp
open CodingAdventures.Intel4004Packager.FSharp

let hex = Intel4004Packager.encodeHexAtZero [| 0xD5uy; 0x01uy |]
let decoded = Intel4004Packager.decodeHex hex
```

The encoder emits 16-byte data records plus the required EOF record. The
decoder verifies record checksums and reconstructs the binary image.
