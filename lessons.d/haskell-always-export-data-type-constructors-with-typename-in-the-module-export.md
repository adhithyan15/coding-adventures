---
category: Compiler / VM / language pipeline
---

# Haskell: always export data type constructors with `TypeName(..)` in the module export list

`TypeName` alone exports the TYPE but not its constructors — `LitInt`/`LitText`/etc. from `LiteralVal` become invisible to callers including the package's own test suite. Symptom: `Data constructor not in scope: LitInt :: t0 -> SqlPlanner.LiteralVal`. On Windows CI tests silently pass as "skipped" (cabal not found), masking the error. Fix: `LiteralVal(..)` in the export list. Applies to every custom ADT whose constructors are referenced by callers or tests.
