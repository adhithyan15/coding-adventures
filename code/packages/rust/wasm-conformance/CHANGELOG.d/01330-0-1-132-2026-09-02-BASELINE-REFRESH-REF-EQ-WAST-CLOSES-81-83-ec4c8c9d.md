## 0.1.132 — 2026-09-02 — baseline refresh: `ref_eq.wast` closes 81/83 (W39 slice 1: `ref.eq` / `0xD3`)

`wasm-wast-parser` 0.1.106 / `wasm-validator` 0.2.92 / `wasm-execution`
0.9.98 implement `ref.eq` (per `code/specs/W39-wasm-gc-ref-eq-cast-br-on-
cast.md`, slice 1 of 5) -- see those crates' own CHANGELOGs for the full
account. This crate's own code is unchanged; only `tests/fixtures/
testsuite-status.json` moves, regenerated via `--write-baseline` and
diffed programmatically (Python, comparing the `files` dict) against the
prior baseline across all 257 files. Exactly 5 files changed, all
strictly `not_yet_supported` -> `pass`/`fail` movement with **zero new
`fail` or `trap` anywhere in the full 257-file corpus**:

| File | Before (pass/fail/trap/NYS, totals) | After |
|---|---|---|
| `ref_eq.wast` | 6/0/0/83 | 87/0/0/2 |
| `array_init_elem.wast` | 23/0/0/13 | 36/0/0/0 |
| `array_new_elem.wast` | 22/0/0/2 | 24/0/0/0 |
| `table_init.wast` | 790/0/0/2 | 792/0/0/0 |
| `table_init64.wast` | 886/0/0/2 | 888/0/0/0 |

Total NYS reduction across the 5 changed files: 100 directives (81 in
`ref_eq.wast` itself; the other 19 across the other four files, all of
which use `ref.eq` as a general-purpose funcref-equality helper in their
OWN fixture bodies -- e.g. `table_init.wast` byte-verified at `(ref.eq
(table.get $table (i32.const 0)) (table.get $table (i32.const 1)))` --
completely unrelated to this slice's own target file, discovered by
diffing the full 257-file corpus rather than assuming only `ref_eq.wast`
could move).

**Honest accounting of the 2 remaining `ref_eq.wast` NYS** (the spec's
own prediction was a full 83 -> 0 close for this slice alone; re-
verifying against the actual corpus found that prediction was NOT
exactly right, in the same "every W-slice re-verifies its own motivating
claim and finds something off" spirit this campaign has followed since
W32): `ref_eq.wast` has 6 `assert_invalid` directives, each pairing a
`ref.eq` call against a param typed `(ref [null] any|func|extern)`,
expecting a "type mismatch" rejection. Before this slice, `ref.eq` wasn't
a recognized instruction NAME at all, so `wasm-wast-parser` rejected all
6 modules outright ("unknown instruction") -- `grade_assert_invalid`
counts any rejection, for any reason, as `Pass`, so all 6 passed by
coincidence. After this slice: 4 of the 6 (the NON-NULL `(ref any)`/`(ref
func)`/`(ref extern)` param forms) still fail to parse, for an unrelated,
pre-existing, already out-of-scope reason (`wasm-wast-parser`'s non-null
`(ref X)` branch only special-cases `i31`/`array` -- Correction 3 in the
W39 spec) -- still genuinely `Pass`. The other 2 (`(ref null func)`/`(ref
null extern)` -- the NULLABLE forms, which already parse) now reach a
real, working `ref.eq` validator arm that -- BY DESIGN (see `wasm-
validator`'s own CHANGELOG entry for why) -- doesn't check operand types
against `eqref`, so these 2 modules validate successfully and downgrade
from an accidental `Pass` to an honest `NotYetSupported`. Net: not a
regression (verified: 0 new `Fail` anywhere in the full corpus; this
file's own total pass count still rises from 6 to 87), just a more
honest count than the spec's own pre-implementation estimate.

Confirmed via `git stash` A/B: `cargo test -p wasm-types -p wasm-wast-
parser -p wasm-opcodes -p wasm-module-parser -p wasm-validator -p
wasm-execution -p wasm-runtime -p wasm-conformance` passes identically
before and after (the only count that moves is `wasm-execution`'s own
unit-test total, 613 -> 620, for 7 new direct `ref.eq` tests -- see that
crate's CHANGELOG).

