# FPGA (C#)

Pure C# simulation primitives for a small field-programmable gate array. The
package composes the existing logic-gates and block-ram packages into SRAM-backed
lookup tables, slices, configurable logic blocks, routing matrices, and I/O pads.

```csharp
using CodingAdventures.Fpga;

var table = new[] { 0, 0, 0, 1 };
var lut = new LUT(2, table);
var output = lut.Evaluate([1, 1]); // 1

var bitstream = Bitstream.ParseJson("""
    {"lut_k":2,"io":{"led":{"mode":"output"}}}
    """);
var fpga = new FPGA(bitstream);
fpga.DriveOutput("led", 1);
```

Inputs are represented as integer bits (`0` or `1`), with the first LUT input
as the least-significant truth-table index bit. Bitstreams can be constructed in
code or parsed from caller-provided JSON; the package performs no file or network
access.
