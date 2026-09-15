## 0.1.19 — 2026-08-15 — vendor simd_const.wast, the first post-MVP-proposal corpus file (task #78)

### Added

- `simd_const.wast` vendored at the same pinned commit SHA
  (`28864811cf03bdbf880733786148feaba339582d`) as the rest of the corpus,
  verbatim (confirmed byte-identical against a fresh independent fetch
  before committing). The narrowest real root-level `simd_*.wast` file --
  almost entirely tests `v128.const`'s own literal syntax across all 6
  shapes, which this repo's `wasm-wast-parser` already fully supports
  (SIMD PR1b-2/1b-3) -- and the first file this repo has ever vendored
  from a post-MVP proposal, made possible by W14's per-module graceful
  degradation (task #76): this file's few genuinely-unsupported opcodes
  (e.g. a single `i64x2.add` usage) no longer abort grading the other
  ~600 directives in the same file.

### Baseline regen: 61 files parsed, 0 failed to parse (up from 60/0)

- `simd_const.wast`: `module` 307/309 pass (99.4%, 3 not yet supported --
  genuinely-unimplemented opcodes/shapes correctly deferred), `assert_return`
  189/240 pass (78.8%, 25 not yet supported), `assert_malformed` 72/72 pass
  (100%, 109 not yet supported). Confirmed via a full JSON diff against the
  prior baseline that every OTHER file's tallies are byte-for-byte
  unchanged (zero regressions).
- **3 real, root-caused bugs surfaced by this file's real assert_return/
  module Fails and Trap — reported honestly, not hidden or miscategorized
  as `NotYetSupported`, and not fixed in this PR (each is a genuine,
  separately-scoped follow-up, logged as tasks #79/#80/#81):**
  1. **v128.const is rejected inside constant expressions**
     (`wasm-execution::evaluate_const_expr` has no `0xFD` arm at all), so
     any module declaring a `(global (mut v128) (v128.const ...))` fails
     to *instantiate* entirely (a `Trap`, cascading into ~20 collateral
     "no module registered" `Fail`s for every subsequent bare `invoke`
     against it) -- task #79.
  2. **v128 globals don't survive a round-trip across separate `invoke`
     calls** -- a `global.set` in one call followed by `global.get` in a
     later call returns the raw, now-meaningless handle instead of the
     real bytes, because `ctx.v128_heap` (what the handle indexes into)
     is scoped to a single call. Already flagged as a known gap in SIMD
     PR1b-3's own CHANGELOG; now concretely triggering real graded
     failures against real corpus data -- same root cause as #1 above
     (v128 needs persistent, not per-call, storage), folded into task #79.
  3. **A real hex-float literal rounding bug**: `parse_float_magnitude`
     accumulates a hex-float mantissa digit-by-digit via plain `f64`
     arithmetic, which double-rounds instead of computing round-to-
     nearest-even from the full-precision value -- fails on the corpus's
     own deliberately-crafted over-precision edge cases (e.g.
     `+0x1.000000000000080000000000p-600`, an exact-halfway tie-break
     case). General MVP-level bug, not SIMD-specific -- task #80.
  4. One additional, less-triaged `module` structural-validation `Fail`
     ("blocktype references type index -5") from a single `(module
     binary ...)` directive, possibly a signed-LEB128 blocktype decoding
     ambiguity -- task #81, not yet root-caused in the same depth as
     #1-3.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and
`code/specs/W14-wasm-conformance-lazy-module-build.md`.

