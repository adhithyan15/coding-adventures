---
category: Testing & coverage
---

# Mutation controls for structured fixtures must preserve valid surrounding syntax

A negative control intended to test a missing operation removed the `turns` key
and opening bracket but left the array contents in place. The JSON parser then
rejected malformed syntax, so the test did not reach the harness's fail-closed
missing-operation path. Mutate one complete field in a structurally valid
document (for example, remove a refusal's `expected_error` key) and assert the
specific harness diagnostic. For escaped-key controls, ensure the generated JSON
contains one escape introducer (`\u0079`), not a doubly escaped literal
(`\\u0079`); otherwise the test sees a distinct key rather than an alias. A
parser failure or unrelated schema mismatch is not evidence for the intended
control.
