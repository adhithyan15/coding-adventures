---
category: Cryptography & security review
---

# Derived Debug on wrapper types can bypass redaction boundaries

A routing wrapper promised profile-only diagnostics but derived `Debug` over
fixed-verifier limit values. The nested limit types also derived `Debug`, so
formatting the wrapper recursively exposed caller-selected work ceilings even
though errors and verification evidence used custom redacted output.

When a public wrapper is part of a redaction boundary, audit every public
`Debug` implementation rather than only errors and success evidence. Prefer a
custom formatter that emits the stable profile or category, and test the
wrapper directly with distinctive nested values so a later derived formatter
cannot silently widen diagnostic disclosure.
