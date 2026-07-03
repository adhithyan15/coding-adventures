# lef-def

Library Exchange Format (LEF) and Design Exchange Format (DEF) writer for the silicon-stack pipeline.

## Pipeline position

```
asic-floorplan ──► lef-def ──► asic-placement
asic-routing   ──► lef-def ──► gdsii-writer
```

## What it does

- **`write_tech_lef_str()`** — emits technology LEF (site definitions, layer rules, via rules).
- **`write_cells_lef_str()`** — emits cell LEF (one MACRO block per stdcell with pin geometries).
- **`write_def_str()`** — emits DEF 5.8 (DIEAREA, ROW, COMPONENTS, PINS, NETS with ROUTED geometry).

## Key types

| Type | Description |
|------|-------------|
| `TechLef` | Layer/site/via definitions for a process |
| `CellLef` | One MACRO block (name, pins, obstructions) |
| `Def` | Complete design: die, rows, components, pins, nets |
| `Net` | Signal name + list of routed `Segment`s |
| `Component` | Placed cell instance (name, cell_type, x, y) |

## Usage

```rust
use lef_def::{write_tech_lef_str, write_def_str, TechLef, Def};

let tech: TechLef = /* from sky130-pdk */;
let def: Def = /* from asic-routing */;

let lef_text = write_tech_lef_str(&tech);
let def_text = write_def_str(&def);
```

## Testing

```
cargo test -p lef-def -- --nocapture
```

14 tests covering: tech LEF keywords, cell LEF pin/obs, DEF sections (DIEAREA, ROW, COMPONENTS, PINS, NETS), routed geometry.
