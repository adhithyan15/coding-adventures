## 0.1.120 — 2026-09-02 — baseline regen: typed `select` (0x1C) closes `select.wast` (126 not-yet-supported → 0)

Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
after `wasm-wast-parser` 0.1.99 / `wasm-validator` 0.2.87 / `wasm-
execution` 0.9.92 added parsing, type-checking, and runtime support for
`select`'s typed `(result t)` form (opcode `0x1C`) — see those three
crates' own CHANGELOGs for the full root-cause writeup. No code changes
in this crate itself.

Live re-verification before fixing anything: ran the report tool fresh
(`cargo run --release --bin wasm_conformance_report`) rather than
trusting Addendum 2's number, confirming `select.wast` was still exactly
at 126 `not_yet_supported` (2 `module`, 118 `assert_return`, 6
`assert_trap`) — unchanged from the addendum. A throwaway probe outside
the repo (`wasm_conformance::run_wast_source` against `select.wast`
alone, via a scratch Cargo project depending on this crate by path) then
pinned the exact failing construct: both `module` failures were the
literal error `unknown instruction "result"`, at byte 656 (`(select
(result i32) ...)`) and byte 23437 — confirming Addendum 2's diagnosis
was still accurate as given, unlike two earlier finds in this same
campaign (W35's closing addendum, W36's own investigation) where a
prior session's diagnosis needed correction on re-check.

Programmatic per-file diff against the pre-fix baseline (Python,
comparing the `files` dict keyed by filename), across all 257 corpus
files: exactly 3 files changed, all strict improvements (no file's pass
count decreased or fail count increased) —

| File | Directive kind | Before | After |
|---|---|---|---|
| `select.wast` | `module` | 1/3 (2 not-yet-supported) | 3/3, 0 NYS |
| `select.wast` | `assert_return` | 0/118 (not-yet-supported) | 118/118 |
| `select.wast` | `assert_trap` | 0/6 (not-yet-supported) | 6/6 |
| `select.wast` | `assert_invalid` | 30/30 (unchanged — see below) | 30/30 |
| `call_indirect.wast` | `module` | 2/3 (1 not-yet-supported) | 3/3, 0 NYS |
| `stack.wast` | `module` | 0/2 (2 not-yet-supported) | 1/2, 1 NYS |

`select.wast` itself is now fully passing on every directive kind it
has — the honest exception is `stack.wast`, which drops from 2 to 1
`module` NYS: the SAME `call_indirect` flat-form fix (needed to fully
close `select.wast`'s own "Flat syntax" section) also fixed 1 of
`stack.wast`'s 2 pre-existing gaps as a real, understood side effect
(both exercise the identical "bare atom right after `call_indirect` in
flat/stream form" ambiguity — `stack.wast`'s remaining 1 NYS is a
genuinely separate, still-open gap, not something this fix touched).
`call_indirect.wast` similarly picked up 1 more real pass for the same
reason. `select.wast`'s own `assert_invalid` count (30/30) is UNCHANGED
in total, but for a subtler reason worth recording honestly: those 30
cases used to pass by ACCIDENT (`grade_assert_invalid` treats a module
build failure as a valid "correctly rejected" `Pass`, and every one of
them failed to even parse before this fix, for the wrong reason —
`select`'s typed form itself, not the type mismatch the case was
actually designed to probe). They now parse and build successfully and
pass for the RIGHT reason instead: `wasm-validator`'s new `0x1C` type-
check rule genuinely rejects them (e.g. `arity-0`/`arity-2`'s "invalid
result arity", the reference-typed-operand cases via the untyped-`0x1B`-
restriction check). Confirmed by reading `wasm-validator`'s new code
path directly, not just by the pass count staying the same.

Aggregate: `module` 2055→2059 pass (+4: 2 in `select.wast`, 1 each in
`call_indirect.wast`/`stack.wast`), not-yet-supported 198→194 (-4, exact
match); `assert_return` 52015→52133 pass (+118, exact match to `select.
wast`'s own count), not-yet-supported 749→631 (-118); `assert_trap`
4828→4834 pass (+6), not-yet-supported 140→134 (-6). Every other
directive kind and every other of the 257 files is byte-for-byte
unchanged from the 0.1.119 baseline.

