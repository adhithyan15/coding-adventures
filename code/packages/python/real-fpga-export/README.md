# real-fpga-export

HIR -> structural Verilog backwriter + driver for the open-tool FPGA flow (yosys, nextpnr, icepack, iceprog). The fast end-to-end win: write Verilog, hand to industrial tools, flash to a real iCE40 dev board.

See [`code/specs/real-fpga-export.md`](../../../specs/real-fpga-export.md).

## Quick start

```python
from hdl_elaboration import elaborate_verilog
from real_fpga_export import write_verilog, to_ice40

# Elaborate Verilog source -> HIR
hir = elaborate_verilog(adder_src, top="adder4")

# Emit a .v file
write_verilog(hir, "build/adder4.v")

# Or run the full toolchain (requires yosys/nextpnr/icepack on PATH)
result = to_ice40(hir, top="adder4", pcf=Path("ice40-hx1k-evn.pcf"))
print(result.bin_path)  # build/adder4.bin

# Optionally flash to a real board
# from real_fpga_export import program_ice40
# program_ice40(result.bin_path)
```

## Pipeline position

```
HIR (hdl-ir) ──> real-fpga-export (THIS PACKAGE)
                        |
                        v
                   .v Verilog file
                        |
                        v
                yosys (synth_ice40)
                        |
                        v
                nextpnr-ice40
                        |
                        v
                icepack -> .bin
                        |
                        v
              iceprog -> real iCE40 board
```

This bypasses our internal synthesis + P&R for the path-to-real-hardware. Useful as:
- An early end-to-end win (real hardware in days, not months).
- A cross-validation oracle once our internal synthesis is implemented (disagreements are bugs).

## v0.1.0 scope

- HIR -> Verilog backwriter:
  - Modules with input/output/inout ports (1-bit and N-bit vectors)
  - Internal nets
  - Continuous assignments
  - Hierarchical instance instantiation with parameter and port maps
  - Expression nodes: Lit, NetRef, PortRef, VarRef, Slice, Concat, Replication, UnaryOp, BinaryOp, Ternary
  - Identifier escaping for Verilog reserved words (`\name `)
- Toolchain driver:
  - `to_ice40(hir, top, pcf, ...)` runs yosys -> nextpnr-ice40 -> icepack
  - `program_ice40(bin_path)` runs iceprog
  - Detects missing tools on PATH; per-step timeouts; captures stdout/stderr in `ToolchainResult`

## Out of scope (v0.2.0)

- Behavioral process emission (`always @(...)` blocks): currently the writer only handles ContAssign-based combinational HIR.
- ECP5 (Project Trellis) and Xilinx 7-series (Project X-Ray) flows.
- EDIF backwriter (could complement Verilog for direct nextpnr input).
- Automatic PCF generation from port lists.

MIT.
