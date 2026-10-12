---
category: Testing & coverage
---

# Repeated corpus parsing and worker heap exhaustion are separate failures

Run 38176850343 built 532 macOS packages and failed only human-language-data.
Three chapter snapshot assertions each reparsed the full corpus and exceeded
their 30-second deadlines. Separately, the figure-target worker exhausted its
2 GiB Node heap while registering dynamically imported case shards; it never
reported that suite's result. Comparing the discovered test owners with the
completed-suite log identified the missing owner instead of guessing from the
nearby timeout messages.

The chapter snapshot cases repeat four corpus loads and three identical gate
reports. The figure-target suite's four real-corpus case groups also retain
independent full corpus snapshots and resolve the same filmstrip targets.
Share immutable measurements within each suite while preserving its assertions
and keeping synthetic fixtures independent. Reproduce and validate the worker
failure under the same 2 GiB heap budget before calling the repair complete.
Keep ordinary test deadlines and heap limits unchanged.
