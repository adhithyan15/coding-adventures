# FPGA (F#)

Pure F# simulation primitives for a small field-programmable gate array. The
package composes the existing logic-gates and block-ram packages into SRAM-backed
lookup tables, slices, configurable logic blocks, routing matrices, and I/O pads.

```fsharp
open CodingAdventures.Fpga.FSharp

let table = [| 0; 0; 0; 1 |]
let lut = LUT(2, table)
let output = lut.Evaluate [| 1; 1 |] // 1

let bitstream =
    Bitstream.ParseJson """{"lut_k":2,"io":{"led":{"mode":"output"}}}"""

let fpga = FPGA bitstream
fpga.DriveOutput("led", 1)
```

Inputs are represented as integer bits (`0` or `1`), with the first LUT input
as the least-significant truth-table index bit. Bitstreams can be constructed in
code or parsed from caller-provided JSON; the package performs no file or network
access.
