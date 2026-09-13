---
category: Cryptography & security review
---

# Security fixes that change error messages break tests

After unifying messages (e.g., generic "Invalid PKCS#7 padding"), `grep -r "old message" */test* */t/` for stale assertions.
