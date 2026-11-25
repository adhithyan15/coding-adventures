---
category: Testing & coverage
---

# A report reader must preserve producer workload identifiers rather than selecting its own workload

because a replay reader validates a report's generic observation schema; it
does not decide which benchmark implementation produced those observations.
Hard-coding one package's current CLI workload ID rejects valid reports from
other producers even when their metadata is complete and their raw pairs,
schedule, bounds, and recomputed analysis are valid. Accept any nonblank
producer-supplied workload ID, reject blank and NUL-containing values, and
keep the CLI's own default workload identifier unchanged. Test both a valid
different ID and malformed workload metadata without relaxing cross-report
comparison rules.
