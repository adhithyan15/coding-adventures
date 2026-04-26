# drc-lvs

DRC (geometric design rule check) and LVS (layout-vs-schematic netlist comparison) for ASIC signoff. **Zero deps.**

See [`code/specs/drc-lvs.md`](../../../specs/drc-lvs.md).

## Quick start — DRC

```python
from drc_lvs import Rect, Rule, run_drc

rects = [
    Rect("met1", 0.0, 0.0, 1.0, 0.5),
    Rect("met1", 0.05, 0.0, 0.10, 0.5),  # too narrow!
]
rules = [
    Rule(name="met1.minwidth", layer="met1", kind="min_width", value=0.14),
    Rule(name="met1.minspacing", layer="met1", kind="min_spacing", value=0.14),
]
report = run_drc(rects, rules)
print(report.clean)            # False
print(len(report.violations))  # 1+
```

## Quick start — LVS

```python
from drc_lvs import LvsNetlist, LvsCell, lvs

layout = LvsNetlist(cells=[
    LvsCell(name="m1", cell_type="NMOS", pins=(("D", "y"), ("G", "a"), ("S", "vss"))),
    LvsCell(name="m2", cell_type="PMOS", pins=(("D", "y"), ("G", "a"), ("S", "vdd"))),
])
schematic = LvsNetlist(cells=[
    LvsCell(name="X1", cell_type="NMOS", pins=(("D", "out"), ("G", "in"), ("S", "vss"))),
    LvsCell(name="X2", cell_type="PMOS", pins=(("D", "out"), ("G", "in"), ("S", "vdd"))),
])
report = lvs(layout, schematic)
print(report.matched)  # True
```

## v0.1.0 scope

- DRC rules: `min_width`, `min_spacing` (pairwise per layer), `min_area`.
- DRC engine: O(n²) pairwise comparisons; sufficient for designs ≤ ~1000 polygons.
- LVS: net-signature partition refinement (cells matched by `(cell_type, pin -> net-equiv-class)` multiset).

## Out of scope (v0.2.0)

- DRC: enclosure, end-of-line, antenna, density rules.
- DRC R-tree spatial index for million-polygon scale.
- LVS: full graph isomorphism via VF2 (catches more subtle differences).
- Parasitic extraction (PEX).
- Electrical Rules Check (ERC).

MIT.
