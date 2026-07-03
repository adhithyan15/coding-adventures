# tech-mapping

Technology mapping pass: converts a **generic-gate HNL** (`AND2`, `OR2`, `XOR2`, `DFF`, …) into a **Sky130 HD stdcell HNL** (`sky130_fd_sc_hd__and2_1`, etc.).

## Pipeline position

```
HIR → synthesis → HNL[GENERIC] → tech-mapping → HNL[STDCELL]
                                                      │
                                        asic-floorplan ◄────────┘
```

## What it does

1. **Cell rename** — each generic type maps to a real Sky130 HD drive-1 cell name.
2. **Pin remap** — Sky130 uses slightly different pin names (`AND2.Y → X`, `BUF.Y → X`).
3. **INV–INV bubble cancellation** — back-to-back inverters on the same net cancel.

## Usage

```rust
use tech_mapping::map_to_sky130;
use synthesis::synthesize;

let hnl_generic = synthesize(&hir);
let (hnl_stdcell, report) = map_to_sky130(&hnl_generic);

println!("cells: {} → {}", report.cells_before, report.cells_after);
println!("bubbles canceled: {}", report.bubbles_canceled);
println!("unmapped: {:?}", report.unmapped);
```

## Default cell map (Sky130 HD, drive=1)

| Generic  | Sky130 stdcell                    | Pin remap        |
|----------|-----------------------------------|------------------|
| BUF      | sky130_fd_sc_hd__buf_1            | Y → X            |
| NOT      | sky130_fd_sc_hd__inv_1            | (identity)       |
| AND2     | sky130_fd_sc_hd__and2_1           | Y → X            |
| OR2      | sky130_fd_sc_hd__or2_1            | Y → X            |
| XOR2     | sky130_fd_sc_hd__xor2_1           | Y → X            |
| NAND2    | sky130_fd_sc_hd__nand2_1          | Y → Y            |
| NOR2     | sky130_fd_sc_hd__nor2_1           | Y → Y            |
| MUX2     | sky130_fd_sc_hd__mux2_1           | A→A0, B→A1, Y→X |
| DFF      | sky130_fd_sc_hd__dfxtp_1          | (identity)       |
| CONST_0  | sky130_fd_sc_hd__conb_1           | Y → LO           |
| CONST_1  | sky130_fd_sc_hd__conb_1           | Y → HI           |
| …        | …                                 | …                |

## Testing

```
cargo test -p tech-mapping -- --nocapture
```

9 tests covering: cell rename, pin remap, unmapped passthrough, INV–INV cancellation, MappingReport counts.
