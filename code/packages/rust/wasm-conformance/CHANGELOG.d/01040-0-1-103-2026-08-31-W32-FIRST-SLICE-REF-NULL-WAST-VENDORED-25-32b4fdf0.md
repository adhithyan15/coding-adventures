## 0.1.103 — 2026-08-31 — W32 first slice: `ref_null.wast` vendored, 256/257

Implements `code/specs/W32-wasm-non-null-concrete-reference-types.md`'s
first slice: the four bottom reference types
(`nullfuncref`/`nullexternref`/`nullexnref`/`nullref`) as real subtypes of
their nullable hierarchy supertypes (`wasm-types` 0.1.13, `wasm-validator`
0.2.74), replacing an earlier pass's lossy aliasing (`wasm-wast-parser`
0.1.86). `ref_null.wast` re-fetched fresh from the pinned SHA, confirmed to
need only the bottom types (no non-null concrete refs, no structural
subtyping), and vendored with a genuine, fully-passing baseline:

- `module`: 2/2 (100%)
- `assert_return`: 32/32 (100%)

Zero `not_yet_supported` directives in this file. Corpus now stands at
**256/257** vendored files. Diffed the regenerated baseline
(`tests/fixtures/testsuite-status.json`) programmatically against the
pre-change one: the ONLY difference is the new `ref_null.wast` entry —
every other vendored file's per-directive-kind tallies are byte-identical,
confirming zero regressions.

Non-null concrete refs (`(ref $t)`) and structural subtyping (needed for
`type-subtyping.wast`) remain out of scope for this slice — see the
spec's own addendum.

