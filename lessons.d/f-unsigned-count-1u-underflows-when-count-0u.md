---
category: Cryptography & security review
---

# F# unsigned `count - 1u` underflows when `count = 0u`

Always guard before writing `0u .. count - 1u`. Same: cap header counts to remaining-payload bytes before looping.
