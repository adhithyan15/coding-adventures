# DRC and LVS

## Overview

Two verification gates between layout and tape-out:

- **DRC** (Design Rule Check) — geometric correctness. Are all polygons wide enough? Are spacings respected? Are wells properly enclosed?
- **LVS** (Layout vs Schematic) — connectivity correctness. Does the layout, when extracted as a netlist, match the gate-level netlist we expect?

A design that fails DRC won't manufacture. A design that fails LVS will manufacture but won't function. Both must pass before tape-out.

This spec defines the rule-engine for DRC and the netlist-extraction + comparison for LVS, sufficient for the Sky130 rule subset relevant to digital flows. Real fab DRC has hundreds of rules; we cover the ~30 most important for a digital circuit; full Sky130 DRC integration is a future spec (or we shell out to KLayout's DRC engine for tape-out runs).

## Layer Position

```
gdsii-writer.md → adder4.gds         sky130-pdk.md (DRC rules, layer map)
              │                                  │
              └──────────────┬───────────────────┘
                             ▼
                ┌────────────────────────────┐
                │  drc-lvs.md                 │  ◀── THIS SPEC
                │  (DRC + LVS verification)   │
                └────────────────────────────┘
                             │
                             ▼ (pass / fail report)
                       tape-out.md
```

## DRC

### Rule kinds

| Rule | Description | Example (Sky130) |
|---|---|---|
| **Min width** | Polygon edges of layer L must be ≥ W apart | met1 ≥ 0.14 µm |
| **Min spacing** | Polygons of layer L must be ≥ S from each other | met1-met1 ≥ 0.14 µm |
| **Min enclosure** | Layer A polygons must extend ≥ E around layer B | nwell encloses pdiff by ≥ 0.18 µm |
| **Min area** | Polygon area ≥ A | met1 ≥ 0.083 µm² |
| **Min density** | Layer fraction within any tile ≥ D | met1 ≥ 5% in any 100×100 µm² |
| **Max density** | ≤ D | met1 ≤ 65% |
| **Antenna** | Polysilicon edge / contact ratio ≤ R | (handled separately) |
| **End-of-line spacing** | Special EOL rules | met1 EOL spacing 0.18 µm |
| **Notch** | Internal-to-polygon spacing | met1 notch ≥ 0.14 µm |

### Rule engine

Geometric rules are implemented as **scanline polygon operations**:

1. **Boolean operations** on layer sets: `met1_oversize = grow(met1, 0.07 µm)`. Then `min_width_violation = met1 minus shrink(met1, 0.07 µm)` finds anything narrower than 0.14 µm.
2. **Pairwise spacing**: for layers A, B: oversize A by S/2 and check intersection with B. If intersection is non-empty, spacing violated.
3. **Enclosure**: for inner I and outer O: shrink O by E. If I extends beyond shrunken O, enclosure violated.
4. **Density**: tile the die into a grid (e.g., 100×100 µm); compute per-tile fraction of layer; check.

For v1, we implement a simple polygon library (rect-based; no curves) and these basic operations. Real fabs use Mentor Calibre or KLayout DRC; tape-out flow shells out to KLayout for confidence.

### Rule deck

Rules are declared in a Python file:

```python
RULES = [
    Rule("met1.minwidth", layer="met1", op=MinWidth, value=0.14),
    Rule("met1.minspacing", layer="met1", op=MinSpacing, value=0.14),
    Rule("met1.eol_spacing", layer="met1", op=MinEolSpacing, value=0.18,
         conditions=Condition(eol_width="<", 0.34)),
    Rule("nwell.enclose_pdiff", outer="nwell", inner="pdiff",
         op=MinEnclosure, value=0.18),
    Rule("met1.density.max", layer="met1", op=MaxDensity,
         value=0.65, tile_size=100.0),
    # ... 25-30 more rules ...
]
```

Sky130 ships full rule decks for KLayout and Magic. We parse them (subset) into our internal Rule format.

## LVS

### Netlist extraction

From the GDS, identify each transistor and connection:

1. **Identify transistors**: a polysilicon (POLY) crossing a diffusion (DIFF) defines a transistor. Source/drain are the diff regions on either side; gate is the poly. NMOS vs PMOS by N+/P+ implant.
2. **Identify nets**: connected metal/poly geometry. Use union-find on connected components.
3. **Identify pins**: by labels in TEXT records on appropriate layer.
4. **Build a netlist**: for each transistor, record (W, L, source net, gate net, drain net, body net).

### Comparison

The extracted netlist (from layout) is compared to the reference netlist (from `gate-netlist-format.md`):

1. **Graph construction**: each netlist is a bipartite graph (transistors / cells, nets). Edges = connections.
2. **Graph isomorphism**: check that the two graphs are isomorphic, with matching transistor types and W/L (within tolerance).
3. **Pin / port matching**: top-level ports must match by name.

Industry tools (Calibre, Netgen) do this via partition refinement. Our implementation uses VF2-style backtracking (slow on huge graphs but fine for our scale).

### What can mismatch

- **Wrong transistor type**: NMOS in layout where schematic says PMOS.
- **Missing transistor**: layout polygon for the gate not present.
- **Extra transistor**: spurious shorts.
- **Wrong W/L**: cell layout doesn't match cell schematic.
- **Wrong connectivity**: net A connects to a different transistor in layout than in schematic.
- **Pin mismatch**: top-level port misnamed or wrong location.

## Public API

```python
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class DrcViolation:
    rule: str
    location: tuple[float, float]   # in µm
    layer: str
    description: str
    severity: str = "error"          # "error" | "warning" | "info"


@dataclass
class DrcReport:
    violations: list[DrcViolation]
    rules_checked: int
    runtime_sec: float
    
    @property
    def clean(self) -> bool:
        return not any(v.severity == "error" for v in self.violations)


@dataclass
class LvsReport:
    matched: bool
    extracted_transistors: int
    schematic_transistors: int
    extracted_nets: int
    schematic_nets: int
    mismatches: list[str]
    runtime_sec: float


@dataclass
class DrcEngine:
    rules: list["Rule"]
    
    @classmethod
    def from_sky130(cls, profile: str = "teaching") -> "DrcEngine": ...
    
    def check(self, gds: "GdsLibrary") -> DrcReport: ...


@dataclass
class LvsEngine:
    pdk: "Pdk"
    
    def extract(self, gds: "GdsLibrary") -> "Netlist": ...
    def compare(self, layout_netlist: "Netlist", schematic_netlist: "Netlist") -> LvsReport: ...
```

## Worked Example — 4-bit Adder DRC

```python
from drc_lvs import DrcEngine

drc = DrcEngine.from_sky130("teaching")
report = drc.check(adder4_gds)

print(report.rules_checked)        # ~30 (teaching subset)
print(len(report.violations))      # 0 (we hope!)
```

If the adder layout is properly built from Sky130 cells (which are themselves DRC-clean) and routing respects min spacing, the result is 0 violations. Common violations come from:
- Routing too close to adjacent metal (fix: increase spacing).
- Pin label outside expected range.
- Density too low (fix: insert fill cells).

## Worked Example — 4-bit Adder LVS

```python
from drc_lvs import LvsEngine

lvs = LvsEngine(pdk=sky130)
layout_netlist = lvs.extract(adder4_gds)
report = lvs.compare(layout_netlist, schematic_netlist=adder4_hnl)

print(report.matched)              # True (we hope)
print(report.extracted_transistors)  # ~480 (16 cells × ~30 trans/cell avg)
print(report.schematic_transistors)  # 480
```

Match means the layout produces the same circuit as the netlist. Mismatches are bugs in the layout.

## Worked Example — Deliberately-broken DRC

A test case where we *want* a violation:

```python
# Take the clean adder GDS, deliberately add a too-narrow met1 path
gds = clean_adder_gds.copy()
gds.cells["adder4"].paths.append(GdsPath(
    layer=68, datatype=20,  # met1
    points=[(1000, 1000), (5000, 1000)],   # in DBU
    width=50    # 0.05 µm — well below the 0.14 µm minimum
))

report = drc.check(gds)
assert not report.clean
assert any("minwidth" in v.rule for v in report.violations)
```

## Edge Cases

| Scenario | Handling |
|---|---|
| Polygon with curved edges (arcs) | Approximate as polygon segments. |
| Self-intersecting polygons | DRC reports as malformed. |
| Density violation in sparse design | Insert fill cells (per `tape-out.md`). |
| LVS mismatch on transistor sizing within tolerance | Configurable W/L tolerance (default 5%). |
| Antenna violations | Separate pass; not in v1 generic DRC. |
| LVS on hierarchical netlists | Flatten both before comparing. |
| Performance on large GDS | Geometric ops are O(n²) naive; spatial-index (R-tree) future. |
| Custom rules from user | Add to rule deck; engine respects unknown rules with warning. |

## Test Strategy

### Unit (95%+)
- Each rule kind: positive (clean) and negative (violating) test.
- Polygon ops (grow, shrink, intersect, union): match expected geometry.
- Connected-components net extraction.
- VF2 isomorphism on small known graphs.

### Integration
- 4-bit adder GDS DRC-clean.
- 4-bit adder GDS LVS-matches reference HNL.
- Deliberate broken GDS produces correct violation.
- (Cross-validation) compare DRC results to KLayout DRC on same GDS — match within rule subset.

## Conformance

| Standard | Coverage |
|---|---|
| **Sky130 DRC rules** | Teaching subset (~30 rules); full deck future via KLayout shell-out |
| **Sky130 LVS rules** | Full subset for digital circuits |
| **Antenna rules** | Future |
| **KLayout DRC engine** | Cross-validation only; not internal use |
| **Magic, Calibre** | Out of scope |

## Open Questions

1. **Spatial indexing** — R-tree for fast geometry queries. Future.
2. **Antenna rules** — needed for tape-out signoff. Future.
3. **Inductive layout extraction** (parasitic-aware) — needs PEX. Future.

## Future Work

- R-tree for fast queries.
- Antenna rule handling.
- PEX (parasitic extraction) for SPICE-level signoff.
- ERC (Electrical Rules Check) for short-circuits, floating gates.
- Density-fill cell insertion as a separate pre-DRC pass.
- Incremental DRC for ECO flows.
