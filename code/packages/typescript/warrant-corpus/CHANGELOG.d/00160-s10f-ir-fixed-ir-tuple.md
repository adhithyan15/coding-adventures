## Unreleased - CV2 S10f: `ir.fixed` and `ir.tuple` in the v2 runner library

This lane now implements three levels, `ir.core`, `ir.fixed` and `ir.tuple`
(`IMPLEMENTED_LEVELS`), and runs every case of the six `v2-*` kit categories
whose `requires` is inside them: **1,452 of 1,452, 0 failed** (839 before, plus
the 613 that S10d added and this lane used to skip by `requires`). The 176 cases
of `v2-binding` are still skipped whole until S7b. `ir.data`, `ir.list` and
`ir.seq` stay unimplemented and are `level-missing` naming the level.

- `src/v2/fixed.ts` (new): the 35 `ir.fixed` tags, one function each, over the
  host forms of `values.ts` (a `number` up to 32 bits, a `bigint` for 64). Every
  operation acts on the `w`-bit pattern and re-reads it in its own type; counts
  are exact `Nat`s compared with the width before any host shift; `Option` is
  `undefined` for `None` and the value for `Some`.
- `src/v2/ir.ts`: typing, cost, evaluation and the pure trace of the 35 tags,
  `tuple`, `project` and `match` on a `Result`. `Option` is an intermediate only
  (its one consumer is `unwrap_or` as the first operand, directly); anywhere else
  it is `level-missing` naming `ir.data`, and a model whose body needs that is
  not live. `match` is typed before it is accepted (R76), runs a usefulness check
  (a match that can fall through, an arm no value reaches, a repeated or
  shadowing `bind` are `ill-typed`), binds into numbered frame slots, classifies
  an arm's overflow by its scrutinee as a guard (R71) and counts the subject
  calls of the arm that runs. A static type argument counts for the level (R69
  (b)); a data pattern needs `ir.data` at the reach step.
- `src/v2/types.ts`: `faultCommon`, the common type of a fault row (R94, R96,
  R97: `Nat` and `Int` widen and `Bytes` widths do not under a tuple) and
  `hasEquality`.
- `src/v2/faults.ts`: a fault row is typed with its declared type handed down
  (R67, R98): every branch, arm, `ok` payload and tuple component is compared,
  and a bytes literal where plain `Bytes` is wanted is typed `Bytes`.
- `src/v2/values.ts`: tuples in JSON, TFV (`L<n>`), equality and the stream
  (`64`, u32-LE count, components). `src/v2/domains.ts`: the `tuple`
  descriptor (1 to 32 entries, the exact arity, checked product, last component
  fastest). `src/v2/chunks.ts`: `expected` needs a common type with
  `result_type`, not a flow (the committed wasm-leb128 suites compare a `Nat`
  result with an `Int`-typed `imax`, R42).
- Tests: `fixed.test.ts` checks every tag against an exact-integer reference
  (every pair of every 8-bit type, the edge values of the wider ones), and
  `tuple.test.ts` the tuple, `match` and fault rules; 13 new falsification
  mutants; `kit.test.ts` and `review-fixes.test.ts` pin the new run count (1,452)
  and skipped count (0); `corpus.test.ts` reads the four committed corpora at
  `ir.core` alone and at all three levels, where nothing is not-live and every
  chunk of every suite (46.6 million cases of wasm-leb128, 17.6 million of lzw)
  reproduces the digests the oracle committed.
- Known and not done here: the lane's pre-flight `checkWork` meter counts every
  `ok` control as re-running the whole slim set of each required claim, which with
  all of wasm-leb128 live is 37,782,980 ordinals against 32,000,000
  (section 5.6 has a control stop at its first failure; a precise meter is the
  corpus runner's, S7b). The `not_live` of the `eq_u64` control in
  `typescript/ct-compare` needs the v2 corpus reader of S7b and S7c and is
  deferred to them.
