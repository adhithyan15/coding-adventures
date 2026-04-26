# Changelog

## [0.1.0] — Unreleased

### Added
- `route(placed_def, nets, options=RouteOptions()) -> (Def, RouteReport)`.
- `PinAccess(cell_instance, pin_name, x, y)`: pin's grid coordinates.
- `RouteOptions(pitch=0.34, layer="met1", max_iters_per_net=100_000)`.
- `RouteReport(nets_routed, nets_failed, failed_nets, total_wire_length, total_vias)`.
- Lee BFS maze routing on 2-D grid; sized from die area / pitch.
- Star routing: source -> sink1, source -> sink2, ...
- Routed cells become blocked; subsequent nets reroute around.
- Cell footprints under placed coordinates marked blocked.
- `segment_length(path)`: Manhattan distance utility.

### Out of scope (v0.2.0)
- Multi-layer routing + via insertion.
- Steiner-tree routing.
- PathFinder negotiation-based routing.
- Layer-direction preferences.
- Antenna-rule fixing.
- DRC-aware search.
