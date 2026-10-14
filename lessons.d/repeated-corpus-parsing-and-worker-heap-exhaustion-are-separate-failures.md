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
reports. Five real-corpus groups across four figure-target case shards retain
independent full corpus snapshots and resolve the same filmstrip targets.
Share immutable measurements within each suite while preserving its assertions
and keeping synthetic fixtures independent. Reproduce and validate the worker
failure under the same 2 GiB heap budget before calling the repair complete.
Keep ordinary test deadlines and heap limits unchanged.

The pre-repair focused run with `NODE_OPTIONS=--max-old-space-size=2048`
reproduced the figure worker crash after 124 seconds and all four chapter
snapshot timeouts (99.6, 55.1, 54.7 and 45.8 seconds). This resource limit is a
reproduction constraint, not an increased allowance or a test-timeout override.

After sharing those immutable fixtures, the identical focused command passed
all 173 tests in 80.95 seconds, with assertions taking 2.50 seconds in total.
The 2 GiB heap cap, two-worker configuration and ordinary 30-second deadlines
were unchanged. The figure collections expose read-only arrays, maps and sets;
synthetic lessons remain independently mutable.
