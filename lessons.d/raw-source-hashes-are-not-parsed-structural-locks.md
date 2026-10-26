---
category: Testing & coverage
---

# Raw source hashes are not parsed structural locks

A whole-file hash over raw source bytes is not a parsed authority lock. It
rejects harmless comments and formatting, while also allowing every hostile
mutation to fail through the same blanket fingerprint instead of proving which
architectural route was protected.

For source-integrity tests, parse first. Normalize the syntax token stream so
layout and comments disappear, then lock the complete parsed source and the
named function bodies that carry each authority route. Compiler-valid hostile
fixtures should assert the specific parsed-body diagnostic they trigger. Add a
positive fixture containing comments, strings, and character literals that
look like forbidden code to prove textual lookalikes cannot satisfy or trip the
structural checks.
