---
category: Rust
---

# A term-depth limit must also bound the levels inside Sort and Const nodes

lean-kernel slice 2 bounded the nesting of `Expr` terms, but a `Sort l` or `Const.{ls}`
node carries universe levels that nest too, with their own recursive code
(`normalize`, `max_leaves`, `instantiate`, `Display`, derived `PartialEq`/`Ord`). A
`Sort (max (max (... v) v) v)` a million deep passed the term-depth check and overflowed
the stack, even inside `ExprStore` hash-consing; `succ^70000 (max u v)` hit an `expect`
in `Level::offset` (the 2^16 cap) and an unchecked `off + k`; and a 100-node DAG of
shared `max`es denotes 2^100 paths, so un-memoised walks never finish.

Fix: cache depth and tree size in every level node, reject any `Sort`/`Const` level over
128 deep or 512 nodes in the checker's single `bounded` gate, make level equality,
ordering, instantiation and `is_never_zero` iterative with visited sets, and make
`normalize` return `Option` internally so no offset can panic. Tests in
`tests/level_limits.rs` run on a 64 MiB stack so a pass means "error returned".

Rule: when you bound a recursive structure, list every *other* recursive structure
hanging off its nodes and bound those in the same gate. Count tree size, not just node
count, for anything that is shared (a DAG).
