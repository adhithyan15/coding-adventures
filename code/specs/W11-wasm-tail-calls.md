# W11 — Tail Calls: `return_call` / `return_call_indirect` (WASM16)

## Why

`code/specs/W07-wasm-post-mvp-epics.md` sizes the tail-call proposal for
future work without a real investigation behind the estimate. This spec
replaces that estimate with a real one, grounded in reading the actual
`wasm-execution` call machinery and the real, pinned-commit testsuite
files — the same discipline every WASM PR this session has used, not a
carried-forward guess.

## What "tail call" means here, and why it's not just "call, then return"

WASM's tail-call proposal adds two instructions:

| Opcode | Name | Shape |
|---|---|---|
| `0x12` | `return_call` | Same immediate as `call` (a `funcidx`) |
| `0x13` | `return_call_indirect` | Same immediates as `call_indirect` (a `typeidx` + `tableidx`) |

Both are confirmed **free, unclaimed** sub-opcodes in `wasm-opcodes` — no
existing entry uses `0x12`/`0x13`. Semantically, `return_call $f` is "call
`$f`, and when it returns, immediately return its result as this
function's own result" — but the spec's actual guarantee is stronger than
that description sounds: it must execute in **constant host stack
space**, no matter how many tail calls chain. A real Scheme/ML-style
tail-call-optimizing implementation is expected to run an unbounded chain
of tail calls (a tail-recursive loop written as a WASM function) without
ever growing the host call stack — that's the entire reason the proposal
exists, not an optimization nicety.

### Why a naive implementation would be a real regression, not a shortcut

`wasm-execution`'s `call_function`/`call_function_inner`
(`wasm-execution/src/lib.rs:3657-3848`, read directly for this spec, not
assumed) is genuine native Rust recursion: `call_function_inner` owns an
inner `while` loop that drives the callee's decoded instructions to
completion, and if one of those instructions is itself `call`/
`call_indirect`, the handler calls `call_function` again —
`call_function` → `call_function_inner` → (nested) `call_function` is a
real, growing Rust call stack, one frame per WASM call, bounded only by
`ctx.call_depth >= MAX_CALL_DEPTH` (WASM01's guard against a host-stack
overflow becoming an uncatchable process abort).

If `return_call` were implemented as "just call the target function, then
immediately return its result" (i.e., handled as an ordinary nested call
inside the current `call_function_inner`'s while loop), it would still
recurse through `call_function` and grow the Rust stack exactly like a
regular `call` — defeating the entire purpose of the instruction, and
worse, silently lying about it: a WASM module using `return_call` for a
tail-recursive loop (the proposal's own primary use case) would still hit
`MAX_CALL_DEPTH` and trap with "call stack exhausted" on exactly the
pattern the instruction exists to make unbounded. That's a correctness
bug wearing a passing test's clothes until someone writes a
deep-tail-recursion regression test — which this PR's own verification
plan requires up front, not as an afterthought.

## Scope per crate

### `wasm-opcodes`

Two new single-byte entries, `0x12` (`return_call`) and `0x13`
(`return_call_indirect`), following this crate's existing flat
`OPCODES` table shape (both are plain, top-level opcodes — no `0xFB`/
`0xFC`/`0xFE` prefix, matching `call`/`call_indirect`'s own encoding).

### `wasm-wast-parser`

`return_call $f (args...)` and `return_call_indirect (type $t) (table
$tab) (idx) (args...)` in both folded and flat form, reusing the exact
immediate-decode path `call`/`call_indirect` already have (a `funcidx`,
or a `typeidx`+`tableidx` pair) — no new immediate shape to parse.

### `wasm-validator`

New type rules for `0x12`/`0x13`, structurally close to `call`/
`call_indirect`'s existing rules (pop the callee's declared param types,
in order) with one real difference: like `return`, a tail call
**terminates the current instruction sequence** — the real spec requires
the callee's result type to match the CURRENT FUNCTION's own declared
result type exactly (not just be pushable to the stack for further use,
since nothing after a tail call can ever execute), and the validator
must treat everything textually after it as unreachable/stack-
polymorphic, the same handling `return`'s own existing rule already has
for the "everything after this is dead code" case.

### `wasm-execution` — the real work

`call_function_inner` needs restructuring so a tail call replaces the
*current* logical frame instead of recursing into a new one:

- The function gains an outer loop around the existing body (from
  resolving `func_type`/`body` through the inner `while` that drives
  execution). A `return_call`/`return_call_indirect` handler doesn't
  call `call_function` — it pops the callee's arguments, decodes the
  callee's body, and stores a "pending tail call" signal (the new
  `func_index` + prepared `locals`) that the OUTER loop checks after the
  inner `while` loop exits early (on the signal, not on natural
  completion). On seeing the signal, the outer loop rebinds
  `func_type`/`body`/`vm_instructions`/`ctx.typed_locals`/
  `ctx.label_stack`/`ctx.control_flow_map`/`ctx.br_table_targets`/
  `ctx.gc_ops` to the callee's own values (the same setup steps
  `call_function_inner` already does for an ordinary call) and resets
  `vm.pc = 0`, looping again — all still inside the SAME
  `call_function_inner` Rust stack frame. Crucially: **no new
  `SavedFrame` is pushed** for this transition (`ctx.saved_frames`
  stays exactly as it was before the tail call) — a tail call reuses the
  current frame's `return_pc`/caller state; only ordinary `call`/
  `call_indirect` push a new one. This is what makes an unbounded tail-
  call chain run in genuinely constant Rust-stack space: every tail call
  is a loop iteration within one existing frame, not a new frame.
- `return_call_indirect` additionally needs the same table-lookup +
  declared-vs-actual-type check `call_indirect`'s existing handler
  already does, before entering the tail-call transition above — no new
  logic there, just routing through the existing check first.
- `MAX_CALL_DEPTH`/`ctx.call_depth` is **not incremented** for a tail
  call (only for genuine `call`/`call_indirect`, which do grow the
  logical call stack) — this is the direct, mechanical consequence of
  not pushing a new `SavedFrame`, not a separate rule to remember.

## The real corpus's own gap (verified by fetching and reading, not assumed)

`return_call.wast` (241 lines, 46 `assert_*`/`invoke` directives) and
`return_call_indirect.wast` (603 lines) both fetched at this repo's
pinned commit and read directly before writing this spec — learning from
WASM05's `imports.wast` surprise, not repeating it blind. Both files
parse almost entirely with grammar this repo already supports, with ONE
narrow exception in each: a single auxiliary helper function declared
as `(func $f (result (ref null $t)) (ref.null $t))` — a **typed**
nullable reference result type (`(ref null $t)`, referencing a concrete
type index), which is function-references-proposal grammar
`wasm-wast-parser` has no support for (it only parses the bare `funcref`/
`externref`/`(ref.null func)` forms WASM17 added). Every actual EXPORTED
test function in both files uses ordinary, already-supported `funcref`
— `$f` is only reached indirectly, via `return_call $f` from one
`"type-funcref"` test case — but `wasm-wast-parser`'s script parser fails
the ENTIRE file on the first unparseable directive it hits, so this one
line blocks all 46 (and the `return_call_indirect.wast` equivalent)
directives from grading at all, not just that one case.

Unlike `imports.wast`'s `tag` gap (scattered through the file's core
structure, no small fix available), this is a single, isolated line.
Whether a minimal, narrowly-scoped parser addition (parsing `(ref null
$idx)`/`(ref $idx)` as a value type, without implementing any real
typed-function-reference semantics) is cheap enough to include is left
to the implementation PR to determine by actually attempting it — this
spec does not commit to vendoring these files, only to investigating
honestly and either vendoring them for real or logging a follow-up
exactly like `imports.wast`'s, not silently skipping the question.

## Explicitly out of scope

- `return_call_ref` (a third tail-call-family instruction) — needs the
  separate function-references proposal's `call_ref`/typed function
  references, which this repo doesn't have at all (WASM17's funcref/
  externref work doesn't reach it). Already out of scope before this PR
  and remains so.
- Any part of the function-references proposal beyond what's needed to
  parse the one narrow `(ref null $t)` case above, if that turns out to
  be worth doing at all.
- SIMD, GC, exceptions, the component model — unrelated, already
  tracked separately in `code/specs/W07-wasm-post-mvp-epics.md`.

## Addendum (2026-08-26): the `(ref null $t)` gap is closed

The "vendor if cheaply fixable, else defer" question this spec left open
(see the verification plan below) was investigated for real and turned
out to be cheap: `return_call.wast`/`return_call_indirect.wast` are now
vendored (`wasm-conformance` 0.1.95).

What actually landed, scoped to exactly the one construct both files
need and nothing more:

- **`wasm-types` 0.1.12**: a new `ValueType::ConcreteFuncRef(u32)`
  variant — a nullable reference to a specific concrete FUNCTION type,
  the direct analogue of the pre-existing `ValueType::StructRef(u32)`
  (nullable ref to a concrete STRUCT type), but indexing the func-type
  array (`WasmModule::types`) directly instead of the struct-type
  array's offset space. Encodes identically to `StructRef` (`0x63` +
  `LEB128(idx)`) — see that variant's own doc comment for why the two
  never collide despite sharing a tag byte. This is a NEW, narrower
  concept than "typed function references" in general: it only tracks
  "this is a nullable ref to *some* concrete function type," never which
  one, and there is still no non-null `(ref $t)` variant — the real
  typed-function-references wall (`call_ref`/`return_call_ref`) remains
  exactly as out-of-scope as the section above already said.
- **`wasm-wast-parser` 0.1.81**: `(ref null $t)` as a value type and
  `ref.null $t` as an instruction, both resolving `$t` via the same
  `type_names` map every `(type $t)` reference already uses.
- **`wasm-validator` 0.2.70**: one subtyping rule — a nullable ref to a
  specific concrete function type IS assignable where `funcref` is
  expected, never the reverse. This is what `return_call`/
  `return_call_indirect`'s existing "callee results must match the
  current function's declared results" check needed to stop being a
  strict `Vec` equality comparison (which would have wrongly REJECTED
  the valid direction) and start being a per-result subtype check
  (which correctly still REJECTS the invalid, mirror-image direction —
  both directions are real corpus `assert_return`/`assert_invalid`
  cases in both files).
- **`wasm-execution` 0.9.71**: the `ref.null` bytecode decoder's
  heap-type-immediate skip was a hardcoded single byte; a concrete
  function-type ref's immediate is a variable-length `0x63 <LEB128>`
  sequence, so the decoder needed a real (if small) fix to consume it
  correctly instead of corrupting the next instruction's decode.

Real, measured numbers (zero new `fail` anywhere in either file — see
`tests/fixtures/testsuite/NOTICE` for the full accounting):
`return_call.wast` — module 2/2 pass (+1 not_yet_supported),
assert_invalid 12/12 pass, assert_return 0/34 (all not_yet_supported,
cascading from both files' `(import "spectest" "print_i32_f32" ...)`,
this crate's pre-existing no-spectest-host gap, not new).
`return_call_indirect.wast` — module 2/2 pass (+1 nys), assert_invalid
15/17 pass (+2 nys, two more pre-existing, unrelated validator gaps —
dead-code polymorphism after a tail call, and no funcref-element-type
check on an indirect call's target table), assert_return 0/43,
assert_trap 0/7, assert_malformed 0/11 (all not_yet_supported — the
malformed cases need inline-signature clause-ordering validation this
crate's parser doesn't do, a pre-existing gap several other vendored
files also have).

## Verification plan

- Unit tests in each touched crate for the new opcode family, following
  established per-crate style.
- **Load-bearing regression test, written FIRST, not as an afterthought**:
  a WASM module using `return_call` to implement an unbounded tail-
  recursive loop (e.g. a countdown/sum-to-N function calling itself via
  `return_call` far beyond `MAX_CALL_DEPTH`'s current value) must
  succeed — proving real constant-stack-space behavior, not just that
  the opcode decodes. A companion test proves *ordinary* (non-tail) deep
  recursion is still correctly guarded by `MAX_CALL_DEPTH` (i.e., this
  PR doesn't accidentally weaken the existing guard for plain `call`).
- `return_call_indirect`'s table-lookup/type-check path gets the same
  "genuine type mismatch traps" coverage `call_indirect`'s own tests
  already have.
- Vendor `return_call.wast`/`return_call_indirect.wast` if the narrow
  `(ref null $t)` gap turns out to be cheaply fixable; if not, defer and
  log a follow-up exactly like `imports.wast`'s, regenerate the baseline
  either way, and diff against the pre-change baseline for zero
  regressions on any already-parsing file — the same discipline every
  WASM PR this session has used.
- `/security-review` before push — particular attention to the outer-
  loop restructuring in `call_function_inner`: confirm the tail-call
  transition can't be used to bypass `MAX_CALL_DEPTH`'s protection for
  *non-tail* calls smuggled in some other way, and that the "no new
  `SavedFrame`" change doesn't leave any stale/inconsistent execution
  context state behind (label stack, control-flow map, GC ops side-
  tables) when a tail call fires mid-way through evaluating the current
  function's own operand stack.
