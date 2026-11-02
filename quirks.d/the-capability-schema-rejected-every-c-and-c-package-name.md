---
package: *
lane: *
rung: E0
status: mitigated
---

# The capability schema rejected every C and C++ package name

The `package` pattern in `required_capabilities.schema.json` omitted `c` and
`cpp`, so all 16 C and C++ manifests were invalid. Nothing noticed, because no
test validated manifests against the full schema: 593 tracked manifests turned
out not to validate, most in a legacy string-list shape.

**Mitigated:** the pattern accepts `c/` and `cpp/`, and a shrink-only,
hash-frozen ledger
(`code/specs/data/required-capabilities-v1-invalid-ledger.json`) with a test
makes every new manifest validate and stops a listed one changing until it is
fixed.
