# CodingAdventures.BarcodeLayout1D.FSharp

Shared layout utilities for one-dimensional barcode symbologies.

This package converts logical bar/space runs into `PaintScene` rectangle instructions, with quiet zones, symbol layout metadata, and render configuration.

```fsharp
open CodingAdventures.BarcodeLayout1D.FSharp

let runs =
    BarcodeLayout1D.runsFromBinaryPattern
        "101"
        { SourceLabel = "guard"; SourceIndex = 0; Role = Guard }

let scene = BarcodeLayout1D.layoutBarcode1D runs None
```

## Portable v1 API

The additive `BarcodeLayout1DV1` module implements the bounded, stable-error
`barcode-layout-1d-v1` contract. Existing `BarcodeLayout1D` functions remain
unchanged for downstream compatibility.

```fsharp
let portableRuns =
    BarcodeLayout1DV1.expandBinary
        "101"
        { SourceLabel = "guard"
          SourceIndex = 0L
          Role = Guard }

let portableScene = BarcodeLayout1DV1.projectScene portableRuns None
```

Portable v1 projection is integer-only and rectangle-only. Human-readable text
requests fail with `human-readable-text-unsupported` before native resolution.
