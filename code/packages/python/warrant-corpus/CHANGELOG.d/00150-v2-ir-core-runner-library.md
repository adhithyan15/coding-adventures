## Unreleased - v2 `ir.core` runner library and conformance harness (CV2-S6a)

Slice S6a of VER04r section 13: the first part of the Python lane's v2 runner.
It reads no v2 corpus, binding or manifest yet (S6b) and `python/ct-compare` still
runs `warrant-corpus-v1` (S6c). Everything new is in the sub-package
`coding_adventures_warrant_corpus.v2`; the v1 modules are unchanged.

- **Library at `ir.core`**, written from the spec (`python/warrant-oracle` is not imported;
  see entry 00160 for what the independence claim is and is not): `wtypes` (section 2.1 types, `flows`
  and `unify`), `values` (section 2.3 value decoder for every form, section 2.4
  stream encoder, signed `11` tag, tuple `64`, variant `65`), `arith` (section 3:
  one wrapping rule, the 128-bit `Nat`/`Int` window), `ir` (section 4 and
  Appendix A: a checker for the eight static refusals, lazy `and`/`or`/`implies`/
  `if`, measured and traced call counts, worst-path cost, the two `ir-overflow`
  classes, `MAX_IR_VALUE` checked before allocation, an iterative depth scan so a
  hostile tree is refused rather than crashing), `domains` (section 5.1: every
  descriptor node, `size()` and `nth()`), `chunks` (sections 5.2 to 5.5: the chunk
  file, enumeration of one chunk alone by unranking and the sequential walk) and
  `faults` (section 8: the three wrappers, written once).
- **Conformance harness** (`v2.kit.run_kit`): runs every v2 kit case whose
  `requires` is a subset of the implemented levels and skips the rest, reporting per
  category `run`, `skipped` and `failed` (case ids). Run plus skipped equals the
  index row's count; a category the runner does not know (`v2-binding`) is
  digest-checked and skipped whole. Result on the committed kit: 734 run (782 after S5e), 0 skipped,
  0 failed (`v2-value` 128, `v2-arith` 129, `v2-ir` 221, `v2-domain` 154,
  `v2-chunk` 66, `v2-fault` 36).
- **"Chunk `c` alone equals sequential"** is checked for every `also_sequential`
  case: chunks `0..=c` are walked from ordinal 0 (never unranking), each chunk is
  compared with the committed chunk file, and chunk `c` with the same chunk
  enumerated alone. The walk over the kit's 2^24-ordinal suite is shared by its
  three cases.
- **Falsification (D42).** Every rule the spec says an implementation can get wrong
  is a method of `v2.semantics.Semantics`; the standard class is the spec. The tests
  hand the harness 16 deliberately wrong subclasses (`tests/v2_mutants.py`): no wrap,
  eager `and`, first-binder-fastest enumeration (the three the spec names), plus
  saturating arithmetic, eager `if`, a signed comparison on the bit pattern, `neg` of
  the minimum raising, an unchecked `Nat` product, `take`/`drop` that do not clamp,
  signed stream values not sign-extended, a big-endian `u32`, decimal text accepting
  a leading zero, the withdrawn unsigned-order signed `fx_edge`, `first_admitted`
  counted on product ordinals, the chunk index missing from the stream prefix, and
  sequential `pre` rewrites. Each is reported by case id (the pinned ids are
  asserted); the standard semantics passes the same cases.
- **Tests:** 429 new tests. Coverage of the new code is 99 percent (the
  package gate of 95 percent is unchanged). `pyproject.toml` selects coverage's
  `sysmon` core, because the default tracer made the 16.7-million-case sequential
  walk three times slower than the code it measures.
- **`runner.src_sha256` moves** (the runner source gained a sub-package), so the
  `python/ct-compare` v1 manifest is regenerated mechanically in the same change
  (see that package's changelog); it is not an adopter change.
- No version bump (unreleased, as the neighbouring entries).
