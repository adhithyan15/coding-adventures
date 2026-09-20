---
category: Security boundaries
---

# TypeScript readonly limit interfaces do not validate or freeze runtime configuration

TypeScript's `readonly` fields disappear at runtime: callers can still pass
`NaN`, infinity, fractions, negative values, or structurally compatible mutable
objects. Comparisons against `NaN` are false, so unchecked limit objects can
silently disable resource ceilings, and retained objects can be mutated after a
cursor is constructed. Security-sensitive public boundaries must validate the
runtime type and domain of every configured bound, copy the validated values,
read each property exactly once so getters cannot change it after validation,
and freeze or otherwise encapsulate the snapshot. A cursor over a resizable
backing buffer must also capture a fixed input extent at construction. Tests
must cover non-finite values, unstable accessors, resizable buffers, shadowed
typed-array extent and slicing properties, and post-construction caller
mutation, not only well-typed source code. Use trusted typed-array intrinsics
when establishing and projecting a hostile byte-container boundary.
