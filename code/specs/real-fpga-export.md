# Real-FPGA Export

## Overview

Emits structural Verilog and EDIF from HNL/HIR for use with the standard open-tool FPGA flow: `yosys` (synthesis) → `nextpnr` (place-and-route) → `icepack`/`ecppack` (bitstream) → `iceprog`/`openFPGAloader` (programming). This bypasses our internal synthesis + P&R for the path-to-real-hardware: write Verilog, hand to yosys, get a real iCE40 bitstream.

Why have this when we have our own synthesis and P&R? Two reasons:
1. **Early end-to-end win.** Implementing yosys-quality synthesis + real-FPGA P&R is months of work. Verilog emission is a few days. The user's adder runs on a real iCE40 board *now*, not in 6 months.
2. **Cross-validation oracle.** When our internal synthesis is implemented, comparing our results to yosys's establishes correctness. Disagreements are bugs; agreements are confidence.

This spec defines:
1. The HIR-to-Verilog backwriter (preferred path; preserves more structure than HNL-to-Verilog).
2. The HNL-to-Verilog backwriter (alternative path; useful when we have post-synthesis HNL but no HIR).
3. The HIR-to-EDIF backwriter (for `nextpnr` direct input).
4. PCF (pin constraint file) generation for IceStorm.
5. The driver script that shells out to yosys/nextpnr/icepack.

## Layer Position

```
HIR  ────► hir-to-verilog ────►  ── Verilog ──┐
                                              │
HNL  ────► hnl-to-verilog ────►              ▼
                                          yosys (synth)
HIR  ────► hir-to-edif    ────►              │
                                              ▼
                                          nextpnr-ice40
                                              │
                                              ▼
                                          icepack
                                              │
                                              ▼
                                          .bin → iceprog → real iCE40
```

## HIR-to-Verilog Backwriter

The backwriter walks the HIR tree and emits IEEE 1364-2005 Verilog. Two modes:

- **Preserve mode**: maintain hierarchy, named processes, named blocks, comments.
- **Flatten mode**: inline all sub-modules; emit a single large module. Useful when the downstream tool struggles with hierarchy.

### Identifier escaping

Verilog identifiers are `[A-Za-z_][A-Za-z0-9_$]*`. HIR allows broader names (especially via Ruby DSL where `_` is fine but unicode might appear). Escape via Verilog's escaped-identifier syntax: `\foo.bar ` (note trailing space).

### Type translation

| HIR type | Verilog |
|---|---|
| `TyLogic` | `wire` (default) or `reg` (if assigned in always) |
| `TyBit` | `bit` (SystemVerilog) — fall back to `wire` for IEEE 1364 |
| `TyStdLogic` | `wire`; multi-driver attributes lost |
| `TyVector(elem, n)` | `[n-1:0] x` |
| `TyInteger(low, high)` | `integer x` (with implicit signed/unsigned by range) |
| `TyReal` | `real x` |
| `TyEnum(name, members)` | `parameter` for each member; signal width = log2(N) |
| `TyRecord(fields)` | Bit-blast into individual signals (Verilog has no struct outside SV) |

### Process emission

| HIR Process kind | Verilog |
|---|---|
| `ALWAYS` with sensitivity | `always @(...) begin ... end` |
| `INITIAL` | `initial begin ... end` |
| `PROCESS` (VHDL → Verilog) | `always @(...)` if synthesizable shape; else `initial` |

### Statement emission

Straightforward for if/case/loops. Tricky cases:
- VHDL `wait until rising_edge(clk)` → `@(posedge clk)`.
- VHDL `wait for 10 ns` → `#10`.
- VHDL `wait` (forever) → emit a comment + termination; not synthesizable.
- VHDL aliases → resolve at emission.
- VHDL records → bit-blast.

### Worked Example

HIR:
```python
Module(name="adder4", ports=[...], cont_assigns=[
  ContAssign(target=Concat((PortRef("cout"), PortRef("sum"))),
             rhs=BinaryOp("+", BinaryOp("+", PortRef("a"), PortRef("b")), PortRef("cin")))
])
```

Emitted Verilog:
```verilog
module adder4 (
  input  [3:0] a,
  input  [3:0] b,
  input        cin,
  output [3:0] sum,
  output       cout
);
  assign {cout, sum} = a + b + cin;
endmodule
```

## HNL-to-Verilog Backwriter

For post-synthesis HNL → structural Verilog. Each HNL Cell becomes a Verilog primitive instantiation:

```verilog
module adder4 (input [3:0] a, input [3:0] b, input cin,
               output [3:0] sum, output cout);
  wire c0, c1, c2;
  
  full_adder u_fa0 (.a(a[0]), .b(b[0]), .cin(cin),
                    .sum(sum[0]), .cout(c0));
  full_adder u_fa1 (.a(a[1]), .b(b[1]), .cin(c0),
                    .sum(sum[1]), .cout(c1));
  full_adder u_fa2 (.a(a[2]), .b(b[2]), .cin(c1),
                    .sum(sum[2]), .cout(c2));
  full_adder u_fa3 (.a(a[3]), .b(b[3]), .cin(c2),
                    .sum(sum[3]), .cout(cout));
endmodule

module full_adder (input a, b, cin, output sum, cout);
  wire axb, ab, axbc;
  xor (axb, a, b);
  xor (sum, axb, cin);
  and (ab, a, b);
  and (axbc, axb, cin);
  or  (cout, ab, axbc);
endmodule
```

Each HNL primitive (`AND2`, `XOR2`, `DFF`) emits a Verilog gate primitive (`and`, `xor`, `dff` wrapper).

For Sky130-mapped HNL (level=stdcell), emit module instantiations matching Sky130 cell ports:

```verilog
sky130_fd_sc_hd__nand2_1 u_nand2_1 (.A(a), .B(b), .Y(y));
```

Yosys recognizes Sky130 cells; nextpnr maps them onto the FPGA fabric or treats them as black-box for ASIC.

## HIR-to-EDIF Backwriter

EDIF (per `gate-netlist-format.md` §"EDIF Importer / Exporter") is the older format that `nextpnr` accepts directly, bypassing yosys entirely. We emit EDIF for cases where:
- The HNL is already post-synthesis and we don't want yosys to re-synthesize.
- The user wants to lock the netlist exactly.

Less commonly used; provided for completeness.

## PCF (Pin Constraint File)

For IceStorm flows, the PCF maps top-module ports to physical iCE40 pins:

```pcf
# 4-bit adder on iCE40-HX1K-EVN
set_io a[0] 105
set_io a[1] 106
set_io a[2] 107
set_io a[3] 110
set_io b[0] 112
set_io b[1] 113
set_io b[2] 114
set_io b[3] 115
set_io cin 117
set_io sum[0] 118
set_io sum[1] 122
set_io sum[2] 124
set_io sum[3] 128
set_io cout 129
```

We don't generate the pin assignments automatically (they depend on board-specific wiring); the user provides a PCF or we copy from a board-specific template.

## Driver

A Python wrapper that orchestrates the open-tool flow:

```python
def to_ice40(hir: HIR, top: str, board: str = "ice40-hx1k-evn",
             pcf: Path | None = None, out_dir: Path = Path("build")) -> Path:
    """Run the full toolchain: emit Verilog, run yosys/nextpnr/icepack.
    
    Returns: path to the produced .bin file.
    """
    # 1. Emit Verilog
    write_verilog(hir, out_dir / f"{top}.v")
    
    # 2. Synthesize with yosys
    run("yosys", "-q", "-p", f"synth_ice40 -top {top} -json {out_dir}/{top}.json",
        f"{out_dir}/{top}.v")
    
    # 3. Place-and-route with nextpnr
    if pcf is None:
        pcf = default_pcf_for_board(board)
    run("nextpnr-ice40", "--hx1k", "--package", "tq144",
        "--json", f"{out_dir}/{top}.json",
        "--pcf", str(pcf),
        "--asc", f"{out_dir}/{top}.asc")
    
    # 4. Bitstream with icepack
    run("icepack", f"{out_dir}/{top}.asc", f"{out_dir}/{top}.bin")
    
    return out_dir / f"{top}.bin"


def program_ice40(bin_path: Path) -> None:
    """Flash the bitstream to a real iCE40 board via iceprog."""
    run("iceprog", str(bin_path))
```

User flow:
```python
from real_fpga_export import to_ice40, program_ice40

hir = HIR.from_verilog("adder4.v")
bin_path = to_ice40(hir, top="adder4", pcf=Path("ice40-hx1k-evn.pcf"))
program_ice40(bin_path)   # flash to real board
```

## Public API

```python
from dataclasses import dataclass
from pathlib import Path


@dataclass
class VerilogEmitOptions:
    flatten: bool = False
    preserve_provenance: bool = True
    use_systemverilog: bool = False   # SV features like 'logic' instead of 'wire'/'reg'


def write_verilog(hir: "HIR", path: Path, options: VerilogEmitOptions = VerilogEmitOptions()) -> None: ...
def write_verilog_from_hnl(hnl: "Netlist", path: Path) -> None: ...
def write_edif(hir: "HIR", path: Path) -> None: ...

@dataclass
class ToolchainOptions:
    yosys: Path = Path("yosys")
    nextpnr: Path = Path("nextpnr-ice40")
    icepack: Path = Path("icepack")
    iceprog: Path = Path("iceprog")
    timeout_s: int = 600


def to_ice40(hir: "HIR", top: str, board: str, pcf: Path | None,
             out_dir: Path, opts: ToolchainOptions = ToolchainOptions()) -> Path: ...
```

## Edge Cases

| Scenario | Handling |
|---|---|
| HIR construct not synthesizable (initial, file I/O) | Emit with comment; yosys will reject; user warned. |
| Verilog reserved word as HIR identifier | Escape via `\foo `. |
| Cyclic instantiation | Detected at emission; error. |
| Toolchain not installed | Driver detects missing executables; raises clear error. |
| PCF doesn't match top module's ports | yosys/nextpnr error; surface to user. |
| iCE40 part too small (overflow LUTs) | nextpnr error; surface; suggest larger part. |
| External IP black-box (e.g., a SerDes macro) | Emit module declaration with no body; yosys with `-blackbox` flag. |

## Test Strategy

### Unit (95%+)
- HIR-to-Verilog emission for every node type.
- HNL-to-Verilog for every cell type.
- Identifier escaping.
- Round-trip: Verilog → HIR → Verilog → HIR (semantically equivalent).

### Integration
- 4-bit adder: emitted Verilog passes yosys synthesis without warnings.
- 4-bit adder + PCF: full toolchain run produces a `.bin` file.
- (If hardware available) Flash to iCE40-HX1K-EVN and verify behavior with hardware test vectors.
- ARM1 reference: emits Verilog; passes yosys; nextpnr places (if part is large enough).

## Conformance

| Standard | Coverage |
|---|---|
| **IEEE 1364-2005** Verilog | Full output (synthesizable subset) |
| **IEEE 1800** SystemVerilog | Optional output (`use_systemverilog=True`) |
| **EDIF 2.0.0** | Subset (`netlist` view) |
| **IceStorm PCF** | Full |
| **Yosys command interface** | Stable; we use documented commands only |
| **nextpnr-ice40 command interface** | Stable |
| **Project X-Ray** (Xilinx 7-series open flow) | Future spec |
| **Project Trellis** (ECP5 open flow) | Future spec |

## Open Questions

1. **Should we cache toolchain artifacts?** Yes; rebuild only when source changes.
2. **Multi-board support** — provide PCF templates for common boards (HX1K-EVN, UP5K-Breakout, ULX3S, ICEBreaker, etc.). Yes.
3. **Yosys script customization** — let users pass extra synth flags? Yes via `synth_extra: str` option.

## Future Work

- ECP5 support via Project Trellis.
- Xilinx 7-series via Project X-Ray.
- GoWin / Efinix support.
- IP integration (Wishbone, AXI).
- Hardware-accelerated regression testing on a fleet of boards.
