# Changelog — iir-to-wasm

All notable changes to this crate are documented here.  The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
