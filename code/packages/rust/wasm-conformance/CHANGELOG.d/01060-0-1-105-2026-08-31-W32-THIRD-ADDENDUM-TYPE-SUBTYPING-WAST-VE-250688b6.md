## 0.1.105 — 2026-08-31 — W32 third addendum: type-subtyping.wast vendored, 257/257 file coverage

Investigated `type-subtyping.wast` per
`code/specs/W32-wasm-non-null-concrete-reference-types.md`'s own "what's
still open" item, re-checking (per this session's own track record) the
prior deferral rather than assuming it still holds. **Confirmed genuinely
open, not tractable as a bounded slice**: every real module in the file
uses `(type $t (sub [final] $parent* (struct|array|func ...)))` explicit
nominal-subtype declarations and/or `(rec ...)` recursive type groups —
GC-proposal constructs, not function-references ones. Cross-checked
against `WebAssembly/function-references`'s own `Overview.md`: that
proposal's real subtyping rule for function types is "invariant for
now" (`$t <: $t'` iff the two are literally equivalent) — the pointwise
contravariant/covariant rule is explicitly listed there as a *possible
future extension*, not something `call_ref`/`return_call_ref` (the W32
second slice's successful vendor) ever needed. `type-subtyping.wast` is
what actually exercises that extension, PLUS struct field width/depth
subtyping, array covariance, and — the genuinely hard part — recursive
type-group canonical equivalence for cross-module import/linking checks
(`assert_unlinkable`'s "incompatible import type" cases) and dynamic
`ref.test`/`ref.cast`/`call_indirect` checks. None of `(sub ...)`,
`final`, or `(rec ...)` parse at all in `wasm-wast-parser` today (grep
confirmed zero matches before this change). See
`code/specs/W33-wasm-gc-recursive-type-subtyping.md` (new) for the full
scoping of what a real implementation needs, citing this file's own
cases.

Rather than force an implementation, vendored the file AS-IS: this
crate's own established `assert_invalid`/`assert_unlinkable` grading
(rejection-by-any-reason counts, no message matching — the same rule
`struct.wast`/`array.wast`/`type-rec.wast` are already vendored under)
already produces an honest, if low, per-directive breakdown, since every
module using unsupported syntax fails to parse cleanly (a real
`NotYetSupported`/rejection, never a panic or a hang). Corpus: **257
files** (every file in the pinned corpus now has a fixture; previously
256). `type-subtyping.wast`'s own real numbers:

- `module` 0/46 (0%, all honestly `not_yet_supported`)
- `assert_invalid` 36/36 (100% — genuinely rejected, since every one of
  these modules also fails to parse for real, unsupported-syntax
  reasons)
- `assert_unlinkable` 8/8 (100%, same reason)
- `register`/`assert_return`/`assert_trap`: all `not_yet_supported`
  (0/11, 0/17, 0/12)

While investigating this file's own two initially-suspicious module
`Pass`es (an honesty check per this session's own precedent for
not trusting a `Pass` without understanding why), found and fixed a
real, general parser bug in `wasm-wast-parser` (see that crate's own
0.1.88 changelog entry): `(rec ...)` top-level module fields were
silently ignored rather than rejected, letting a module made ENTIRELY of
unreferenced `rec`-declared types "pass" as trivially empty. Fixing that
flipped this file's 2 false module passes to honest failures (0/46 net,
not 2/46) and, as a corpus-wide side effect, corrected the same latent
issue in `type-canon.wast` and `type-rec.wast` (see their own diffs
below) — `array.wast`/`struct.wast`/`tag.wast`/`type-equivalence.wast`
also use `rec` but were unaffected (their `rec`-declared types are
always referenced elsewhere, so they were already correctly rejected via
"unknown type identifier" before this fix).

Diffed the regenerated baseline programmatically against the pre-change
one: **zero regressions** — total `fail` count across the whole corpus
is unchanged (198 → 198). Three pre-existing files besides the new one
changed, every change an improvement or an honest reclassification:
`type-canon.wast` (`module` 2/2 → 0/2, false passes fixed),
`type-rec.wast` (`module` 1/11 → 0/11 false pass fixed; `assert_invalid`
8/10 → 9/10, a reclassification from `not_yet_supported` to a real
pass), `annotations.wast` (`assert_malformed` 62/64 → 63/64, a
quote-text `rec`-containing module now correctly rejected).

Re-investigated `extern.wast` per the same ask (lower priority): the W32
second slice's declarative element segment support (`(elem declare func
...)`) does NOT close its gap — confirmed by re-fetching the file fresh
and tracing it line by line. It needs, independently: `(type $t
(struct))`/`(type $t (array i8))` text declarations (same gap as
`struct.wast`/`array.wast`, unrelated to this file), `struct.new_default`/
`array.new_default` instructions (not implemented at all), `ref.i31`
(not implemented), `any.convert_extern`/`extern.convert_any` conversion
opcodes (not implemented, as already tracked), and a `ref.host N` script
literal (not implemented — only `ref.extern N` exists today). The last
one is fatal for vendoring as-is: `ref.host` makes the WHOLE SCRIPT fail
to parse (`wasm_wast_parser::script::parse_script` errors before any
per-directive grading can even start), unlike every other gap above
which degrades gracefully to a per-directive `NotYetSupported`. Not
vendored — vendoring it today would introduce this corpus's first
whole-file parse failure, a strictly worse and less honest signal than
simply not vendoring it. Status noted here for a future session; no
spec changes needed since W32's own "explicitly out of scope" section
already tracked every one of these gaps correctly.

