---
category: Compiler / VM / language pipeline
---

# A join register's type is the consensus of every writer, including synthetic ones

**What went wrong (#18448).** In `twig-ir-compiler`, `if`, `and`, `or` and
`match` each `mov` one value per path into a single temporary register.
`emit_move` recorded the source's type on the destination at *every* `mov`,
so the temporary was typed with whichever path was compiled last.
`(+ (or (< 1 2) 5) 1)` became `add [i64]` on `#t`, and
`(let ((s (or (< 1 2) "abc"))) (string-length s))` became `str_len` on `#t`.
The native backends trust those hints, so this was memory-unsafe. Only
`compile_if` computed a consensus, by hand.

The writers that are easiest to forget are the ones with no source
expression of their own: `and`'s `#f` short-circuit path, `or`'s truthy test
value, and the `nil` a `match` holds when no arm matches.

**Fix.** A `JoinType` fold counts every writer's type; `settle_join` keeps
the type only if all writers agree on a concrete one, otherwise it clears it.
All four joins use the same helper.

**Do differently.**

- Any register with more than one writer needs a meet over *all* its writers,
  not "the last one wins". Keep that logic in one helper so a new join form
  cannot skip it.
- A soundness check that looks only at typed ops' direct operands misses
  type facts that flow through a `mov` or a function `ret`. In
  `(let ((s (or …))) (string-length s))`, `s` has a single consistent writer,
  `mov s ← _or [str]`. Treat a typed `mov` or `ret` as a claim about its own
  operand and check it too (`assert_typed_ops_sound` does this now).
