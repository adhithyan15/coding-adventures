---
category: Python
---

# Reject negative indexes explicitly

Reject negative indexes explicitly in bytecode/constant-pool decoders, and in
pool/table-indexed decoders generally. Python sequence indexing accepts
negatives as offsets-from-end, so `IndexError` alone won't catch a malformed
`operand=-1` — the read succeeds and silently returns the wrong entry.

Recorded twice in the pre-sharding `lessons.md`, once under Python and once
under Cryptography & security review; merged here when the split surfaced the
duplicate. It is worth knowing in both contexts: it is a Python indexing
behaviour, and it reaches you as a decoder security bug.
