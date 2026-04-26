# fpga-place-route-bridge

Bridges an HNL netlist to the existing `fpga` package's JSON config format. v0.1.0 implements a one-LUT-per-cell packing with row-major placement and direct net-to-source routing.

See [`code/specs/fpga-place-route-bridge.md`](../../../specs/fpga-place-route-bridge.md).

## Quick start

```python
from fpga_place_route_bridge import hnl_to_fpga_json, FpgaBridgeOptions
import json

# An HNL Netlist (e.g. from synthesis)
hnl = ...

config, report = hnl_to_fpga_json(hnl, options=FpgaBridgeOptions(rows=4, cols=4))
print(report.cells_packed, report.routes_emitted)

# Save as JSON to feed the fpga package's simulator
with open("adder4_fpga.json", "w") as f:
    json.dump(config, f, indent=2)
```

## v0.1.0 scope

- `TRUTH_TABLES`: pre-computed 2^k truth tables for HNL primitive cells
  (BUF/NOT/AND/OR/NAND/NOR/XOR/XNOR 2-4 input + MUX2 + CONST_0/1).
- One cell -> one LUT (slice 0, lut_a).
- Row-major CLB placement on a configurable rows×cols grid.
- IO pin auto-generation for top-level input/output ports.
- Truth-table expansion: pad to lut_inputs (default 4) by repeating outputs.
- `FpgaBridgeReport`: cells packed, unmapped types, routes emitted.

## Out of scope (v0.2.0)

- Multi-cell-per-CLB packing (cone packing).
- SA-based placement on HPWL.
- PathFinder negotiation routing through switch matrices.
- Sequential cell mapping (DFFs into CLB FFs).
- Block RAM mapping.

MIT.
