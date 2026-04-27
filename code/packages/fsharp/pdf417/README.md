# CodingAdventures.PDF417.FSharp

F# facade for the shared .NET PDF417 stacked linear barcode encoder.

## Usage

```fsharp
open CodingAdventures.PDF417.FSharp

let grid = PDF417.encode "HELLO WORLD"
let custom =
    PDF417.encodeText
        "HELLO WORLD"
        (Some { PDF417.defaultOptions with Columns = Some 5; RowHeight = 1 })
```

The returned grid is the `CodingAdventures.Barcode2D.ModuleGrid` produced by the C# PDF417 engine, so it can be sent directly to the .NET barcode layout pipeline.
