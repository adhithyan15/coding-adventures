# CodingAdventures.QRCode.CSharp

Native C# QR Code encoder for .NET. It encodes UTF-8 strings into a square
`ModuleGrid` from `CodingAdventures.Barcode2D`.

```csharp
using CodingAdventures.QRCode;

var grid = QRCodeEncoder.Encode("HELLO WORLD", EccLevel.M);
```

The encoder implements the same v0.1.0 surface as the F# QR package: numeric,
alphanumeric, and byte modes; automatic version selection; GF(256)
Reed-Solomon error correction; finder, separator, timing, alignment, format,
and version patterns; data zigzag placement; and all eight mask patterns with
ISO penalty scoring.

Kanji mode and manual mask/version selection are deferred.
