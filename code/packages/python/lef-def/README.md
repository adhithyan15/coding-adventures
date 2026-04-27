# lef-def

LEF (Library Exchange Format) and DEF (Design Exchange Format) emitters. v0.1.0 is write-only; the parser lands in v0.2.0 once we have `klayout` cross-validation infrastructure.

See [`code/specs/lef-def.md`](../../../specs/lef-def.md).

## Quick start

```python
from lef_def import (
    TechLef, LayerDef, ViaDef, ViaLayer, SiteDef, Rect,
    write_tech_lef, write_cells_lef,
    CellLef, PinDef, PinPort, Direction, Use,
    Def, Row, Component, DefPin, Net, Segment, write_def,
)

# Write a tech LEF
tech = TechLef(version="5.8", units_microns=1000)
tech.layers.append(LayerDef("met1", "ROUTING", direction="HORIZONTAL",
                             pitch=0.34, width=0.14, spacing=0.14))
tech.sites.append(SiteDef("unithd", "CORE", width=0.46, height=2.72))
write_tech_lef(tech, "tech.lef")

# Write a cells LEF
cells = [
    CellLef(
        name="nand2_1", class_="CORE", width=1.38, height=2.72, site="unithd",
        pins=[
            PinDef("A", Direction.INPUT, Use.SIGNAL,
                   ports=(PinPort("li1", Rect(0.1, 0.1, 0.3, 0.3)),)),
            PinDef("Y", Direction.OUTPUT, Use.SIGNAL,
                   ports=(PinPort("li1", Rect(1.0, 1.0, 1.2, 1.2)),)),
        ],
    ),
]
write_cells_lef(cells, "cells.lef")

# Write a DEF
def_obj = Def(design="adder4", die_area=Rect(0, 0, 100, 50))
def_obj.rows.append(Row("row1", "unithd", 0, 0, "N", 217, 1, 0.46, 0))
def_obj.components.append(
    Component("u_fa0", "nand2_1", placed=True, location_x=10, location_y=0)
)
def_obj.pins.append(
    DefPin("a[0]", "a[0]", Direction.INPUT, Use.SIGNAL,
           layer="met2", rect=Rect(-0.1, 1.0, 0.0, 1.2))
)
write_def(def_obj, "adder4.def")
```

## v0.1.0 scope

- LEF: VERSION, UNITS, LAYER (ROUTING / MASTERSLICE / CUT), VIA, SITE, MACRO with PIN and OBS.
- DEF: VERSION, DESIGN, UNITS, DIEAREA, ROW, COMPONENTS (placed/unplaced), PINS, NETS with optional ROUTED segments.
- Strongly-typed data classes (`Direction`, `Use`, `Rect`, etc.).

## Out of scope (v0.2.0)

- LEF / DEF parsers (the heavier lift).
- SPECIALNETS, GROUPS, REGIONS, BLOCKAGES.
- LEF 5.9 features.

MIT.
