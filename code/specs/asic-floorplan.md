# ASIC Floorplan

## Overview

Floorplanning sets the chip's overall geometry: die size, core area, IO ring, power grid, macro placement. Everything downstream (placement, routing, GDSII) operates inside the chosen geometry. Bad floorplan choices lead to unroutable designs or wasted area; good ones leave generous routing channels and predictable timing.

This spec defines the floorplan stage that produces a **floorplan DEF** — a DEF file with DIEAREA, ROW definitions, IO PINS placed on the boundary, power grid (SPECIALNETS), and macros placed (none in our 4-bit adder; relevant for designs with RAM macros). COMPONENTS are listed but unplaced.

## Layer Position

```
HNL (stdcell, post-tech-mapping)   sky130-pdk.md (cell sizes from LEF)
              │                                  │
              └──────────────┬───────────────────┘
                             ▼
                ┌────────────────────────────┐
                │ asic-floorplan.md           │  ◀── THIS SPEC
                │ (die area + rows + IO + power) │
                └────────────────────────────┘
                             │
                             ▼ (floorplan DEF)
                       asic-placement.md
```

## Concepts

### Die size estimation

Total cell area = sum of cell areas from `standard-cell-library.md`:
```
total_cell_area = Σ cell.area for cell in netlist
```

Plus a utilization factor (typically 60-80%; lower for designs with congestion concerns):
```
core_area = total_cell_area / utilization
```

Plus IO ring area (depends on number of IO pins). Add 50-100 µm wide ring for 1.8V IO pads.

```
die_size = core_size + 2 × (io_ring_width)
```

### Aspect ratio and rows

The core is divided into **rows** of standard-cell **sites** (height matches `unithd` site = 2.72 µm for sky130_fd_sc_hd). Aspect ratio choice trades horizontal vs vertical routing congestion:

```
core_height = N_rows × site_height
core_width  = total_cell_area / core_height (approximately)
```

For square aspect ratio: `N_rows ≈ sqrt(total_cell_area / site_height²)`.

### Pin placement

Top-level IO pins go on the boundary. Strategies:
- **Edge-balanced**: distribute pins evenly across the four edges.
- **By-direction**: inputs left, outputs right.
- **By-bus**: keep related signals together (e.g., `a[3:0]` adjacent).

For the 4-bit adder: 14 pins on a 100×50 µm die fits easily. A simple balanced placement with `a[3:0]` and `b[3:0]` on the left, `cin` bottom-left, `sum[3:0]` and `cout` on the right is fine.

### Power grid

VDD and GND must reach every cell. A regular **mesh** of wide metal traces:
- **VDD/GND rings** around the core (continuous ring on outer metal layers).
- **VDD/GND straps** crossing horizontally on one metal layer and vertically on another.
- **Cell rows** alternate orientation (`N`, `FS`) so abutting cells share VDD/VSS rails (Sky130 cells have VPWR on top, VGND on bottom; flipping cells lets adjacent rows share rails).

### Macro placement

Macros (RAM blocks, custom IP) are large pre-designed blocks. Placement options:
- **Edge-pinned**: macros along die edges; standard cells fill the middle.
- **Cluster**: macros grouped near the cells that use them.
- **Manual**: floorplan designer specifies coordinates.

For the 4-bit adder: no macros. For larger designs (CPU + RAM), macros first; cells fill around.

## Public API

```python
from dataclasses import dataclass
from enum import Enum


class Aspect(Enum):
    SQUARE = "square"
    WIDE = "wide"          # 2:1 width:height
    TALL = "tall"          # 1:2


@dataclass
class FloorplanOptions:
    utilization: float = 0.7       # 70% target
    aspect: Aspect = Aspect.SQUARE
    io_ring_width: float = 50.0    # µm
    pin_strategy: str = "edge_balanced"
    power_strap_pitch: float = 50.0  # µm between VDD/GND straps
    macro_placement: str = "edge"


@dataclass
class Floorplan:
    die_width: float       # µm
    die_height: float
    core_x0: float
    core_y0: float
    core_x1: float
    core_y1: float
    rows: list["Row"]              # from lef-def
    pins: list["DefPin"]
    macros: list["MacroPlacement"]
    power_rings: list["PowerRing"]
    power_straps: list["PowerStrap"]


@dataclass
class MacroPlacement:
    instance_name: str
    cell_type: str
    location: tuple[float, float]
    orientation: str


@dataclass
class PowerRing:
    layer: str
    net: str          # "VDD" | "VSS"
    width: float
    rect: "Rect"


@dataclass
class PowerStrap:
    layer: str
    net: str
    points: list[tuple[float, float]]
    width: float


def floorplan(hnl: "Netlist", pdk: "Pdk", options: FloorplanOptions) -> Floorplan: ...

def to_def(fp: Floorplan, hnl: "Netlist") -> "Def": ...
```

## Worked Example — 4-bit Adder

After tech-mapping: 16 cells, total area ~37 µm² (rough estimate using Sky130 cell sizes).

Floorplan options: `utilization=0.7`, `aspect=SQUARE`, `io_ring_width=10` (small for our toy chip).

```
total_cell_area ≈ 37 µm²
core_area = 37 / 0.7 ≈ 53 µm²
core_size ≈ sqrt(53) ≈ 7.3 µm × 7.3 µm

But row height is 2.72 µm; need N rows with N=3 for ~8 µm core height.
core_height = 3 × 2.72 = 8.16 µm
core_width = 53 / 8.16 ≈ 6.5 µm  → round up to 7 µm (multiples of site width 0.46 µm: 16 sites)

die_size = (7 + 2×10) × (8.16 + 2×10) = 27 × 28.16 µm²
```

Output DEF: 3 rows of 16 sites each, 14 pins on the boundary, VDD/VSS rings on met4/met5, no macros.

## Worked Example — 32-bit ALU

~430 cells, total area ~1500 µm².

```
core_area = 1500 / 0.7 = 2143 µm²
core_size ≈ sqrt(2143) ≈ 46 µm × 46 µm
N_rows = 46 / 2.72 ≈ 17 rows

die_size ≈ 50 × 50 µm² (core + small ring)
```

For ~70 IO pins (32 a, 32 b, 4 op, 32 y, 1 zero, plus clk/reset = ~106 pins): pin density on 200 µm of edge is 1 pin per 2 µm — comfortable.

## Edge Cases

| Scenario | Handling |
|---|---|
| Insufficient die area for cells | Detected post-floorplan when placement fails; user must adjust utilization. |
| IO count exceeds available perimeter pins | Warn; suggest larger die or pin sharing. |
| Macros wider than core | Warn; user must increase die size. |
| Macro overlaps another macro | Reject. |
| Power straps don't reach corners | Detected; add more straps or rings. |
| Power-grid pitch finer than the cell row | Warn; potentially infeasible. |
| Sub-aspect-ratio designs (very wide or very tall) | Allowed; warn if extreme. |

## Test Strategy

### Unit (95%+)
- Die size from utilization is monotonic.
- Row generation respects site grid.
- Pin placement: 4 strategies all produce valid pins on the boundary.
- Power-grid generation produces non-overlapping rings.

### Integration
- 4-bit adder: floorplan DEF opens in KLayout; rows + pins look right.
- 32-bit ALU: floorplan DEF passes OpenROAD's `read_def` without warnings.

## Conformance

| Standard | Coverage |
|---|---|
| **DEF 5.8** floorplan section | Full output |
| **OpenROAD** read compatibility | Yes |
| **KLayout** read compatibility | Yes |

## Open Questions

1. **Power ring width** — affects IR drop; static analysis future spec.
2. **IO planning** — relate to package pinout? Future for tape-out spec.
3. **PDN extraction** — for power signoff; future spec.

## Future Work

- IR drop analysis.
- Pin-grouping optimizer for related signals.
- Mixed-signal floorplan (analog + digital regions).
- Hierarchical floorplanning for SoCs.
