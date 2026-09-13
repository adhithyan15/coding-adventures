---
category: Cryptography & security review
---

# Typed import boundaries: `Some("")` ≠ epsilon

If the runtime uses an empty-string sentinel internally for epsilon transitions, the typed contract uses `None` — reject `Some("")` at imports so malformed defs can't smuggle free moves past the validator.
