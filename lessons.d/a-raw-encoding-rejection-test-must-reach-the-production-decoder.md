---
category: Testing & coverage
---

# A raw encoding rejection test must reach the production decoder

Merging Unicode probes into the coverage harness retained a source-first call
after the existing mutation helper had changed to needle/replacement/source
order. That mismatch would fail in the helper before exercising production.

Use labeled arguments for raw token replacement and require the source to be the
complete encoded report. Keep the existing object-shape check for schema mutants,
but do not decode intentionally malformed Unicode in a test helper: that rejection
must come from the production decoder. Positive Unicode controls and full-report
preservation make these boundaries observable instead of merely counting errors.
