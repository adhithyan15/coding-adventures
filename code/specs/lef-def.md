# LEF / DEF

## Overview

LEF (Library Exchange Format) and DEF (Design Exchange Format) are the industry-standard text formats for exchanging cell-library information and design layout/placement/routing data between EDA tools. Sky130 ships LEF for its cells; OpenROAD's flow consumes DEF; commercial tools speak both natively. This spec defines parsers and writers for both, sufficient to:

1. Read Sky130's cells.lef and tech.lef for `tech-mapping.md` and `asic-routing.md`.
2. Write DEF for floorplan / placed / routed designs across stages of `asic-floorplan.md`, `asic-placement.md`, `asic-routing.md`.
3. Round-trip through external tools (OpenROAD, KLayout) for cross-validation.

LEF and DEF are LISP-flavored hierarchical text formats with strict grammar. The full specs (Si2 LEF/DEF Reference Manual) are 200+ pages each; we implement the essential subset for our flow.

## Layer Position

```
Sky130 LEF/DEF files       sky130-pdk.md
            │                       │
            └──────────┬────────────┘
                       ▼
        ┌────────────────────────────┐
        │  lef-def.md                │  ◀── THIS SPEC
        │  (parse + emit)            │
        └────────────────────────────┘
                       │
            ┌──────────┴──────────┐
            ▼                     ▼
    asic-floorplan        asic-placement → asic-routing → gdsii-writer
```

## LEF Format

### Tech LEF (one per technology)

```lef
VERSION 5.8 ;
BUSBITCHARS "[]" ;
DIVIDERCHAR "/" ;
UNITS
  DATABASE MICRONS 1000 ;
END UNITS

LAYER li1
  TYPE ROUTING ;
  DIRECTION HORIZONTAL ;
  PITCH 0.34 ;
  WIDTH 0.17 ;
  SPACING 0.17 ;
  RESISTANCE RPERSQ 12.8 ;
  CAPACITANCE CPERSQDIST 0.025 ;
END li1

LAYER met1
  TYPE ROUTING ;
  DIRECTION HORIZONTAL ;
  PITCH 0.34 ;
  WIDTH 0.14 ;
  SPACING 0.14 ;
END met1

VIA via0_default DEFAULT
  LAYER li1 ;
    RECT -0.085 -0.085 0.085 0.085 ;
  LAYER mcon ;
    RECT -0.085 -0.085 0.085 0.085 ;
  LAYER met1 ;
    RECT -0.115 -0.115 0.115 0.115 ;
END via0_default

SITE unithd
  CLASS CORE ;
  SIZE 0.46 BY 2.72 ;
END unithd

END LIBRARY
```

### Cell LEF (cells.lef)

```lef
MACRO sky130_fd_sc_hd__nand2_1
  CLASS CORE ;
  ORIGIN 0 0 ;
  FOREIGN sky130_fd_sc_hd__nand2_1 ;
  SIZE 1.38 BY 2.72 ;
  SITE unithd ;

  PIN A
    DIRECTION INPUT ;
    USE SIGNAL ;
    PORT
      LAYER li1 ;
      RECT 0.135 1.05 0.305 1.275 ;
    END
  END A

  PIN B
    DIRECTION INPUT ;
    USE SIGNAL ;
    PORT
      LAYER li1 ;
      RECT 0.475 0.93 0.645 1.105 ;
    END
  END B

  PIN Y
    DIRECTION OUTPUT ;
    USE SIGNAL ;
    PORT
      LAYER li1 ;
      RECT 0.815 1.06 1.245 1.235 ;
    END
  END Y

  PIN VPWR
    DIRECTION INOUT ;
    USE POWER ;
    PORT
      LAYER met1 ;
      RECT 0 2.48 1.38 2.96 ;
    END
  END VPWR

  PIN VGND
    DIRECTION INOUT ;
    USE GROUND ;
    PORT
      LAYER met1 ;
      RECT 0 -0.24 1.38 0.24 ;
    END
  END VGND

  OBS
    LAYER li1 ;
    RECT 0.305 0.475 0.475 0.815 ;
    ...
  END OBS
END sky130_fd_sc_hd__nand2_1
```

Cell describes: footprint (1.38 µm × 2.72 µm), pin locations (in µm), obstructions (where router can't put metal).

## DEF Format

```def
VERSION 5.8 ;
DIVIDERCHAR "/" ;
BUSBITCHARS "[]" ;
DESIGN adder4 ;
UNITS DISTANCE MICRONS 1000 ;

DIEAREA ( 0 0 ) ( 100000 50000 ) ;

ROW row1 unithd 0 0 N DO 217 BY 1 STEP 460 0 ;
ROW row2 unithd 0 2720 FS DO 217 BY 1 STEP 460 0 ;
... more rows ...

COMPONENTS 16 ;
  - u_fa0_x1 sky130_fd_sc_hd__xor2_1 + PLACED ( 460 0 ) N ;
  - u_fa0_x2 sky130_fd_sc_hd__xor2_1 + PLACED ( 920 0 ) N ;
  - u_fa0_a1 sky130_fd_sc_hd__and2_1 + PLACED ( 1380 0 ) N ;
  ...
END COMPONENTS

PINS 12 ;
  - a[0] + NET a[0] + DIRECTION INPUT + USE SIGNAL + LAYER met2 ( -100 -100 ) ( 100 100 ) ;
  ...
END PINS

NETS 12 ;
  - axb_0 ( u_fa0_x1 Y ) ( u_fa0_x2 A ) + USE SIGNAL ;
  - axb_0_routed ( u_fa0_x1 Y ) ( u_fa0_x2 A ) + USE SIGNAL +
    ROUTED met1 ( 1500 1300 ) ( 1700 1300 )
    NEW met1 ( 1700 1300 ) ( 1700 1100 ) ;
  ...
END NETS

END DESIGN
```

Stages of DEF a design passes through:
1. **Floorplan DEF** — DIEAREA, ROWs, PINs, but COMPONENTS unplaced.
2. **Placed DEF** — COMPONENTS now have `PLACED (x y)` coordinates.
3. **Routed DEF** — NETs now have routed segments.

## Public API

```python
from dataclasses import dataclass, field


@dataclass
class TechLef:
    version: str = "5.8"
    units_microns: int = 1000
    layers: list["LayerDef"] = field(default_factory=list)
    vias: list["ViaDef"] = field(default_factory=list)
    sites: list["SiteDef"] = field(default_factory=list)


@dataclass
class LayerDef:
    name: str
    type: str            # "ROUTING" | "MASTERSLICE" | "CUT"
    direction: str | None = None  # "HORIZONTAL" | "VERTICAL"
    pitch: float = 0.0
    width: float = 0.0
    spacing: float = 0.0
    resistance_per_sq: float = 0.0
    capacitance_per_sq_dist: float = 0.0


@dataclass
class ViaDef:
    name: str
    is_default: bool = False
    layers: list[tuple[str, "Rect"]] = field(default_factory=list)


@dataclass
class SiteDef:
    name: str
    class_: str        # "CORE" | "PAD"
    size: tuple[float, float]


@dataclass
class CellLef:
    name: str
    class_: str
    foreign: str | None
    size: tuple[float, float]    # (width µm, height µm)
    site: str
    pins: list["PinDef"]
    obs: list[tuple[str, "Rect"]] = field(default_factory=list)


@dataclass
class PinDef:
    name: str
    direction: str       # "INPUT" | "OUTPUT" | "INOUT"
    use: str             # "SIGNAL" | "POWER" | "GROUND" | "CLOCK"
    rects: list[tuple[str, "Rect"]]   # [(layer_name, rect), ...]


@dataclass
class Rect:
    x1: float; y1: float; x2: float; y2: float


@dataclass
class Def:
    design: str
    units_microns: int = 1000
    die_area: tuple[Rect, Rect] | None = None    # actually a single rect; use Rect
    rows: list["Row"] = field(default_factory=list)
    components: list["Component"] = field(default_factory=list)
    pins: list["DefPin"] = field(default_factory=list)
    nets: list["Net"] = field(default_factory=list)


@dataclass
class Row:
    name: str
    site: str
    origin: tuple[float, float]
    orientation: str    # "N" | "S" | "FN" | "FS"
    num_x: int
    num_y: int
    step_x: float
    step_y: float


@dataclass
class Component:
    name: str
    cell_type: str
    placed: bool = False
    location: tuple[float, float] | None = None
    orientation: str = "N"


@dataclass
class DefPin:
    name: str
    net: str
    direction: str
    use: str
    layer: str | None = None
    rect: Rect | None = None


@dataclass
class Net:
    name: str
    connections: list[tuple[str, str]]   # [(component_name, pin_name), ...]
    routed_segments: list["Segment"] = field(default_factory=list)


@dataclass
class Segment:
    layer: str
    points: list[tuple[float, float]]


# Parser / writer

def read_tech_lef(path: Path) -> TechLef: ...
def read_cells_lef(path: Path) -> list[CellLef]: ...
def write_tech_lef(tech: TechLef, path: Path) -> None: ...
def write_cells_lef(cells: list[CellLef], path: Path) -> None: ...

def read_def(path: Path) -> Def: ...
def write_def(def_obj: Def, path: Path) -> None: ...
```

## Worked Example — Round-trip Sky130 cell LEF

```python
cells = read_cells_lef(Path("sky130_fd_sc_hd.lef"))
print(len(cells))           # ~250
nand2 = next(c for c in cells if c.name == "sky130_fd_sc_hd__nand2_1")
print(nand2.size)           # (1.38, 2.72)
print(len(nand2.pins))      # 5 (A, B, Y, VPWR, VGND)

# Round-trip
write_cells_lef(cells, Path("/tmp/copy.lef"))
cells2 = read_cells_lef(Path("/tmp/copy.lef"))
assert cells == cells2
```

## Worked Example — emitting placed DEF for 4-bit adder

```python
from lef_def import Def, Row, Component, DefPin, Net, Rect, write_def

def_obj = Def(design="adder4", die_area=Rect(0, 0, 100, 50))

# 16 cells in one row of unithd sites (height 2.72 µm; cells are 1.38 - 2.76 µm wide)
def_obj.rows.append(Row(
    name="row1", site="unithd",
    origin=(0, 0), orientation="N",
    num_x=70, num_y=1, step_x=0.46, step_y=0
))

# Place the 16 mapped cells from tech-mapping
for i, comp in enumerate(mapped_cells):
    def_obj.components.append(Component(
        name=comp.name,
        cell_type=comp.stdcell,
        placed=True,
        location=(i * 1.4, 0),  # 1.4 µm spacing
        orientation="N"
    ))

# IO pins on left edge (inputs) and right edge (outputs)
for name, kind in [("a[0]", "INPUT"), ("a[1]", "INPUT"), ...]:
    def_obj.pins.append(DefPin(name=name, net=name, direction=kind,
                               use="SIGNAL", layer="met2",
                               rect=Rect(-0.5, 1.0 + i*0.5, 0, 1.5 + i*0.5)))

# Nets from the netlist
for net in mapped_netlist.nets:
    def_obj.nets.append(Net(
        name=net.name,
        connections=[(inst.name, pin) for inst, pin in net.connections]
    ))

write_def(def_obj, Path("adder4_placed.def"))
# 16 cells + 12 pins + 12 nets, ~6 KB
```

This DEF can be opened in KLayout (with the tech LEF) and inspected visually — a row of 16 standard cells, IO pins on the boundaries.

## Edge Cases

| Scenario | Handling |
|---|---|
| LEF / DEF version mismatch | Best-effort parse; warn. |
| Macro size with unusual aspect ratio | Allowed; placement / routing must accommodate. |
| Cell with no `OBS` block | Treat as obstruction-free except via PIN areas. |
| Net with > 1 driver | LEF/DEF doesn't catch this; HNL validators do. |
| `SPECIALNETS` (power/ground rails) | Parsed; emitted; not analyzed in v1 (power signoff future). |
| Comments (`#` line) in LEF/DEF | Preserved on round-trip if `preserve_comments=True`. |
| Hierarchical components | LEF/DEF support hierarchy via instance names; we flatten before LEF/DEF emission. |
| Site outside die area | Reject. |
| Components placed outside die area | Reject. |
| Routed segments without endpoints on pin shapes | Warn; potential connectivity issue. |

## Test Strategy

### Unit (95%+)
- Each LEF/DEF token type parses.
- Round-trip: read → write → read produces identical objects.
- Sky130 cells.lef parses without errors.

### Integration
- Read Sky130 sky130_fd_sc_hd.lef; ~250 cells loaded.
- Read Sky130 sky130_fd_sc_hd.tlef; layers/vias/sites loaded.
- Generate floorplan DEF, placed DEF, routed DEF for 4-bit adder; opens in KLayout.
- OpenROAD reads our DEF without modification (cross-validation).

## Conformance

| Standard | Coverage |
|---|---|
| **LEF 5.8** (Si2 reference) | Subset: VERSION, UNITS, LAYER (ROUTING/MASTERSLICE/CUT), VIA, SITE, MACRO, PIN, OBS |
| **DEF 5.8** (Si2 reference) | Subset: DESIGN, DIEAREA, ROW, COMPONENTS, PINS, NETS, SPECIALNETS, TRACKS |
| **Bookshelf format** (academic placement benchmarks) | Out of scope; future spec |
| **OpenAccess** (industrial database) | Out of scope |

## Open Questions

1. **Comments preservation** — yes, optional flag.
2. **TRACKS** statement (track grid for routing) — implement; needed by routing.
3. **SPECIALNETS** for power/ground — parse + emit; analysis future.
4. **GROUPS / REGIONS** — defer.

## Future Work

- LEF 5.9 / DEF 5.9 features.
- LEF/DEF compression handling.
- Streaming readers for very large designs.
- Support for OpenAccess via external library.
- Bookshelf format.
