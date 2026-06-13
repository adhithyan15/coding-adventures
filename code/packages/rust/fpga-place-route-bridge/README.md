# fpga-place-route-bridge

Converts a gate-level `Netlist` (HNL format) into an FPGA JSON configuration describing CLB
placement, LUT truth tables, I/O pins, and routing stubs.

## What it does

The `fpga-place-route-bridge` crate bridges the technology-mapped gate netlist produced by
`tech-mapping` with the physical FPGA bitstream stage.  It:

1. **Looks up truth tables** for standard cell types (AND2, OR2, NOT, XOR2, MUX2, etc.) using a
   compile-time `match` table — no HashMap allocation at runtime.
2. **Expands truth tables** from the cell's native input count to the target LUT width (default:
   4-input, 16-entry) by repeating entries so upper address bits are don't-cares.
3. **Packs cells into CLBs** using row-major placement (`clb_{row}_{col}`).
4. **Emits I/O pin entries** for every module port.
5. **Emits routing stubs** connecting net sources to cell inputs.
6. **Returns a `serde_json::Value`** so callers can post-process, serialise to disk, or hand
   directly to `fpga-bitstream`.

## How it fits in the stack

```
gate-netlist-format (HNL)
         │
         ▼
fpga-place-route-bridge
         │
         ▼  JSON { clbs, io, routing, device }
         │
fpga-bitstream  (iCE40 bitstream bytes)
```

## Usage

```rust
use gate_netlist_format::Netlist;
use fpga_place_route_bridge::{hnl_to_fpga_json, FpgaBridgeOptions};

let nl: Netlist = /* loaded from file or built in memory */;

let opts = FpgaBridgeOptions {
    rows: 8, cols: 8,
    lut_inputs: 4,
    seed: 42,
};
let (cfg, report) = hnl_to_fpga_json(&nl, Some(&opts));

println!("cells packed: {}", report.cells_packed);
println!("unmapped:     {:?}", report.cells_unmapped);
println!("routes:       {}", report.routes_emitted);
println!("{}", serde_json::to_string_pretty(&cfg).unwrap());
```

## JSON schema

The returned `Value` has four top-level keys:

| Key       | Type             | Description                                      |
|-----------|------------------|--------------------------------------------------|
| `device`  | object           | `rows`, `cols`, `lut_inputs` from options        |
| `clbs`    | object (map)     | keyed by `clb_{row}_{col}`, each has `lut_a`     |
| `io`      | object (map)     | keyed by `io_{n}`, each has `name` and `dir`     |
| `routing` | array of objects | `{ from, to }` routing stubs                     |

Each `lut_a` entry has `truth_table: [0|1, ...]` with `2^lut_inputs` entries.

## Supported cell types

BUF, NOT, AND2, OR2, NAND2, NOR2, XOR2, XNOR2, AND3, OR3, NAND3, NOR3, XOR3, AND4, OR4, NAND4,
NOR4, MUX2, CONST_0, CONST_1.

Unknown cell types are listed in `FpgaBridgeReport::cells_unmapped` and skipped; the caller
decides whether to error or warn.

## Truth table expansion

A 1-input cell (BUF, NOT) has a 2-entry truth table.  When targeting a 4-input LUT the table is
expanded to 16 entries by repeating: the upper address bits become don't-cares.  The expansion
formula is: `entry[i] = original[i % 2^n_inputs]`.
