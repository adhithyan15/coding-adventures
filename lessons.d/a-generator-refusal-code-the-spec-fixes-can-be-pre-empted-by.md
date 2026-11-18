---
category: Rust
---

# A generator refusal code the spec fixes can be pre-empted by an earlier checker that refuses the same document under its own code


The warrant generator runs the S2 checker before it lowers, and the checker enforces the same `match`
rules (non-exhaustive, redundant arm, shadowing bind) as the oracle's `corpus` errors. A document the
spec says the generator refuses with code `ir` therefore arrived as code `check`, and the oracle's
committed refusals dump (which says `ir`) could not have passed. Nothing failed in lowering: the
pre-empting stage simply answered first.

Fix: when a spec fixes the code of a refusal, write the test from the committed dump first and look at
which stage answers. Map exactly the checker kinds that are the spec's corpus errors (and only those,
located at a match arm) to the spec's code, and re-lower without the checker to name the precise error
in the oracle's order. Keep the checker's other refusals as `check`.
