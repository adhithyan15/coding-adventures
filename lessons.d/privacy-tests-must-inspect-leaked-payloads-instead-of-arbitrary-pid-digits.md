---
category: Testing & coverage
---

# Privacy tests must inspect leaked payloads instead of arbitrary PID digits

An stderr privacy test rejected the serialized sidecar whenever it contained
the digits of the fake strace PID. A valid opaque SHA-256 binding identifier
also contained those digits, so the test failed even though stderr stayed out
of the sidecar.

Reject duplicate JSON members before decoding into a map; compare names after
escape decoding so two spellings of the same key cannot overwrite a leak with a
safe value. Preserve whole-sidecar checks for secret and fixture-PID text,
excluding only valid SHA-256 values in the schema's actual digest fields.
Include valid digest collisions in every such field as positive controls, plus
top-level/nested duplicate keys, partial-secret, PID-string, numeric-PID,
escaped-payload, and raw-stderr negative controls. Unknown fields and malformed
digest values do not receive the exemption.
