# W19 — WASM Relaxed SIMD: re-verified encoding, the `either` grading gap, and a real first slice

## Purpose

PR1-PR47 (all merged) shipped the **base** SIMD proposal end to end — the
completion agent for PR47 reported a 100% exact match against
`BinarySIMD.md` for the base proposal's opcode set, zero gaps remaining.
That agent did a *lightweight* scan for what epic to pick up next and
recommended **relaxed SIMD** (~19 opcodes, reported sub-opcode range
`0x100`-`0x113`) as the natural continuation of the same `0xFD`-prefixed
`v128` infrastructure, but explicitly flagged its own numbers as
unverified.

This doc re-verifies every one of those claims from scratch — spec
source, real encoding, real vendorable corpus — the same discipline W13
used to kick off the base SIMD epic itself, and scopes a genuinely
bounded PR-1.

## What's confirmed, not assumed

### The spec source

Relaxed SIMD is its own proposal repo, not a section of the base SIMD
spec: **`https://github.com/WebAssembly/relaxed-simd`**, encoding table at
`proposals/relaxed-simd/Overview.md`. Fetched directly (not guessed):

> All opcodes have the `0xfd` prefix (same as SIMD proposal) ... Opcodes
> `0x100` to `0x12F` (32 opcodes) are reserved for this proposal.

This is a **different document** from `BinarySIMD.md` (the base SIMD
proposal's own encoding table, used throughout PR1-PR47) — a new
authoritative source needs citing for this epic, not a re-read of the old
one.

### The real encoding — confirms the prior agent's range, corrects the count

The Overview.md table lists **19 real instructions** (not "~19" — exactly
19), `0x100`-`0x113`, with `0x114`-`0x12F` reserved/unused:

| instruction | opcode |
|---|---|
| `i8x16.relaxed_swizzle` | `0x100` |
| `i32x4.relaxed_trunc_f32x4_s` | `0x101` |
| `i32x4.relaxed_trunc_f32x4_u` | `0x102` |
| `i32x4.relaxed_trunc_f64x2_s_zero` | `0x103` |
| `i32x4.relaxed_trunc_f64x2_u_zero` | `0x104` |
| `f32x4.relaxed_madd` | `0x105` |
| `f32x4.relaxed_nmadd` | `0x106` |
| `f64x2.relaxed_madd` | `0x107` |
| `f64x2.relaxed_nmadd` | `0x108` |
| `i8x16.relaxed_laneselect` | `0x109` |
| `i16x8.relaxed_laneselect` | `0x10a` |
| `i32x4.relaxed_laneselect` | `0x10b` |
| `i64x2.relaxed_laneselect` | `0x10c` |
| `f32x4.relaxed_min` | `0x10d` |
| `f32x4.relaxed_max` | `0x10e` |
| `f64x2.relaxed_min` | `0x10f` |
| `f64x2.relaxed_max` | `0x110` |
| `i16x8.relaxed_q15mulr_s` | `0x111` |
| `i16x8.relaxed_dot_i8x16_i7x16_s` | `0x112` |
| `i32x4.relaxed_dot_i8x16_i7x16_add_s` | `0x113` |

Every value is `>= 0x100` (256 decimal), so **every** relaxed-simd opcode
needs the LEB128 continuation byte — there is no single-byte-encodable
relaxed opcode, unlike base SIMD where only the high half (`0x80`-`0xFF`)
did.

### Decode: already built, not a new gap (unlike base SIMD's original one)

W13 had to add genuine new decoder infrastructure because the existing
`0xFB`/`0xFC`/`0xFE` two-byte-prefix families all happened to have
sub-opcodes `< 128` (single LEB128 byte), so `wasm-execution` had never
needed a real multi-byte LEB128 read at a sub-opcode position before SIMD
forced it. That gap is **already closed**: base SIMD's own table already
contains sub-opcodes past 127 (e.g. `i32x4.add` is byte value 174,
encoded `[0xAE, 0x01]`), so both `wasm-execution`'s function-body decoder
and `wasm-validator`'s `type_check.rs` mirror already call a real
`u32` LEB128 reader (`decode_leb_u32`/`decode_unsigned`) at the SIMD
sub-opcode position, tested today up to `0xFF`. `0x100`-`0x113` need the
identical 2-byte LEB128 shape (`0x100` = `[0x80, 0x02]`, `0x113` =
`[0x93, 0x02]`) — confirmed by hand-encoding, no 3rd byte needed (max
value 275 still fits `7 + 7 = 14` bits). **Zero new decoder code is
needed for this epic** — every relaxed opcode is just a new
`SimdOpInfo` table row plus new match arms, exactly like every other
"widen" PR in the base-SIMD campaign.

### The real, structural new gap: the `either` grading combinator

This is the one genuinely new piece of infrastructure this epic needs,
discovered by reading the real corpus content (not assumed from the
opcode list):

```wast
(assert_return (invoke "i8x16.relaxed_swizzle" ...)
               (either (v128.const i8x16 0 0 0 ... 0)
                       (v128.const i8x16 0 1 2 ... 15)))
```

Relaxed-simd ops are **implementation-defined** for specific input
patterns (the spec deliberately leaves them unconstrained, so engines can
pick whichever behavior maps to the host's native SIMD instruction — the
whole point of the proposal). The upstream corpus encodes this with a
new `(either A B)` assert_return wrapper: the actual result must equal
**either** `A` or `B`, not necessarily a single exact value. Grepped
every one of the 7 relaxed-simd `.wast` files at the pinned SHA (see
below) — **`either` appears in all 7**; there is no relaxed-simd file
that only uses plain exact `assert_return`. Supporting `either` is a
prerequisite for vendoring *any* relaxed-simd corpus file, not an
optional nicety — without it, `wasm-wast-parser` fails to parse the
`either` form at all (an `UnexpectedToken` on the `either` head symbol),
and the fixture can't be vendored.

Confirmed this repo has no such thing today: `wasm-wast-parser::script::
Expected` has `Value`/`NanCanonicalF32`/`NanArithmeticF32`/`NanCanonicalF64`/
`NanArithmeticF64`/`V128F32x4`/`V128F64x2`/`RefNullAny`/`RefFuncAny` — no
"either of two full expected values" variant. `wasm-conformance`'s single
grading function, `value_matches_expected(actual, v128_bytes, expected)`
(`wasm-conformance/src/lib.rs`), has no `Either` arm either.

**The fix is small and fully generic** — this is exactly the kind of
bounded, real new infrastructure this campaign takes on rather than
working around:

- `Expected::Either(Box<Expected>, Box<Expected>)` — a new variant whose
  two children are themselves full `Expected` values (recursive, so
  `either` can in principle wrap any other expected-value shape, not just
  plain `V128`).
- `parse_expected`: a new `("either", _)` match arm that recursively
  parses its two child S-expressions as `Expected` values and wraps them.
- `value_matches_expected`: `Expected::Either(a, b) => value_matches_expected(actual, v128_bytes, a) || value_matches_expected(actual, v128_bytes, b)`.

This is a one-time, fully reusable harness addition — every future
relaxed-simd PR in this epic reuses it unchanged; no opcode-specific
grading logic is needed.

### The corpus — real, vendorable, confirmed via a live GitHub API tree listing at the pinned SHA

Same discipline as WASM08's own fetch verification: queried
`GET /repos/WebAssembly/testsuite/git/trees/28864811cf03bdbf880733786148feaba339582d?recursive=1`
(the exact pinned commit every `TESTSUITE_FILES` entry in this repo is
fetched from, never re-pinned) directly, not guessed from file-naming
conventions. **Relaxed-simd test files DO exist at this pinned SHA** —
this is the outcome this task's own brief flagged as a real, expected
possibility of *not* being true (relaxed-simd was still an early-phase
proposal when many testsuite snapshots were cut); it turned out to be
false in this repo's case. All live at the testsuite repo **root**
(unlike `atomic.wast`, which lives under `proposals/threads/` — no
`PROPOSAL_FILES` entry is needed here, a plain `TESTSUITE_FILES` append
suffices):

| file | bytes | opcodes covered | real `assert_return`? |
|---|---|---|---|
| `i8x16_relaxed_swizzle.wast` | 2438 | `i8x16.relaxed_swizzle` | yes (5 cases, uses `either`) |
| `i16x8_relaxed_q15mulr_s.wast` | 1264 | `i16x8.relaxed_q15mulr_s` | yes (2 cases, uses `either`) |
| `i32x4_relaxed_trunc.wast` | 658 | 4 `i32x4.relaxed_trunc_*` ops | **no** — module-only, zero `assert_return`/`assert_*` directives at all |
| `relaxed_dot_product.wast` | 5935 | `i16x8.relaxed_dot_i8x16_i7x16_s`, `i32x4.relaxed_dot_i8x16_i7x16_add_s` | yes (uses `either`) |
| `relaxed_laneselect.wast` | 6517 | 4 `*.relaxed_laneselect` ops | yes (uses `either`) |
| `relaxed_madd_nmadd.wast` | 12550 | 4 `f32x4`/`f64x2` `relaxed_madd`/`relaxed_nmadd` | yes (uses `either`) |
| `relaxed_min_max.wast` | 8577 | 4 `f32x4`/`f64x2` `relaxed_min`/`relaxed_max` | yes (uses `either`) |

`i32x4_relaxed_trunc.wast` is flagged explicitly: it's the smallest file
by bytes, but it carries **zero test assertions** — vendoring it would
add opcodes with no real conformance signal, which is weaker than every
other file here and than every prior "widen" PR in this campaign (each
of which vendored a file with genuine `assert_return` coverage). It is
deliberately **not** part of this PR's first slice; a future PR that
implements the `relaxed_trunc` family should look for whether upstream
gained real assertions for it since this pinned SHA, or accept the
weaker coverage explicitly if not, rather than silently treating it the
same as the other 6.

### Semantics: implementation-defined, but this repo can pick a deterministic behavior

Every relaxed op's spec text explicitly allows (does not require)
platform-dependent results for specific input classes — that's the
entire premise of the proposal. This does **not** mean this repo's
interpreter needs nondeterministic or "either" execution: it means the
interpreter is free to pick **any one** conforming deterministic
behavior, and the corpus's `either` wrapper is exactly what makes both
choices pass. For `i8x16.relaxed_swizzle` specifically: the base
(non-relaxed) `i8x16.swizzle` op already implemented in this repo
(`SimdOpKind::Swizzle`) clamps any out-of-range index (`>= 16`) to `0`.
Checked against `i8x16_relaxed_swizzle.wast`'s actual `either` pairs by
hand: the "out of range, `< 128`" case accepts *either* all-zero *or* the
mod-16-wrapped select, and the "`>= 128`" case accepts the same two
alternatives — the existing clamp-to-zero behavior is a **valid,
literal, deterministic member of both `either` pairs**. So
`i8x16.relaxed_swizzle` can reuse `Swizzle`'s existing execution body
completely unchanged in semantics (new opcode entry, same code shape) —
no new numeric/rounding logic, no new harness support beyond the
`either` grading fix above. This is confirmed by hand-checking the
corpus file's literal expected values, not assumed from the spec prose
alone.

The other 6 opcode families (`relaxed_trunc`, `relaxed_madd`/`nmadd`,
`relaxed_laneselect`, `relaxed_min`/`max`, `relaxed_q15mulr_s`,
`relaxed_dot_*`) each need their own semantics investigation before
implementation — e.g. `relaxed_madd`/`nmadd` may or may not fuse the
multiply-add depending on host FMA availability, which is a real open
question for a *future* PR in this epic, not this one. Flagged here so
the next PR doesn't skip it.

## First slice: `i8x16.relaxed_swizzle` only

The smallest self-contained, **real-assertion-bearing** unit in the
corpus that exercises the new `either` machinery:

- 1 new opcode (`i8x16.relaxed_swizzle`, sub-opcode `0x100`).
- 1 new `SimdOpKind` variant, reusing `Swizzle`'s existing binary
  `(v128, v128) -> v128` validator arm and execution body verbatim in
  semantics (own match arm, referencing `Swizzle`'s doc comment, per this
  codebase's existing convention of duplicating small per-kind bodies
  rather than sharing a helper across differently-named opcodes).
- 1 new vendored corpus file (`i8x16_relaxed_swizzle.wast`, real
  `assert_return`/`either` coverage).
- The generic `either` grading fix in `wasm-wast-parser` +
  `wasm-conformance` described above — reusable, unchanged, by every
  subsequent relaxed-simd PR.

Deliberately excludes the other 18 opcodes and 6 remaining corpus files —
each is its own future PR, following this epic's own "one opcode family
per PR" discipline (matching PR18/PR22/PR33/etc.'s granularity in the
base-SIMD epic).

## Blast radius

- `wasm-opcodes`: +1 `SimdOpKind` variant, +1 `SIMD_OPS` row, bump the
  crate's own `assert_eq!(SIMD_OPS.len(), ...)` count.
- `wasm-validator`: +1 arm in the existing shared binary-`v128`-shape
  match in `type_check.rs` (same arm `Swizzle` is already in).
- `wasm-execution`: +1 match arm in the SIMD dispatch (same shape as
  `Swizzle`'s).
- `wasm-wast-parser`: +1 `Expected` variant, +1 parsing arm (generic,
  not `i8x16.relaxed_swizzle`-specific).
- `wasm-conformance`: +1 grading arm (generic), +1 vendored fixture, an
  updated `NOTICE`, an updated `TESTSUITE_FILES` entry in
  `fetch_testsuite.py`, and a regenerated `testsuite-status.json`
  baseline.

Every touched crate gets its own version bump and `CHANGELOG.md` entry,
same discipline as every prior SIMD PR.
