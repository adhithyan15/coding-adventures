---
category: Rust
---

# Enumerate backend option fields when the resolver type changes

The Timeline minima test candidate used `..defaults` after replacing a
FakeResolver with a recording Resolver. The options type is generic over that
backend, so static review withheld the candidate for a type-changing update
risk. No compiler failure was observed and no semantic red was claimed.

Enumerate the seven retained option fields, as the existing product controls
do, when changing a backend's concrete type. Preserve the refused candidate
and obtain review of the corrected complete body before execution.
