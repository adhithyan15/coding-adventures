## 0.1.108 — 2026-08-31 — W33 second slice: real `ref.cast`/`ref.test`/`call_indirect` dynamic dispatch (item 4)

Regenerated the baseline after `wasm-wast-parser` 0.1.90 (`ref.test`/
`ref.cast` TEXT-format parsing), `wasm-execution` 0.9.81 (real dynamic
dispatch for both plus a fixed `call_indirect` subtype check), `wasm-
validator` 0.2.78 (a `ref.cast` byte-layout fix), and `wasm-runtime` 0.6.18
(threading the new subtyping info through) — see each crate's own
changelog for its half of the implementation, and `code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s second addendum for the design.

Diffed programmatically against the pre-change baseline for every one of
the 257 vendored files, not just `type-subtyping.wast` — **exactly ONE
file changed, zero unexplained regressions, zero new `fail`/`trap`
anywhere in the other 256 files.**

- **`type-subtyping.wast`** (the only change):
  - `module`: 8/46 → **12/46** (+4 real passes; 34 still `not_yet_
    supported` — confirmed via a direct parse-error probe, ALL 34 are
    blocked purely on this crate's still-fully-open struct/array
    TEXT-format type-declaration gap (W33's own "Recommended scope" step
    2, unrelated to this slice), NOT on anything this slice attempted:
    every one of the "Runtime types" section's `rec`-group modules past
    line 401 (M3-M11) interleaves a `func` type and an anonymous `struct`
    type in the SAME `rec` group, and `(type (struct ...))` still isn't a
    parseable type body.
  - `assert_return`: 7/17 → **10/17** (+3; the "Runtime types" section's
    `call_indirect`/`ref.cast` module — lines 283-343 — now fully
    reachable and correct).
  - `assert_trap`: 2 fail + 10 `not_yet_supported` (0/12 real passes) →
    **12/12, zero fail, zero not_yet_supported** — every single-module
    `call_indirect`/`ref.cast` trap case in the "Runtime types" section
    (lines 337-400) now passes, including the 2 pre-existing fails the
    W33 first-slice addendum traced to the (until now) equality-only
    `call_indirect` check.
  - `assert_unlinkable`: unchanged, 6/8 (the 2 remaining fails are the
    genuine M10/M11 `rec`-group topology mismatch, confirmed still
    requiring item 3b — cross-module canonical equivalence, still
    explicitly out of scope; see the spec's own accounting, unchanged by
    this slice).
  - `assert_invalid`: unchanged, 36/36 (already fully passing).

No other file's tally moved by even one directive — verified via a
programmatic before/after diff over the full `testsuite-status.json`, not
spot-checked.

