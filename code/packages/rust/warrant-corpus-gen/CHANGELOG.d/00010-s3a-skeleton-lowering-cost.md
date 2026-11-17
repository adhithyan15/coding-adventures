# 0.1.0 (VER04r CV2-S3a): crate skeleton, `ir.core` lowering, cost accounting

New leaf crate `coding_adventures_warrant_corpus_gen`, the first of four S3 pull requests
(S3b evaluator, enumeration and the 18 digests; S3c faults, emission and `--check`; S3d the
committed ct-compare fixture). Registered in the `code/packages/rust` workspace.

## Added

- `json`: the VER04i 9.1 canonical JSON writer (sorted keys, two-space indent, LF, lowercase
  `\uXXXX` escapes, duplicate keys refused), hand-written, no `serde`.
- `profile`: the strict `corpus.profile` reader of VER04r 8.4, mirroring the oracle's
  `profile.py`; carries `profile_sha256`.
- `types`, `ir`: the `ir.core` types and the 26 tags of Appendix A with their JSON form.
- `lower`: syntax-directed lowering of a checked document with an annotating typer
  (refinements fold left into binary `and`, a bare literal is `Nat` in a `Nat` context and
  `Int` otherwise, `-128i8` keeps `neg` of a literal, type aliases resolve away).
  Constructs beyond `ir.core` refuse the whole document with the level named (`ir.fixed`,
  `ir.data`, `ir.seq`, `inout`, `ensures`, `vectors`, `assume`); `census` reports per item.
- `analysis`: the pure-guard rule (VER04r 4.4) and the v0 static arithmetic guard
  (3.3 rule 3), both `W0701`.
- `arith`: the single wrapping rule, `wrapped` flags for `wraps`, the 2^127 exact window,
  `MAX_IR_VALUE`, the v0 guard rule 2.
- `cost`: worst-path `ir_cost` with early-stop checked arithmetic, per-model costs in
  declaration order with call depth, `max_ir_cost`, suite `ir_cost`, total IR work bounds.
- `view` and `tests/fixtures/dump_oracle_lowering.py`: the oracle parity view; four
  committed oracle snapshots (ct-compare, gf256, two synthetic documents) and the script
  that regenerates or checks them.
- `generate`: a stub that fails closed (`unimplemented`) after the S3a checks.

## Verified

- Lowered ct-compare, gf256 and two synthetic documents are byte-identical to the oracle's.
- gf256 `gmul` costs 2,011, `ginv` 26,204, four of 38 suites exceed 2,000, largest
  expression 26,210 (VER04r 4.2).
- ct-compare's IR equals the committed v1 corpus (`admit`, `lhs`, `rhs`, models, ops).
