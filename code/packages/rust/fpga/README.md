# fpga

**FPGA** — Field-Programmable Gate Array simulation with LUTs, CLBs, routing, and bitstream configuration.

## What is this?

This crate models a simplified but structurally accurate FPGA, following the Xilinx-style architecture. It builds on `logic-gates` (for combinational and sequential primitives) and `block-ram` (for SRAM-based LUT storage).

## Module hierarchy

```
LUT            -- K-input look-up table (the atom of programmable logic)
    |
Slice          -- 2 LUTs + 2 flip-flops + carry chain
    |
CLB            -- 2 slices (the core compute tile)
    |
SwitchMatrix   -- programmable routing crossbar
IOBlock        -- bidirectional I/O pad
    |
Bitstream      -- configuration data (JSON-based)
    |
FPGA           -- top-level fabric combining all elements
```

## How it fits in the stack

- **logic-gates** provides primitive gates, MUX, sequential elements
- **block-ram** provides SRAM cells used inside LUTs
- **fpga** (this crate) combines everything into a programmable fabric

## Key types

| Type | Description |
|------|-------------|
| `LUT` | K-input look-up table storing a truth table in SRAM |
| `Slice` | 2 LUTs + 2 flip-flops + carry chain |
| `CLB` | Configurable Logic Block with 2 slices |
| `SwitchMatrix` | Programmable routing crossbar with named ports |
| `IOBlock` | Bidirectional I/O pad (input/output/tristate) |
| `Bitstream` | JSON-based configuration data |
| `FPGA` | Top-level fabric model |

## Usage

```rust
use fpga::bitstream::Bitstream;
use fpga::fabric::FPGA;

let json = r#"{
    "clbs": {
        "clb_0": {
            "slice0": {
                "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0]
            }
        }
    },
    "io": {
        "in_a": { "mode": "input" },
        "out":  { "mode": "output" }
    }
}"#;

let bs = Bitstream::from_json_str(json).unwrap();
let mut fpga = FPGA::new(bs);

// Evaluate the AND gate
let out = fpga.evaluate_clb("clb_0", &[1,1,0,0], &[0;4], &[0;4], &[0;4], 0, 0);
assert_eq!(out.slice0.output_a, 1); // AND(1,1) = 1
```

## Dependencies

- `logic-gates` (path dependency)
- `block-ram` (path dependency)
- `serde` + `serde_json` (for bitstream parsing)
