# asic-placement

Simulated-annealing placement for the silicon-stack pipeline. Minimizes half-perimeter wire length (HPWL) by iteratively swapping pairs of placed cells.

## Pipeline position

```
asic-floorplan ──► asic-placement ──► asic-routing
```

## Algorithm

1. **Initialization** — cells are packed left-to-right, row-by-row, respecting cell height/width.
2. **Simulated annealing** — random cell-pair swaps accepted with probability exp(−ΔE/T); temperature cooled geometrically each iteration.
3. **Legalization** — final re-pack to ensure cells sit on row boundaries with no overlaps.

Uses an internal `Xorshift64` PRNG (no external `rand` dependency) seeded via `PlacementOptions::seed`.

## Key types

| Type | Description |
|------|-------------|
| `CellSize` | cell_type → (width, height) in µm |
| `PlacementOptions` | iterations (default 50 000), seed, legalize flag |
| `PlacementReport` | final_hpwl, cells_placed, accepted/rejected swap counts |
| `PlacementError` | `NoRows`, `CellDoesNotFit` |

## Usage

```rust
use asic_placement::{place, CellSize, PlacementOptions};

let (placed_def, report) = place(&floorplan_def, &cell_sizes, &nets, PlacementOptions::default())?;
println!("HPWL: {:.2} µm", report.final_hpwl);
```

## Testing

```
cargo test -p asic-placement -- --nocapture
```

6 integration tests + 1 doc-test covering: basic placement, HPWL computation, empty net handling, options validation.
