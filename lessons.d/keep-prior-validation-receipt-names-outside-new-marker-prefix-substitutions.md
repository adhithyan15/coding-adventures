---
category: Testing & coverage
---

# Keep prior validation receipt names outside new marker-prefix substitutions

A runner derivation replaced a marker prefix after updating its baseline path.
The broad replacement also renamed that historical receipt to a nonexistent
successor filename. The run retained an actual exception receipt and stopped
before Cargo. Preserve that failed runner and receipt; derive a new version
that restores the exact historical path and hash. Limit future marker edits
to the marker expressions, or independently verify every dependency path and
hash before launching native validation. A derivation failure is not semantic
red or native acceptance.
