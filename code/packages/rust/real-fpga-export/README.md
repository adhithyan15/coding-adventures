# real-fpga-export

HIR → structural Verilog emitter and iCE40 open-tool driver (yosys / nextpnr-ice40 / icepack / iceprog).

## What it does

This crate bridges the in-memory HDL Intermediate Representation (`hdl-ir`) to the physical FPGA
toolchain.  It has two responsibilities:

1. **Verilog emission** — converts an `Hir` into IEEE 1364-2005 structural Verilog, which any
   synthesis tool can consume.
2. **Toolchain driver** — shells out to the iCE40 open-source flow (yosys → nextpnr-ice40 →
   icepack → iceprog) to synthesise, place-and-route, pack a bitstream, and optionally flash a
   real board over USB.

Both stages are independently usable: you can emit Verilog without touching the toolchain, and the
toolchain driver accepts a pre-written PCF constraints file.

## How it fits in the stack

```
hdl-ir  ──►  real-fpga-export  ──►  yosys
                │                     │
                │ (Verilog)            ▼
                │              nextpnr-ice40
                │                     │
                │ (JSON)              ▼
                │               icepack
                │                     │
                └──────────────────► (bin)
                                      │
                                   iceprog → board
```

## Usage

```rust
use hdl_ir::Hir;
use real_fpga_export::{write_verilog_str, to_ice40, ToolchainOptions};
use std::path::Path;

// Emit Verilog to a string
let v = write_verilog_str(&hir);
println!("{v}");

// Run the full iCE40 flow (skip missing tools gracefully)
let result = to_ice40(
    &hir,
    "top",
    Some(Path::new("constraints.pcf")),
    Path::new("build/"),
    "hx1k", "tq144",
    None,          // use default ToolchainOptions
    true,          // skip_missing: true → don't fail if yosys absent
).unwrap();
println!("Verilog written to {:?}", result.verilog_path);
```

## Verilog emitter details

- Emits one `module` block per HIR module, in arbitrary order.
- Port declarations use the standard `input` / `output` / `inout` keywords with `[w-1:0]` ranges
  for vector types and bare scalars for single-bit types.
- Internal wires are declared with `wire` statements.
- Continuous assignments map to `assign lhs = rhs;`.
- Sub-module instances use named port connections (`.pin(net)` syntax).
- All identifiers matching the IEEE 1364-2005 reserved-word list are emitted as escaped
  identifiers (`\name ` with trailing space) so the output is always legal.

## Supported HIR expression forms

| Expr variant          | Verilog output                    |
|-----------------------|-----------------------------------|
| `Lit(Int)`            | `<w>'d<value>`                    |
| `Lit(Bool)`           | `1'b0` / `1'b1`                   |
| `PortRef` / `NetRef`  | identifier (escaped if reserved)  |
| `Slice`               | `base[msb:lsb]`                   |
| `Concat`              | `{a, b, ...}`                     |
| `Replication`         | `{n{body}}`                       |
| `Unary`               | `(~a)`, `(-a)`, `(&a)` …          |
| `Binary`              | `(a & b)`, `(a + b)` …            |
| `Ternary`             | `(cond ? then : else)`            |
| `FunCall`             | `f(a, b)`                         |
| `SystemCall`          | `$display(...)` etc.              |

## Toolchain options

```rust
pub struct ToolchainOptions {
    pub yosys:         String,   // default "yosys"
    pub nextpnr_ice40: String,   // default "nextpnr-ice40"
    pub icepack:       String,   // default "icepack"
    pub iceprog:       String,   // default "iceprog"
    pub timeout_s:     u64,      // default 600
}
```

Set `skip_missing = true` to gracefully stop the flow when a tool is not on `PATH` instead of
returning an error — useful in CI environments without a full FPGA toolchain.
