# asic-routing

Lee maze router for the silicon-stack pipeline. Routes each net on a single metal layer (met1) using BFS on a 2-D grid.

## Pipeline position

```
asic-placement ──► asic-routing ──► gdsii-writer
                                 ──► lef-def (NETS section)
```

## Algorithm

Each net is routed sequentially:

1. Mark all placed cell bodies as blocked on the grid.
2. For each pair of consecutive pins in the net, run Lee BFS from source to target.
3. Reconstruct the path via back-pointer map and convert grid cells to `Segment` records.
4. Mark routed paths as blocked to avoid future overlaps.

Grid pitch defaults to 0.34 µm (Sky130 met1 minimum pitch).

## Key types

| Type | Description |
|------|-------------|
| `PinAccess` | cell instance + pin name + grid coordinates |
| `RouteOptions` | pitch (µm), layer name, max BFS iterations per net |
| `RouteReport` | nets_routed, nets_failed, failed_nets list, total_wire_length |
| `RouteError` | `NoDieArea` |

## Usage

```rust
use asic_routing::{route, RouteOptions};

let (routed_def, report) = route(&placed_def, &nets, RouteOptions::default())?;
println!("{}/{} nets routed", report.nets_routed, report.nets_routed + report.nets_failed);
```

## Testing

```
cargo test -p asic-routing -- --nocapture
```

9 integration tests + 1 doc-test covering: basic routing, unreachable net handling, pin access coordinates, grid conversion, wire length accumulation.
