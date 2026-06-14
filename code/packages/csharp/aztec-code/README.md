# CodingAdventures.AztecCode.CSharp

Native C# Aztec Code encoder for .NET. It encodes UTF-8 strings or raw byte
payloads into a `ModuleGrid` from `CodingAdventures.Barcode2D`.

```csharp
using CodingAdventures.AztecCode;

var grid = AztecCodeEncoder.Encode("HELLO");
var highEcc = AztecCodeEncoder.Encode("MY DATA", new AztecOptions(50));
var binary = AztecCodeEncoder.EncodeBytes([0x01, 0x02, 0xff]);
```

The encoder implements the same v0.1.0 byte-mode pipeline as the F# package:
binary-shift byte encoding, automatic compact/full symbol selection, GF(256)
Reed-Solomon over primitive polynomial `0x12D`, bit stuffing, GF(16)
mode-message ECC, bullseye/orientation/reference patterns, and clockwise data
placement.

Current limitations match the F# package: byte mode only, 8-bit data codewords
only, default 23 percent ECC, and no force-compact option.
