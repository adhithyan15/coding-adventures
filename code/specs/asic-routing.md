# ASIC Routing

## Overview

Routing connects every net's pins with metal traces, respecting the layer stack, design rules, and routing channels. Two-phase, like nextpnr's PathFinder but for multi-layer ASIC routing:

1. **Global routing** — partition the die into a grid of "GCells"; for each net, decide which GCells the route passes through. Coarse — gives a rough plan.
2. **Detailed routing** — within each GCell and across boundaries, lay down concrete metal segments respecting DRC.

The output is a routed DEF — input ready for `gdsii-writer.md`.

This is the hardest algorithmic problem in the stack. Real industrial routers (Synopsys IC Compiler, Cadence Innovus) are millions of lines of code. Open-source routers (TritonRoute, OpenROAD) are tens of thousands. Our v1 uses simple algorithms (Lee maze routing in detailed; bin-based congestion in global) — slow but correct on small designs (≤ 1000 nets). Future spec for negotiation-based routing matching TritonRoute quality.

## Layer Position

```
asic-placement.md (placed DEF)        sky130-pdk.md (layer stack, design rules)
              │                                  │
              └──────────────┬───────────────────┘
                             ▼
                ┌────────────────────────────┐
                │  asic-routing.md            │  ◀── THIS SPEC
                │  (global + detailed)        │
                └────────────────────────────┘
                             │
                             ▼ (routed DEF)
                       gdsii-writer.md → drc-lvs.md
```

## Concepts

### Layer assignment

Sky130 metal layers: li1 (local interconnect), met1, met2, met3, met4, met5. Each layer has a preferred direction (horizontal or vertical). A typical assignment:
- Cell internals on li1 (handled by cell layout, not by us).
- met1: horizontal short routes.
- met2: vertical row-to-row.
- met3: horizontal block-level.
- met4: vertical, cross-block.
- met5: horizontal, very long routes + power.

Vias connect adjacent layers. Each via has cost (size, parasitic capacitance); routers prefer fewer vias.

### Global routing

Partition the die into a grid of **GCells** (each ~10 µm square typically). Net routing in global is just "which GCells does the route traverse?" — a path finding problem on the GCell graph.

```
Each GCell tracks: per-edge capacity (number of routes that can cross),
                    per-edge usage  (current count).
For each net (driver, sinks):
    for each (driver, sink) pair:
        find shortest path in the GCell graph
        update GCell edge usage
```

Congestion: when usage > capacity on some edge. Two ways to handle:
- **PathFinder negotiation**: re-route congested nets with cost penalty proportional to overuse.
- **Maze with replanning**: rip-up and re-route the worst nets.

### Detailed routing

Within each GCell, the actual metal segments. Steps:
1. **Pin access**: every pin (a rectangle on a layer) needs to be connectable from the routing grid. Pin-access regions are the legal entry points.
2. **Track grid**: each layer has a track grid (e.g., met1 horizontal tracks every 0.34 µm). Routes follow tracks.
3. **Maze routing**: Lee's algorithm in BFS form on the (track, layer) grid; each cell tracks distance from source; backtracking from sink gives the path.
4. **Vias**: when the path must change layer.
5. **DRC awareness**: every path step checks design rules (minimum width, spacing, end-of-line).

For high-quality routing, replace Lee with A* (admissible heuristic = Manhattan distance) or use 3-D maze for layer changes inline.

### Net ordering

Routing order matters. Strategies:
- **Critical nets first** (with timing slack < 0): get best routes.
- **Long nets first**: avoid getting trapped by short nets.
- **Random**: simple; reasonable for small designs.

### Power routing (separate pass)

Power straps are pre-allocated by `asic-floorplan.md` (SPECIALNETS in DEF). Routing must avoid power straps. The router sees them as obstructions.

### Clock routing (separate pass)

Clocks are special: low skew is required. Clock-tree synthesis (CTS) builds a balanced tree of buffers. Out of scope for v1; we route clock nets like any signal but with priority. Real CTS is a future spec.

## Public API

```python
from dataclasses import dataclass


@dataclass
class RouteOptions:
    method: str = "lee"           # "lee" | "astar"
    global_routing: bool = True
    max_iterations: int = 10
    layer_assignment: str = "auto"   # "auto" | per-net spec
    via_cost: float = 5.0


@dataclass
class RouteReport:
    nets_routed: int
    nets_failed: int
    total_wirelength: float        # µm
    total_vias: int
    drc_violations: list[str]      # post-route check
    runtime_sec: float


@dataclass
class Router:
    placed_def: "Def"
    tech_lef: "TechLef"
    
    def route(self, options: RouteOptions = RouteOptions()) -> tuple["Def", RouteReport]: ...
    def global_route(self) -> "GlobalRoutingResult": ...
    def detailed_route(self, global_result: "GlobalRoutingResult") -> "Def": ...
```

## Algorithm (detailed)

```
1. Build the routing graph:
   For each layer L:
     For each track T on L:
       Cells = subdivision of T at via-grid spacing
       For each cell, neighbors = adjacent cells on T + via cells to L+1, L-1

2. For each cell, mark blocked if:
   - Inside a pre-placed obstruction (cell OBS or power strap)
   - Inside a wider-than-min-spacing reservation around an existing route

3. For each net (in order of criticality):
   a. Multi-source: all driver pins; collect connectable cells in the graph
   b. For each sink:
      Lee's BFS from sources; halt when sink is reached or graph exhausted
      Backtrack to find the path
      Mark path cells as blocked (with appropriate spacing)
   c. Reconstruct: turn the path into DEF Segment objects (one segment per layer change)

4. Check DRC on the final routes; if violations:
   - Try re-routing the violating net with stricter constraints
   - If still failing, report
```

## Worked Example — 4-bit Adder

After tech-mapping + placement: 16 cells in 2-3 rows; ~20 nets.

Routing:
- Most nets are short (intra-row, 2-3 cell distances).
- The carry-chain nets (`c0`, `c1`, `c2`) connect sequential FAs; route on met2 vertically + met1 horizontally.
- IO pins on the boundary; route on met2/met3 to the closest cell.

For ~20 nets, Lee's algorithm converges in <1 sec. ~30 segments total. 0 DRC violations expected for a clean placement.

## Worked Example — 32-bit ALU

~430 cells, ~700 nets.

Routing:
- Global: 17 × 17 GCell grid; ~10 sec runtime; 1-2 PathFinder iterations.
- Detailed: ~5 minutes runtime per pass; possibly 2 passes.
- ~5000 segments total.
- A few DRC violations expected on first pass; resolved by ripping up and re-routing congested regions.

## Edge Cases

| Scenario | Handling |
|---|---|
| Net with no path | Try wider via-grid; report if still failing. |
| Track exhaustion (over-congestion) | Re-route with higher cost penalty; rerun PathFinder. |
| Pin only accessible from blocked layer | Insert via to a clear layer. |
| Long net (e.g., 50 µm clk) | Route on highest available metal; pre-buffer if delay-critical. |
| Power-strap collision | Strap is obstruction; route around. |
| Antenna violation (long polysilicon edge) | Insert antenna diode or bridge to higher metal. Out of scope for v1; document as future. |
| Spacing-rule violations from neighbor net | Detected in DRC pass; re-route with reservation. |
| Detour through unrelated channel | Allowed; cost is wirelength, not topology. |

## Test Strategy

### Unit (95%+)
- Lee's BFS finds shortest path on a small grid.
- Layer assignment respects preferred direction.
- Via insertion at layer changes.
- Net ordering preserves priority.

### Integration
- 4-bit adder: routes in <5 sec; 0 DRC violations.
- 32-bit ALU: routes in <10 min; <5 DRC violations on first pass; resolved on re-route.
- Output DEF reads cleanly in KLayout (`klayout -e adder4_routed.def`).
- (Cross-validation) compare wirelength to TritonRoute on small designs; within 30%.

## Conformance

| Standard | Coverage |
|---|---|
| DEF 5.8 routing section | Full output |
| Sky130 design rules | DRC-aware routing for: min width, min spacing, via enclosure |
| Sky130 antenna rules | Out of scope for v1; future |
| TritonRoute interoperability | Output DEF readable; congestion metrics comparable on small designs |
| EDIF / SPEF | Out of scope |

## Open Questions

1. **A\* vs Lee** — A* is faster but slightly more complex. Recommendation: implement Lee for v1; A* as future.
2. **Negotiation-based routing (TritonRoute style)** — production-quality. Future spec.
3. **Multi-net rip-up and replan** — improves quality on congested designs. Future.
4. **Detailed routing with parasitic-aware optimization** — needs SPEF integration. Future.

## Future Work

- A* and bidirectional search.
- Negotiation-based routing.
- Steiner-tree-aware routing.
- Antenna-rule fixing.
- Layer-skip via insertion.
- Multi-die routing for chiplets.
- Parasitic-aware (RC-driven) routing.
- Clock-tree synthesis as a separate spec.
