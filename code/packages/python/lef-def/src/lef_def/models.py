"""LEF/DEF data classes."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class Direction(Enum):
    INPUT = "INPUT"
    OUTPUT = "OUTPUT"
    INOUT = "INOUT"


class Use(Enum):
    SIGNAL = "SIGNAL"
    POWER = "POWER"
    GROUND = "GROUND"
    CLOCK = "CLOCK"


@dataclass(frozen=True, slots=True)
class Rect:
    x1: float
    y1: float
    x2: float
    y2: float


# ----------------------------------------------------------------------------
# LEF (technology + cells)
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class LayerDef:
    name: str
    type: str
    direction: str | None = None
    pitch: float = 0.0
    width: float = 0.0
    spacing: float = 0.0


@dataclass(frozen=True, slots=True)
class ViaLayer:
    layer: str
    rect: Rect


@dataclass(frozen=True, slots=True)
class ViaDef:
    name: str
    is_default: bool
    layers: tuple[ViaLayer, ...]


@dataclass(frozen=True, slots=True)
class SiteDef:
    name: str
    class_: str
    width: float
    height: float


@dataclass
class TechLef:
    version: str = "5.8"
    units_microns: int = 1000
    layers: list[LayerDef] = field(default_factory=list)
    vias: list[ViaDef] = field(default_factory=list)
    sites: list[SiteDef] = field(default_factory=list)


@dataclass(frozen=True, slots=True)
class PinPort:
    layer: str
    rect: Rect


@dataclass(frozen=True, slots=True)
class PinDef:
    name: str
    direction: Direction
    use: Use
    ports: tuple[PinPort, ...]


@dataclass
class CellLef:
    name: str
    class_: str = "CORE"
    foreign: str | None = None
    width: float = 0.0
    height: float = 0.0
    site: str = ""
    pins: list[PinDef] = field(default_factory=list)
    obs: list[tuple[str, Rect]] = field(default_factory=list)


# ----------------------------------------------------------------------------
# DEF
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Row:
    name: str
    site: str
    origin_x: float
    origin_y: float
    orientation: str
    num_x: int
    num_y: int
    step_x: float
    step_y: float


@dataclass(frozen=True, slots=True)
class Component:
    name: str
    cell_type: str
    placed: bool = False
    location_x: float | None = None
    location_y: float | None = None
    orientation: str = "N"


@dataclass(frozen=True, slots=True)
class DefPin:
    name: str
    net: str
    direction: Direction
    use: Use
    layer: str | None = None
    rect: Rect | None = None


@dataclass(frozen=True, slots=True)
class Segment:
    layer: str
    points: tuple[tuple[float, float], ...]


@dataclass
class Net:
    name: str
    connections: list[tuple[str, str]] = field(default_factory=list)
    routed_segments: list[Segment] = field(default_factory=list)


@dataclass
class Def:
    design: str
    version: str = "5.8"
    units_microns: int = 1000
    die_area: Rect | None = None
    rows: list[Row] = field(default_factory=list)
    components: list[Component] = field(default_factory=list)
    pins: list[DefPin] = field(default_factory=list)
    nets: list[Net] = field(default_factory=list)
