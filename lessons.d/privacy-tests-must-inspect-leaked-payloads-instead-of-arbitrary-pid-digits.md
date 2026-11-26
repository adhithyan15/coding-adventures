---
category: Testing & coverage
---

# Privacy tests must inspect leaked payloads instead of arbitrary PID digits

An stderr privacy test rejected a serialized sidecar whenever it contained the
fake strace PID digits. A valid opaque SHA-256 binding ID also contained those
digits, so the test failed even though stderr stayed out of the sidecar.

Reject duplicate JSON members before decoding into a map, comparing names
after escape decoding so an unsafe value cannot be replaced by a safe alias.
Keep whole-sidecar checks for secrets, PID text and raw stderr, exempting only
valid SHA-256 values in the schema's digest fields. Include valid digest
collisions as positive controls, along with top-level and nested duplicates,
escaped payloads, unknown fields and malformed digest values.

