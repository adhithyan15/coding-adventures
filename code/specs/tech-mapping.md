# Technology Mapping

## Overview

Tech mapping rewrites a generic-cell HNL netlist (AND, OR, NOT, DFF, MUX2 — the built-ins from `gate-netlist-format.md`) into a netlist of *technology-specific* standard cells (sky130_fd_sc_hd__nand2_1, sky130_fd_sc_hd__inv_2, sky130_fd_sc_hd__dfrtp_1, etc.). The output remains HNL but with `level=stdcell` and cell types from `standard-cell-library.md`.

Why does it matter? Two reasons:
1. **Real cells are richer**: NAND/NOR are faster than AND/OR (no bubble-elimination INV). AOI/OAI cells fold complex logic into one cell. A naive 1-to-1 mapping wastes 30-50% of area and timing.
2. **Drive-strength selection**: each cell type has multiple drive variants. Larger drive strengths switch large loads faster but cost area + leakage. Mapping must pick the right drive.

## Layer Position

```
synthesis.md → HNL (generic, ~20 cells for 4-bit adder)
                │
                ▼
        tech-mapping.md  ◀── THIS SPEC
                │
                ▼
HNL (stdcell, ~20 Sky130 cells with drive strengths)
                │
                ▼
        asic-placement.md, asic-routing.md, gdsii-writer.md
```

## Concepts

### Pattern matching vs covering

Two main approaches:

| Approach | How | Pros | Cons |
|---|---|---|---|
| **Rule-based** (pattern matching) | Match generic-cell sub-trees against handwritten library cell patterns | Simple, fast, predictable | Misses combinatorial opportunities (e.g., AOI21 = AND2+OR2+NOT folded). Manual rules. |
| **DAG covering** (DAGON, ABC) | Each library cell is a small DAG; tile the netlist DAG with overlapping covers; pick area-optimal | Better quality, automatic | Complex; costs O(n × patterns); harder to debug. |

We implement **rule-based** for v1 (clear, deterministic, debuggable). Document DAG covering as future work.

### Bubble-pushing

Generic synthesis emits ANDs/ORs because they're natural to the user. Real CMOS implements NAND and NOR more efficiently (one inversion stage less). Bubble-pushing is the systematic transformation:

```
A — AND — Y       becomes        A — NAND — !Y — INV — Y
B                                B
```

Then look for places where the next stage is also inverting:

```
NAND — !Y — INV — INV — NEXT      becomes      NAND — !Y — NEXT
```

Pairs of INVs cancel. Each canceled pair = one fewer cell.

### AOI/OAI recognition

AOI21 ("AND-OR-Invert 2-1") cell function: `Y = !((A·B) + C)`. If the netlist has:

```
A — AND2 — t1 — OR2 — t2 — INV — Y
B            C
```

Match this 3-cell sub-tree, replace with 1 AOI21 cell. Saves 2 cells. Common pattern in ALUs.

Similarly OAI21: `Y = !((A+B)·C)`.

Sky130 ships AOI21, AOI22, AOI211, AOI221, AOI311, OAI21, OAI22, OAI211, OAI221, OAI311 cells.

### Drive-strength selection

After mapping (with default `_1`), walk the netlist:
1. For each cell, estimate its load: sum of input pin capacitances of all sinks + estimated wire capacitance.
2. Pick the smallest drive that meets a target output transition time (typically 1-2× period of fastest input transition).

This is iterative: as drives change, loads change, requiring re-evaluation. Run 2-3 passes; converges quickly.

For initial mapping in pre-floorplan flow, use heuristic estimates of wire load. After floorplan/place, redo with actual extracted parasitics.

### Mapping a DFF

Generic `DFF(D, CLK, Q)` → Sky130's `sky130_fd_sc_hd__dfxtp_1` (D-flip-flop, no reset, low-active scan). With reset:

| Generic | Sky130 |
|---|---|
| `DFF` | `dfxtp_1` |
| `DFF_R` (async reset, active high) | `dfrtp_1` |
| `DFF_R` (async reset, active low) | Mapping: invert reset upstream + use `dfxtp_1` with reset, or use `dfxbp_1` |
| `DFF_S` (async set) | `dfstp_1` |
| `DFF_RS` | `dfsrtp_1` |
| `DLATCH` | `dlxtp_1` |

### Mapping a MUX2

Generic `MUX2(A, B, S, Y)` (`Y = S?B:A`) → Sky130 `mux2_1`.

If S can be derived (constant, or driven by a comparison whose result is unused elsewhere), sometimes a more compact AOI/OAI suffices.

## Algorithm

```
1. For each Cell in HNL (generic):
   a. Match against the library-pattern table.
   b. If a unique pattern matches, replace cell with the corresponding stdcell at default drive.
2. Run bubble-pushing passes:
   a. Replace AND/OR with NAND/NOR + INV.
   b. Eliminate INV-INV pairs.
   c. Recognize AOI/OAI patterns and fold.
3. Re-validate: HNL (stdcell) is well-formed.
4. Compute load on every cell output.
5. Drive-strength selection: iterate to fixed point.
6. Emit final HNL with level=stdcell.
```

## Public API

```python
from dataclasses import dataclass


@dataclass(frozen=True)
class Pattern:
    generic_subtree: list[tuple[str, list[str]]]  # (cell_type, input_pin_names)
    stdcell: str                                    # target Sky130 cell
    pin_remap: dict[str, str]                       # generic pin → stdcell pin


@dataclass
class TechMapper:
    library: "Library"     # standard-cell-library
    patterns: list[Pattern]
    
    def map(self, generic_hnl: "Netlist") -> "Netlist":
        """Apply pattern matching, bubble-pushing, AOI/OAI folding, drive-strength selection."""
        ...
    
    def select_drives(self, hnl: "Netlist") -> "Netlist":
        """Iterate to fixed point on drive selection."""
        ...
    
    def report(self) -> "MappingReport":
        ...


@dataclass
class MappingReport:
    cells_before: int
    cells_after: int
    aoi_oai_folded: int
    bubbles_canceled: int
    average_drive: float
    estimated_area: float        # in µm² using cell areas from Liberty
```

## Worked Example — 4-bit Adder

Generic HNL: 20 cells (4 × {2 XOR2 + 2 AND2 + 1 OR2}).

Tech mapping:
1. Pattern match each: `XOR2 → xor2_1`, `AND2 → and2_1`, `OR2 → or2_1`. 20 stdcells, default drive.
2. Bubble-push: 
   - `(A·B) | (C·D)` (the carry-out of each FA) is an AOI22 candidate, but here outputs go directly to `cout` — no folding (output is a port). Actually each FA's `cout = (a·b) + ((a^b)·cin)` — that *is* an AOI22 candidate (`Y = !!(A·B + C·D)`). After bubble-push, we have 4 AOI22 cells.
   - Actually `cout = !!(a·b + (a^b)·cin)` requires the output to be inverted twice; if downstream uses `cout`, we need an INV (or a different cell).
3. AOI22 vs AND+OR: 1 AOI22 cell vs 3 cells (AND, AND, OR). Saves 2 cells per FA × 4 FAs = 8 cells.
4. Drive selection: most cells stay at `_1`. Carry path may bump to `_2` if timing-critical (decided during static-timing on placed netlist, post-floorplan).

Result: ~16 cells (8 fewer than generic, ~30% smaller). Area ~70 µm² with `sky130_fd_sc_hd` (TT corner).

## Worked Example — 32-bit ALU

Generic: ~600 cells. Tech mapping:
- Bitwise AND/OR/XOR rows: NAND/NOR with INV folding → ~25% fewer cells.
- Adder/subtractor: full-adder pattern → use Sky130's `fa_1` cell directly (one cell per bit, instead of 5). Saves 4 cells × 32 bits = 128 cells.
- Comparator: AOI/OAI fold reduces.
- Mux tree: each mux2 maps directly to `mux2_1`.

Result: ~430 cells. Area ~1500 µm². 30% smaller than naive mapping.

## Edge Cases

| Scenario | Handling |
|---|---|
| Generic cell has no library equivalent | Reject with error. |
| Library cell function doesn't match any generic pattern | Cell is unused; not an error. |
| Multiple library cells match the same pattern | Pick the smallest by area (then by drive). |
| Drive selection: required drive > max available | Buffer insertion (BUF_8 or two stages). |
| Cyclic combinational loops (illegal) | Detected post-mapping; error. |
| Assertions in HIR survive past synthesis | Stripped before tech-mapping (assertions are sim-only). |
| `(* keep *)` attribute | Cell preserved; not folded into AOI. |

## Test Strategy

### Unit (95%+)
- Each pattern matches its target sub-tree.
- Bubble-pushing: AND→NAND+INV, INV+INV→nothing.
- AOI/OAI recognition.
- Drive selection convergence.

### Integration
- 4-bit adder: 20 → 16 cells.
- 32-bit ALU: 600 → 430 cells.
- ARM1 reference: completes mapping in <10 sec; gate count within 20% of `arm1-gatelevel` reference.

### Property
- Functional equivalence: simulating generic and mapped HNL gives identical output for the same stimulus (modulo timing).
- Area monotonic: with optimization, mapped area < generic.

## Conformance

| Standard | Coverage |
|---|---|
| Generic cell types from `gate-netlist-format.md` | Full |
| Sky130 cells (teaching subset) | Full |
| Sky130 cells (full library) | Full (via library lookup) |
| Bubble-pushing | Yes |
| AOI/OAI folding | Yes (up to AOI22, OAI22; AOI311/OAI311 future) |
| DAG covering (DAGON-style) | Future |
| Multi-objective (area vs delay vs power) | Area-only for v1; delay-aware drive selection |

## Open Questions

1. DAG covering vs rule-based — defer DAG for v1.
2. Re-tech-mapping after floorplan with extracted parasitics — yes; run twice with feedback.
3. ABC integration as a backend? — future spec; not required for end-to-end demo.

## Future Work

- DAG-covering tech mapper.
- ABC integration.
- Multi-objective optimization.
- Cell sizing as continuous variable.
- Layout-aware mapping (cell shape affects placement legality).
