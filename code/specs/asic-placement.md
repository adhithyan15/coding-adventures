# ASIC Placement

## Overview

Given a floorplan (rows + die area + IO pins) and a tech-mapped netlist, placement assigns each standard cell to a legal site location, minimizing wirelength while respecting non-overlap and row constraints. The output is a placed DEF — input ready for `asic-routing.md`.

Two phases:
1. **Global placement** — coarse positions; cells may overlap or sit between rows. Optimization-driven (analytical or simulated annealing) for total wirelength.
2. **Detailed placement** (legalization) — snap to row sites, eliminate overlaps. Local refinement around each cell.

For v1, **simulated annealing** for global (consistent with `fpga-place-route-bridge.md` placement; familiar; teaching-friendly). Detailed placement: greedy left-to-right packing per row.

## Layer Position

```
HNL (stdcell) + asic-floorplan.md output
              │
              ▼
asic-placement.md  ◀── THIS SPEC
              │
              ▼ (placed DEF)
asic-routing.md
```

## Concepts

### Wirelength estimation

Used in placement cost function:
- **HPWL** (half-perimeter wirelength): for each net, `HPWL = (max_x - min_x) + (max_y - min_y)` over all connected pins. Fast to compute, good correlate of actual routed length.
- **Star wirelength**: distance from each pin to the centroid; sum. Better for high-fanout nets.
- **Steiner**: actual rectilinear Steiner tree; expensive to compute.

For v1: HPWL only. Recompute per swap during SA. Caches per-net contributions to avoid full recompute.

### Global placement: simulated annealing

Same algorithm as `fpga-place-route-bridge.md`'s placement, scaled up:

```
T = T_start
positions = random_assignment(cells, sites)
cost = total_HPWL(positions)

for iteration in range(N):
    pick two cells; tentative swap
    delta = HPWL_change(swap)
    if delta < 0 or random() < exp(-delta/T):
        accept swap
        cost += delta
    else:
        revert
    T *= cooling_factor
```

For ~430 cells (32-bit ALU): converges in ~5 sec. For 10K cells: ~5 min.

### Analytical placement (alternative; future)

Quadratic placement: minimize Σ (x_i - x_j)² over all connected (i, j). Solved as a sparse linear system. Order of magnitude faster than SA for large designs; more complex to implement. Documented as future work.

### Detailed placement (legalization)

After global placement, cells may overlap or be off-grid. Legalization:
1. For each row, sort cells by global x-position.
2. Walk left-to-right; place each cell at the first free site that fits.
3. If a row overflows, shift cells to neighboring rows.
4. Iterate until all cells placed.

Some implementations use a "tetris" packing or "Abacus" algorithm for higher quality. We use the straightforward greedy version for v1.

### Placement-aware constraints

- **Pre-placed cells** (e.g., specific cells must be near a particular IO pin): respected as fixed locations.
- **Region constraints** (e.g., cells with `(* group = "decoder" *)` should be near each other): soft constraint; weighted in cost.
- **Don't-touch areas** (e.g., reserved for power straps): cells avoid those sites.

## Public API

```python
from dataclasses import dataclass


@dataclass
class PlaceOptions:
    method: str = "anneal"          # "anneal" | "analytical"
    annealing_iterations: int = 100000
    seed: int = 42
    legalize: bool = True
    fix_pre_placed: bool = True
    target_density: float = 0.7      # for analytical placement


@dataclass
class PlaceReport:
    final_hpwl: float                 # total HPWL after place
    cells_placed: int
    overlaps_resolved: int
    legalization_displacement_avg: float  # how far cells moved during legalize
    runtime_sec: float


@dataclass
class Placer:
    floorplan: "Floorplan"
    netlist: "Netlist"
    library: "Library"          # for cell sizes
    
    def place(self, options: PlaceOptions = PlaceOptions()) -> tuple["Def", PlaceReport]:
        ...
    
    def global_place_anneal(self) -> dict[str, tuple[float, float]]: ...
    def legalize(self, global_positions: dict[str, tuple[float, float]]) -> dict[str, tuple[float, float]]: ...
```

## Worked Example — 4-bit Adder

16 cells; floorplan with 3 rows of 16 sites each (48 sites).

Global: SA with 50K iterations. HPWL starts at ~80 µm (random) and converges to ~25 µm after ~30K iterations. Linear arrangement emerges: full-adder bits 0..3 from left to right; XOR/AND/OR cells in each FA cluster together.

Detailed: place left-to-right. All 16 cells fit in one row (16 cells × 1.4 µm = 22.4 µm; row width 7 µm × 16 sites/0.46 µm = ~16 sites, so cells span 2 rows).

Output: 16 placed cells, average HPWL per net ≈ 1.5 µm.

## Worked Example — 32-bit ALU

~430 cells across 17 rows of ~80 sites.

Global: SA with 200K iterations; ~5 sec runtime. Final HPWL ~450 µm. 

Visualization shows: similar-purpose cells cluster (e.g., all the bit-i adders adjacent for short carry paths; bit-i mux-tree cells near their inputs).

Detailed: legalization in <1 sec; average displacement < 1 site (most cells already legal).

## Edge Cases

| Scenario | Handling |
|---|---|
| Overflow (more cells than sites) | Detected during legalize; report failure. |
| Cells too tall for the row | Detected (different site); reject or use multi-height floorplan. |
| Pre-placed cell location is illegal | Validate; reject. |
| All cells of one kind want the same location | SA will scatter them due to overlap penalty. |
| Region constraint with no feasible region | Warn; cells placed best-effort. |
| Designs with tight congestion | Suggest larger die; rerun floorplan. |
| Stale cell sizes (HNL doesn't match LEF) | Validate at start. |
| Re-run same input | With same seed, deterministic. |

## Test Strategy

### Unit (95%+)
- HPWL: matches hand-computed for small examples.
- SA: monotonic decrease in cost (with occasional uphill).
- Legalization: produces non-overlapping placement.
- Pre-placed cells: location preserved.

### Integration
- 4-bit adder placed; HPWL within 20% of optimal.
- 32-bit ALU placed in <10 sec.
- Output DEF reads cleanly in KLayout and OpenROAD.
- Compare HPWL to OpenROAD's RePlAce: within 10% on benchmarks.

## Conformance

| Standard | Coverage |
|---|---|
| DEF 5.8 placement section | Full output |
| OpenROAD read compatibility | Yes |
| Bookshelf placement format | Out of scope; future for benchmark suites |

## Open Questions

1. **SA vs analytical** — implement analytical for large-scale runs; future work.
2. **Multi-row cells** (tall cells in mixed-height libraries) — defer.
3. **Timing-driven placement** — use slack to weight nets in HPWL. Future.

## Future Work

- Analytical (quadratic / nonlinear) placement.
- Timing-driven placement (Steiner tree timing model).
- Power-driven placement (cluster high-activity cells away from each other).
- Multi-row cell support.
- Detailed placement via Abacus / tetris.
- Incremental placement for ECO flows.
