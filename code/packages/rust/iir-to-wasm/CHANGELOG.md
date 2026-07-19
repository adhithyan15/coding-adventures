# Changelog — iir-to-wasm

## [0.39.0] — 2026-07-19 (LANG-FULL E4-dyn E4d-3b: runtime `str_cmp`)

Give `str_cmp` a **runtime path** on WASM, mirroring the runtime `str_eq` that
already landed. A `str_cmp` whose operands are both compile-time literals still
folds to a `-1`/`0`/`1` constant. But when an operand is a runtime string handle
(a function parameter, a call result, a branch-selected slot), there is no
compile-time answer — the two `[i32 len][bytes]` blocks must be compared at run
time. Previously that shape errored (`str_cmp left source … is not a direct
str_const local`); now it lowers to a `call` of a self-contained in-module
`$__str_cmp(i32,i32) -> i32` helper.

- **`$__str_cmp` helper.** A shared-prefix scan (`n = min(len a, len b)`, then a
  byte-by-byte `i32.load8_u` compare — **unsigned**, so bytes ≥ 0x80 sort above
  ASCII) with a length tiebreak (a prefix sorts before the longer string),
  returning `-1`/`0`/`1`. The result is **byte-identical to the folded literal
  path** (`left.bytes.cmp(&right.bytes)`, Rust slice ordering). Emitted once per
  module (gated by a new `uses_str_cmp_runtime` feature) and appended directly
  after the `$__str_eq` helper; `str_cmp_fn_idx` accounts for that preceding slot.
  In-module rather than a host import for the same reason as `$__str_eq`: string
  ordering is pure computation, so the emitted WASM stays self-contained (mirrors
  the native/LLVM `__twig_str_cmp`).
- **Signed widening.** Unlike `str_eq`'s `0`/`1` result (zero-extended), a
  `str_cmp` result is a **signed** `-1`/`0`/`1`, so widening to an `i64` result
  slot uses `i64.extend_i32_s` — a `-1` stays `-1`, matching the folded
  `encode_i64_const(-1)` path exactly.
- A folded-literal operand paired with a runtime operand is promoted to a runtime
  `[i32 len][bytes]` block (same `lay_runtime_str_block` path as `str_eq`) so it
  presents a real header to the helper.
- No validator change: `str_cmp` already accepted two `Var` operands (it never
  enforced literals); only the stale "materialises literal ordering" comment was
  refreshed.
- Tests: `tests/str_cmp_runtime.rs` runs the emitted module on the real
  `WasmRuntime` and checks equal / first-differing-byte / prefix / byte-value /
  empty-string cases against the `left.bytes.cmp(&right.bytes)` oracle.

Runtime `str_slice`/`str_index` over promoted operands remain the last deferred
E4d-3b pieces.

## [0.38.0] — 2026-07-12 (LANG-FULL E6d-3b: nil `const 0 : ref<…>` → `ref.null`)

The `const` lowering's nil special-case previously required an **empty** source
operand (`const : ref<LispyPair>` → `ref.null`). But `make_nil` and the E6d-3a
`list` desugar emit nil as `const 0 : ref<LispyPair>` — **with** an `Int(0)`
sentinel source — which fell through to `i32.const 0`. So nil became an `i32(0)`,
and `is_null` (`ref.is_null`) never recognised it: `null?` on the empty list, and
any cons-walk (`length`, …) that must stop at the terminator, failed — the walk
ran past the end into `struct.get` on an `i32` (trap: "expected a struct
reference, got I32(0)"). Fix: a `ref<…>`-typed const is nil when its source is
empty **or** `Int(0)`, so both forms emit `ref.null`. This aligns WASM with the
CLR backend, which already lowers `const 0 : ref<…>` to `ldnull`. (Car/cdr on a
list never dereference the nil tail, so E6d-1/E6d-3a were unaffected and stay
green.)

## [0.37.0] — 2026-07-11 (LANG-FULL E6d-2a: i64-width `box`/`unbox`)

`box`/`unbox` become i64-slot aware for E6d-2 dynamic arithmetic (which works uniformly in i64). `unbox` sign-extends `i31.get_s` (i32) with the new `i64.extend_i32_s` (0xAC) when the destination rides an i64 register; `box` narrows an i64 source with `i32.wrap_i64` before `ref.i31`. Existing i32-atom lisp box/unbox are unchanged (the guard is `slot_is_i64`).

## [0.36.0] — 2026-07-10 (LANG-FULL E4-dyn — E4d-BA-arr: `array<str>` elements)

BASIC string arrays (`DIM A$(n)`) store an E4-dyn runtime string **handle** per
element. A `str` handle on WASM is a 4-byte `i32` linear-memory offset (unlike the
8-byte `i64`/`f64` elements E5 arrays used so far), so `array<str>` is a flat block
of i32 handles.

- **`wasm_array_elem`** gains a `"str" => (I32, 4)` branch; `alloc_array` sizes the
  block by the 4-byte element, and `array_get`/`array_set` select `i32.load`/
  `i32.store` for a `str` element.
- **`collect_runtime_str_vars`** now promotes a folded str literal used as the
  *value* of an `array_set` to a runtime-block handle — the same treatment call
  arguments already get. Without it, `array_set` would store the val local's
  uninitialised `0` (a folded literal's handle lives only in the compile-time
  `string_literals` table, never in its runtime local), and a later `array_get` +
  `print_str`/`str_concat` would read the module header as a bogus length and trap
  (`out of bounds memory.copy`).
- **Validator** (`validate.rs`) accepts a `str` type_hint on `array_get`/`array_set`.

**Tests:** `str_array_uses_i32_element_store` (an `array<str>` element `array_set`
emits `i32.store`), `str_array_elem_is_i32_4_bytes`.

## [0.35.0] — 2026-07-08 (LANG-FULL tail: runtime `str_eq` via a self-contained in-module `$__str_eq` helper)

Adds a runtime path for `str_eq`. Previously `str_eq` only supported the case where
BOTH operands are folded literals (constant-folded at compile time) and errored the
moment either operand was a runtime string handle — a parameter, a call result. That
error (`str_eq left source "a" is not a direct str_const local`) blocked the Twig cell
`(define (same a b) (if (string=? a b) 42 0)) (same "OK" (string-append "O" "K"))` on
WASM. This is the final lang-full string-tail item.

- **In-module helper** (`build_str_eq_helper`): emits a self-contained
  `$__str_eq(i32, i32) -> i32` WASM function — a header-length check followed by a
  byte-compare loop over the two `[i32 len][bytes]` blocks (its own `len`/`i` scratch
  locals; structured `loop`/`if`/`br`). It is **not** a host import: unlike I/O, string
  equality is pure computation, so keeping it inside the module makes the WASM
  self-contained (mirrors the native/LLVM `__twig_str_eq` archive helper). Emitted once
  per module, gated by the new `uses_str_eq_runtime` feature, appended after all IIR
  functions (index `fn_idx_base + module.functions.len()`), its FuncType registered with
  the same `+ struct_type_offset` convention as host-import types.
- **`str_eq` lowering**: keeps the both-literal compile-time fold; otherwise loads both
  operand handles and `call`s `$__str_eq` (widening the i32 result to i64 when the dest
  slot is i64).
- **Operand promotion**: when a runtime `str_eq` has a folded-literal operand, that
  operand is promoted to a runtime block in `collect_module_features` (via
  `lay_runtime_str_block`) so it presents a real header to the helper — covering the
  mixed `string=? x "OK"` case, not just two-param comparisons.
- Tests: `tests/str_eq_runtime.rs` (equal / same-length-different-bytes / different-length
  / empty-string cases, run-verified via `wasm-runtime`).

## [0.34.0] — 2026-07-08 (LANG-FULL tail: promote a folded `str_concat`/`str_slice` result passed across a call)

Extends the 0.33.0 `str_const`-across-call promotion to folded `str_concat` and
`str_slice` results. A `str_concat`/`str_slice` whose operands are all compile-time
literals folds to a raw data-segment offset (its length lives only in the compile-time
`string_literals` table). When that folded result is handed to a *callee* — e.g.
`(strlen (substring …))` or a `let*`-derived `string-append` fed to a function — the
callee has no compile-time length and reads a `[i32 len][bytes]` header at run time, so
it must be promoted to a runtime block exactly like `str_const`. Before this fix those
programs returned **72** (`'H'`, the first data byte read as a bogus length).

- `collect_runtime_str_vars`: the "promote a folded literal used as a call arg" rule now
  covers `str_concat`/`str_slice` dests, not just `str_const` (renamed the tracked set to
  `folding_str_dests`). A `str` value that is instead a live handle (param/call result/
  branch-selected) is deliberately excluded — it already carries a runtime block.
- `collect_module_features`: a promoted `str_concat`/`str_slice` folded result now lays
  down a static length-prefixed runtime block, via the new shared `lay_runtime_str_block`
  helper (the `str_const` path was refactored onto it too, so identical literals across
  ops share one block). No bump allocator is involved — the block is baked into the data
  segment, like the `str_const` runtime blocks.
- The `str_concat`/`str_slice` lowerings emit the runtime-block **handle** (not the raw
  offset) when the dest is promoted. The non-folded runtime `str_concat` path is
  unchanged — it already bump-allocates a header'd block, so its base is a valid handle.
- Tests: `str_concat_result_passed_to_strlen_returns_length` and
  `str_slice_result_passed_to_strlen_returns_length` (run-verified via `wasm-runtime`).

## [0.33.0] — 2026-07-07 (LANG-FULL tail: promote a `str_const` literal passed across a call)

Fixes a wrong-value bug where a string literal handed to a *callee* was read with a
bogus length. On wasm a single-block `str_const` literal takes the folded fast path:
its handle is the RAW-byte data offset and its length is compile-time metadata. But a
callee has no compile-time length for its string parameter — its `str_len`/`str_concat`
/`str_slice`/`str_eq` read a length-prefixed `[i32 len][bytes]` block header at run
time. Passing the raw-byte handle made `str_len` read data bytes as a length (e.g.
`(strlen "HELLO")` returned 72 = `'H'`).

- `collect_runtime_str_vars` now also promotes a `str_const` destination that appears as
  a **call argument** (not only strings assigned in >1 basic block). A promoted literal
  gets a real `[i32 len][bytes]` runtime block whose handle points at the length prefix,
  so the callee reads the correct length. Single-block literals *not* passed to a callee
  keep the folded fast path unchanged.
- Test: `str_literal_call_arg.rs` — `strlen("HELLO")` across a call returns 5.

## [0.32.0] — 2026-07-07 (LANG-FULL E4-dyn: runtime `str_concat` in linear memory)

`str_concat` gains a runtime-operand path (it was literal-fold-only). When `dest`
isn't in the module string table (an operand is a runtime handle), the concat is
built entirely in wasm:

- Bump-allocate a `[i32 len][bytes]` block from `__array_bump` (a `str_concat` op now
  triggers the same memory + bump-global injection as an array op / `input_str`).
- `i32.store` the length header (`la + lb`), then splice each operand's bytes with a
  `memory.copy` (bulk-memory `0xFC 0x0A`) — `new+4 ← a+4` (`la` bytes), then
  `new+4+la ← b+4` (`lb` bytes). Each length is re-read from its header with
  `i32.load`, so the sequence needs **no scratch locals** (only the destination local
  is written).
- Both-literal concats keep the compile-time fold to a data-segment offset.
- New codegen helpers: `encode_i32_store` (0x36) and `encode_memory_copy`
  (`0xFC 0x0A 0x00 0x00`). Executed by `wasm-execution` ≥ 0.5.0.

## [0.31.0] — 2026-07-07 (LANG-FULL E4-dyn: BASIC string `INPUT A$`)

`input_str` (BASIC string `INPUT A$`) now lowers on WASM — the final backend, so
`INPUT A$` runs on all seven.

- **Validator** (`validate.rs`): `input_str` added to the `call_builtin` whitelist;
  the `str`-type gate now also accepts `str` on `call_builtin` and `mov` (was
  `str_const`/`str_concat`/`str_slice`/`call`/`ret` only).
- **Lowering** (`lower.rs`): a `str` value is an i32 **handle** — a linear-memory
  offset of a `[i32 len][bytes]` block. `call_builtin "input_str"` bump-allocates a
  `[i32 len][MAX=256 bytes]` region from `__array_bump` (its base is the handle),
  then calls the new `env.__input_str(i32 block, i32 max) -> ()` host import, which
  writes the whole block (length header + bytes) into linear memory. Single `call`,
  no `i32.store` in codegen — the host owns the writes. `print_str` reads the length
  via `i32.load` at the handle. `input_str` also injects linear memory + the
  `__array_bump` global (a pure INPUT-A$ program has no array op). New feature flag
  `uses_input_str` + import index threaded through the lowering, mirroring
  `input_i64`. MAX is 256 because the module's memory is a single fixed 64 KiB page.
- Test `input_str_lowers_and_declares_env_import`.

## [0.30.0] — 2026-07-04 (LANG-FULL E4-dyn E4d-3b: runtime string as return value / call result)

Extended the E4-dyn runtime-string support so a runtime string that arrives as a
function **return value**, **call result**, or **parameter** — not only a
branch-selected local slot — is a first-class value, mirroring the LLVM E4d-2b
change. This lets an ALGOL `string procedure` (which returns a runtime string)
run on the WASM column.

WASM already types a `str` as an `i32` handle everywhere (`hint_to_value_type`),
so a `str` parameter, a `str` return type, and a `call` whose result is `str`
already lowered correctly — no boundary/typing change was needed. The only gap
was that `print_str` / `str_len` took the runtime header-read path only for a
promoted `runtime_str_vars` slot; a call result / return value / parameter (in
neither `runtime_str_vars` nor the compile-time `string_literals` map) fell to
the literal fast path and errored.

- **`print_str`**: the runtime-path guard became
  `runtime_str_vars.contains(v) || !string_literals.contains_key(v)`, so any
  string without a compile-time literal entry reads its length from the
  `[i32 len][bytes]` block header (`i32.load` at the handle) and passes
  `handle + 4` + that length to `env.__print_str`.
- **`str_len`**: gained the same runtime branch — `i32.load` the length at the
  handle (widened with `i64.extend_i32_u` for an i64 dest) instead of folding a
  compile-time constant.

The `lang-aot` ALGOL string-procedure matrix cell adds the `Wasm` column
(verified end-to-end in-process via `wasm-runtime`).

## [0.29.0] — 2026-07-03 (LANG-FULL E4-dyn E4d-3: runtime branch-selected strings)

Gave the WASM backend a **runtime** string representation, mirroring the LLVM
E4d-2 lowering, so a string chosen by control flow — not foldable to one
literal at compile time — runs correctly.

**The problem.** The pre-existing string machinery keyed every string to one
compile-time `{offset, len}` by its destination variable. That is exact for a
straight-line program (even `s := "OK"; s := "NO"` folds, last-writer-wins),
but wrong the moment control flow chooses the value:

```basic
10 INPUT N
20 IF N > 0 THEN 50
30 LET A$ = "LO"      ← str_const A$ "LO"   (block B1)
40 GOTO 60
50 LET A$ = "HI"      ← str_const A$ "HI"   (block B2)
60 PRINT A$
```

`A$` is the dest of `str_const` in two different basic blocks, so a by-dest
table can only remember one literal and its length would be wrong whenever the
other branch runs and the two differ in length.

**The fix — a runtime handle.** A string variable assigned by `str_const` in
**more than one basic block** is promoted to carry an i32 **handle** = the byte
offset of a length-prefixed block `[i32 len (little-endian)][bytes]` in linear
memory:

- `collect_runtime_str_vars` computes the promoted set with the same
  basic-block rule as `iir-to-llvm`'s `collect_slot_vars` (a `label` starts a
  block; a `jmp*`/`ret*` ends one), so both backends promote identical
  variables from identical IIR.
- `collect_module_features` lays down one length-prefixed block per distinct
  promoted literal (deduplicated by text) in the string data segment and
  records its offset.
- `str_const` of a promoted var stores its block offset (the handle); every
  other `str_const` keeps the folded raw-byte-offset fast path.
- `print_str` of a promoted var reads the length back with `i32.load` at the
  handle and passes `handle + 4` (the bytes) + that length to
  `env.__print_str(ptr, len)` — the WASM sibling of LLVM's `inttoptr` + `load`
  + `getelementptr … i64 8` sequence.

Single-assignment (and straight-line-reassigned) strings are unchanged: zero
behavioural difference for every existing WASM string cell.

Added `encode_i32_load` (opcode `0x28`) to `codegen.rs`. New unit tests
(`e4dyn_branch_selected_string_emits_runtime_handle_and_load`,
`e4dyn_single_block_string_keeps_literal_fast_path`,
`e4dyn_straight_line_reassignment_is_not_promoted`) assert the emitted wasm.
The `lang-aot` E4-dyn foothold matrix cell now proves this end-to-end on the
Wasm column (via the in-process `wasm-runtime` + `env.__print_str` host).

## [0.28.0] — 2026-06-30 (BA-INPUT: `input_i64` → `env.__input_i64` host import)

Added `"input_i64"` to `CALL_BUILTIN_SUPPORTED_NAMES` in `validate.rs` and
implemented the lowering in `lower.rs`:

- A new `input_i64_fn_idx: Option<u32>` parameter threads the host import index
  through `emit_instr` (same pattern as `getchar_fn_idx`).
- The builtin table adds `env.__input_i64` with signature `() -> i64` when
  `input_i64` is used; the host runtime provides this import.
- The lowering emits `call $env.__input_i64; local.set $dest` — no widening
  needed since `env.__input_i64` already returns `i64`.

Enables `10 INPUT X\n20 PRINT X` to run on the WASM backend in
`matrix_every_proven_cell_agrees`.

## [0.27.0] — 2026-06-30 (LANG-FULL — boolean i64/i32 width-coherence for `and`/`or`/`xor`)

Fixed a WASM type-validity bug in the `and`/`or`/`xor` lowering arm that
surfaced when ALGOL string-comparison boolean chains (type_hint "i64") fed
into a logical `and` with type_hint "bool".

**Root cause**: WASM is strictly typed. When `cmp_ne` with type_hint "i64"
produces an i64 local, feeding it into `and` with type_hint "bool" caused
the backend to select `I32_AND` (based on the instruction's type_hint), but
both operand locals were i64 → WASM validation error.

**Fix** (`lower.rs`, `and`|`or`|`xor` arm):
- Compute `use_i64 = type_is_i64 || (r1_is_i64 || r2_is_i64)` — upgrade to
  i64 arithmetic when EITHER operand's local is i64, even if the instruction
  type_hint is "bool".
- Emit `i64.extend_i32_u` for narrower operands BEFORE both locals are
  pushed, so the WASM stack types match the chosen opcode.
- After the i64 op, emit `i32.wrap_i64` when the dest local is i32 (e.g. a
  "bool"-typed result slot) so the store type is consistent.

This also fixes the mixed case (`a and (not b)`) where `a` is i32 but
`not b` (a `cmp_eq` result) is i64 — previously the i64 check was only on
r1, missing the case where r2 is the i64 operand.

## [0.25.0] — 2026-06-29 (LANG-FULL BA-pow — `f64_pow` WASM lowering)

Added `env.__pow(f64, f64) -> f64` host import (slot 4, after getchar) to
`ModuleFeatures`/`collect_module_features` scan and the import section.
`lower_function` and `emit_instr` both receive `pow_fn_idx: Option<u32>` so
both the dispatch-loop path and the linear emission path can emit:
`local.get base; local.get exp; call env.__pow; local.set dest`.

All notable changes to this crate are documented here.  The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.26.0] — 2026-06-29 (LANG-FULL AL8-arctan — `f64_atan/f64_tan` via host imports)

WASM has no built-in atan/tan opcodes — same pattern as the AL8-trig imports.
Two new host-imported functions added to the import table:
- `env.__atan` (index 8)  — `f64 → f64` host import for inverse tangent
- `env.__tan`  (index 9)  — `f64 → f64` host import for tangent

`collect_module_features` detects `f64_atan` / `f64_tan` usage; two new booleans
(`uses_f64_atan`, `uses_f64_tan`) control conditional import injection.
`emit_instr` dispatches via `atan_fn_idx` / `tan_fn_idx` with `local.get + call + local.set`.
The test host (`lang_matrix.rs`) registers `AtanFunc` / `TanFunc` resolvers that call
Rust's `f64::atan` / `f64::tan`.

## [0.25.0] — 2026-06-28 (LANG-FULL AL8-trig — transcendentals via host imports)

WASM has no built-in sin/cos/log/exp opcodes; the four ops are resolved via
host-imported functions `env.__sin`, `env.__cos`, `env.__ln`, `env.__exp`
(each `f64 → f64`).  `collect_module_features` detects usage; Step 3 assigns
import indices; Step 4 injects `FuncType` + `Import` entries; `emit_instr` emits
`local.get arg; call <import_idx>; local.set dest`.  The test host (`PrintHost`
in `lang_matrix.rs`) resolves these to Rust's `f64::sin/cos/ln/exp`.

## [0.24.0] — 2026-06-28 (LANG-FULL AL8-sqrt — `f64_sqrt` lowers to `f64.sqrt`)

Added `F64_SQRT = 0x9F` constant (WASM MVP opcode) to `codegen.rs` and an
`f64_sqrt` dispatch arm in `lower.rs`.  Emission: `local.get r; f64.sqrt;
local.set rd` — a single WASM MVP instruction, no imports or feature gates
needed.  NaN propagates; negative input returns NaN (IEEE-754).

## [0.23.0] — 2026-06-28 (LANG-FULL E4 — literal string comparison on WASM)

The WASM backend now accepts and lowers literal-only `str_cmp`, materialising
`-1`, `0`, or `1` as an `i64.const`/`i32.const` from literal byte metadata.

## [0.22.0] — 2026-06-28 (LANG-FULL E4 — literal string slice on WASM)

The WASM backend now accepts and lowers literal-only `str_slice`. The module
feature pass derives the sliced byte range from prior string metadata and
constant start/end bounds, appends it to the string data segment, and binds the
slice destination to that offset so downstream `str_index` still emits the same
guarded `i32.load8_u` path.

## [0.21.0] — 2026-06-27 (LANG-FULL E4 — literal string index on WASM)

The WASM backend now accepts and lowers direct-literal `str_index`:

- `str_index s, i` over a prior `str_const` emits a literal-length guard
  (`idx >=u len` -> `unreachable`) and then loads the byte with `i32.load8_u`
  from the string data segment.
- `i64` results zero-extend the loaded byte with `i64.extend_i32_u`.
- Twig `(string-ref "ABC" 1)` now returns `66` in the WASM matrix column.
- Non-literal string values and broader dynamic string algebra remain outside
  this slice.

## [0.20.0] — 2026-06-27 (LANG-FULL E4 — literal string metadata on WASM)

The WASM backend now accepts and lowers `str_len`, `str_eq`, and literal
`str_concat` for direct string literals:

- `str_len s` over a prior `str_const s = "..."` materialises the stored byte
  length as an `i64.const`/`i32.const`, depending on the destination local type.
- `str_eq a, b` over two prior literal `str_const` values materialises `1`/`0`
  from the stored bytes, again matching the destination local type.
- `str_concat a, b` over two prior literal `str_const` values creates another
  string-data entry whose bytes can feed the same `str_len`, `str_eq`, or
  `print_str` metadata path.
- The lowering reuses the same linear-memory data metadata introduced for
  `print_str`; no dynamic runtime string representation is exposed.
- Richer string ops (`str_index`) and non-literal string values still fail
  closed.
- Unit coverage now locks the validator acceptance path, and the lang matrix
  proves `(string-length "HELLO")` returns `5` and
  `(string=? "HELLO" "HELLO")` returns `1`, and
  `(string-length (string-append "AB" "CDE"))` returns `5` in the WASM column.

## [0.19.0] — 2026-06-27 (LANG-FULL E4 / BA4 — BASIC string literal PRINT on WASM)

The WASM backend now lowers the E4 literal-output pair:

- `str_const` stores printable ASCII literal bytes in a linear-memory data
  segment and materialises the string value as an `i32` byte pointer.
- `print_str` calls the new host import `env.__print_str(i32 ptr, i32 len)`.
- Modules using string output get a one-page linear memory; if E5 arrays share
  that memory later, the `__array_bump` global starts after the string data.

The literal-output scope is intentionally narrow. `str_len`, `str_index`,
`str_concat`, and `str_eq` still fail closed until the full byte-string runtime
lands. Verified by the backend tests and by RUNNING the lang matrix row
`10 PRINT "HELLO"` on the in-repo `wasm-runtime`.

## [0.18.0] — 2026-06-26 (LANG-FULL BA1-WASM — GOSUB/RETURN dispatch-loop fix)

**Root-cause fix:** the WASM dispatch-loop lowering used a depth formula that
assumed the last basic block never needs to restart the dispatch loop.  Dartmouth
BASIC's `GOSUB`/`RETURN` lowers the return-address dispatch (`RETURN` → computed
`goto`) via `jmp_if_true` chains, and those chains live in `line_N` — which the
compiler emits last in the flat instruction stream, making it the last basic block.

When `bb_{N-1}` (the last block) is entered via the `br_table`, `execute_branch`
truncates the `label_stack` to `[$exit, $dispatch]`, then the matching `END`
instruction pops `$dispatch`, leaving only `[$exit]`.  A `jmp_if_true` depth=1
inside the `if` block exits `$exit` (terminating the function without pushing a
return value), which causes `wasm-execution` to raise `StackUnderflow` when it
tries to collect the `i64` return.

**Fix:** in `lower_function`, after `split_into_blocks`, check whether the last
basic block contains any `jmp_if_true` or `jmp_if_false`.  If so, append a
sentinel empty block so the real last block is now `bb_{N-1}` (second-to-last),
where the formula `n_blocks - block_idx - 1` correctly resolves to the `$dispatch`
loop label.  The sentinel `bb_N` is unreachable — `dispatch_reg` is never set to
`N`.  All 107 existing iir-to-wasm tests pass unchanged; the matrix guard
`matrix_every_proven_cell_agrees` passes with Wasm added to both BA1 cells.

## [0.17.0] — 2026-06-23 (LANG-FULL E8 — numeric conversions integer↔real, PR-3)

WASM lowering for the three E8 conversion opcodes (vm-core 0.9.0 gave the
reference semantics; spec `lang-full-e8-numeric-conversions.md`). All three are
wasm-MVP opcodes — no feature gate.

- **`int_to_real`** → `f64.convert_i64_s` (0xB9).
- **`real_to_int_trunc`** → `i64.trunc_f64_s` (0xB0) — truncate toward zero.
- **`real_to_int_floor`** → `f64.floor` (0x9C) then `i64.trunc_f64_s`.
- The dest local is typed `f64`/`i64` automatically by `infer_local_type_hints`
  (it reads each var's type from the producing instruction's `type_hint`).
- **Trap matches the VM for free.** The **non-saturating** `i64.trunc_f64_s`
  traps on NaN/±∞/out-of-`i64`-range — exactly vm-core's `real_to_i64_checked`
  fail-closed contract. (The saturating `i64.trunc_sat_f64_s` would clamp and
  silently diverge, so it is deliberately *not* used; no explicit guard needed.)
- Verified by RUNNING on a real wasm runtime (`tests/e8_conversions.rs`): the
  integer→real→integer round trip `floor(int_to_real(45) − 2.7)` ⇒ 42, plus
  trunc-toward-zero (`44 + trunc(-2.9)` ⇒ 42) and floor-toward-−∞
  (`45 + floor(-2.5)` ⇒ 42, the negative-rounding difference).

## [0.16.0] — 2026-06-21 (LANG-FULL E5 — arrays via linear memory + explicit bounds-trap)

The four E5 array opcodes now lower to the **static** array model in WASM **linear
memory** — the same length-prefixed layout + explicit out-of-bounds trap as the
LLVM backend (WASM, like a native target, has no managed runtime to bounds-check
for it). Per array, memory holds `[i64 length][elem 0][elem 1]…`; the *handle* is
the byte offset of the block.

| IIR op | WASM |
|--------|------|
| `alloc_array dest <- count` (`array<T>`) | `dest = global.get __array_bump`; advance bump by `8 + count*elemsize`; `i64.store` the length header |
| `array_get dest <- handle, idx` | `idx >=u len` → `if … unreachable`; then `i64.load`/`f64.load` at `wrap(handle)+idx*elemsize` offset 8 |
| `array_set handle, idx, val` | same bounds trap; `i64.store`/`f64.store` |
| `array_len dest <- handle` | `i64.load` the header at `wrap(handle)+0` |

- **Bump pointer**: a synthetic mutable `i64` global `__array_bump` (injected into
  the global section when a module uses any array op, init 0) hands each
  `alloc_array` a fresh region, so multiple arrays coexist. Using arrays also
  triggers the 1-page linear `(memory …)` (as the Brainfuck tape does).
- **Bounds check**: one **unsigned** compare `i64.ge_u` (0x5A — newly added) traps
  via `unreachable` on both a `>= len` index and a negative one (a negative i64 is
  a huge unsigned value). The wasm twin of LLVM's `icmp uge` + `llvm.trap`.
- Element type from `T`: `i64`/`f64` elements (the ALGOL `integer`/`real` arrays);
  the handle rides an `i64` register (a byte offset) wrapped to `i32` for
  addressing, exactly like the byte-tape base. New `i64.load`/`i64.store`/
  `f64.load`/`f64.store` encoders (with an offset immediate).
- 3 new unit tests (memory + bump global + trap + load/store opcodes, handle
  typing, `f64` element ops). Verified end to end: a straight-line ALGOL array
  program runs on the in-repo `wasm-runtime` → exit 42.

Scope: `i64`/`f64` elements (the ALGOL array element types). Narrower element
widths and multidimensional arrays are follow-up; native x86_64/aarch64 arrays
land in E5 PR-4c.

## [0.15.1] — 2026-06-20 (LANG-FULL E3 — f64 regression tests; ALGOL reals run on WASM)

### Added — f64 `mul`/`div`/comparison op-selection tests (no code change)

The WASM backend **already** executes `f64` correctly: its typed-local model
carries an `f64` variable in an `F64` local, and `hint_to_value_type`/the op
tables select `f64.mul`/`f64.div`/`f64.eq`/`f64.lt` from the `f64` type_hint.
So — unlike the LLVM and JVM backends, whose uniform-`i64`/`long` slot models
needed rework (LANG-FULL E3-codegen-slots) — WASM needed **no change** to run
ALGOL 60 reals. This release adds `emit_f64_mul_div_opcodes` and
`emit_f64_comparison_opcodes` tests to lock that op-selection against future
regression, alongside the executed cross-backend proof (`lang-aot`'s
`lang_matrix.rs` now runs the ALGOL real programs on the WASM column).

## [0.15.0] — 2026-06-15 (LANG-FULL E2 integration — compute-wide + mask)

### Changed — narrow unsigned types ride the i64 register model

The v0.14.0 E2 masking typed a narrow op at its natural WASM width (`u8` → `i32`)
and masked with `i32.and`. That is only valid when the **operands** are also
`i32`. A real frontend's value model isn't: Nib (and the other LANG languages)
materialise every `const`/`let`/`ret` as `i64` for module uniformity and carry
the narrow width *only on the arithmetic op*. So a Nib `u8` add emitted
`i32.add` over two `i64` locals → **`type mismatch: expected i32, got I64`** at
run time. (The v0.14.0 unit tests never caught it because they built
self-consistent narrow-width modules — every operand `u8` too.)

The fix makes narrow **unsigned** integers (`u4`/`u8`/`u16`/`u32`) use the **i64
register model**, exactly like the vm-core/jit-core/LLVM/native backends:

- `hint_to_value_type`: `u4`/`u8`/`u16`/`u32` → **I64** (were I32). Signed narrow
  (`i8`/`i16`/`i32`) and `bool` keep I32 (no frontend emits narrow signed
  register arithmetic; booleans are i32 0/1).
- New `uses_i64_register(hint)` gates op selection — narrow unsigned now pick
  `i64.*` opcodes (add/sub/mul/div/mod/and/or/xor/shl/shr, `const`, `neg`, `not`,
  and the relational ops) over their i64-slot operands.
- `emit_wasm_width_mask`: emits `i64.const <mask>; i64.and` (was `i32.*`), and now
  covers `u32` (`0xFFFFFFFF`) too — within i64 a 32-bit op no longer self-wraps.
- Relational ops use signed `i64.*` compares; a masked narrow value is in
  `[0, 2ⁿ)` (positive in i64), so the signed result is the correct unsigned one.

So `200u8 + 100u8` wraps to `44` **with i64 operands** — the shape a frontend
actually emits. Verified end-to-end on the real `wasm-runtime` by a new test
(`u8_op_over_i64_operands_wraps_on_real_wasm`) that builds `200i64 + 100i64 : u8`
and compares `== 44` in-register (→ 1). The full `lang-aot` matrix (Brainfuck,
BASIC, Twig, i64-Nib) and all wasm consumers stay green — the change is a no-op
for every i64/u64 program. Two unit tests updated to the i64 model
(`unsigned_8_16_32_map_to_i32`, `emit_i32_div_u_opcode`).

This is the first of the 3 stack-backend reworks (wasm, then jvm, cil) the E2
Nib integration needs; the other 4 backends already compute wide and mask.

## [0.14.0] — 2026-06-14 (LANG-FULL E2 — register width & wrap, backend 3 of 6)

### Added — narrow-width arithmetic wraps mod-2ⁿ on real wasm

WASM maps every narrow integer type to `i32`, and an `i32` op already wraps
mod-2³² — so `u32`/`i32` arithmetic was already correct.  But `u8`/`u16` left a
full 32-bit result, so a lowered `200u8 + 100u8` gave `300`, not `44`.

This masks a narrow-width (`u4`/`u8`/`u16`) arithmetic / bitwise / shift / `neg`
/ `not` result with `i32.const <mask>; i32.and` after the i32 op
(`emit_wasm_width_mask`), mirroring vm-core's `mask_result` and jit-core's
`MASK_WIDTH` — the register-arithmetic analogue of the existing byte-tape
`i32.store8`.  `u4` is now mapped to `i32` (it was previously unrepresentable);
`u32`/`i32` need no mask; `i64`/`u64`/floats are unchanged.

Verified by **running** the lowered wasm on `wasm-runtime` (new dev-dependency):
`tests/width_wrap.rs` executes `200u8+100u8=44`, `~0u8=255`, `1u8<<8=0`,
u16/u4 wraps, the native u32 wrap, and `i64`-does-not-mask.

## [0.13.0] — 2026-06-12 (LANG-MATRIX LM-W Brainfuck — byte-tape ops on wasm)

Adds the lowering Brainfuck needs to run on the WASM backend — the last code-gen
gap in Brainfuck's row after LLVM (LM-L). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on the in-repo `wasm-runtime` in
`lang-aot/tests/lang_matrix.rs`: it prints `A`.

`lower_brainfuck_for_aot` rewrites Brainfuck's tape into `alloc_bytes` /
`load_byte` / `store_byte` and widens every cell/pointer register to `i64`. The
wasm module already has linear memory, `putchar`/`getchar` host imports, and a
dispatch-loop control flow — what was missing was the byte-tape ops and the
i64↔i32 conversions the widened value model requires.

### Added

- **`alloc_bytes dest <- size`** → `i64.const 0` (the wasm linear memory *is* the
  tape and starts at offset 0; `size` only triggers the fixed 1-page memory). The
  base is bound in the register `dest`.
- **`load_byte dest <- base, idx`** → `i32.wrap_i64` (narrow the i64 base+idx to
  the i32 address), `i32.add`, `i32.load8_u`, then `i64.extend_i32_u` to widen the
  loaded byte back to the i64 cell register. The wasm twin of LLVM's
  `getelementptr i8 + load + zext`.
- **`store_byte base, idx, val`** → `i32.wrap_i64` on base+idx (address) and on
  `val` (the low byte), then `i32.store8`. The `i32.store8`'s implicit `& 0xFF` is
  what gives Brainfuck's 8-bit cell wrap-around (`255 + 1 == 0`) for free even
  though the arithmetic ran at i64 width. `store_byte` with a `dest` is rejected.
- `collect_module_features` now flags `uses_memory` for these ops (so the linear
  memory is emitted), alongside the existing `load_mem`/`store_mem`.
- `codegen`: `I64_EQZ` (0x50) constant; `lower` gains `i32.add` / `i32.wrap_i64` /
  `i64.extend_i32_u` encoders.

### Fixed (the i64-widening ripple)

- **`putchar`/`getchar`**: the `env.putchar`/`env.getchar` imports are `(i32)->()` /
  `()->i32`, but the Brainfuck cell register is now `i64`. `putchar` now narrows its
  arg with `i32.wrap_i64`; `getchar` widens its i32 result with `i64.extend_i32_u`.
- **i64 branch conditions**: `jmp_if_true`/`jmp_if_false` assumed an `i32` condition
  (`if` / `i32.eqz`). The widened Brainfuck loop guard is an `i64` cell value, so an
  i64 condition now branches via `i64.eqz` (false-test) / `i64.eqz; i32.eqz`
  (true-test). Width is chosen per the register's declared type.
- **i64-declared comparison results**: a wasm comparison always yields `i32`, but
  `concretize_scalar_any_for_wasm` may declare the result local `i64`. The i32
  boolean is now widened with `i64.extend_i32_u` when the dest is i64, so the module
  is well-typed (previously an i32 sat in an i64 local — tolerated only because every
  consumer happened to use i32 ops; the new `i64.eqz` guard would have tripped on it).

Three new tests in `tests/test_backend.rs` cover the tape ops + memory injection,
the `store_byte`-with-dest rejection, and the i64-guard / widened-cmp conversions.

## [0.12.0] — 2026-06-08 (LANG77 / McCarthy L3b-3a-4c — `EQ` / `equal?`)

### Added

- `call_builtin "equal?"` (McCarthy `EQ` on atoms) lowers to **unbox-both +
  `i32.eq`**: each argument arrives boxed as an `i31ref` (the structural pass
  boxes a lisp atom before a predicate), so we `i31.get_s` each and compare.
  `(EQ 5 5)` → 1, `(EQ 5 6)` → 0. `equal?` is added to the call_builtin
  whitelist. (This is McCarthy `eq` / atom equality; deep structural `equal`
  over cons cells is a separate, later builtin.)

## [0.11.0] — 2026-06-08 (LANG77 / McCarthy L3b-3a-4b — `pair?` / `not`)

### Added

- The `call_builtin` whitelist gains the McCarthy lisp predicates **`pair?`**
  and **`not`**, with their lowerings:
  - `pair?` → **`ref.test $LispyPair`** — "is this lisp value a cons cell?"
    (pushes `i32 1` for a cons, `0` for a boxed atom or nil).
  - `not` → **`i32.eqz`** — boolean negation of a predicate's machine boolean.
    (Distinct from the numeric `not` *op*, a bitwise XOR -1.)
- `module_uses_lispy_pair` now also triggers on a `pair?` call, so the
  `$LispyPair` struct type is emitted even for a program that never `cons`es
  (e.g. `(ATOM 5)`, where `pair?`'s `ref.test` still needs the type).

With these and the predicate-atom boxing in `iir-builtin-lowering`, `ATOM x`
(= `not(pair? x)`) compiles and runs: `(ATOM 5)` → 1, `(ATOM (CONS 1 2))` → 0.

## [0.10.0] — 2026-06-08 (LANG77 / McCarthy L3b-3a-3c — `alloc` actually allocates)

### Fixed

- **`alloc` now emits a real allocation.** It previously lowered to a bare
  `ref.null` (a placeholder), so the cons cell was *null* and the very next
  `field_store` (`struct.set`) trapped on a null reference. It now pushes a
  typed null for each of the `$LispyPair`'s two `anyref` fields and then
  `struct.new`, yielding a real `(null . null)` heap object that the following
  `field_store`s overwrite. Uses only the already-supported `struct.new` /
  `struct.set` / `struct.get` ops (no engine change).

This completes the wasm side of the McCarthy cons end-to-end: with the
structural representation pass (boxing atoms / unboxing the result) in
`iir-builtin-lowering`, `(CAR (CONS 7 9))` now compiles to a `.wasm` that runs
to `7` on the in-repo `wasm-runtime`.

## [0.9.0] — 2026-06-04 (LANG77 / McCarthy L3b-3a — i31ref `box`/`unbox`)

### Added — WasmGC integer boxing

- `box` and `unbox` are no longer in `UNSUPPORTED_OPS`. They now lower to the
  WasmGC i31 reference ops:
  - **`box dest, src`** → `ref.i31` (`GcInstruction::I31New`, bytes `0xFB 0x1C`):
    box an `i32` into an `i31ref` (a tagged 31-bit integer reference).
  - **`unbox dest, src`** → `i31.get_s` (`GcInstruction::I31GetS`, bytes
    `0xFB 0x1D`): read it back as a sign-extended `i32`.
- These are the boxing primitives the **uniform-anyref lisp value model**
  needs: a lisp integer atom becomes an `i31ref` so it can live in a
  `$LispyPair`'s `anyref` field alongside heap pairs, and is unboxed only at
  the numeric boundary (the program's return value) — mirroring the native
  NaN-box `(n << 3)` / arithmetic `>> 3` discipline. The retype/box pass that
  *emits* these ops for a McCarthy module is the next slice (L3b-3a-2).

### Verification note

The repo has no WasmGC runtime or validator (its `wasm-simulator` is MVP-only;
`wasm-validator` is structural-only), so these are verified at the **opcode-byte
level** — the new tests assert the emitted code contains `0xFB 0x1C` / `0xFB
0x1D` and that `box`/`unbox` pass validation. End-to-end execution of WasmGC
output remains out of scope (documented like the macOS-native-exe gap).

## [0.8.0] — 2026-06-01 (G2 — whitelist `call_builtin "print_i64"`)

### Changed — `call_builtin "print_i64"` now reaches real wasm bytecode

Pre-0.8.0, BASIC's `PRINT` lowered to `call_builtin "print_i64"`,
and the validator rejected it with
`UnsupportedOp ... print_i64 ... not in the WASM backend's
host-import whitelist (supported: ["putchar", "getchar"])`.

The host import already existed under a different name: the
`io_out` opcode (a Twig/Lispy mechanism) wires
`env.__print_i64 : (i64) -> ()`.  G2 makes `call_builtin
"print_i64"` reuse that same import — no new host function needed,
no breaking change for existing `io_out` users.

The end-to-end effect is that BASIC programs containing `PRINT`
statements now reach real `.wasm` bytecode through the same single
encoder pipeline as Twig.

### Implementation

- `validate.rs::CALL_BUILTIN_SUPPORTED_NAMES`: `"print_i64"` added.
- `lower.rs::collect_module_features`: a `call_builtin "print_i64"`
  flips `uses_io_out` so the `env.__print_i64` import is wired in
  even when the module never uses the `io_out` opcode.
- `lower.rs::emit_instr`: new `"print_i64"` arm in the
  `call_builtin` branch loads the i64 argument and emits
  `call <print_fn_idx>` — identical lowering to the `io_out`
  opcode.

### Tests

- 4 new tests in `tests/test_backend.rs`:
  - `g2_call_builtin_print_i64_validator_accepts`
  - `g2_call_builtin_print_i64_lowers_to_wasm_bytes`
  - `g2_call_builtin_print_i64_injects_host_import`
  - `g2_unknown_builtin_still_rejected` (regression marker —
    confirms G2 didn't widen the whitelist beyond `print_i64`)
- All 95 existing tests still pass.


## [0.7.0] — 2026-06-01 (G1 — accept `cmp_*`-prefixed comparison opcodes)

### Changed — `cmp_eq` / `cmp_ne` / `cmp_lt` / `cmp_le` / `cmp_gt` / `cmp_ge` now lower

Pre-0.7.0, the `lower_iir_to_wasm` step only recognised the bare
shape (`eq` / `ne` / `lt` / `le` / `gt` / `ge`) — the form
`twig-ir-compiler` emits.  Languages that prefix the mnemonic with
`cmp_` — BASIC, Nib, Oct — would fail at lowering even though the
validator accepted them, surfacing as
`IIR -> WasmModule: UnsupportedOp { op: "cmp_gt" }`.

This release accepts both shapes.  The implementation strips a
leading `cmp_` from the opcode name and routes the bare form
through the existing per-type opcode dispatch (i32/i64 signed/
unsigned + f32/f64).  No new opcodes are added; the wasm
comparison opcode table is unchanged.  Twig's existing bare-form
emissions continue to lower identically.

This unblocks:
- BASIC `IF A > 5 THEN 100` lowering to wasm
- BASIC `FOR I = 1 TO 3 / NEXT I` (cmp_le) lowering to wasm
- Nib `if a < b { ... }` lowering to wasm
- Oct `while x < 10 { ... }` lowering to wasm

### Tests

- 7 new tests in `tests/test_backend.rs`: one per `cmp_*` variant
  asserting `lower_iir_to_wasm` no longer rejects, plus a
  back-compat test for the bare-`eq` form.
- All 95 existing unit tests still pass.

## [0.6.0] — 2026-05-26 (Validator accepts `ref<any>` for `field_load`)

### Changed — `ref<any>` joins `SUPPORTED_REF_TYPES`

Companion to Twig path-A increment 6c.  The Phase 2 heap-lowering
convention is `field_load dest, pair, idx [ref<any>]` — the loaded
value's type is `ref<any>` because cons-cell fields can hold any
Lisp value.  WasmGC lowering already declares cons-cell fields as
`(mut (ref null any))`, so the actual code shape is `struct.get`
returning `anyref`, which matches `ref<any>`.

This release widens the WASM validator:

- `SUPPORTED_REF_TYPES` now includes `ref<any>` (in addition to
  `ref<LispyPair>`).
- `alloc` continues to require `ref<LispyPair>` only (we can't
  allocate an unknown struct shape).
- `field_load` accepts either `ref<any>` (canonical Phase 2) or
  `ref<LispyPair>` (forward-compat).
- Other ops with `ref<any>` type_hint flow through Check 4
  (UnsupportedType) without rejection.

No lowering changes — `struct.get` already produces an `anyref`-
compatible value.

## [0.5.0] — 2026-05-24 (Validator accepts `field_store [void]`)

### Changed — `validate_for_wasm` now accepts `field_store [void]`

Companion to Twig path-A increment 6b.  The Phase 2 heap-lowering
convention is `field_store cell, idx, value [void]` (the store has no
result, so its type_hint is `"void"`).  iir-builtin-lowering emits
this form, and BEAM, JVM, CLR validators all accept it.  WASM
previously required `type_hint == "ref<LispyPair>"` for `field_store`,
which was inconsistent.

This release widens WASM's `field_store` rule: `"void"` is accepted
canonically; `"ref<LispyPair>"` continues to work for forward
compatibility with frontends that propagate the object type onto the
store.

No lowering changes — `lower.rs` already produces `struct.set` from
both shapes.

## [0.4.0] — 2026-05-22 (Brainfuck — linear memory + I/O imports)

### Added — Brainfuck `load_mem` / `store_mem` / `call_builtin` lowering

#### Validator changes

- `validate_for_wasm` now **accepts** `load_mem` and `store_mem` — they
  were previously in `UNSUPPORTED_OPS`.  Both lower to WASM linear-memory
  ops (`i32.load8_u`, `i32.store8`) over a module-defined memory.
- `call_builtin` is now **conditionally** accepted: the builtin name
  carried in `srcs[0]` must be in the new
  `CALL_BUILTIN_SUPPORTED_NAMES` whitelist.  Today's whitelist covers
  Brainfuck's two I/O builtins (`putchar`, `getchar`); extending it
  takes three steps documented in the constant's doc comment.
- Unknown builtin names still produce a clear `UnsupportedOp` error
  with the builtin name and the whitelist included.

#### Lowering changes

- New `ModuleFeatures` struct collected by `collect_module_features`
  (replaces the narrower `collect_globals_and_io`).  Captures
  `uses_io_out`, `uses_putchar`, `uses_getchar`, and `uses_memory`
  flags in a single module walk.
- When `uses_putchar`: inject `env.putchar : (i32) -> ()` host import.
- When `uses_getchar`: inject `env.getchar : () -> i32` host import.
- When `uses_memory`: inject a single 1-page (64 KiB) linear `Memory`
  — the Brainfuck tape.  Programs that don't use memory ops get no
  memory section, preserving binary compatibility with existing
  non-BF callers (Twig, BASIC, Oct, Nib, Lispy).
- Function-index space: imports occupy the first slots in
  declaration order — `env.__print_i64` (LANG32, when used),
  then `env.putchar`, then `env.getchar`.  Defined functions follow.
- New `emit_instr` arms:
  - `load_mem` → `local.get addr; i32.load8_u; local.set dest`
  - `store_mem` → `local.get addr; local.get val; i32.store8`
  - `call_builtin "putchar"` → `local.get val; call <putchar_idx>`
  - `call_builtin "getchar"` → `call <getchar_idx>; local.set dest`

#### Why this matters

After this PR, Brainfuck's IIR — including `+++.` (memory + putchar),
`,[.,]` (cat), and the multiplication idiom — flows through the
*same* `iir-to-wasm` backend that Twig, BASIC, Oct, Nib, and Lispy
use.  Stage 1 of 4 for the BF→{wasm,jvm,clr,beam} story; the JVM,
CLR, and BEAM lowerings are queued behind this PR.

#### Tests

- `validate.rs::tests` — 5 new unit tests for the validator changes:
  - `load_mem_accepted_for_bf`
  - `store_mem_accepted_for_bf`
  - `call_builtin_putchar_accepted`
  - `call_builtin_getchar_accepted`
  - `call_builtin_unknown_name_rejected`
- Existing `unsupported_ops_rejected` updated: `load_mem`, `store_mem`,
  `call_builtin` removed from the unconditional-reject list; comments
  point readers to the new tests.
- Existing `tests/test_backend.rs::validate_memory_ops_rejected`
  renamed → `validate_memory_ops_accepted` and updated to assert the
  promotion.
- Doc-tests unchanged.

Total: 45 lib + 88 integration tests pass.

---

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Improved `ClosureOpcode` validator error

- `validate_for_wasm` now emits a dedicated `ClosureOpcode` error message
  (format: `"[fn_name] ClosureOpcode: alloc_closure/call_closure require the
  BEAM backend — WASM does not support heap-allocated closures"`) when it
  encounters `alloc_closure` or `call_closure`.
- Previously these fell through to the generic `UntypedInstruction` path
  because their type hints are `"closure"` and `"any"` respectively — now the
  closure check runs first so the error message is actionable.

#### Tests

- `lang35_alloc_closure_closure_opcode_error`: asserts `validate_for_wasm`
  returns an error containing "ClosureOpcode" for a module with `alloc_closure`.
- `lang35_call_closure_closure_opcode_error`: same for `call_closure`.
- `lang35_closure_opcode_error_not_untyped`: asserts the error does NOT
  contain "UntypedInstruction", confirming the new code path fires first.

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### Global variable support via WASM global section

- Pre-pass `collect_globals_and_io` scans all functions to find `global_store` /
  `global_load` instructions and `io_out` instructions before emitting code.
- Each named global maps to a `(global i64 (mut (i64.const 0)))` entry added to
  `WasmModule::globals`.  Slot indices are assigned lazily (first encounter = next
  free slot).
- `global_store "x", %v` → `local.get <slot_of_%v>; global.set <idx_of_x>`.
- `global_load "x" → %r` → `global.get <idx_of_x>; local.set <slot_of_%r>`.

#### I/O support via host import

- If any function uses `io_out`, the host import `env.__print_i64 (func (param i64))`
  is prepended to `WasmModule::imports`.
- `io_out %v` → `local.get <slot_of_%v>; call $__print_i64`.
- **Function index offset**: importing `$__print_i64` assigns it function index 0,
  shifting all defined functions up by 1.  The lowerer applies `fn_idx_base = 1`
  when building `fn_map` and export indices so calls remain correct.

---

## [0.1.0] — 2026-05-11

### Added

- Initial release of the `iir-to-wasm` crate.
- `validate_for_wasm()` — pre-flight validation of `IIRModule` for WASM
  lowering.  Reports human-readable errors for empty modules, empty
  functions, untyped instructions, unsupported types, and unsupported ops.
  Unlike the BEAM backend, float type hints (`f32`, `f64`) and float
  constants (`Operand::Float`) are fully supported.
- `IIRWasmConfig` — configuration struct for the lowering pass.  Carries the
  WASM module name.
- `IIRWasmError` — structured error enum for lowering failures, covering
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, and `InvalidOperand`.
- `lower_iir_to_wasm()` — two-pass lowering from `IIRModule` to `WasmModule`.
  - Pass 1: per-function register allocation and local type inference.
  - Pass 2: instruction code generation — arithmetic, bitwise, comparisons,
    constants (i32/i64/f64), function calls, and control flow.
  - Control flow: dispatch-loop pattern for functions with labels/jumps;
    linear emission for functions without.
  - Every function is exported by name.
- `codegen.rs` — internal encoding helpers for WASM binary opcodes: signed
  and unsigned LEB128 immediates, `local.get`/`local.set`, `br`/`br_if`,
  `i32.const`, `i64.const`, `f64.const`, and the binary opcode table.
- `tests/test_backend.rs` — 40+ integration tests covering validation, module
  structure, FunctionBody correctness, encoding round-trips, and all
  major opcode families.
