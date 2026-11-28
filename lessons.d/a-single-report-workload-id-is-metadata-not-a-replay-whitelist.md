---
category: Testing & coverage
---

# A single report workload id is metadata, not a replay whitelist

The report's workload field names the producer's experiment. A reader that
replays stored durations and recomputes analysis must validate the identifier's
type and basic string bounds, but rejecting every identifier except its own
sample CLI unnecessarily ties analysis to one workload. Keep workload-match
checks in tools that compare multiple experiments, and never use a report
identifier to execute a workload.
