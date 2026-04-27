# CodingAdventures.BarcodeLayout1D.CSharp

Shared layout utilities for one-dimensional barcode symbologies.

This package converts logical bar/space runs into `PaintScene` rectangle instructions, with quiet zones, symbol layout metadata, and render configuration.

```csharp
using CodingAdventures.BarcodeLayout1D;

var runs = BarcodeLayout1D.RunsFromBinaryPattern(
    "101",
    new RunsFromBinaryPatternOptions("guard", 0, Barcode1DRunRole.Guard));

var scene = BarcodeLayout1D.LayoutBarcode1D(runs);
```
