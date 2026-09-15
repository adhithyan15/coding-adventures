---
category: Cryptography & security review
---

# Decompressors must cap declared output sizes

from untrusted headers BEFORE allocation. Expose an override for trusted callers; fail closed by default.
