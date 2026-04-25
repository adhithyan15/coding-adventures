"""ASIC floorplanning: die area, IO ring, rows.

Computes a sensible floorplan from cell-area estimates and a target utilization,
emits a DEF file with DIEAREA + ROW + (placeholder) COMPONENTS.
"""

from __future__ import annotations

import math
from collections.abc import Iterable
from dataclasses import dataclass

from lef_def import Component, Def, DefPin, Direction, Rect, Row, Use


@dataclass(frozen=True, slots=True)
class CellInstanceEstimate:
    """A cell that will be placed eventually. We just need its name + area
    + reference cell type for floorplanning."""

    instance_name: str
    cell_type: str
    area: float  # micrometers^2


@dataclass(frozen=True, slots=True)
class IoSpec:
    """A top-level pin to be placed on the die boundary."""

    name: str
    direction: Direction
    use: Use = Use.SIGNAL


@dataclass(frozen=True, slots=True)
class Floorplan:
    """A computed floorplan ready to emit as DEF."""

    die: Rect
    core: Rect
    rows: tuple[Row, ...]
    components: tuple[Component, ...]
    pins: tuple[DefPin, ...]


def compute_floorplan(
    *,
    cells: list[CellInstanceEstimate],
    site_height: float,
    site_width: float,
    site_name: str,
    utilization: float = 0.7,
    aspect: float = 1.0,
    io_ring_width: float = 10.0,
    io_pins: list[IoSpec] | None = None,
    pin_layer: str = "met2",
    design_name: str = "design",
) -> Floorplan:
    """Compute a floorplan and return the resulting Floorplan dataclass.

    Parameters:
      cells: list of CellInstanceEstimate (instance name, cell type, area in
        sq µm).
      site_height: height of one row (e.g. 2.72 for sky130_fd_sc_hd).
      site_width: width of one site (e.g. 0.46).
      site_name: site name for ROWs (e.g. "unithd").
      utilization: target core utilization (0 < utilization <= 1). Default 0.7.
      aspect: ratio of core_width / core_height. Default 1.0 (square).
      io_ring_width: micrometers of buffer around the core for IO.
      io_pins: list of IoSpec; placed evenly on edges.
      pin_layer: layer for IO pin shapes (e.g. "met2").
      design_name: DEF design name (used in pins / DEF metadata).

    Validates that utilization is in (0, 1] and area > 0.
    """
    if not (0.0 < utilization <= 1.0):
        raise ValueError(f"utilization must be in (0, 1], got {utilization}")
    if aspect <= 0:
        raise ValueError(f"aspect must be > 0, got {aspect}")

    total_area = sum(c.area for c in cells)
    if total_area <= 0:
        raise ValueError(f"total cell area must be > 0, got {total_area}")

    core_area = total_area / utilization

    # core_width × core_height = core_area; core_width = aspect × core_height
    core_height = math.sqrt(core_area / aspect)
    core_width = aspect * core_height

    # Snap height to integer rows
    n_rows = max(1, int(math.ceil(core_height / site_height)))
    core_height = n_rows * site_height

    # Snap width to integer site count
    n_sites = max(1, int(math.ceil(core_width / site_width)))
    core_width = n_sites * site_width

    # Place core inside die with io_ring_width margin on every side.
    core_x0 = io_ring_width
    core_y0 = io_ring_width
    core_x1 = core_x0 + core_width
    core_y1 = core_y0 + core_height
    die = Rect(0.0, 0.0, core_x1 + io_ring_width, core_y1 + io_ring_width)

    # Generate rows.
    rows = tuple(
        Row(
            name=f"row_{i}",
            site=site_name,
            origin_x=core_x0,
            origin_y=core_y0 + i * site_height,
            orientation="N" if i % 2 == 0 else "FS",
            num_x=n_sites,
            num_y=1,
            step_x=site_width,
            step_y=0.0,
        )
        for i in range(n_rows)
    )

    # Place components left-to-right across rows; first-fit packing.
    components = tuple(
        Component(name=c.instance_name, cell_type=c.cell_type, placed=False)
        for c in cells
    )

    # IO pin placement: evenly along the four edges.
    pins = _place_io_pins(io_pins or [], die, pin_layer, design_name=design_name)

    core = Rect(core_x0, core_y0, core_x1, core_y1)
    return Floorplan(die=die, core=core, rows=rows, components=components, pins=pins)


def _place_io_pins(
    io: Iterable[IoSpec],
    die: Rect,
    pin_layer: str,
    design_name: str,
) -> tuple[DefPin, ...]:
    """Distribute pins evenly around the die. Inputs go left+bottom; outputs
    right+top. Power/ground anywhere."""
    inputs = [p for p in io if p.direction == Direction.INPUT]
    outputs = [p for p in io if p.direction == Direction.OUTPUT]
    others = [p for p in io if p.direction not in (Direction.INPUT, Direction.OUTPUT)]
    del design_name  # not yet used

    pins: list[DefPin] = []

    # Inputs on left edge.
    if inputs:
        edge_h = die.y2 - die.y1
        spacing = edge_h / (len(inputs) + 1)
        for i, p in enumerate(inputs, start=1):
            y = die.y1 + i * spacing
            pins.append(
                DefPin(
                    name=p.name, net=p.name, direction=p.direction,
                    use=p.use, layer=pin_layer,
                    rect=Rect(die.x1 - 0.5, y - 0.1, die.x1, y + 0.1),
                )
            )

    # Outputs on right edge.
    if outputs:
        edge_h = die.y2 - die.y1
        spacing = edge_h / (len(outputs) + 1)
        for i, p in enumerate(outputs, start=1):
            y = die.y1 + i * spacing
            pins.append(
                DefPin(
                    name=p.name, net=p.name, direction=p.direction,
                    use=p.use, layer=pin_layer,
                    rect=Rect(die.x2, y - 0.1, die.x2 + 0.5, y + 0.1),
                )
            )

    # Others on bottom edge.
    if others:
        edge_w = die.x2 - die.x1
        spacing = edge_w / (len(others) + 1)
        for i, p in enumerate(others, start=1):
            x = die.x1 + i * spacing
            pins.append(
                DefPin(
                    name=p.name, net=p.name, direction=p.direction,
                    use=p.use, layer=pin_layer,
                    rect=Rect(x - 0.1, die.y1 - 0.5, x + 0.1, die.y1),
                )
            )

    return tuple(pins)


def floorplan_to_def(fp: Floorplan, design_name: str) -> Def:
    """Build a `lef_def.Def` from a Floorplan. Components are unplaced; the
    placer fills in coordinates later."""
    return Def(
        design=design_name,
        die_area=fp.die,
        rows=list(fp.rows),
        components=list(fp.components),
        pins=list(fp.pins),
    )
