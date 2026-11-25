---
category: Rust
---

# A canonical emitter's byte identity with a fixture is a statement about call order, not just structure

`pif-writer` numbers PIF records in creation order with maximal sharing, so its output is a
pure function of the *sequence* of builder calls, not of the term being built. The first
rebuild of the VER32 4.6 worked example differed from the fixture by one line: the binder name
`b` was created before the constants `Bool.rec`, `Bool.false` and `Bool.true`, while the
fixture's reference script created it after them. The two files are the same term; only the
record order (and so every later index that follows it) differed.

Rule: when a test compares an order-sensitive emitter with a committed fixture, read the
fixture's creation order off the file and make the calls in that order; do not "fix" the
writer. Document in the README that canonical here means "creation order", so a producer that
wants a different layout knows it gets different, equally valid, bytes. Also validate every
component of a multi-part input (a dotted name) before emitting the first record, so a refused
call leaves no half-built prefix in the output.
