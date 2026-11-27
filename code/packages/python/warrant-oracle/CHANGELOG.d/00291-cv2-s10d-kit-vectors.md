## Unreleased - CV2 S10d: the kit vectors at `ir.fixed` and `ir.tuple` (VER04r section 13; [D70], one PR with the regenerated kit)

Adds 670 cases to the v2 conformance kit (958 to 1,628 v2 cases; 2,089 with the four v1 categories). Only cases are added: the 958
existing ones are byte for byte unchanged (`tests/test_s10d_vectors.py` pins a digest of their canonical documents per category).

- **`v2-value` +49.** Tuple values (stream tag `64`, arity 2 to 8, nesting, Bytes and Result components), the `value` and `type` refusals of a tuple,
  16 and 17 nested tuples.
- **`v2-ir` +469.** Every `ir.fixed` and `ir.tuple` tag of Appendix A.1 with its edges (the count compared with `w` before any shift, the
  floor and truncating `div`/`rem`, the static width of `from_be`/`from_le`, `unwrap_or` evaluating both operands) and its refusals; tuples in
  models and at the op boundary; `match` on a `Result` with every pattern and only the taken arm counted; the vectors of R76 (a `match` on a bare
  tuple or a `Bool` is `level-missing`), R100 (a stand-in with an `ir.fixed` tag under an `ir.core` tree: `requires` `["ir.core","ir.fixed"]`),
  R111 (d) (a model with an `Int` parameter, [D75]) and R120 (d) (an `env` type the reader refuses).
- **`v2-domain` +57.** The `tuple` descriptor (last component fastest, nesting to 16 levels) and `nat_shift` (73 values).
- **`v2-chunk` +23.** A fourth kit document, `kit-tuple.warrant` (six suites with tuple binders, `warrant-stream-v2`), and the ops and models of a
  chunk corpus part (R125).
- **`v2-fault` +72.** The `ir.fixed` and `ir.tuple` templates, `off-by-one-consumed` on a `Result<(Bytes,Nat),E>` (R96), `missing-error` with a tuple
  payload and no `Bytes` parameter (R67), `level-missing` at each missing level, `ill-typed` for R96's second vector, R98 and R121, and the
  `unknown-tag`, `depth` and `model-order` of R119 (e). Every code of every closed set of the kit is now used.
- **`v2-arith` +0.** Its closed `op` set (`add sub mul neg` and the six comparisons) is `ir.core` only.
- **The oracle.** `Row.requires` (a row of S10d states the `requires` it expects and `finish_case` fails the build if `requires_of` computes
  another; an S5 row still has to be `["ir.core"]`, R24); `_rewrite_subjects` rewrites a subject call inside a `match` arm to its stand-in (it
  left it unrewritten, so no case with a subject call in an arm could be derived); `make_conformance.py` derives `kit-tuple.warrant`.
- **Tests.** `tests/test_s10d_vectors.py` (counts per category, the exact allowlist of the 57 cases whose `requires` is `["ir.core"]`, every
  vector of the row, six mutants); `test_conformance_cross.py` re-derives the tuple streams with `struct.pack` and the six tuple suites with a naive
  enumerator; `test_conformance_tables.py` scopes its S5 assertions to the S5 rows.
- **Lanes.** Both lanes run the 57 cases that need `ir.core` alone and skip the other 613 by `requires` (839 run, 0 failed in each). Their kit
  tests' count assertions moved: the Python lane's `test_v2_conformance.py` (782 to 839 run, the skipped counts) and the TypeScript lane's `kit.test.ts` and
  `review-fixes.test.ts` (782 to 839 run, the skipped counts). No lane source moved.
- **Review fixes (same PR).** `requires_of` now levels the binder `type` of a `v2-domain` case as it does a `v2-value` type, so
  `descriptor-scalar-for-a-tuple-type` is `["ir.core","ir.tuple"]` (no S5 case moves); the hand-written op rows of the tables carry the `requires`
  their signature has (`ir.tuple` for a tuple type); `level-missing-tuple-op-signature-at-the-fixed-level` has no tuple tag and a stand-in with the
  op's signature; three cases dropped (`ill-typed-match-err-case-no-error-type-declares`, whose arm is also unreachable, `int-model-parameter-with-an-int-result`,
  an R112 question, and the two record-`types` cases beyond R121 (c) and R125); `int-model-parameter-with-a-nat-operand` no longer carries an unused
  env entry; added 199 `ir.fixed` vectors at 16, 32 and 64 bits and 6 `match` cost and overflow-class vectors.
