"""Tests for ASIC routing."""

import pytest

from asic_routing import PinAccess, RouteOptions, route, segment_length
from asic_routing.router import _lee_maze_route
from lef_def import Component, Def, Rect


def make_placed_def(n_cells: int = 4) -> Def:
    return Def(
        design="x",
        die_area=Rect(0.0, 0.0, 20.0, 20.0),
        components=[
            Component(
                name=f"u{i}", cell_type="cell",
                placed=True, location_x=2.0 * i, location_y=0.0,
            )
            for i in range(n_cells)
        ],
    )


# ---- Lee maze ----


def test_lee_maze_short_path():
    blocked = [[False] * 5 for _ in range(5)]
    s = PinAccess("u0", "Y", x=0, y=0)
    t = PinAccess("u1", "A", x=4, y=4)
    path = _lee_maze_route(blocked, s, t, max_iters=1000)
    assert path is not None
    assert path[0] == (0, 0)
    assert path[-1] == (4, 4)


def test_lee_maze_finds_around_obstacle():
    blocked = [[False] * 5 for _ in range(5)]
    # Wall at x=2 from y=0..3
    for y in range(4):
        blocked[2][y] = True
    s = PinAccess("u0", "Y", x=0, y=0)
    t = PinAccess("u1", "A", x=4, y=0)
    path = _lee_maze_route(blocked, s, t, max_iters=1000)
    assert path is not None
    assert path[0] == (0, 0)
    assert path[-1] == (4, 0)
    # Must go around the wall (path should rise above y=3 somewhere)
    assert any(y >= 4 for x, y in path)


def test_lee_maze_no_path():
    blocked = [[False] * 5 for _ in range(5)]
    # Surround the target completely
    for x, y in ((3, 4), (4, 3), (3, 3)):
        blocked[x][y] = True
    blocked[4][4] = False  # target itself open, but enclosed
    s = PinAccess("u0", "Y", x=0, y=0)
    t = PinAccess("u1", "A", x=4, y=4)
    path = _lee_maze_route(blocked, s, t, max_iters=1000)
    assert path is None


def test_lee_maze_source_equals_sink():
    blocked = [[False] * 5 for _ in range(5)]
    s = PinAccess("u0", "Y", x=2, y=2)
    t = PinAccess("u0", "A", x=2, y=2)
    path = _lee_maze_route(blocked, s, t, max_iters=1000)
    assert path == [(2, 2)]


def test_lee_maze_source_out_of_bounds():
    blocked = [[False] * 5 for _ in range(5)]
    s = PinAccess("u0", "Y", x=99, y=99)
    t = PinAccess("u1", "A", x=0, y=0)
    path = _lee_maze_route(blocked, s, t, max_iters=1000)
    assert path is None


# ---- segment_length ----


def test_segment_length_zero_path():
    assert segment_length([]) == 0
    assert segment_length([(0, 0)]) == 0


def test_segment_length_manhattan():
    # (0,0) -> (3,0) -> (3,2) total length = 3 + 2 = 5
    assert segment_length([(0, 0), (3, 0), (3, 2)]) == 5


# ---- route() ----


def test_route_simple_two_pin_net():
    placed = make_placed_def(4)
    nets = [
        ("c0", [
            PinAccess("u0", "Y", x=4, y=10),
            PinAccess("u1", "A", x=15, y=10),
        ]),
    ]
    routed_def, report = route(placed, nets=nets, options=RouteOptions(pitch=1.0))
    assert report.nets_routed == 1
    assert report.nets_failed == 0
    assert len(routed_def.nets) == 1
    assert len(routed_def.nets[0].routed_segments) == 1
    assert report.total_wire_length > 0


def test_route_three_pin_star():
    placed = make_placed_def(4)
    nets = [
        ("c0", [
            PinAccess("u0", "Y", x=4, y=10),
            PinAccess("u1", "A", x=10, y=10),
            PinAccess("u2", "A", x=15, y=10),
        ]),
    ]
    routed_def, report = route(placed, nets=nets, options=RouteOptions(pitch=1.0))
    assert report.nets_routed == 1
    # Star routing: 2 segments (source-sink1, source-sink2)
    assert len(routed_def.nets[0].routed_segments) == 2


def test_route_single_pin_net_no_op():
    placed = make_placed_def(2)
    nets = [("c0", [PinAccess("u0", "Y", x=4, y=10)])]
    routed_def, report = route(placed, nets=nets, options=RouteOptions(pitch=1.0))
    # No routing needed for a single-pin net
    assert report.nets_routed == 0
    assert report.nets_failed == 0
    assert routed_def.nets[0].routed_segments == []


def test_route_no_die_area_raises():
    placed = Def(design="x", die_area=None)
    with pytest.raises(ValueError, match="die_area"):
        route(placed, nets=[])


def test_route_failed_net_in_report():
    """If routing can't find a path, it appears in failed_nets."""
    placed = make_placed_def(2)
    nets = [
        ("c0", [
            PinAccess("u0", "Y", x=999, y=999),  # off-grid
            PinAccess("u1", "A", x=15, y=10),
        ]),
    ]
    _, report = route(placed, nets=nets, options=RouteOptions(pitch=1.0))
    assert report.nets_failed == 1
    assert "c0" in report.failed_nets


# ---- 4-bit adder smoke ----


def test_adder4_routing_smoke():
    """Route a 4-bit-adder-shape: 4 stages of cells, simple chain nets."""
    placed = Def(
        design="adder4",
        die_area=Rect(0.0, 0.0, 30.0, 10.0),
        components=[
            Component(name=f"fa{i}", cell_type="cell",
                      placed=True, location_x=4.0 * i, location_y=0.0)
            for i in range(4)
        ],
    )
    # Chain nets: c0 connects fa0 to fa1, c1 connects fa1 to fa2, etc.
    nets = [
        (f"c{i}", [
            PinAccess(f"fa{i}", "cout", x=int(4.0 * i + 2), y=5),
            PinAccess(f"fa{i+1}", "cin", x=int(4.0 * (i+1)), y=5),
        ])
        for i in range(3)
    ]
    routed_def, report = route(placed, nets=nets, options=RouteOptions(pitch=1.0))
    assert report.nets_routed == 3
    assert report.nets_failed == 0
