# Changelog — iir-to-ge225

All notable changes to this crate are documented here.

## v0.10.0 — 2026-06-03 — Crate **DEPRECATED** — Phase 3 of historical-arch backend migration

This crate is now deprecated.  It sits at the wrong layer in the
compiler pipeline (consumes dynamically-typed IIR directly,
bypasses the `jit_core::backend::Backend` trait that
`aarch64-backend` and `x86_64-backend` use).

### Replacement

| Old | New |
|-----|-----|
| `iir_to_ge225::HALT_WORD`, `LDA_OPCODE_NIBBLE`, … | [`ge225_encoder::*`](../ge225-encoder) — same values, single source of truth |
| `iir_to_ge225::lower_iir_to_ge225` | [`ge225_backend::compile`](../ge225-backend) — consumes monomorphised CIR via `aot_core::specialise` |
| Direct caller from `lang-aot` `compile_file_to_ge225_bin` | now routes through `aot_core::infer` + `aot_core::specialise` + `ge225_backend::compile` |

### What changed in this release

- Marked `lower_iir_to_ge225` with `#[deprecated(since = "0.10.0",
  note = "use ge225_backend::compile over CIR")]`.
- Added deprecation banner to the module-level docs.
- Test suite now carries `#![allow(deprecated)]` so it can keep
  exercising the old API as a regression invariant.
- `lang-aot` Cargo.toml dropped its `iir-to-ge225` path dep —
  this crate is no longer used in the build graph.

### What did NOT change

- All public constants and functions still work — every existing
  caller compiles, just with a deprecation warning.
- All 33 unit tests + 1 doctest still pass.
- Byte-for-byte output is unchanged (verified by the lang-aot
  GE-225 + BASIC e2e smoke tests producing identical bytes
  through the new pipeline).

### Reference

- Migration plan: [`code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)
- Replacement spec: [`code/specs/ge225-backend.md`](../../../specs/ge225-backend.md)

## v0.9.0 — 2026-06-02 — A5++++++++++ `neg` lowering (closes BASIC unary-minus gap)

Eighth lowering increment.  Adds `neg dest, src` via the canonical
two's-complement-by-subtract-from-zero pattern.  Closes the BASIC
unary-minus gap so programs containing `-x` expressions compile
end-to-end.

### Lowering shape

```
(STA r_evict_src)?  ; if src in ACC, evict to register
(STA r_evict_acc)?  ; if any other var in ACC, evict
LDA 0               ; ACC ← 0
SUB r_src           ; ACC ← 0 - src = -src
```

After this sequence: `env[dest] = ACC_MARKER`, `acc_owner = Some(dest)`.

### Byte costs

- Trivial `const v=N; neg w, v; ret w` (entry function): **15 bytes**
  `LDA N + STA r0 + LDA 0 + SUB r0 + HLT`
- `neg` itself contributes 6 bytes after prep (`LDA 0` + `SUB r_src`).

### Public surface delta

- `"neg"` added to `SUPPORTED_OPS`.
- No new pub constants, no new error variants — validation flows
  through existing `InvalidOperand` and `UndefinedVariable`.

### Opcode map (unchanged — neg uses existing 0x1 LDA + 0x5 SUB)

The lowering reuses the existing LDA and SUB opcodes; no new
opcode nibble is added.  All 11 opcodes (0x0..0xB) keep their
v0.6.0 assignments.

### Tests (33 unit + 1 doctest, all passing)

New v0.9.0 coverage:
- `canonical_neg_byte_sequence` — exact 15-byte trivial ROM
- `neg_when_src_in_register_skips_first_eviction` — 21-byte
  chained-const sequence
- `double_neg_works` — chained negs lower cleanly (no exact byte
  pin, just semantics)
- `neg_undefined_src_errors` — `UndefinedVariable`
- `neg_no_srcs_errors` — `InvalidOperand`
- `neg_result_feeds_directly_into_ret` — result is in ACC for
  ret-without-LD

Regressions still pinned: trivial 6-byte ROM, trivial-add 21-byte,
trivial-cmp 33-byte, call_builtin no-op, all 11 opcode nibbles.

Lang-aot e2e: 10/10 BASIC tests pass (no test contents changed —
neg just becomes part of BASIC programs that use unary minus).

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5

## v0.8.0 — 2026-06-02 — A5+++++++++ `call_builtin` no-op lowering (closes BASIC PRINT gap)

Seventh lowering increment.  Adds a minimal `call_builtin`
lowering that lets BASIC programs containing `PRINT` (and other
built-in calls) compile end-to-end without errors.  The GE-225
historically routed I/O through a teletype the modern simulator
doesn't model, so this is genuinely a no-op at the byte level.

### Lowering shape

| Case | Bytes emitted |
|------|---------------|
| `call_builtin print_i64, v` (no dest) | **zero bytes** (true no-op) |
| `call_builtin x = input_i64` (with dest) | `(STA r_evict)?` + `LDA 0` (deterministic placeholder return value) |

### Why a no-op?

The GE-225 in 1959 had a teletype-connected terminal for I/O;
modern GE-225 simulators don't model the teletype, and our
skeleton ISA has no I/O opcode.  Rather than fail the lowering
(which would break BASIC PRINT end-to-end), we emit zero bytes
for the void case and a deterministic `LDA 0` placeholder for
the return-value case.

This is **exactly enough** to round-trip Dartmouth BASIC programs
that use PRINT through the full pipeline:

```text
10 LET A = 5
20 PRINT A
30 END
```

now compiles to a non-empty word-aligned .bin (instead of erroring
with `UnsupportedOp { op: "call_builtin" }`).

A future increment could:
- Synthesise a `JSR <host_stub>` to a known address the simulator
  watches for I/O hooking.
- Add a new opcode (e.g. `0xC TTY`) dedicated to teletype output.
- Emit a busy-loop pattern matching real GE-225 teletype I/O timing.

### Public surface delta

- `"call_builtin"` added to `SUPPORTED_OPS`.
- No new pub constants, no new error variants — validation flows
  through existing `InvalidOperand` and `UndefinedVariable`.

### Tests (27 unit + 1 doctest, all passing)

New v0.8.0 coverage:
- `call_builtin_no_dest_emits_zero_bytes` — print case is truly
  zero bytes; trivial-print-of-const ROM stays at 6 bytes.
- `call_builtin_with_dest_emits_lda_zero` — input case → `LDA 0`.
- `call_builtin_undefined_arg_errors` — undefined arg →
  `UndefinedVariable`.
- `call_builtin_no_srcs_errors` — missing builtin name →
  `InvalidOperand`.
- `call_builtin_with_dest_evicts_acc_owner` — LDA 0 doesn't
  silently clobber a live ACC owner.

### Lang-aot e2e (3 BASIC tests, all 3 now pass on Ok path)

The `end_to_end_basic_print_documents_call_builtin_gap` test in
lang-aot, originally a gap documentation test, now takes the Ok
path — BASIC PRINT compiles to a non-empty word-aligned GE-225
.bin without errors.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5

## v0.7.0 — 2026-06-02 — A5+++++++ comparison ops (`cmp_lt/eq/ne/le/gt/ge`)

Sixth lowering increment.  Adds six new IIR ops that materialise a
0/1 boolean in ACC via the canonical SUB-then-test pattern.
Finally activates the `BMI` opcode (reserved in v0.6.0) for
`cmp_lt` and `cmp_le`.  `cmp_gt` and `cmp_ge` reuse the lt/le emit
paths via operand swap.  Mirrors `iir-to-intel4004` v0.5.0 /
`iir-to-armv7` v0.4.x cmp slices.

### Added

- `"cmp_lt"`, `"cmp_eq"`, `"cmp_ne"`, `"cmp_le"`, `"cmp_gt"`,
  `"cmp_ge"` added to `SUPPORTED_OPS`.

### Lowering table

| IIR op | GE-225 emit shape (after operand-eviction prep) |
|--------|--------------------------------------------------|
| `cmp_lt c, a, b` | `LD r_a; SUB r_b; BMI true; LDA 0; BR end; LDA 1; end:` |
| `cmp_gt c, a, b` | identical to `cmp_lt c, b, a` (operand swap) |
| `cmp_eq c, a, b` | `LD r_a; SUB r_b; BZ true; LDA 0; BR end; LDA 1; end:` |
| `cmp_ne c, a, b` | `LD r_a; SUB r_b; BNZ true; LDA 0; BR end; LDA 1; end:` |
| `cmp_le c, a, b` | `LD r_a; SUB r_b; BMI true; BZ true; LDA 0; BR end; LDA 1; end:` |
| `cmp_ge c, a, b` | identical to `cmp_le c, b, a` (operand swap) |

After the SUB instruction sets ACC's sign / zero state, one or two
conditional branches skip to a `LDA 1` "true" arm; the fallthrough
emits `LDA 0` then `BR end`.  Result is the dest `c` taking over
ACC as the new owner.

### Byte costs (after const-prep eviction)

- Single-test cmps (`cmp_lt`, `cmp_gt`, `cmp_eq`, `cmp_ne`):
  21 bytes (LD + SUB + BMI/BZ/BNZ + LDA 0 + BR + LDA 1).
- Double-test cmps (`cmp_le`, `cmp_ge`):
  24 bytes (LD + SUB + BMI + BZ + LDA 0 + BR + LDA 1).

### Why operand swap for `gt` / `ge`?

`a > b` ⇔ `b < a`, and `a ≥ b` ⇔ `b ≤ a`.  Swapping the operands
before the LD/SUB sequence reuses the `cmp_lt` / `cmp_le` emit
path verbatim.  This trick is documented in the iir-to-intel8008
and iir-to-armv7 backends; we adopt it here for symmetry and to
avoid a 2x increase in cmp-emit code.

### Opcode map (unchanged — BMI activated, no new opcodes)

| Nibble | Mnemonic | Word | Status in v0.7.0 |
|--------|----------|------|--------------------|
| `0xB` | `BMI a` | `[0x0B, hi, lo]` | **now actively used** by `cmp_lt`/`cmp_gt`/`cmp_le`/`cmp_ge` |

All other opcodes (`0x0..0xA`) unchanged from v0.6.0.

### Canonical `cmp_lt c, a, b; ret c` byte trace pinned

```
const a=2; const b=5; cmp_lt c, a, b; ret c (entry function)
0:  LDA 2      [0x01, 0x00, 0x02]
3:  STA r0     [0x02, 0x00, 0x00]
6:  LDA 5      [0x01, 0x00, 0x05]
9:  STA r1     [0x02, 0x00, 0x01]
12: LD r0      [0x03, 0x00, 0x00]
15: SUB r1     [0x05, 0x00, 0x01]
18: BMI 27     [0x0B, 0x00, 0x1B]   ← cmp_lt's true branch
21: LDA 0      [0x01, 0x00, 0x00]   ← false branch
24: BR 30      [0x06, 0x00, 0x1E]
27: LDA 1      [0x01, 0x00, 0x01]   ← true branch (BMI target)
30: HLT        [0x00, 0x00, 0x00]   ← end (c already in ACC)
Total: 33 bytes
```

### Tests (22 unit + 1 doctest, all passing)

New v0.7.0 coverage:
- `canonical_cmp_lt_byte_sequence` — exact 33-byte sequence above.
- `cmp_eq_emits_bz_pattern` — BZ at offset 18 instead of BMI.
- `cmp_ne_emits_bnz_pattern` — BNZ at offset 18.
- `canonical_cmp_le_byte_sequence` — double-test exact 36-byte
  sequence with BMI + BZ pointing at the same true target.
- `cmp_gt_uses_operand_swap` — `LD r1; SUB r0` after a/b are
  evicted to r0/r1.
- `cmp_ge_uses_operand_swap_and_double_test` — swap + BMI + BZ.
- `cmp_with_lhs_in_acc_evicts_then_runs_normally` — eviction prep.
- `cmp_undefined_lhs_errors` / `cmp_eq_undefined_rhs_errors`.
- `cmp_result_feeds_directly_into_jmp_if_true` — `c` is ACC owner
  after cmp_lt, so `jmp_if_true c, skip` skips the LD prefix.
- `cmp_result_can_be_added` — chained arithmetic works (the cmp
  output is a real value living in ACC).

Regressions still pinned:
- All 11 opcode nibbles.
- `trivial_rom_still_six_bytes`, `trivial_add_still_works`.
- `mul_still_unsupported`.

Lang-aot e2e smoke tests still pass (4 GE-225 paths unchanged —
Twig doesn't emit cmp ops in its trivial smoke programs).

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors `iir-to-intel4004` v0.5.0 / `iir-to-armv7` v0.4.x cmp slice.

## v0.6.0 — 2026-06-02 — A5++++++ call/return (`JSR`, `RTS`) + `BMI` reserved

Fifth lowering increment.  Adds the call/return discipline: the
new `JSR` opcode pushes a return address and branches, `RTS` pops
and branches back.  `call` IIR ops resolve to JSR via module-level
backpatching (after every function has been emitted).  Reserves
the `BMI` opcode for future signed-comparison branches.

### Added

- `pub const JSR_OPCODE_NIBBLE: u8 = 0x9` — Jump SubRoutine.  Word
  `[0x09, hi, lo]` (16-bit callee entry byte address).  Pushes the
  return address (PC+3) onto the internal call stack, then branches.
- `pub const RTS_OPCODE_NIBBLE: u8 = 0xA` — Return from SubRoutine.
  Word `[0x0A, 0x00, 0x00]` (address field unused — target is
  popped from the call stack).
- `pub const RTS_WORD: [u8; 3] = [0x0A, 0x00, 0x00]` — canonical
  RTS 3-byte word, pinned for symmetry with `HALT_WORD`.
- `pub const BMI_OPCODE_NIBBLE: u8 = 0xB` — branch if minus.
  **Reserved**: no IIR op currently lowers to BMI.  A future
  `jmp_if_neg` IIR op (driven by `cmp_lt` lowerings) will emit it.
- `IIRGe225Error::UndefinedFunction { caller, callee }` — a `call`
  references a function name not defined in the module.
- `IIRGe225Error::CallTargetOutOfRange { caller, callee, offset }`
  — a callee's entry byte offset exceeds the 16-bit JSR address
  field.
- `"call"` added to `SUPPORTED_OPS`.

### Lowering changes

| IIR op | GE-225 lowering |
|--------|-----------------|
| `call (dest =)? fn_name` | (evict ACC owner)? + `JSR <callee_addr>` + (claim ACC for dest)? |
| `ret <var>` in **entry** function | `(LD r_var)?` + `HLT` |
| `ret <var>` in non-entry function | `(LD r_var)?` + `RTS` |
| `ret_void` in **entry** function | `HLT` |
| `ret_void` in non-entry function | `RTS` |

Entry function = the one named by `IIRModule::entry_point`.  When
`entry_point` is `None`, ALL functions emit RTS (no function gets
HLT) — conservative: the IR author opted out of an entry, so they
own the consequences.

### Module-level call backpatching

Pass 1 (per-instruction loop, in every function): each `call`
evicts any current ACC owner, emits `[0x09, 0x00, 0x00]`
placeholder bytes, and pushes
`(slot_byte_offset, callee_name, caller_name)` into module-level
`pending_calls`.  Every function's entry byte offset is recorded
in `function_addrs` at the start of its emission.

Pass 2 (after every function has been emitted): for each
`(slot, callee, caller)` in `pending_calls`, look up
`function_addrs[callee]` and write the 16-bit byte address.
Errors with `UndefinedFunction` if the callee isn't in the
module, or `CallTargetOutOfRange` if the offset exceeds
`u16::MAX`.

### Opcode map (cumulative through v0.6.0)

| Nibble | Mnemonic | Word | Effect |
|--------|----------|------|--------|
| `0x0` | `HLT`   | `[0x00, 0x00, 0x00]` | halt |
| `0x1` | `LDA n` | `[0x01, hi, lo]` | ACC ← n |
| `0x2` | `STA r` | `[0x02, 0x00, r]` | ACC ↔ r (XCH) |
| `0x3` | `LD r`  | `[0x03, 0x00, r]` | ACC ← r |
| `0x4` | `ADD r` | `[0x04, 0x00, r]` | ACC ← ACC + r |
| `0x5` | `SUB r` | `[0x05, 0x00, r]` | ACC ← ACC - r |
| `0x6` | `BR a`  | `[0x06, hi, lo]` | unconditional branch |
| `0x7` | `BNZ a` | `[0x07, hi, lo]` | branch if ACC ≠ 0 |
| `0x8` | `BZ a`  | `[0x08, hi, lo]` | branch if ACC = 0 |
| `0x9` | `JSR a` | `[0x09, hi, lo]` | push PC+3, branch to `a` |
| `0xA` | `RTS`   | `[0x0A, 0x00, 0x00]` | pop, branch to popped address |
| `0xB` | `BMI a` | `[0x0B, hi, lo]` | branch if ACC sign bit set (reserved) |

Future slices reserve `0xC..0xF` for arithmetic extensions (MUL,
DIV) or wider-immediate variants.

### Tests (21 unit + 1 doctest, all passing)

New v0.6.0 coverage:
- All 11 opcode nibbles pinned.
- `RTS_WORD` constant pinned to `[0x0A, 0x00, 0x00]`.
- `non_entry_ret_void_emits_rts_not_halt` — non-entry function ret
  uses RTS.
- `non_entry_ret_with_var_emits_rts` — ret-with-value in non-entry
  function: LDA + RTS.
- `trivial_call_no_return_emits_jsr_then_helper_rts` — main calls
  helper; backpatched `JSR 6`.
- `call_with_return_captures_value` — JSR returns into ACC, dest
  claims it for ret-without-LD.
- `call_to_undefined_function_errors` — `UndefinedFunction`.
- `multiple_calls_resolve_independently` — two JSRs backpatch
  to two distinct callee addresses.
- `forward_call_resolves_via_backpatching` — callee defined
  AFTER caller works (the whole point of module-level pass 2).
- `call_evicts_live_acc_owner_before_jsr` — STA r0 inserted
  before JSR; LD r0 after JSR restores caller's value.
- `no_entry_point_means_all_functions_use_rts` — entry=None.

Regressions still pinned:
- `trivial_rom_in_entry_function_still_six_bytes` — entry ret
  still HLT (6 N values).
- `trivial_add_still_works` — 21-byte ADD ROM unchanged.
- `trivial_branch_still_works` — BR + HLT unchanged.
- `mul_still_unsupported` — `UnsupportedOp { op: "mul" }`.

Lang-aot e2e smoke tests still pass (4 GE-225 paths unchanged —
Twig sets `entry_point = Some("main")` so all existing tests get
HLT as before).

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors `iir-to-intel8008` v0.3.9 module-level call-backpatching.

## v0.5.0 — 2026-06-02 — A5+++++ branch family (`BR`, `BNZ`, `BZ`) + label backpatching

Fourth lowering increment.  Adds three new branch opcodes, the
`label` / `jmp` / `jmp_if_true` / `jmp_if_false` IIR ops, and a
per-function two-pass backpatching pipeline that resolves forward
and backward branches to 16-bit absolute byte addresses.

### Added

- `pub const BR_OPCODE_NIBBLE: u8 = 0x6` — unconditional branch.
  Word `[0x06, hi, lo]` (16-bit byte address).
- `pub const BNZ_OPCODE_NIBBLE: u8 = 0x7` — branch if ACC ≠ 0.
  Word `[0x07, hi, lo]`.
- `pub const BZ_OPCODE_NIBBLE: u8 = 0x8` — branch if ACC = 0.
  Word `[0x08, hi, lo]`.
- `IIRGe225Error::UndefinedLabel { function, label }` — a
  branch references a label not defined in the same function.
  Labels are per-function — cross-function jumps are rejected.
- `IIRGe225Error::BranchTargetOutOfRange { function, label, offset }`
  — a label's resolved byte offset exceeds the 16-bit address
  field (cap: 65 536 bytes ≈ 21 845 instruction words).
- `"label"`, `"jmp"`, `"jmp_if_true"`, `"jmp_if_false"` added to
  `SUPPORTED_OPS`.

### Lowering table (new)

| IIR op | GE-225 lowering |
|--------|-----------------|
| `label "<name>"` | zero bytes; records `bytes.len()` |
| `jmp "<target>"` | `BR <target_addr>` (placeholder + backpatch) |
| `jmp_if_true cond, "<target>"` | `(LD r_cond)?` + `BNZ <target_addr>` |
| `jmp_if_false cond, "<target>"` | `(LD r_cond)?` + `BZ <target_addr>` |

### Per-function backpatching strategy

Pass 1 (during the per-instruction loop): every `jmp` / `jmp_if_*`
emits `[opcode, 0x00, 0x00]` placeholder bytes and records
`(slot_byte_offset, target_label)` in `pending_branches`.  Every
`label` records `bytes.len()` in `labels`.

Pass 2 (after the per-instruction loop): for each
`(slot, target)` in `pending_branches`, look up `labels[target]`
and write the 16-bit byte address (big-endian: byte at `slot` =
hi, byte at `slot+1` = lo).  Errors with `UndefinedLabel` if
`labels` doesn't contain the target, or
`BranchTargetOutOfRange` if the offset exceeds `u16::MAX`.

### Opcode map (cumulative through v0.5.0)

| Nibble | Mnemonic | Word | Effect |
|--------|----------|------|--------|
| `0x0` | `HLT`   | `[0x00, 0x00, 0x00]` | halt |
| `0x1` | `LDA n` | `[0x01, hi, lo]` | ACC ← n |
| `0x2` | `STA r` | `[0x02, 0x00, r]` | ACC ↔ r (XCH) |
| `0x3` | `LD r`  | `[0x03, 0x00, r]` | ACC ← r |
| `0x4` | `ADD r` | `[0x04, 0x00, r]` | ACC ← ACC + r |
| `0x5` | `SUB r` | `[0x05, 0x00, r]` | ACC ← ACC - r |
| `0x6` | `BR a`  | `[0x06, hi, lo]` | unconditional branch |
| `0x7` | `BNZ a` | `[0x07, hi, lo]` | branch if ACC ≠ 0 |
| `0x8` | `BZ a`  | `[0x08, hi, lo]` | branch if ACC = 0 |

Future slices reserve `0x9..0xF` for `BMI`, `JSR`, `RTS`, etc.

### Tests (20 unit + 1 doctest, all passing)

New v0.5.0 coverage:
- All 3 new opcode nibbles pinned.
- `label_only_emits_no_bytes` — `label` is a marker, not bytes.
- `trivial_jmp_emits_br_with_backpatched_address` — `jmp x;
  label x; ret_void` → `BR 0x0003` + `HLT`.
- `jmp_to_undefined_label_errors` → `UndefinedLabel`.
- `backward_jmp_resolves_correctly` — `label top; jmp top` →
  `BR 0x0000` (1-word infinite loop).
- `jmp_if_true_with_cond_in_acc_skips_ld` — cond is ACC owner,
  no `LD` needed before `BNZ`.
- `jmp_if_false_with_cond_in_acc_emits_bz` — opposite polarity.
- `jmp_if_true_with_cond_in_register_emits_ld` — cond evicted to
  GP register, `LD r1` reload before `BNZ`.
- `canonical_if_then_else_sequence` — full if/then/else byte
  sequence with two labels and one BZ + one BR.
- `jmp_if_true_with_unbound_cond_errors` → `UndefinedVariable`.
- `cross_function_labels_dont_resolve` — labels are per-function;
  `f2` referencing a label in `f1` → `UndefinedLabel`.

Regressions from v0.2.0 / v0.3.0 / v0.4.0 still pinned:
- Trivial 6-byte ROM for `const v; ret v` (6 N values).
- Trivial 21-byte ADD ROM for `const a; const b; add c, a, b; ret c`.
- All 6 prior opcode nibbles still at their nibbles.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors `iir-to-intel8008` v0.3.4 (jump+label slice) /
  `iir-to-intel4004` v0.4.0 jump-family lowering.

## v0.4.0 — 2026-06-02 — A5+++ accumulator arithmetic (`ADD r`, `SUB r`)

Third lowering increment.  Adds `add dest, lhs, rhs` and
`sub dest, lhs, rhs` IIR lowering via two new opcodes — `ADD r`
and `SUB r` — both 20-bit words emitted after an `LD r_lhs` that
stages the lhs into ACC.

| IIR op | GE-225 lowering |
|--------|-----------------|
| `add dest, lhs, rhs` | (evict ACC pieces)? + `LD r_lhs` + `ADD r_rhs` |
| `sub dest, lhs, rhs` | (evict ACC pieces)? + `LD r_lhs` + `SUB r_rhs` |

### Added

- `pub const ADD_OPCODE_NIBBLE: u8 = 0x4` — accumulator-anchored
  addition.  Word layout `[0x04, 0x00, r]` (r = 4-bit register
  index in the low nibble of byte 2).
- `pub const SUB_OPCODE_NIBBLE: u8 = 0x5` — accumulator-anchored
  subtraction.  Word layout `[0x05, 0x00, r]`.
- `"add"` and `"sub"` in `SUPPORTED_OPS`.
- `parse_binop_srcs` helper — extracts `(lhs, rhs)` Var names
  from a binary-op `IIRInstr`, returning `InvalidOperand` if
  either is not a `Var`.

### Lowering strategy

For both add and sub the lowering walks 3 prep steps then emits 2
arithmetic words:

1. Evict lhs from ACC (if there).  After this, lhs has a stable
   register home.
2. Evict rhs from ACC (if there — only possible when lhs == rhs).
3. Evict any remaining ACC owner so ACC is free.
4. Emit `LD r_lhs` (ACC ← lhs's value).
5. Emit `ADD r_rhs` or `SUB r_rhs` (ACC ← lhs ± rhs).
6. `env[dest] = ACC_MARKER; acc_owner = Some(dest)` — the result
   takes over the accumulator.

The conservative scheme always emits the `LD` even when lhs
happens to be the current ACC owner.  This keeps the arithmetic
shape predictable (always 2 words) at a small byte cost; a future
release may peephole-elide that `LD`.

### Opcode map (cumulative)

| Nibble | Mnemonic | Word | Effect |
|--------|----------|------|--------|
| `0x0` | `HLT`   | `[0x00, 0x00, 0x00]`         | halt              |
| `0x1` | `LDA n` | `[0x01, hi, lo]`             | ACC ← n           |
| `0x2` | `STA r` | `[0x02, 0x00, r]`            | ACC ↔ r (XCH)     |
| `0x3` | `LD r`  | `[0x03, 0x00, r]`            | ACC ← r           |
| `0x4` | `ADD r` | `[0x04, 0x00, r]`            | ACC ← ACC + r     |
| `0x5` | `SUB r` | `[0x05, 0x00, r]`            | ACC ← ACC - r     |

Future slices reserve `0x6..0xF` for `BR`/`BMI`/`BNZ`/`JSR`/etc.

### Tests (24 unit + 1 doctest, all passing)

New v0.4.0 coverage:
- `add_opcode_nibble_pinned_to_0x4`, `sub_opcode_nibble_pinned_to_0x5`.
- `trivial_add_byte_sequence` — exact 7-word sequence for
  `const a=3; const b=4; add c, a, b; ret c` = 21 bytes.
- `trivial_sub_byte_sequence` — same shape with `SUB r1`.
- `self_add_uses_same_register_twice` — `add c, a, a` emits
  `LD r0 + ADD r0` (same register cited twice).
- `chained_add_works` — `(a+b)+d` = 12-word / 36-byte sequence.
- `add_undefined_lhs_errors` / `add_undefined_rhs_errors`.
- `sub_undefined_rhs_errors`.
- `add_with_immediate_operand_errors` — `InvalidOperand` (both
  operands must be `Var`).
- `mul_still_unsupported` — `UnsupportedOp { op: "mul" }`.

Regressions from v0.2.0 / v0.3.0 still pinned:
- All opcode constants pinned to their nibbles.
- `trivial_rom_is_still_six_bytes` (6 N values).
- `ret_void_only_still_emits_just_halt`.
- `const_out_of_range_still_errors`.
- `mov_still_works`.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors `iir-to-intel4004` v0.4.0 (A4++++) — same accumulator-
  through-LD-then-ADD/SUB pattern.

## v0.3.0 — 2026-06-02 — A5++ ACC-first GP allocator + `mov`

Second lowering increment.  Adds the GE-225 GP register file
(`r0..r15`), the `STA r` and `LD r` opcodes, the `mov dest, src`
instruction lowering, and an ACC-first linear allocator over the
17-slot pool (ACC + r0..r15).

| IIR op | GE-225 lowering |
|--------|-----------------|
| `const dest, Int(n)` | `(STA r_evict)?` + `LDA n` |
| `mov dest, src` | `(STA r_evict_src)?` + `LD r_src` + `STA r_dest` |
| `ret <var>` | `(LD r_var)?` + `HLT` |
| `ret_void` | `HLT` |

### Added

- `pub const STA_OPCODE_NIBBLE: u8 = 0x2` — GE-225 store-with-XCH-
  semantics opcode.  Word layout `[0x02, 0x00, r]` where `r` is the
  4-bit register index in the low nibble of byte 2.
- `pub const LD_OPCODE_NIBBLE: u8 = 0x3` — load-ACC-from-register.
  Word layout `[0x03, 0x00, r]`.  Pure copy (does not modify `r`).
- `IIRGe225Error::OutOfRegisters { function, name }` — fired when
  the 17-slot ACC + r0..r15 pool is exhausted (18th `const` of a
  function); memory spilling is not yet supported.

### Allocator strategy (ACC-first linear)

The first `const` of each function lands in the accumulator.  Each
subsequent `const` first evicts the current ACC owner to the
next-free GP register via `STA r`, then emits `LDA n`.  This
preserves the v0.2.0 6-byte trivial-case ROM for `const v; ret v`.

`mov dest, src` follows the same pattern as `iir-to-intel4004`
v0.3.0: if `src` lives in ACC, evict it first to a stable register
home, then `LD r_src` + `STA r_dest` (XCH semantics) places src's
value into a fresh register for `dest`.

`ret <var>` is now allocator-aware: if `var` is the current ACC
owner it emits just `HLT`; otherwise it reloads via `LD r_var`
first.

### Opcode map (cumulative through v0.3.0)

| Nibble | Mnemonic | Word |
|--------|----------|------|
| `0x0` | `HLT`   | `[0x00, 0x00, 0x00]` |
| `0x1` | `LDA n` | `[0x01, hi, lo]` |
| `0x2` | `STA r` | `[0x02, 0x00, r]` |
| `0x3` | `LD r`  | `[0x03, 0x00, r]` |

Future slices reserve `0x4..0xF` for `ADD`, `SUB`, `BR`, `BMI`,
`BNZ`, `JSR`, etc.

### A note on `STA` semantics

Real GE-225 silicon's `STA` was a pure store.  This skeleton models
`STA r` as exchange-with-ACC (XCH semantics) so the eviction
pattern is **one instruction** instead of two.  Documented as a
deliberate educational simplification; future versions may split
this back into pure `STA` + restore-via-`LD` if historical
fidelity becomes important.

### Tests (21 unit + 1 doctest, all passing)

New coverage:
- `sta_opcode_nibble_pinned_to_0x2`, `ld_opcode_nibble_pinned_to_0x3`.
- `two_consts_then_ret_of_current_acc_evicts_first_to_r0` — exact
  4-word sequence `LDA + STA + LDA + HLT`.
- `ret_of_evicted_var_emits_ld_to_reload` — 5-word sequence
  including the `LD r0` reload.
- `mov_when_src_in_acc_evicts_then_ld_sta` — 6-word `mov`-from-ACC.
- `mov_when_src_already_in_register_skips_eviction` — 8-word
  chained `mov` pair.
- `allocator_at_seventeenth_const_still_succeeds` — pool ceiling.
- `allocator_exhausts_on_eighteenth_const` — fails on v16 eviction.
- `mov_from_undefined_src_errors` — `UndefinedVariable`.
- `unsupported_op_add_errors` — `UnsupportedOp { op: "add" }`.

Regressions from v0.2.0 still pinned:
- `trivial_rom_is_still_six_bytes` (8 N values).
- `ret_void_only_still_emits_just_halt`.
- `const_negative_one_still_uses_twos_complement`.
- `const_out_of_range_still_errors`.
- All 6 `IIRGe225Error` variants Display correctly (incl. new
  `OutOfRegisters`).

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors `iir-to-intel4004` v0.3.0 (A4++) — same 17-slot pool
  capacity, same eviction-on-second-const allocator shape.

## v0.2.0 — 2026-06-02 — A5+ first real lowering

First lowering increment after the A5 skeleton.  Adds:

| IIR op | GE-225 lowering |
|--------|-----------------|
| `const dest, Int(n)` (16-bit signed/unsigned) | `LDA n` — 20-bit word: opcode nibble `0x1` + 16-bit immediate, packed `[0x01, hi, lo]` |
| `const dest, Bool(b)` | `LDA 0` or `LDA 1` |
| `ret <var>` | `HLT` (the all-zeros 20-bit word) — but only if `<var>` is the current ACC owner |
| `ret_void` | `HLT` |

### Added

- `pub const LDA_OPCODE_NIBBLE: u8 = 0x1` — the GE-225 `LDA` opcode
  nibble, lives in the low 4 bits of byte 0 in the 3-byte word
  packing.
- `IIRGe225Error::UndefinedVariable { function, name }` — fired
  when `ret <var>` references a var that is either never bound or
  no longer the current accumulator owner.  v0.2.0 has only the
  accumulator; multi-register liveness arrives in A5++.
- Real per-function lowering with accumulator tracking:
  `env: HashMap<String, u8>` (single sentinel `ACC_MARKER = 16`
  in v0.2.0) and `acc_owner: Option<String>`.  Mirrors the
  iir-to-intel4004 v0.2.0 → v0.3.0 progression.

### Acceptance test

The trivial-case ROM (`const v = N; ret v`) is always 6 bytes —
3 for `LDA N` + 3 for `HLT` — regardless of `N`.  Pinned by the
`trivial_rom_is_six_bytes` test across N ∈ {0, 1, 42, 255, 256,
32767, -1, -32768}.

### Word format pinned by this PR

```
byte 0: 0000 OOOO   (top 4 bits zero + 4-bit opcode nibble)
byte 1: IIII IIII   (high 8 bits of the 16-bit immediate)
byte 2: IIII IIII   (low  8 bits of the 16-bit immediate)
```

Opcodes used in v0.2.0: `0x0` (HLT), `0x1` (LDA).  Future opcodes
will populate `0x2..0xF`.

### Tests

21 unit + 1 doctest passing.  New coverage:
- LDA opcode nibble pinned.
- `const N; ret v` for N ∈ {0, 5}: exact byte sequence.
- max positive (32767), min negative (-32768), and -1: two's
  complement reinterpretation.
- 16-bit overflow (65536) errors with `InvalidOperand`.
- `Bool(true)` / `Bool(false)` lower to `LDA 1` / `LDA 0`.
- `ret_void`-only function: 3-byte HLT.
- Trivial-case 6-byte ROM size pinned across 8 value points.
- Multi-const where ret targets the current ACC owner: works.
- `ret` of a stale ACC owner: `UndefinedVariable`.
- `ret` of a never-defined variable: `UndefinedVariable`.
- `mov` (unsupported in v0.2.0): `UnsupportedOp { op: "mov" }`.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors iir-to-intel4004 v0.2.0 (A4+) exactly in spirit.

## v0.1.0 — 2026-06-02 — A5 skeleton

Initial release.  Establishes the IIR → GE-225 backend's public
surface as the fifth architecture-backend slot (after iir-to-riscv,
iir-to-intel8008, iir-to-armv7, iir-to-intel4004) — and the most
exotic by a wide margin (20-bit words, mainframe accumulator model,
1959 silicon).

### Added

- `IIRGe225Config` — module-name-carrying config (reserved for
  future symbol-table / `.bin` header use).
- `IIRGe225Error` — backend-side error type with four variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`,
  `InvalidOperand`.  Mirrors the iir-to-intel4004 / iir-to-armv7 /
  iir-to-intel8008 / iir-to-riscv error surface so callers can
  pattern-match identically across backends.
- `validate_for_ge225(&IIRModule) -> Vec<String>` — stub validator
  (always returns `[]` in v0.1.0).
- `lower_iir_to_ge225(&IIRModule, &IIRGe225Config) -> Result<Vec<u8>,
  IIRGe225Error>` — lowering entry point.  Currently emits the
  3-byte canonical HLT sentinel regardless of input.
- `pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00]` — the all-zeros
  20-bit GE-225 HLT word, packed big-endian.  Documented choice
  (vs branch-to-self / unimplemented-opcode) recorded in the spec
  and the constant's doc comment.

### Why the all-zeros HLT halt sentinel?

The GE-225's `HLT` instruction is the all-zeros 20-bit word.
Emitted at the start of program ROM, it halts the machine
deterministically — recognized by every GE-225 simulator and the
historical silicon.  Alternative halt idioms (branch-to-self) would
work but produce less visually obvious bytes.

### Word packing

GE-225 words are 20 bits; we pack each into 3 bytes (24 bits),
big-endian, with the top 4 bits of byte 0 always zero.  A downstream
simulator reads 3 bytes per instruction, masks off the top 4 bits,
and recovers the original 20-bit word.

### Scope notes

- No instruction lowering — deferred to v0.2.0 (A5+).
- No `lang-aot --emit=ge225` wiring — deferred to A5+++.
- No external assembler / linker integration.

### Tests

7 tests covering: empty-module validation, output shape, exact
halt bytes, `HALT_WORD` constant pinning, default-config invariant,
`IIRGe225Config::new` builder contract, and Display smoke for all
four `IIRGe225Error` variants.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
