## 0.1.99 — 2026-08-31 — LEB128/malformed-binary hardening: 3 files vendored

Vendors `binary.wast`, `binary-leb128.wast`, `binary_leb128_64.wast` from
the pinned SHA — dense `assert_malformed` corpora specifically targeting
the binary format's LEB128 encoding rules (over-long encodings,
out-of-range values, truncated streams) and other structural
malformed-binary shapes (unrecognized/out-of-order/repeated section ids,
section-size mismatches, malformed limits flags, a function body missing
its terminating `end`). Investigating these surfaced real,
previously-invisible gaps in `wasm-module-parser` — see that crate's own
`0.2.10` changelog entry (and `wasm-leb128`'s `0.2.0`) for the full list
of fixes; this entry covers what changed HERE.

### Real, measured per-file numbers

| file | module | assert_malformed |
|---|---|---|
| `binary.wast` | 20/20 | 105/107 |
| `binary-leb128.wast` | 30/33 (+3 nys) | 51/58 |
| `binary_leb128_64.wast` | 1/1 | 1/1 |

The `assert_malformed` gaps in `binary.wast` (2) and `binary-leb128.wast`
(7) are exactly the function-body-embedded LEB128/opcode cases
`wasm-module-parser`'s own changelog documents as deliberately deferred
(needs a per-opcode instruction-level walker this crate doesn't have
yet, not a LEB128-boundary fix). `binary-leb128.wast`'s 3 `module`
`not_yet_supported` entries are unrelated pre-existing capability gaps
in that file's non-malformed modules, not something this pass touched.

### Confirmed: zero regressions elsewhere, one already-latent bug surfaced and fixed

Diffed the full regenerated `testsuite-status.json` against the
pre-change baseline programmatically. Every one of the other 230
already-vendored files' tallies is byte-for-byte identical — the
stricter LEB128/section-structure rejection does NOT change behavior on
any other file's `assert_malformed` (or any other directive kind's)
cases.

One file's numbers DID move: `elem.wast`'s `module` tally (before this
pass: 25/76 pass; immediately after adding the section-size-mismatch
check, before investigating why: 21/76). That check didn't introduce a
bug — it exposed one that was already there, invisibly, in
`wasm-module-parser`'s element/table section parsers (a missing
`elemkind` byte read on mode-2 element segments silently desynced
subsequent fields by one byte; a function-references-proposal table
encoding was silently misread as an ordinary one). The element-segment
bug is now fixed for real (recovered one of the four: `elem.wast` sits
at 22/76 in the final baseline); the table encoding is a genuine,
separate feature gap (needs a `wasm-types`/`wasm-runtime` change to
store and evaluate an init expression) now reported honestly as
`not_yet_supported` instead of silently misparsed as a false `Pass` —
see the task tracker for that follow-up.

