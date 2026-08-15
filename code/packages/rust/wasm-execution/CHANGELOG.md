# Changelog

All notable changes to this package will be documented in this file.

## [0.6.11] - 2026-08-15 (WASM16 — real tail calls, genuinely constant Rust-stack space)

### Added

- Real `return_call`/`return_call_indirect` handlers. The naive
  implementation ("call the target, then return its result") would
  still recurse through `call_function`/`call_function_inner` exactly
  like an ordinary `call` -- defeating the entire point of the
  instruction, and silently failing its own primary use case (an
  unbounded tail-recursive loop would still hit `MAX_CALL_DEPTH` and
  trap on exactly the pattern the instruction exists to make
  unbounded). The real fix restructures BOTH of this crate's
  "run a function body" implementations (`call_function_inner`, used
  for nested calls, and `WasmExecutionEngine::call_function`, the
  separate top-level entry point with its own independent dispatch
  loop) with an outer loop: a tail call swaps in the callee's state
  and continues the SAME Rust stack frame, pushing no new `SavedFrame`
  and not incrementing `call_depth` -- the mechanical reason an
  unbounded `return_call` chain runs in genuinely constant Rust-stack
  space. See `code/specs/W11-wasm-tail-calls.md`.
- New `WasmExecutionContext::pending_tail_call: Option<(usize,
  Vec<WasmValue>)>` field -- set by the `return_call`/
  `return_call_indirect` opcode handlers to signal the enclosing
  "run a function body" loop, checked once per outer-loop iteration
  right after the inner instruction-dispatch loop halts.
- `code/packages/rust/wasm-execution/tests/wasm16_tail_calls.rs`: 5 new
  integration tests built from real WAT via `wasm-wast-parser`,
  including the load-bearing proof -- a self-tail-recursive accumulator
  running 20,000 iterations deep (250x `MAX_CALL_DEPTH`'s value of 80)
  succeeding cleanly, plus a companion test proving the SAME depth
  written with plain `call` still correctly traps (this PR doesn't
  weaken `MAX_CALL_DEPTH`'s existing guard), mutual tail recursion
  across two distinct functions at the same depth, `return_call_indirect`
  through a table, and a single non-recursive `return_call`.
- Full existing test suite (227 tests, spanning both "run a function
  body" implementations) passes completely unchanged by this
  restructuring -- strong evidence the refactor preserves every
  existing non-tail-call code path exactly.

### Investigated, not fixed this release

- The real, pinned-commit `return_call.wast`/`return_call_indirect.wast`
  each fail to parse (see `wasm-wast-parser` 0.1.10's CHANGELOG for
  why, and why a minimal fix was investigated and rejected as
  incorrect, not just skipped). Not vendored this release; the real
  tail-call machinery is fully verified via the hand-written
  integration tests above instead.

### Fixed

- Security review caught the new `return_call_indirect` (`0x13`)
  handler directly indexing `ctx.func_types[func_index]` where
  `func_index` comes from a TABLE ENTRY (data, not a static part of
  the bytecode a validator necessarily already checked -- this engine
  can be, and in this crate's own tests sometimes is, driven without
  `wasm-validator` running first) -- a table slot pointing past
  `func_types.len()` panicked the host process instead of trapping
  cleanly. Fixed with `.get()`, matching the sibling `return_call`
  (`0x12`) handler's already-safe pattern. 1 new regression test,
  confirmed to panic without the fix via a temporary revert.

## [0.6.10] — 2026-08-15 (WASM05 — Table gains Clone, LinearMemory/Table gain limit accessors)

### Added

- `Table` now derives `Clone` (matching `LinearMemory`, which already
  did) -- `wasm-conformance`'s new registry-backed `HostInterface`
  (WASM05/W10) needs to hand back an owned `Table`/`LinearMemory` when
  resolving a cross-module import, since `HostInterface::resolve_table`/
  `resolve_memory` return owned values, not references.
- `LinearMemory::max_pages()` and `Table::max_size()` -- both already
  had `size()`; the declared maximum was tracked internally but had no
  public getter. `wasm-runtime`'s new link-time limits-compatibility
  check needs both halves of a `Limits` pair, not just the current size.

## [0.6.9] — 2026-08-15 (WASM18 — plain atomic memory op handlers, no real concurrency)

### Added

- Real opcode handlers for the entire `0xFE`-prefixed atomics family
  (load/store/RMW/cmpxchg/fence/notify/wait), dispatched via
  `wasm_opcodes::ATOMIC_OPS` -- one `register_context_opcode(0xFE, ...)`
  handler matching on `AtomicOpKind` rather than 67 individual handlers.
  RMW ops (`add`/`sub`/`and`/`or`/`xor`/`xchg`) and `cmpxchg` reuse the
  existing narrow-width `LinearMemory` load/store methods already built
  for MVP `i32`/`i64` load/store, so no new memory-access code paths
  were needed -- only new dispatch and arithmetic.
- `memory.atomic.notify` always returns 0 woken; `wait32`/`wait64`
  return 1 ("not-equal") or 2 ("timed-out") based on a plain memory
  comparison against `expected` -- both fully deterministic without any
  real threading, since a single-threaded VM can never have a second
  agent blocked in `wait`. See `wasm-opcodes` 0.2.3's CHANGELOG for why
  this contradicts the merged W09 spec's original "meaningless without
  real threads" framing.
- Runtime alignment trap: atomic instructions now check that the
  *effective* address (`base + offset`, where `base` can be a runtime
  value) is a multiple of the operation's natural alignment, trapping
  with `"unaligned atomic"` if not. This is distinct from (and in
  addition to) `wasm-validator`'s existing check of the *declared*
  `align=` immediate, which is a static property of the bytecode and
  can't see a runtime-computed address. Confirmed against the real,
  pinned-commit `atomic.wast` testsuite's own 45 `assert_trap
  ... "unaligned atomic"` cases -- all 45 now pass (previously 0/45,
  since this check didn't exist at all).
- 13 new tests, including a misalignment/naturally-aligned pair proving
  the new runtime check traps exactly where it should and doesn't
  over-trigger on valid accesses.

### Fixed

- Security review caught the `AtomicOpKind::Wait` handler (`memory.
  atomic.wait32`/`wait64`) missing the `check_atomic_alignment` call
  every sibling atomic-op arm has -- a misaligned wait address would
  silently proceed instead of trapping with `"unaligned atomic"` like
  the spec requires. Not a memory-safety issue (still routes through
  bounds-checked `LinearMemory` accessors), but a real spec-conformance
  gap the vendored `atomic.wast` corpus doesn't happen to exercise
  (its own `"unaligned atomic"` cases only cover load/store/RMW/
  cmpxchg, not wait). Fixed with a dedicated regression test, confirmed
  to fail without the fix via a temporary revert.

## [0.6.8] — 2026-08-15 (WASM17 — ref.func, table.get, table.set opcode handlers)

### Added

- `ref.func` (0xD2): bounds-checks the `funcidx` immediate against
  `ctx.func_types` (same source of truth `call_function_inner`'s own
  bounds check uses) and pushes `WasmValue::Ref(Some(func_index))`.
- `table.get`/`table.set` (0x25/0x26): thin wrappers around the
  already-existing, already-tested `Table::get`/`Table::set` *methods* --
  only `call_indirect`'s hardcoded table-0 lookup and element-segment
  initialization reached them directly before this; no WASM *instruction*
  did. Honors the real decoded `tableidx` (unlike `call_indirect`, which
  hardcodes table 0) so a named `$t` resolves to whichever index
  `wasm-wast-parser` actually emitted.
- `ref.null` (0xD0)/`ref.is_null` (0xD1) needed NO changes -- both already
  had real, tested handlers from before WASM17 (this repo's existing GC
  slice already needed them); they just become *reachable* now that
  `wasm-wast-parser` can emit them from text. See `code/specs/
  W08-wasm-funcref-externref.md`.
- `WasmValue::default_for` and `wasm-runtime`'s lossy `call()` argument
  conversion updated to cover the two new `ValueType` variants
  (`Funcref`/`Externref` default to the null reference, same as every
  other nullable reference type).
- 5 new opcode-level unit tests (`ref.func` valid + out-of-range,
  `table.get`/`table.set` round-trip, uninitialized-slot, out-of-bounds).

## [0.6.7] — 2026-08-13 (WASM04 — multi-value block/loop/if blocktypes)

### Fixed — `block_arity` resolved a type-index blocktype against the wrong table

`block`/`loop`/`if`'s blocktype immediate can now be a signed LEB128
type-section index, not just the MVP's single-byte empty/valtype encoding
(paired with `wasm-wast-parser` 0.1.6, which now emits this form). Wiring
it into the interpreter surfaced two bugs, both latent until a multi-value
blocktype could ever be decoded at all:

- `block_arity` resolved a type-index blocktype against `ctx.func_types`
  (indexed by FUNCTION index — one entry per function, sized to the
  function count) instead of `ctx.types` (the module's real, deduplicated
  TYPE SECTION, the actual index space a blocktype's type-index refers
  to). `call_indirect`'s handler already had this exact wrong-table bug
  fixed once; `block_arity` had the same bug, just never reachable
  before now. Fixed by changing its signature to take the real type
  section and return `(param_arity, result_arity)` instead of a bare
  result-only arity.
- `execute_branch` hardcoded a branch-to-a-loop's arity to `0`, with an
  explicit `// MVP` comment — correct then, since a loop's blocktype could
  never declare params before the multi-value extension. A branch to a
  loop's label re-enters its START, which needs the loop's declared PARAM
  arity preserved on the stack (re-entry re-consumes them), not its result
  arity (which is what a branch to a block/`if`'s END needs instead).
  Added `Label::param_arity` and threaded it through every `Label`
  construction site (`block`/`loop`/`if`'s handlers, the implicit
  function-body label in both `call_function_inner` and the top-level
  entry point), and fixed `execute_branch`'s loop-vs-block arity split.
- 4 new regression tests, including a hand-built case (not lifted from the
  testsuite) that forces a branch to a loop's label through an
  intervening inner scope with real scratch data that must be discarded —
  the shape that actually exercises `param_arity`, since the official
  testsuite's own `params`/`params-break` cases happen to never need real
  stack surgery on this interpreter's unwind step. Verified via
  TEMP-REVERT-CHECK that all 4 fail (one cleanly, one by hanging) with
  either bug reintroduced.
- Baseline effect: `block.wast`, `if.wast`, and `loop.wast` previously
  failed to PARSE AT ALL (multi-value blocktypes were unrecognized
  syntax); all three now pass in full (`assert_return` 52/52, 123/123,
  78/78). `br.wast` and `func.wast` also gained 1 and 3 previously-failing
  `assert_return` cases respectively. `fac.wast` newly parses too. Zero
  regressions — see `wasm-conformance`'s own `0.1.9` changelog entry for
  the full per-file diff.

## [0.6.6] — 2026-08-13 (WASM13 — f32 NaN payloads silently canonicalized on every stack push/pop)

### Fixed — `WasmValue::to_typed`/`from_typed` destroyed f32 NaN bit patterns

`GenericVM`'s typed operand stack has exactly ONE float slot
(`Value::Float(f64)`), shared by both WASM float widths — so every `f32`
value that's merely pushed or popped (locals, params, results, operands;
not just values an opcode actually computed on) round-trips through this
f64 box via `to_typed`/`from_typed`. Those conversions used an ARITHMETIC
`as f64` widen on the way in and `as f32` narrow on the way back out.
Confirmed empirically that Rust's `as` cast between float widths does
**not** guarantee NaN payload preservation on the narrowing leg: `f32::
from_bits(0x7fa00000) as f64 as f32` produces `0x7fc00000` — LLVM's
`fpext`/`fptrunc` canonicalize the payload to the target type's generic
quiet NaN. So ANY f32 NaN merely sitting on the stack silently lost its
exact bit pattern by the time it came back off, independent of which
opcode touched it.

This was invisible for most NaN-producing operations because the WASM
spec itself only requires them to produce a value in the `nan:arithmetic`
CLASS (any quiet NaN, exact payload unspecified) — `wasm-conformance`'s
grading already accepts that loosely, so canonicalization to `0x7fc00000`
still graded `Pass`. It was NOT invisible for `f32.reinterpret_i32`/
`i32.reinterpret_f32` (pure bit reinterpretation, where the testsuite
asserts an EXACT value, not a class) and for any case that round-trips a
NaN through a `local.tee`/param/result boundary without touching it
arithmetically at all — both are supposed to preserve the bits exactly by
construction, and both went through `to_typed`/`from_typed` regardless.

Fixed by making both conversions bit-preserving reinterpretations
(`f64::from_bits(v.to_bits() as u64)` / `f32::from_bits(v.to_bits() as
u32)`) instead of arithmetic casts — lossless for every case (NaN,
normal, ±0.0, ±inf), not just NaN, since it does no rounding at all.
Confirmed `virtual-machine`'s own `GenericVM` never interprets
`Value::Float` numerically outside a `Display` impl (used for debug
printing only), so re-purposing that f64 slot as an opaque bit-carrier
for f32 values has no effect on anything else built on `GenericVM`.

Verified via TEMP-REVERT-CHECK: reverting the fix (restoring the
arithmetic casts) makes all 3 new regression tests fail with exactly the
`0x7fc00000` canonicalization this changelog describes; restoring the fix
makes them pass again.

Baseline: `assert_return` 13495/13518 (99.8%) → 13512/13518 (100.0%,
+17) — closes 4 files' worth of previously-tracked NaN-payload gaps in
one fix: `conversions.wast` (the 4 `reinterpret` cases WASM03 surfaced),
`float_literals.wast`, `float_misc.wast`, and `local_tee.wast`'s
"as-unary-operand" case — this WASM13 backlog item's own two originally-
named repro cases. Verified via a full per-file diff against the previous
baseline that these 4 files are the ONLY ones whose tally changed, and
every one of their fails went to exactly 0 (not just down).

New tests: a direct `to_typed`/`from_typed` round-trip over 6 real NaN bit
patterns (distinct payloads, both signs, quiet and would-be-signaling,
plus the canonical NaN itself as a sanity check that the fix doesn't
break the ALREADY-canonical case); end-to-end `f32.reinterpret_i32`/
`i32.reinterpret_f32` tests through the actual interpreter using the
exact bit patterns the real testsuite asserts against.

## [0.6.5] — 2026-08-13 (WASM03 — sign-extension, trunc_sat, and a real trapping-trunc boundary bug)

### Added

- The 5 sign-extension opcodes (0xC0-0xC4): each pops an int, sign-extends
  its low 8/16/32 bits to the full width via Rust's own `as i8 as i32`-style
  truncate-then-sign-extend cast (exactly matching the spec's `signed_N`
  definition), pushes the result.
- The 8 `trunc_sat` sub-opcodes (`0xFC 0x00`-`0x07`, decoding already
  existed for the `0xFC` prefix from bulk-memory's `memory.copy`/
  `memory.fill` — only the sub-opcode dispatch needed extending): the
  non-trapping float-to-int conversions. Implemented as a straight `as`
  cast with no bounds checking at all, because Rust's own float→int `as`
  cast has used SATURATING semantics (NaN → 0, out-of-range → the nearest
  bound) since Rust 1.45 — a direct, built-in match for the spec's
  definition, needing no hand-rolled boundary logic.

### Fixed — the TRAPPING `trunc_f32/f64_s/u` handlers (0xA8-0xB1) had real, pre-existing boundary bugs

Investigating why `conversions.wast` (only parseable for the first time
after this release's own opcode additions) still had `assert_trap`/
`assert_return` failures after the two additions above found these
**already-existing**, entirely unrelated bugs, invisible until now because
`conversions.wast` was the only vendored file exercising these boundary
cases and it could never parse before:

- `i32.trunc_f32_u`/`i32.trunc_f64_u` (0xA9/0xAB) used an inclusive `0.0..`
  lower bound, so any negative input — even a tiny one that truncates
  toward zero to a perfectly valid `0` — incorrectly trapped.
- `i32.trunc_f64_s` (0xAA) used an inclusive lower bound (`-2147483649.0..`)
  where the spec requires a STRICT exclusion — `-2147483649.0` itself (one
  past the valid range) was wrongly accepted instead of trapping.
- All four i64-destination handlers — `i64.trunc_f32_s`/`i64.trunc_f32_u`/
  `i64.trunc_f64_s`/`i64.trunc_f64_u` (0xAE/0xAF/0xB0/0xB1) — had **no
  overflow check at all**, only a NaN check — `a as i64`/`a as u64 as i64`
  alone is that same Rust-1.45+ SATURATING cast the new `trunc_sat`
  handlers correctly rely on above, so these TRAPPING opcodes were silently
  behaving like their non-trapping `trunc_sat` counterparts instead of
  trapping on overflow, the opposite of their contract.

Fixed with 4 new shared boundary-check functions (`trunc_s_i32_in_range`,
`trunc_u_i32_in_range`, `trunc_s_i64_in_range`, `trunc_u_i64_in_range`),
each doc-commented with the exact spec inequality and why the chosen f64
literal constants are exact (not approximations) for that specific
boundary — see their own doc comments for the precision reasoning, which
differs between the i32 case (plain strict inequalities against
exactly-representable `f64` constants) and the i64 case (the true boundary
constant itself isn't exactly representable in `f64`, but no representable
`f64` value exists in the gap where it would matter, so a carefully chosen
inclusive/exclusive form is still exact). `f32` sources are widened to
`f64` first (lossless) before applying the same checks, avoiding a second,
separate set of `f32`-precision boundary constants.

Baseline: `i32.wast`/`i64.wast` go from full parse failures to 100% passing
every directive kind they have; `conversions.wast` goes from a full parse
failure to `assert_trap` 67/67 (100%) and `assert_return` 522/526 (99.2%,
+1253 across the three newly-parseable files against the pre-WASM03
baseline). The 4 remaining `conversions.wast` `assert_return` fails are
`f32.reinterpret_i32`/`i32.reinterpret_f32` NaN-bit-pattern cases —
unrelated to this release, corroborating evidence for the already-tracked
WASM13 (NaN payload preservation) backlog item, not fixed here. Verified
via a full per-file diff against the previous baseline that these 3 newly-
parseable files are the ONLY files whose tally changed anywhere in the
corpus.

New tests: 5 sign-extension round-trips, 8 `trunc_sat` cases (ordinary
value, NaN-saturates-not-traps, overflow/underflow saturation for both
signed and unsigned, both `i32` and `i64` destinations), and 7 regression
tests for the trapping-trunc boundary fix (the exact tiny-negative,
exact-lower-boundary, and previously-never-trapping-on-overflow cases the
old code got wrong, plus the i64::MIN exact-boundary acceptance case).

## [0.6.4] — 2026-08-13 (WASM11 — a real branch double-pop bug)

`execute_branch` (the shared handler behind `br`/`br_if`/`br_table`) used
to `ctx.label_stack.truncate(label_stack_index)` — removing the TARGET
label — and then jump to `label.target_pc`. For a `block`/`if` label,
`target_pc` is the literal position of that block's own `end` opcode (not
one past it — see `block`'s handler and `build_control_flow_map`), and
the `end` handler unconditionally pops one label whenever it runs,
whether reached by ordinary fall-through or landed on by a branch.
Removing the target via `truncate` and THEN landing on its own `end`
byte popped it a SECOND time — a genuine double-pop that silently
removed one extra label (belonging to whatever the next enclosing block
happened to be) on any branch that unwound past one or more already-open
outer blocks. This was invisible for the extremely common "the
branched-into block is effectively the last thing in the function"
shape (the accidental extra pop just triggered the function-end path a
little early, with no observable difference), but produced a real
`StackUnderflow` trap for anything with real code still to run after the
target block closes — found running the official WebAssembly spec
testsuite's own `switch.wast` (a `br_table` dispatching through 10
levels of nested named blocks — some targets land in the MIDDLE of the
nesting, not just the innermost or outermost), not by inspection.

Fixed by keeping a `block`/`if` target's label ON `label_stack` when
branching to it (`truncate(label_stack_index + 1)` instead of
`truncate(label_stack_index)`), so landing on its own `end` byte pops it
EXACTLY once — identical to what ordinary, non-branching fall-through to
that same `end` already does. A `loop` target keeps the ORIGINAL
behavior (`truncate(label_stack_index)`, no `+ 1`): a loop's
`target_pc` is the position of the `loop` OPCODE ITSELF (not an `end`
byte), so branching back to it re-executes that opcode, which
unconditionally re-pushes a fresh label — keeping the old one too, as an
early draft of this fix did uniformly for both kinds, left both the
retained old label and the freshly re-pushed one on the stack every
iteration: an unbounded per-iteration duplicate that hung (an
effectively infinite loop, not a clean trap) instead of terminating,
caught by hand-testing a simple bounded `loop`+`br_if`-break before ever
reaching the testsuite (no vendored `.wast` file with a simple bounded
loop currently parses).

This ALSO surfaced a real, since-fixed bug in `iir-to-wasm`'s own
"dispatch-loop" codegen strategy (used for any IIR function with
control flow — COND/if-chains lower to labels/jumps, compiled to one
outer exit `block` wrapping a `loop` wrapping N nested per-block
`block`s + a `br_table`): its branch-depth formulas for "re-enter the
LOOP to redispatch" were empirically tuned against THIS crate's old
double-pop bug, not real WASM semantics — so fixing `execute_branch`
alone made every such branch land one label too shallow, silently
falling into the wrong basic block (confirmed via `lang-aot`'s
`mccarthy_is_uniform_across_every_backend`: the WASM backend computed
`0` instead of `22` for a 2-clause COND). See `iir-to-wasm`'s own
CHANGELOG for that fix's detail; both fixes are needed together for this
PR to be safe to ship.

4 new regression tests (`tests/wasm11_regression.rs`): the exact
`switch.wast` "stmt" shape reproduced in isolation (all 9 real
assert_return cases), a minimal out-of-depth-order `br_table` case, a
bounded `loop`+`br_if`-break that must terminate (not hang), and two
sequential loops on the same call confirming a loop's own label count
stays stable across iterations. Baseline: `assert_return` 12171/12238
(99.4%) → 12215/12238 (99.8%).

## [0.6.3] — 2026-08-13 (WASM07 — two real assert_return correctness bugs + a security-review fix)

Investigating why `wasm-conformance`'s `assert_return` pass rate sat at
98.3% (208 real failures, not opcode-coverage gaps) surfaced two genuine
bugs in this crate, found by running the official spec testsuite, not by
inspection.

- **A WASM function body is itself an implicit outer `block`, whose label
  is the function's own end — this crate never modeled that.** `br`/
  `br_if`/`br_table` at a depth that walks out of every *explicit* block
  (including a completely ordinary, spec-legal bare top-level `(br 0)`,
  meaning "return" — `func.wast`'s own `break-empty`/`break-i32`/etc.
  cases are exactly this) had no label on `ctx.label_stack` to resolve
  against, and traps with a spurious "branch target N out of range"
  instead of returning. Fixed by pushing an implicit label at call entry
  (`arity` = the function's own result count, `target_pc` one past the
  last instruction) — in **both** independent call-entry code paths this
  crate has (`call_function_inner`, used for nested `call`/
  `call_indirect`, and the separate, duplicated dispatch loop inside the
  public `WasmExecutionEngine::call_function`, the one true top-level
  entry point) since neither reuses the other's instruction-decode-and-
  dispatch logic.
- **`call_indirect $type`'s immediate indexes the module's TYPE SECTION —
  a completely different index space from `ctx.func_types`, which is
  indexed by FUNCTION index** (one entry per function, resolved to
  whichever type that function happens to declare; two functions can
  easily share a type, or a type can go unused by any function at all).
  The type check compared the callee's real type against
  `func_types[type_idx]` — an arbitrary, usually-unrelated function's
  type, not the type the call site actually declared — so legitimate
  `call_indirect` calls across dozens of real testsuite cases
  (`load.wast`/`local_tee.wast`/`nop.wast`/`call.wast`'s many
  `as-call_indirect-*` cases, `func.wast`'s `signature-*-duplicate`
  cases) spuriously trapped "indirect call type mismatch" even though the
  callee's real type matched exactly. Fixed the same way
  `struct_field_counts` already solves an analogous "the parser doesn't
  yet surface this to the engine" gap: a new engine-level
  `type_section: Vec<FuncType>` field (empty by default — deliberately
  **not** added to `WasmEngineConfig`, which would have forced every one
  of this crate's ~40 hand-built single/few-function unit-test modules to
  supply it) with a `set_type_section` setter, threaded into
  `WasmExecutionContext.types`. Left unset, the check is skipped
  (permissive — "no type info available" is not the same claim as "the
  type section is empty"); `wasm-runtime`'s real embedding path always
  sets it now (see that crate's own changelog).
- **A security review of this PR's `wasm-runtime` fix (a trapped call must
  not permanently lose an instance's memory/tables — see that crate's own
  changelog) found the identical bug pattern one layer further in.**
  `WasmExecutionEngine::call_function` (the public, top-level entry point)
  also `mem::take`s `self.host_functions` before running, and its own
  restore line (`self.host_functions = ctx.host_functions;`) used to sit
  AFTER `execute_with_context(...)?` — skipped on any trap, exactly the
  same bug the `wasm-runtime` fix addressed one call frame further out.
  Since `wasm-runtime::instantiate()` wires real WASI imports (`fd_write`,
  `random_get`, `clock_time_get`, `environ_get`, `proc_exit`, ...)
  through this exact field, ANY instance that trapped even once would
  silently and permanently lose every WASI import for the rest of its
  life — the `wasm-runtime` fix alone only restored `instance.memory`/
  `instance.tables` (pointer-aliased into the engine, never moved, so
  they survived a trap fine even with the old code) but not
  `host_functions` (genuinely moved via `mem::take`, one layer further
  in). Fixed the same way: capture the `Result` from
  `execute_with_context` first, restore `self.globals`/
  `self.host_functions`/`self.last_gc_state` unconditionally, THEN
  propagate the trap.
- 5 new regression tests (`tests/wasm07_regression.rs`): a bare top-level
  `br 0` returning correctly through both call-entry paths, a
  `call_indirect` case specifically shaped so the old bug (grabbing an
  unrelated function's type) is unmissable, verified both unset
  (permissive) and set (the real check) against the module's own actual
  parsed type section, a case confirming a genuine type mismatch still
  traps once the real type section is wired in, and a host-imported
  function confirmed to survive an unrelated trapped call and remain
  callable afterward.

Together with the matching `wasm-runtime` 0.5.1 fix (a trapped call
losing an instance's memory/tables forever after — see that crate's own
changelog), these three bugs closed 139 of the 208 real `assert_return`
failures the pre-fix baseline had: 12030/12238 (98.3%) → 12169/12238
(99.4%). The remaining 69 are tracked in the session backlog, not blindly
chased further in this PR: a distinct StackUnderflow bug in named-label
depth resolution through 3+ levels of nested blocks (`switch.wast`,
`labels.wast`'s `if`/`if2`, `func.wast`'s `break-br_table-nested-*`),
`comments.wast`'s CR/CRLF line-comment-terminator handling in the `quote`
re-parse path, a handful of NaN-payload-preservation gaps beyond the
0.6.1 fixes, `call.wast`'s `even`/`odd` (the already-documented, accepted
WASM01 trade-off), and `br.wast`'s one remaining case (a genuine
multi-value block signature, already tracked as WASM04) — each is its
own investigation, not a continuation of these three.

178 tests passing (up from 173: 5 new `wasm07_regression.rs` cases),
clippy clean.

## [0.6.2] — 2026-08-13 (WASM01 — a real call-depth guard)

`call_function` had NO limit on WASM call nesting: `call`/`call_indirect`
recurse through this crate's own Rust call stack one level per nested
WASM call, with no counter anywhere. A WASM program that recurses
without bound — the official spec testsuite's own
`call.wast`/`call_indirect.wast`/`fac.wast` deliberately test exactly
this, expecting a clean "call stack exhausted" trap — used to overflow
the REAL host thread stack: an uncatchable process abort, not a WASM
trap any caller could observe or recover from. `wasm-conformance` had to
route around this entirely (never executing `assert_exhaustion`
directives at all) rather than test it, for exactly this reason.

- **New `MAX_CALL_DEPTH` constant and `WasmExecutionContext::call_depth`
  field.** `call_function` is now a thin wrapper enforcing the limit
  around the previously-unguarded body (renamed `call_function_inner`):
  increments on entry, decrements on exit, traps with `"call stack
  exhausted"` instead of recursing further once the limit is hit.
- **A security review of this PR's first version of the constant (200)
  found its justification was wrong, and reproduced a real crash.** It
  reasoned from a *different* crate's measured overflow floor on a
  *different*, lighter recursive path, rather than measuring THIS
  crate's own (heavier) recursion directly — 200 reliably overflowed the
  real host stack in a **debug build** on any thread stack at or below
  ~1 MiB, reproduced with the PR's own regression test. Corrected by
  directly measuring this crate's own debug-build crash floor at a
  documented minimum assumed caller stack (512 KiB — chosen well under
  Rust's own 2 MiB default spawned-thread stack): safe at 120, crashes at
  130. **`MAX_CALL_DEPTH` is 80**, a ~33% margin below that measured
  floor, matching this repo's other recursive-descent crates' own
  25-45%-margin convention (e.g. `mccarthy-lisp-parser`).
- **Known, deliberate trade-off, not swept under the rug**: 80 is
  genuinely too low for 2 legitimate, bounded (terminating) recursion
  cases in the official testsuite's `call.wast` (`even(100)`/`odd(200)`,
  mutual recursion) — they now correctly-but-unfortunately trap "call
  stack exhausted" instead of completing (before this fix, they only
  "passed" by relying on the previously-unguarded, unsafe recursion
  path). The real fix for both safety AND depth capacity is running WASM
  execution on a dedicated thread with a guaranteed larger stack
  (tracked as its own follow-up; blocked on this crate's `*mut
  LinearMemory`/`*mut Table` raw pointers not being `Send`) — shipping
  the safe, conservative value now rather than leaving the host-crash
  risk in place while that larger change is pending.
- 5 new integration tests (`tests/call_depth_guard.rs`): unbounded
  self-recursion, unbounded mutual recursion, ordinary bounded recursion
  well under the limit still works, a depth-counter-leak regression guard
  (a trapped recursive call must not corrupt `call_depth` for a later,
  unrelated top-level call on the same engine), and a committed
  regression test running the exact overflow scenario on a real 512 KiB
  `Builder::stack_size` thread (no `RUST_MIN_STACK` simulation) so this
  can never silently regress back to an unsafe value.
- `wasm-conformance`'s `assert_exhaustion` directives are now genuinely
  executed and graded (previously always `NotYetSupported`, unconditionally,
  specifically because of this gap) — both vendored cases now pass for
  real. See that crate's own changelog.
- **A second security-review round** found the 512 KiB minimum-stack
  assumption behind `MAX_CALL_DEPTH` lived only in a doc comment on a
  *private* `const` — invisible to any downstream consumer reading just
  the public API. Surfaced it directly on the public
  `WasmExecutionEngine::call_function`'s own doc comment instead. Making
  `MAX_CALL_DEPTH` itself caller-configurable (for embedders who know
  they're running on a smaller stack) was judged out of scope for this
  PR — folded into the WASM10 follow-up alongside the dedicated-thread
  work, rather than widening this PR's surface further.

173 tests passing (up from 168), clippy clean. Downstream consumers
(`lang-aot` and the WASM-compiler crates) re-checked: no new failures
attributable to this change.

## [0.6.1] — 2026-08-13 (W05 PR-4 — three float NaN/sign correctness bugs)

Found running the official WebAssembly spec testsuite against this crate
for the first time, via the new `wasm-conformance` harness — these are
exactly the kind of gap that harness exists to surface. All three follow
the same shape: `f32`/`f64` opcode handlers used a Rust `std` float method
directly, whose semantics turned out not to match what the WASM spec
actually requires for that opcode.

- **`f32.min`/`f32.max`/`f64.min`/`f64.max` didn't propagate NaN.** WASM's
  `min`/`max` MUST return NaN if EITHER operand is NaN. Rust's native
  `f32::min`/`max` follow IEEE 754-2008 minNum/maxNum semantics instead:
  "if one of the arguments is NaN, then the OTHER argument is returned."
  `min(NaN, -0.0)` was silently returning `-0.0`. This was, by a wide
  margin, the single largest source of `assert_return` failures in the
  vendored corpus — fixing it alone moved the aggregate `assert_return`
  pass rate from 94.1% to 98.3%. Fixed with explicit NaN and signed-zero
  handling (`min(+0.0, -0.0)` must be `-0.0`; `max` the reverse).
- **`f32.nearest`/`f64.nearest` (round-ties-to-even) didn't preserve the
  sign of a result that rounds to zero.** `nearest(-0.25)` must be `-0.0`,
  not `0.0`, per IEEE 754's own roundTiesToEven rule. Fixed with an
  explicit `copysign` fixup when the rounded result is exactly zero.
- **`f32.ceil`/`floor`/`trunc`/`f64.ceil`/`floor`/`trunc` didn't
  reliably quiet a signaling NaN input.** WASM's spec requires any NaN
  propagated through these to have its quiet bit set, unconditionally.
  The platform libm's own behavior for a signaling-NaN input to
  `ceil`/`floor`/`trunc` turned out to genuinely differ between macOS and
  Linux — confirmed empirically by running the exact same conformance
  suite against both (via a Linux container) and diffing the results bit
  for bit. Fixed by explicitly checking `is_nan()` and returning the
  canonical `f32::NAN`/`f64::NAN` instead of relying on the platform's
  native rounding-function NaN handling at all.

9 new regression tests (2 min/max NaN-propagation, 2 min/max signed-zero,
2 nearest sign-of-zero, 2 ceil/floor/trunc signaling-NaN-quieting, plus
an f64 min/max pair the crate didn't have coverage for at all). 168 tests
passing (up from 159), clippy clean, verified against both macOS and a
Linux container (the platform that originally surfaced bug #3).

## [0.6.0] — 2026-08-03 (W04 — real garbage collection for `gc_heap`)

The WasmGC struct heap (`gc_heap`) was, until now, an append-only arena with
no reclamation — its own doc comment justified this as "bounded by the VM's
instruction budget," a budget that does not actually exist anywhere in this
crate. A long-running WASM-compiled program could allocate without bound.

- **New `gc` module**: a real mark-and-sweep collector over a **tombstone +
  free-list slot arena** — `gc_heap: Vec<GcStruct>` becomes
  `Vec<Option<GcStruct>>` (`Some` = live, `None` = a reclaimed slot ready
  for reuse). Compaction is out of scope: a `WasmValue::Ref(Some(handle))`
  is a `Vec` index (a WASM-spec-mandated representation), so shrinking the
  arena would silently invalidate every other live handle past the removed
  index. No generation tag is needed to guard a reused slot against
  aliasing a stale reference — mark-sweep's ordinary soundness argument
  (nothing left unmarked is reachable, given an exhaustive root walk)
  already rules that out, since every handle a program can hold either sat
  in a scanned root or was freshly minted.
- **Root set**: every `WasmValue::Ref(Some(_))` in `ctx.globals`,
  `ctx.typed_locals`, **every** `ctx.saved_frames[*].locals` (a paused
  caller's locals — missing this would free something a suspended caller
  still references), and the interpreter's operand stack
  (`GenericVM::typed_stack`, via the existing `REF_TAG` convention),
  traced transitively through each object's own fields — cycle-safe by
  construction (a worklist walk with a `marked` visited set, not naive
  recursion). Precise, with no `HeapKind`-style schema needed: a
  `GcStruct` field is a tagged `WasmValue`, self-describing as a reference
  or not.
- **Checked at two chokepoints**: `execute_branch` (every taken
  `br`/`br_if`/`br_table` — a loop's back-edge is a branch to its own loop
  label) and the internal `call_function` helper (every `call`/
  `call_indirect`, nested or not) — both of this crate's independent
  dispatch loops (`GenericVM::execute_with_context` and the hand-inlined
  loop inside `call_function`) route through these two shared functions, so
  instrumenting them covers both loops without adding anything WASM-specific
  to the generic `virtual-machine` crate's own dispatch code. Mirrors the
  existing "safepoints at back-edges and calls" convention rather than a
  per-instruction counter.
- **Adaptive object-count threshold**, mirroring `gc-core::FlatHeap::should_collect`/
  `adapt_threshold` verbatim (just in units of live objects rather than
  bytes, since this heap has no byte-size concept). New `gc-core` path
  dependency, reusing its representation-agnostic `GcProfile`/`GcCycleStats`
  for diagnostic consistency with the native-AOT and `vm-core` GC paths —
  exposed via new `WasmExecutionEngine::gc_live_object_count()` /
  `gc_profile()` accessors (`gc_heap` itself still resets every call, as it
  always has; only the counters are written back).
- Tests: `gc`'s own unit tests (`mark`/`sweep`/`alloc`/threshold-adaptation
  in isolation, including cycle-safety and a saved-caller-frame root) plus
  `end_to_end_loop_reclaims_garbage_and_preserves_kept_object` — a real WASM
  loop, driven through actual bytecode dispatch, that allocates 2000
  objects while keeping exactly one alive, proving both that the kept
  object survives with its field intact and that the live count is
  reclaimed mid-run rather than left to accumulate.
- **Security-review fixes, before this landed**:
  - `sweep` now also drops `gc_heap`'s trailing run of tombstoned slots
    (removing their indices from `free_list` too). Without this, the
    arena's length — and therefore `mark`/`sweep`'s O(len) cost — was a
    monotonically non-decreasing high-water mark for the life of a call: a
    program that transiently spiked the live count high, then settled into
    low-retention churn, would keep paying that peak cost on every
    subsequent collection. Not compaction (no live object moves or is
    renumbered) — only provably-all-garbage trailing capacity is dropped.
  - `gc::alloc`'s new-handle assignment now uses a checked `u32` conversion
    (clean trap on failure) instead of an unchecked `as u32` cast, so an
    eventual overflow (practically unreachable — it implies 100+GB of
    live, uncollected heap) can't silently alias a fresh object's handle
    onto an already-occupied lower index.
  - Tests: `sweep_shrinks_arena_when_everything_is_reclaimed`,
    `sweep_does_not_truncate_past_a_live_object_in_the_middle`.
- See `code/specs/W04-wasm-gc.md` for the full design rationale.

## [0.5.0] — 2026-07-07

### Added — `memory.copy` bulk-memory op (E4-dyn runtime string concat)

The engine now runs the `memory.copy` bulk-memory instruction, which
`iir-to-wasm` emits for runtime `str_concat` (splicing two operands' bytes into a
freshly allocated `[i32 len][bytes]` block).

- **Decoder**: a new `0xFC` two-byte-prefix branch (mirroring the `0xFB` GC prefix)
  reads the sub-opcode and its memory-index immediates, carrying the sub-opcode in a
  plain `Int` operand. `memory.copy` (`0x0A`, two index bytes) and `memory.fill`
  (`0x0B`, one index byte) are decoded; only `memory.copy` is executed.
- **Execution**: a `0xFC` handler pops `size`, `src`, `dest` and delegates to the new
  `LinearMemory::copy`, which bounds-checks both ranges (either out of range traps —
  never panics) and uses `copy_within` for overlap-safe (memmove) semantics. A
  zero-length copy is a no-op even at the end of memory.
- Tests: `memory_copy_decodes_as_one_0xfc_instruction`,
  `linear_memory_copy_moves_bytes_overlap_safe`.

## [0.4.0] — 2026-06-08

### Added — `ref.test` execution (LANG77 / McCarthy L3b-3a-4)

The engine now runs the WasmGC **`ref.test`** type-test op — the primitive
McCarthy `pair?` lowers to ("is this lisp value a cons cell?"):

- The decoder reads the heap-type immediate of `ref.test` (`0xFB 0x14`) and the
  nullable `ref.test null` (`0xFB 0x15`).
- The engine executes them: pop a reference, push `i32 1` if it is a (non-null)
  struct reference, else `0`. Our value model has exactly one struct type
  (`$LispyPair`), so a `struct.new` result — the only `Ref(Some(_))` — is that
  type; a boxed integer (`i31` payload / `I32`) or the null reference yields `0`.
  The `0x15` variant additionally accepts the null reference.

So `pair?(cons)` → 1, `pair?(atom)` → 0, `pair?(nil)` → 0. 3 new tests.

## [0.3.0] — 2026-06-04

### Added — WasmGC struct heap + references (LANG77 / McCarthy L3b-3a-3b)

The engine now *runs* the WasmGC object opcodes, so a lisp **cons cell**
(`$LispyPair`) can be allocated, read, and mutated in-repo — `(CAR (CONS 7 9))`
executes to `7` on the engine.

- **`WasmValue::Ref(Option<u32>)`** — a new value variant for GC references:
  `None` is the null reference (`ref.null` / lisp `nil`), `Some(handle)` indexes
  the engine's GC object heap. Round-trips through the typed stack tagged as
  `anyref` (`0x6E`). (An `i31ref` stays an `I32` payload, as in 0.2.0.)
- **GC object heap** — an append-only arena of `GcStruct { type_idx, fields }`
  on the execution context. `struct.new` allocates and returns a handle; the
  heap persists across calls within a run (a cons built in a callee survives).
  No reclamation: total allocations are bounded by the VM instruction budget.
- **Opcodes executed:**
  - `struct.new <type>` (`0xFB 0x00`) — pops the registered field count of
    values, allocates a `GcStruct`, pushes `Ref(Some(handle))`.
  - `struct.get <type> <field>` (`0xFB 0x02`) — reads a field of a non-null ref.
  - `struct.set <type> <field>` (`0xFB 0x04`) — writes a field of a non-null ref.
  - `ref.null` (`0xD0 0x0F`) — pushes the null reference.
  - `ref.is_null` (`0xD1`) — pops an anyref, pushes `1`/`0`.
- **Decoder** — the `0xFB` block now reads the struct ops' index immediates
  (type/field) into a `Gc { sub, type_idx, field_idx }` operand; `0xD0` consumes
  its one-byte heap-type immediate so it isn't mis-decoded.
- **`WasmExecutionEngine::set_struct_field_counts`** — registers struct type
  field counts (the parser doesn't yet surface struct types to the engine; this
  is populated from the parsed module in L3b-3a-3c).
- **Defaults** — nullable reference locals (`anyref`, `structref`) now default
  to `Ref(None)`; `i31ref` stays `I32(0)`.

Every failure mode (unknown type/field index, null dereference, missing arity,
type mismatch, unknown sub-opcode) is a **clean trap**, never a panic. 10 new
tests (cons/car/cdr round-trips, `struct.set` mutation, `ref.is_null`, and the
null-deref / out-of-range / missing-arity traps).

Also removed a pre-existing dead no-op branch in `decode_immediates`.

## [0.2.0] — 2026-06-05

### Added — WasmGC `i31` execution (LANG77 / McCarthy L3b-3a-3a)

First step of executing WebAssembly **GC** opcodes (so the McCarthy-Lisp → wasm
value model can be run in-repo, not just emitted):

- **`decode_function_body`** now decodes the two-byte `0xFB` GC prefix: it reads
  the sub-opcode byte and carries it in the instruction's operand (the MVP
  opcode table is single-byte and doesn't know `0xFB`). Previously a `0xFB`
  stream would be mis-decoded as separate single-byte instructions.
- The engine executes the `i31` boxing pair: **`i31.new`** (`0xFB 0x1C`) and
  **`i31.get_s`** (`0xFB 0x1D`). An `i31ref` is represented as its plain `i32`
  payload on the value stack (the small lisp integers we box never need the
  reference identity), so both are stack-identity no-ops — the integer passes
  straight through. `i32.const 42 → i31.new → i31.get_s` returns `42`.
- Unimplemented GC sub-opcodes (`struct.*`, `ref.*`) are a clean `Err`, not a
  panic — they land with the GC-object-heap slice (L3b-3a-3b).

3 new tests: the two-byte decode, the i31 box/unbox round-trip executed on the
engine, and the unsupported-opcode clean error.

## [0.1.1] — 2026-05-13

### Fixed

- **`br_table` handler** — corrected out-of-range index handling: when the
  branch index exceeds the target table length, the default label is now
  used (as per the WASM spec).  Previously the handler indexed past the
  end of the table, causing a panic.
- **`call_indirect` stub** — added a graceful error message for
  `call_indirect` (not yet supported) so the engine reports the opcode name
  instead of panicking on an unknown byte.
- **`i32.const` sign extension** — `i32.const` operands are now
  LEB128-decoded as signed (i32) and zero-extended to i64, matching the WASM
  spec for negative literal values.
- Various opcode stubs improved to match the WASM binary format byte layout.

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
