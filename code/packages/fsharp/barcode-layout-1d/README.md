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
