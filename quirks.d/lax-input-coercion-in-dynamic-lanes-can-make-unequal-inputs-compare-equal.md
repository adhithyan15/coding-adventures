---
package: */ct-compare
lane: typescript python csharp
rung: E0
status: mitigated
---

# Lax input coercion in dynamic lanes can make unequal inputs compare equal

A byte-comparison API that *converts* its inputs is only as strong as the
conversion. Standard-library conversions in dynamic languages silently rewrite
values (wrapping out-of-range numbers, mapping non-numbers to zero, treating an
integer as a length, treating a null array as empty), so two different inputs
can become the same bytes. Subclasses, proxies and look-alike objects can also
misreport their contents.

**Mitigated** (adhithyan15/coding-adventures#18780, merged) for the TypeScript, Python and C# `ct-compare` lanes: inputs are
validated rather than coerced, only genuine built-in byte types are accepted,
and each input is read once into a fresh copy. The general lesson for the
property engine: generators must produce values that **violate** the declared
types, or they never reach this class of bug.
