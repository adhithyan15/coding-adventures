---
category: Cryptography & security review
---

# Count protocol registry rows from the source table rather than a prose estimate

While translating RFC 8446 Table 1 into an extension-context data table, the
specification called it a 21-row registry even though the source table has 22
rows. The typed array length exposed the mismatch before tests or staging. The
fix was to count and transcribe the source rows into one explicit test oracle,
then make the production array length and prose agree with that oracle. For
future protocol registries, derive the claimed count from the transcribed data
and compare every identifier and policy value against the source table; do not
hand-count a long standards table while drafting prose.
