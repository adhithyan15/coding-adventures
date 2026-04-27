# asic-routing

Lee maze routing on a 2-D track grid. Single metal layer in v0.1.0; multi-layer with via insertion lands in v0.2.0.

See [`code/specs/asic-routing.md`](../../../specs/asic-routing.md).

## Quick start

```python
from asic_routing import route, RouteOptions, PinAccess
from lef_def import write_def

placed_def = ...  # from asic-placement

# Pin accesses for each net (cell instance + pin name + grid position)
nets = [
    ("c0", [
        PinAccess("u_fa0", "Y", x=10, y=5),
        PinAccess("u_fa1", "A", x=12, y=5),
    ]),
    ("c1", [
        PinAccess("u_fa1", "Y", x=14, y=5),
        PinAccess("u_fa2", "A", x=16, y=5),
    ]),
]

routed_def, report = route(placed_def, nets=nets, options=RouteOptions(pitch=0.34))

print(f"{report.nets_routed} nets routed; {report.nets_failed} failed")
print(f"total wire length: {report.total_wire_length:.2f} µm")
write_def(routed_def, "adder4_routed.def")
```

## v0.1.0 scope

- Lee BFS maze routing on a 2-D pitch-regular grid
- Single metal layer (configurable; default `met1`)
- Star topology: route source pin to every other pin in the net
- Marks routed paths as blocked so subsequent nets avoid them
- Cell footprints block grid cells under their location
- `RouteReport`: routed/failed counts, list of failed nets, total wirelength

## Out of scope (v0.2.0)

- Multi-layer routing with via insertion
- Steiner-tree routing (currently star)
- PathFinder negotiation-based congestion handling
- Layer-direction preferences (met1 horizontal, met2 vertical, etc.)
- Antenna-rule fixing
- DRC-aware routing during search

MIT.
