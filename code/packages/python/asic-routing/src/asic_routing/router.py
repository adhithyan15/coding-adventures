"""Lee maze routing on a 2-D track grid (single metal layer in v0.1.0).

The router takes a placed Def + a list of nets (each a list of cell-pin
references) and finds connecting paths via BFS on a track grid. Routed
segments are written back into the Def as `Segment` records on the chosen
metal layer.

For v0.1.0 we route on a single layer (`met1` by default) since 2-D Lee on
a single layer is the clearest pedagogical implementation. Multi-layer
routing with via insertion lands in v0.2.0.
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field

from lef_def import Component, Def, Net, Segment


@dataclass(frozen=True, slots=True)
class PinAccess:
    """Where a pin lives on the routing grid."""

    cell_instance: str
    pin_name: str
    x: int  # grid coordinates
    y: int


@dataclass
class RouteOptions:
    pitch: float = 0.34  # micrometers per grid step
    layer: str = "met1"
    max_iters_per_net: int = 100_000


@dataclass
class RouteReport:
    nets_routed: int = 0
    nets_failed: int = 0
    failed_nets: list[str] = field(default_factory=list)
    total_wire_length: float = 0.0
    total_vias: int = 0


def route(
    placed: Def,
    *,
    nets: list[tuple[str, list[PinAccess]]],
    options: RouteOptions | None = None,
) -> tuple[Def, RouteReport]:
    """Route every net via Lee maze on a 2-D grid.

    `nets`: list of (net_name, list of pin accesses) tuples.

    Returns a Def with Net.routed_segments populated and a RouteReport with
    success/failure counts."""
    if options is None:
        options = RouteOptions()
    if placed.die_area is None:
        raise ValueError("placed Def has no die_area; can't size grid")

    die = placed.die_area
    width_grid = max(1, int((die.x2 - die.x1) / options.pitch) + 1)
    height_grid = max(1, int((die.y2 - die.y1) / options.pitch) + 1)

    # Single global blocked map (cells block, routes don't until they're laid)
    blocked = [[False] * height_grid for _ in range(width_grid)]
    _mark_components_blocked(placed.components, options.pitch, blocked)

    new_nets: list[Net] = []
    report = RouteReport()
    pitch = options.pitch

    for net_name, pins in nets:
        if len(pins) < 2:
            new_nets.append(Net(name=net_name))
            continue
        # Route from pins[0] to pins[1], pins[2], ... (star topology)
        net_segments: list[Segment] = []
        source = pins[0]
        for sink in pins[1:]:
            path = _lee_maze_route(
                blocked, source, sink, options.max_iters_per_net,
            )
            if path is None:
                report.nets_failed += 1
                report.failed_nets.append(net_name)
                break
            # Convert grid path to segment in user units.
            segment = _path_to_segment(path, options.layer, pitch)
            net_segments.append(segment)
            report.total_wire_length += segment_length(path) * pitch
            # Mark route cells as blocked (single-layer router; can't share)
            for x, y in path:
                blocked[x][y] = True
        else:
            report.nets_routed += 1

        connections = [(p.cell_instance, p.pin_name) for p in pins]
        new_nets.append(Net(
            name=net_name,
            connections=connections,
            routed_segments=net_segments,
        ))

    routed_def = Def(
        design=placed.design,
        version=placed.version,
        units_microns=placed.units_microns,
        die_area=placed.die_area,
        rows=list(placed.rows),
        components=list(placed.components),
        pins=list(placed.pins),
        nets=new_nets,
    )
    return (routed_def, report)


def _mark_components_blocked(
    components: list[Component], pitch: float, blocked: list[list[bool]],
) -> None:
    """Mark grid cells under each placed component as blocked."""
    for c in components:
        if c.location_x is None or c.location_y is None:
            continue
        # Approximate cell footprint as a small rectangle around location.
        # Real impl would use cell width/height; we'll just block 1 grid cell.
        gx = int(c.location_x / pitch)
        gy = int(c.location_y / pitch)
        if 0 <= gx < len(blocked) and 0 <= gy < len(blocked[0]):
            blocked[gx][gy] = True


def _lee_maze_route(
    blocked: list[list[bool]],
    source: PinAccess,
    sink: PinAccess,
    max_iters: int,
) -> list[tuple[int, int]] | None:
    """BFS from source to sink avoiding blocked cells. Returns path or None."""
    width = len(blocked)
    height = len(blocked[0]) if width else 0

    if not (0 <= source.x < width and 0 <= source.y < height):
        return None
    if not (0 <= sink.x < width and 0 <= sink.y < height):
        return None

    if source.x == sink.x and source.y == sink.y:
        return [(source.x, source.y)]

    parent: dict[tuple[int, int], tuple[int, int] | None] = {(source.x, source.y): None}
    queue = deque([(source.x, source.y)])
    iters = 0
    target = (sink.x, sink.y)

    while queue and iters < max_iters:
        x, y = queue.popleft()
        iters += 1
        if (x, y) == target:
            return _reconstruct_path(parent, target)
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nx, ny = x + dx, y + dy
            if not (0 <= nx < width and 0 <= ny < height):
                continue
            if (nx, ny) in parent:
                continue
            # Allow target cell even if blocked (it's the pin)
            if blocked[nx][ny] and (nx, ny) != target:
                continue
            parent[(nx, ny)] = (x, y)
            queue.append((nx, ny))

    return None


def _reconstruct_path(
    parent: dict[tuple[int, int], tuple[int, int] | None],
    target: tuple[int, int],
) -> list[tuple[int, int]]:
    path = []
    cur: tuple[int, int] | None = target
    while cur is not None:
        path.append(cur)
        cur = parent[cur]
    path.reverse()
    return path


def _path_to_segment(
    path: list[tuple[int, int]],
    layer: str,
    pitch: float,
) -> Segment:
    """Convert a grid path to a routed segment (in user-unit coordinates)."""
    points = tuple((x * pitch, y * pitch) for x, y in path)
    return Segment(layer=layer, points=points)


def segment_length(path: list[tuple[int, int]]) -> int:
    """Manhattan distance along a path in grid units."""
    total = 0
    for i in range(1, len(path)):
        x0, y0 = path[i - 1]
        x1, y1 = path[i]
        total += abs(x1 - x0) + abs(y1 - y0)
    return total
