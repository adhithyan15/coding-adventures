# drc-lvs

Design Rule Check (DRC) and Layout vs Schematic (LVS) verification for the silicon-stack pipeline.

## Pipeline position

```
gdsii-writer ──► drc-lvs ──► tape-out
synthesis    ──► drc-lvs (LVS schematic side)
```

## DRC

Geometric checks on axis-aligned rectangles:

| Rule kind | Description |
|-----------|-------------|
| `MinWidth` | Every rect must be ≥ W µm wide **and** tall |
| `MinSpacing` | Any two rects on the same layer must be ≥ S µm apart (center-to-edge) |
| `MinArea` | Every rect must have area ≥ A µm² |

Returns -1.0 spacing for overlapping rects (overlap is a separate class of violation not caught by min_spacing).

## LVS

Bag-of-cell-signatures comparison via partition refinement:

1. For each net, compute a connectivity signature: sorted `"cell_type.pin_name"` strings for every cell-pin touching that net.
2. For each cell, replace pin net-names with their net's equivalence-class signature: `"cell_type(pin=sig,...)"`.
3. Compare the multisets of cell signatures. Matching multisets → topologically equivalent netlists.

Instance names are ignored — only connectivity topology matters.

## Usage

```rust
use drc_lvs::{run_drc, DrcRect, Rule, lvs, LvsNetlist};

// DRC
let report = run_drc(&rects, &[Rule::min_width("met1.W", "met1", 0.14)]);
assert!(report.clean());

// LVS
let report = lvs(&layout_nl, &schematic_nl);
assert!(report.matched);
```

## Testing

```
cargo test -p drc-lvs -- --nocapture
```

13 integration tests + 2 doc-tests covering: DRC clean/violation for all three rule kinds, spacing edge cases (overlapping rects), LVS match/mismatch scenarios.
