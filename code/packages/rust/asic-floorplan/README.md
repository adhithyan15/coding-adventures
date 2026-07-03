# asic-floorplan

ASIC floorplan generation for the silicon-stack pipeline: given a list of cell instances and IO pins, computes die area, core area, row grid, and IO pin placement.

## Pipeline position

```
tech-mapping ──► asic-floorplan ──► asic-placement ──► asic-routing
```

## What it does

1. **Die sizing** — total cell area ÷ utilization factor → core area; die = core + IO ring margin.
2. **Row snapping** — die dimensions rounded to the nearest site row height / width.
3. **IO pin placement** — inputs on the left edge, outputs on the right, others on the bottom.
4. **`floorplan_to_def()`** — converts the `Floorplan` result into a `Def` ready for placement.

## Key types

| Type | Description |
|------|-------------|
| `FloorplanOptions` | utilization, aspect ratio, site geometry; use `sky130_hd()` for defaults |
| `CellInstanceEstimate` | cell name + type + area (µm²) |
| `IoSpec` | pin name, direction, use |
| `Floorplan` | die rect, core rect, rows, components, IO pins |
| `FloorplanError` | `InvalidUtilization`, `InvalidAspect`, `ZeroArea` |

## Usage

```rust
use asic_floorplan::{compute_floorplan, FloorplanOptions, CellInstanceEstimate, IoSpec, floorplan_to_def};

let opts = FloorplanOptions::sky130_hd();
let fp = compute_floorplan(&cells, &io_pins, &opts)?;
let def = floorplan_to_def(&fp, "adder4");
```

## Testing

```
cargo test -p asic-floorplan -- --nocapture
```

10 integration tests + 1 doc-test covering: die sizing, row count, IO placement, utilization/aspect validation, zero-area guard.
