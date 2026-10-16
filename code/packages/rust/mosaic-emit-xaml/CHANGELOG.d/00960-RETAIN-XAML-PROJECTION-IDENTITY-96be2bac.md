### Retain XAML projection identity

Generated For projections retain observable collections and structurally match
constructor inputs to preserve unchanged row and nested cell VMs. Computed
bindings notify only changed observed values; owner selection still refreshes
nested projections. Changed rows, null sources, growth, shrinkage, widths and
captures are covered by a compiled emitted-C# probe. Native evidence and remaining
latency limits are recorded in the projection identity spec (#16865).

Focus and accessibility cell lookup now returns the first matching realized cell
without collecting the entire visual tree. A compiled emitted-method probe checks
early termination, nested matches and missing coordinates.

Queued focus now reuses a retained native cell after validating its current
coordinates and table ancestry. Detached, replaced, reparented and moved cells
fall back to current-tree lookup; executable emitted-C# cases cover each path.
