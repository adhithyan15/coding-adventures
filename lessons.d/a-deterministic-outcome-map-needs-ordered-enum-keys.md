---
category: Rust
---

# A deterministic outcome map needs ordered enum keys

A conformance test switched branch counts from `HashMap` to `BTreeMap` so printed evidence would
have stable order. Compilation then failed because the outcome enum derived equality and hashing but
not ordering. When a test uses `BTreeMap<Outcome, Count>`, derive `Ord` and `PartialOrd` for that
closed enum, or count into a fixed array with an explicit index.
