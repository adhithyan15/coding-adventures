---
category: Testing & coverage
---

# Resolve HEAD before invoking a validator that freezes an exact commit identity

The first Timeline size-red invocation supplied the symbolic word HEAD to a
validator whose contract requires the resolved full SHA. Its initial identity
assertion refused before starting Cargo, creating streams or writing a receipt.
This was a caller preflight failure, not a compiled semantic red or a test result.

Preserve the failed invocation and actual exit separately. Resolve git rev-parse
HEAD before constructing the validator arguments, pass that full identity, and
consume the real run before claiming red or green. Do not relax the validator's
identity guard to accommodate a mutable symbolic reference.
