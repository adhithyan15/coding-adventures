---
category: Security boundaries
---

# Tagged serde unit variants still need exact closed fields checked before conversion

The real checked importer accepted `{"kind":"converged","changed":false}`
despite deny_unknown_fields: serde's internally tagged unit variant discarded
the otherwise known field. Check the decoded map's exact per-variant keys in
the bounded parser before conversion. Apply this to operations, scopes and
outcomes, including missing fields and cross-kind claims. Detect duplicates
before reading their values.
