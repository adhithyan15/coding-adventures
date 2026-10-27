---
category: Rust
---

# DFF-backed simulators should commit candidates only after validation

Project stable Q into an ephemeral candidate, perform decode and execution on
that candidate, and clock the complete result back only after every check and
allocation succeeds. This keeps faults, bounded runs, DMA, interrupts, and
injected allocation failures atomic without making a host-state mirror an
alternative authority. Derive topology counts from the owned storage
containers, and compile hostile full-crate mutations before asking the
structural validator to reject their bypasses.
