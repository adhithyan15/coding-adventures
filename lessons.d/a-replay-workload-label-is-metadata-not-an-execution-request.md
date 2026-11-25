---
category: Testing & coverage
---

# A replay workload label is metadata, not an execution request

The report schema carries a workload identifier, while a collector or CLI knows
which workload it actually runs. Replay accepts a label only if it contains a
non-NUL character outside ASCII whitespace (U+0020 and U+0009 through U+000D).
Do not use a language's Unicode-aware `isBlank`/`IsNullOrWhiteSpace` predicate:
it rejects labels containing valid Unicode whitespace such as U+2003. Replay
should validate the measurements without rejecting a valid report just because
another producer labels its measured workload differently. Never execute
callbacks or dispatch code from a label during replay. Keep the local CLI's
workload choice explicit and test generic labels, U+2003, all six ASCII
whitespace characters, NUL, null, and non-string values.
