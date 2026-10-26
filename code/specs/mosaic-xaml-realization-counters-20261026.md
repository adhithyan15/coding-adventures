# Native repeater realization diagnostics

For #16865, count WinUI ItemsRepeater ElementPrepared and ElementClearing events
in generated components, separately from row VM projection work. These count
item lifecycle events, not every descendant control, allocations, unique objects,
or a guarantee that an item is visible. Recycled items can be prepared again.

When MOSAIC_PROFILE_PATH is set, expose cumulative long Prepared/Cleared counters
through optional MosaicRealizationCounts. The host reads this exact tuple contract
at interval boundaries and exports optional realization deltas. No traversal,
application values, or element references are retained. Missing/malformed/throwing
contracts must not break interactions. Keep existing sample limits, filtering,
phase names and projection fields. Overlapping intervals are not exclusive work.

Validate generated nested repeaters, standalone host compatibility, exported
counter deltas and malformed contracts, native project build and real selection
and scroll interaction. Instrumentation does not close performance acceptance;
matched warm Release/Debug percentile workloads remain required.