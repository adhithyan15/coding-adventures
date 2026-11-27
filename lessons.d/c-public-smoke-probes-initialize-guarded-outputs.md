---
category: Cross-platform & Windows BUILD_windows
---

# C public smoke probes initialize guarded outputs

The C HDD smoke test reads a sector, then fills a snapshot only when the
read succeeds. It checks the outputs under a final success guard. Native
MSVC still reported C4701 for the snapshot and the harness promoted that
warning to a compile failure.

Initialize both caller-owned output aggregates at their declarations.
Keep the success guards, cleanup and tick comparison. Run the public
header smoke executable alongside the full native suite, since the
Windows harness builds them as separate targets.

[Windows public smoke compile diagnostic](https://github.com/adhithyan15/coding-adventures/actions/runs/46026065328/job/140598450777)
