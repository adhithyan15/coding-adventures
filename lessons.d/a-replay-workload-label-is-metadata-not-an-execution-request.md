---
category: Testing & coverage
---

# A replay workload label is metadata, not an execution request

The report schema carries a workload identifier, while a collector or CLI knows
which workload it actually runs. Replay should validate the report's measurements
and require a nonblank, NUL-free identifier, but it should not reject a valid report just
because another producer labels its measured workload differently. Never execute
callbacks or dispatch code from a label during replay. Keep the local CLI's
workload choice explicit and test generic labels with valid reports plus blank-label
rejection controls.
