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

## Portable v1 API

The additive `BarcodeLayout1DV1` facade implements the bounded, stable-error
`barcode-layout-1d-v1` contract. Existing `BarcodeLayout1D` APIs remain
unchanged for downstream compatibility.

```csharp
var portableRuns = BarcodeLayout1DV1.ExpandBinary(
    "101",
    new Barcode1DV1BinaryOptions("guard", 0, Barcode1DRunRole.Guard));

var portableScene = BarcodeLayout1DV1.ProjectScene(portableRuns);
```

Portable v1 projection is integer-only and rectangle-only. Human-readable text
requests fail with `human-readable-text-unsupported` before native resolution.
