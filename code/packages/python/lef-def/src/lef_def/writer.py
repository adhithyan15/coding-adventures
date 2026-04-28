"""LEF/DEF text writers.

For v0.1.0 we focus on emission. Parsers live in `parser.py` and cover the
subset needed by Sky130 and OpenROAD interop."""

from __future__ import annotations

from io import StringIO
from pathlib import Path

from lef_def.models import (
    CellLef,
    Def,
    LayerDef,
    PinDef,
    SiteDef,
    TechLef,
    ViaDef,
)

# ----------------------------------------------------------------------------
# LEF writers
# ----------------------------------------------------------------------------


def write_tech_lef(tech: TechLef, path: str | Path) -> None:
    Path(path).write_text(write_tech_lef_str(tech))


def write_tech_lef_str(tech: TechLef) -> str:
    out = StringIO()
    out.write(f"VERSION {tech.version} ;\n")
    out.write('BUSBITCHARS "[]" ;\n')
    out.write('DIVIDERCHAR "/" ;\n')
    out.write(f"UNITS\n  DATABASE MICRONS {tech.units_microns} ;\nEND UNITS\n\n")

    for layer in tech.layers:
        out.write(_layer_to_lef(layer))
        out.write("\n")
    for via in tech.vias:
        out.write(_via_to_lef(via))
        out.write("\n")
    for site in tech.sites:
        out.write(_site_to_lef(site))
        out.write("\n")

    out.write("END LIBRARY\n")
    return out.getvalue()


def _layer_to_lef(layer: LayerDef) -> str:
    out = StringIO()
    out.write(f"LAYER {layer.name}\n")
    out.write(f"  TYPE {layer.type} ;\n")
    if layer.direction:
        out.write(f"  DIRECTION {layer.direction} ;\n")
    if layer.pitch:
        out.write(f"  PITCH {layer.pitch} ;\n")
    if layer.width:
        out.write(f"  WIDTH {layer.width} ;\n")
    if layer.spacing:
        out.write(f"  SPACING {layer.spacing} ;\n")
    out.write(f"END {layer.name}\n")
    return out.getvalue()


def _via_to_lef(via: ViaDef) -> str:
    out = StringIO()
    out.write(f"VIA {via.name}{' DEFAULT' if via.is_default else ''}\n")
    for vl in via.layers:
        r = vl.rect
        out.write(f"  LAYER {vl.layer} ;\n")
        out.write(f"    RECT {r.x1} {r.y1} {r.x2} {r.y2} ;\n")
    out.write(f"END {via.name}\n")
    return out.getvalue()


def _site_to_lef(site: SiteDef) -> str:
    out = StringIO()
    out.write(f"SITE {site.name}\n")
    out.write(f"  CLASS {site.class_} ;\n")
    out.write(f"  SIZE {site.width} BY {site.height} ;\n")
    out.write(f"END {site.name}\n")
    return out.getvalue()


def write_cells_lef(cells: list[CellLef], path: str | Path) -> None:
    Path(path).write_text(write_cells_lef_str(cells))


def write_cells_lef_str(cells: list[CellLef]) -> str:
    out = StringIO()
    for cell in cells:
        out.write(_cell_to_lef(cell))
        out.write("\n")
    return out.getvalue()


def _cell_to_lef(cell: CellLef) -> str:
    out = StringIO()
    out.write(f"MACRO {cell.name}\n")
    out.write(f"  CLASS {cell.class_} ;\n")
    out.write("  ORIGIN 0 0 ;\n")
    if cell.foreign:
        out.write(f"  FOREIGN {cell.foreign} ;\n")
    out.write(f"  SIZE {cell.width} BY {cell.height} ;\n")
    if cell.site:
        out.write(f"  SITE {cell.site} ;\n")
    for pin in cell.pins:
        out.write(_pin_to_lef(pin))
    if cell.obs:
        out.write("  OBS\n")
        for layer, rect in cell.obs:
            out.write(f"    LAYER {layer} ;\n")
            out.write(f"    RECT {rect.x1} {rect.y1} {rect.x2} {rect.y2} ;\n")
        out.write("  END\n")
    out.write(f"END {cell.name}\n")
    return out.getvalue()


def _pin_to_lef(pin: PinDef) -> str:
    out = StringIO()
    out.write(f"  PIN {pin.name}\n")
    out.write(f"    DIRECTION {pin.direction.value} ;\n")
    out.write(f"    USE {pin.use.value} ;\n")
    out.write("    PORT\n")
    for port in pin.ports:
        r = port.rect
        out.write(f"      LAYER {port.layer} ;\n")
        out.write(f"      RECT {r.x1} {r.y1} {r.x2} {r.y2} ;\n")
    out.write("    END\n")
    out.write(f"  END {pin.name}\n")
    return out.getvalue()


# ----------------------------------------------------------------------------
# DEF writer
# ----------------------------------------------------------------------------


def write_def(def_obj: Def, path: str | Path) -> None:
    Path(path).write_text(write_def_str(def_obj))


def write_def_str(def_obj: Def) -> str:
    out = StringIO()
    out.write(f"VERSION {def_obj.version} ;\n")
    out.write('DIVIDERCHAR "/" ;\n')
    out.write('BUSBITCHARS "[]" ;\n')
    out.write(f"DESIGN {def_obj.design} ;\n")
    out.write(f"UNITS DISTANCE MICRONS {def_obj.units_microns} ;\n\n")

    if def_obj.die_area:
        d = def_obj.die_area
        out.write(f"DIEAREA ( {d.x1} {d.y1} ) ( {d.x2} {d.y2} ) ;\n\n")

    for row in def_obj.rows:
        out.write(
            f"ROW {row.name} {row.site} {row.origin_x} {row.origin_y} "
            f"{row.orientation} DO {row.num_x} BY {row.num_y} "
            f"STEP {row.step_x} {row.step_y} ;\n"
        )
    if def_obj.rows:
        out.write("\n")

    if def_obj.components:
        out.write(f"COMPONENTS {len(def_obj.components)} ;\n")
        for c in def_obj.components:
            line = f"  - {c.name} {c.cell_type}"
            if c.placed and c.location_x is not None and c.location_y is not None:
                line += f" + PLACED ( {c.location_x} {c.location_y} ) {c.orientation}"
            line += " ;"
            out.write(line + "\n")
        out.write("END COMPONENTS\n\n")

    if def_obj.pins:
        out.write(f"PINS {len(def_obj.pins)} ;\n")
        for p in def_obj.pins:
            line = (
                f"  - {p.name} + NET {p.net} + DIRECTION {p.direction.value} "
                f"+ USE {p.use.value}"
            )
            if p.layer and p.rect:
                r = p.rect
                line += f" + LAYER {p.layer} ( {r.x1} {r.y1} ) ( {r.x2} {r.y2} )"
            line += " ;"
            out.write(line + "\n")
        out.write("END PINS\n\n")

    if def_obj.nets:
        out.write(f"NETS {len(def_obj.nets)} ;\n")
        for n in def_obj.nets:
            conns = " ".join(f"( {comp} {pin} )" for comp, pin in n.connections)
            line = f"  - {n.name} {conns} + USE SIGNAL"
            if n.routed_segments:
                line += "\n    ROUTED"
                for seg in n.routed_segments:
                    pts = " ".join(f"( {x} {y} )" for x, y in seg.points)
                    line += f" {seg.layer} {pts}"
            line += " ;"
            out.write(line + "\n")
        out.write("END NETS\n\n")

    out.write("END DESIGN\n")
    return out.getvalue()
