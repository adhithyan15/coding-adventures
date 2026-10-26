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
## Native receipt (2026-10-26)

Source base cb2c6eb86b1 plus this change; generated VisiCalc sources were not
modified. Release x64 build passed with zero errors and two existing CS0067
warnings. 395 emitter unit tests and the emitted C# host execution test passed,
including independent realization-only, malformed, missing and throwing contracts.
Security review passed.

Real native Computer Use sequence: startup A1=15, click B2=14, Right to C2=7,
wheel down to visible rows 16–34 with headers retained, normal close. Profile
mode: MOSAIC_PROFILE_INTERACTIONS_ONLY=1. Next-render records in sequence:

| Interval | ms | current-thread bytes | projection created/reused/removed | prepared/cleared |
|---|---:|---:|---|---|
| Startup viewport | 16.5121 | 459032 | 0/539/11 | 0/3 |
| B2 selection | 17.0194 | 412264 | 0/539/0 | 0/0 |
| C2 keyboard selection | 12.3517 | 371376 | 0/539/0 | 0/0 |
| Wheel viewport shift | 67.6533 | 1850544 | 135/404/5 | 132/67 |

These are four smoke intervals, not percentiles or before/after performance proof.
The scroll interval exceeds 50 ms; full warm Release/Debug selection, scrolling,
editing, larger viewport and durability acceptance remains open under #16865.
Counter values count lifecycle callbacks, not all descendant native controls.

Raw local evidence: C:/Users/adhit/.codex/worktrees/visicalc-realization-20261026-output/native-realization.jsonl
and native-release-build.log. Unit/host/generation logs are retained in the sibling
visicalc-filtered-measurement-20261025-output directory as realization-unit.log,
realization-host.log and realization-generation.log. Screenshots were inspected
in this chat. The unchanged Oct16 Rust runtime was reused (SHA256
A18198012F2985EC5CEA6E10A2151388D2E9093F567A59F46A72E45C28BC2124).
