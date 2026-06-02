# Changelog — iir-to-armv7

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.6] — 2026-06-02 (A3++.6 — real `call` via BL + module-level call backpatching)

### Added — function calls

Wires the final control-flow primitive needed before AOT integration:
real cross-function calls via ARMv7's `BL` (branch with link).

| IIR op | A32 lowering |
|--------|--------------|
| `call dest, "fn"` | `BL <fn_pc_rel>` (1 word) + (optional `MOV dest_reg, r0`) |

The silicon writes `PC + 4` (the return address) into `LR` (`r14`)
before branching, so a subsequent `BX LR` in the callee returns to
the next instruction in the caller.  `ret` already emits `BX LR`
from v0.3.0 — no changes needed there.

#### CRITICAL encoding note

`BL` is `0xEB00_0000`, NOT `0xEA00_0000`.  The bit-24 difference
distinguishes "branch with link" (function call) from "branch"
(goto).  Same family-bit hazard as the 8008's `JMP 0x7C ↔ CAL 0x7E`
and `JFC 0x40 ↔ JTC 0x44` confusions flagged in v0.3.4 and v0.3.8.

#### Module-level call backpatching

Module-level resolution mirrors the 8008's v0.3.9 pattern:

* `function_addrs: HashMap<String, usize>` records each function's
  start **word index** as we walk `module.functions` in source order.
* `pending_calls: Vec<(usize, String, String)>` records `(slot,
  callee, caller)` for every emitted BL.
* Post-loop pass walks `pending_calls`, resolves callees via
  `function_addrs` (returns `UndefinedFunction` if missing),
  computes `imm24 = target - slot - 2` (PC-relative with the +8
  prefetch quirk), range-checks against signed 24-bit
  (`BranchOutOfRange`), and ORs the encoded word into the
  placeholder slot.

#### Why no entry-point HLT-vs-RET special case (unlike the 8008)?

On the 8008, calling RET from the entry-point function would
underflow the empty internal return stack — that's why v0.3.9
introduced the `is_entry` HLT-vs-RET discipline.

ARMv7 has no internal return stack — LR is just a normal register.
At module entry, LR holds whatever the OS (or boot loader) passed
in; `BX LR` from the entry function returns to that address, which
is the correct behaviour for a userspace program.  No special case
needed.

#### Calling convention

`call dest, "fn"` lowers to:

```text
BL <fn>                       ; 1 word (0xEB00_0000 | imm24)
[optional]  MOV dest_reg, r0  ; capture return value if dest != r0
```

ARMv7 AAPCS uses `r0` as both the first argument and the return
value register.  v0.4.6 only supports zero-arg calls — the
register-allocator contract for arg passing folds into the AOT
wiring in A3+++ (mirrors the 8008's deferral pattern).

#### New constant

* `pub const BL_BASE: u32 = 0xEB00_0000;` — BL with cond=AL.

#### New error variant

* `IIRArmv7Error::UndefinedFunction { caller, callee }` — `call`
  referenced a name not present in `module.functions`.

#### Tests added (67 total, was 61)

* `bl_base_pinned_to_0xeb000000` — guards against the bit-24
  `B ↔ BL` confusion.
* `call_emits_bl_with_backpatched_pc_relative_offset` — pinned
  full 4-word `main → helper` byte stream `EB000000 E12FFF1E
  E3A00007 E12FFF1E` for a forward call with imm24=0.
* `call_with_helper_before_main_emits_negative_offset` — pinned
  `0xEBFFFFFC` for a backward call (imm24=-4 in 24-bit
  two's-complement).
* `call_to_undefined_function_is_rejected`.
* `call_with_no_dest_discards_return_value` — void-call shape.
* `errors_for_undefined_function_display_without_panic`.

A new `multi_fn_module(entry, functions)` test helper was added.

### What is NOT in v0.4.6 (deferred to A3+++ / future)

* **Argument passing** — calls are zero-arg.  Arg passing needs a
  per-call register-allocation contract (which AAPCS argument
  registers the callee preserves) and folds into the AOT wiring.
* **Cross-module calls** — `call` to a function defined in a
  different module needs external symbol resolution.  Same.
* **Stack spilling** once the 13-register pool exhausts.
* **`lang-aot --emit=armv7`** — A3+++ (v0.5.0).

### A3++.6 complete: ARMv7 backend feature-complete for AOT

After 6 slices (A3 skeleton + A3+/A3++ register allocator + A3++.5
arithmetic + A3++.5.5 bitwise/carry/cmp/branches + A3++.6 calls),
the ARMv7 backend now supports the full IIR core:

* All arithmetic + bitwise ops (add/sub/adc/sbb/and/or/xor)
* All six boolean comparisons (cmp/cmp_ne/cmp_lt/cmp_gt/cmp_gte/cmp_lte)
* Control flow (label/jmp/jmp_if_true/jmp_if_false)
* Function calls and returns (call/ret/ret_void)

~60 IIR opcodes mapped to ~25 distinct ARMv7-A instruction families,
all byte-exact tested.  The crate is now feature-complete for AOT
wiring in A3+++.

## [0.4.5] — 2026-06-02 (A3++.5.5 fifth slice — `label` + `jmp` + `jmp_if_true`/`jmp_if_false`)

### Added — control flow via Bcond + two-pass backpatching

Wires the first control-flow primitives.  Four new IIR ops:

| IIR op | A32 lowering |
|--------|--------------|
| `label "<name>"` | zero words; records `(name → current_word_index)` |
| `jmp "<name>"` | `B target` (1 word; cond = AL = `0xEA00_0000`) |
| `jmp_if_true cond, "<name>"` | `CMP cond_reg, #0; BNE target` (2 words) |
| `jmp_if_false cond, "<name>"` | `CMP cond_reg, #0; BEQ target` (2 words) |

#### Why the CMP + Bcond pattern for boolean branches?

ARMv7 has no branch-on-register.  But unlike the 8008 (which needed
ANA A as the TEST idiom), ARMv7 ships a dedicated **CMP Rn, #0**
form on the data-processing-immediate family — it sets the Z flag
from `Rn - 0 == 0` and discards the difference.  Pair it with BNE
(branch if Z clear) or BEQ (branch if Z set) and you've got
2-word boolean branches.

#### Two-pass backpatching, ARMv7-style

ARMv7's B carries a **24-bit signed PC-relative offset in WORDS**
(the silicon shifts left 2 to get bytes).  Pass 1 emits each branch
with a placeholder 0 offset and records `(slot, target_label,
cond_base)` in `pending_branches`.  Pass 2 computes:

```rust
imm24 = target_word_index - slot - 2
```

The `- 2` accounts for ARM's classic 2-stage pipeline prefetch:
at execute time PC = current_instruction_address + 8 bytes = +2 words.
The branch silicon then computes `target = current + 8 + imm24*4`,
giving us back the intended target word index.

Range check: imm24 must fit in signed 24 bits (±2^23 words = ±32 MiB
of code).  Practical functions are far below this; the
`BranchOutOfRange` error variant is forward-compat with much larger
modules.

#### New constants

* `pub const B_BASE: u32 = 0xEA00_0000;` — B with cond=AL
* `pub const B_NE_BASE: u32 = 0x1A00_0000;` — BNE
* `pub const B_EQ_BASE: u32 = 0x0A00_0000;` — BEQ
* `pub const CMP_IMM_ZERO_BASE: u32 = 0xE350_0000;` — CMP Rn, #0

The condition nibble pattern continues from v0.4.4: each
conditional branch base is just `(cond << 28) | 0x0A00_0000`.

#### New encoder helpers

* `encode_cmp_imm_zero(rn: u8) -> u32`
* `encode_branch(cond_base: u32, imm24: i32) -> u32` — masks the
  signed 24-bit imm and ORs it with the base.

#### New error variants

* `IIRArmv7Error::UndefinedLabel { function, label }`
* `IIRArmv7Error::BranchOutOfRange { function, target, current }`

#### Tests added (61 total, was 51)

* 4 constant-pinning tests (`B_BASE`, `B_NE_BASE`, `B_EQ_BASE`,
  `CMP_IMM_ZERO_BASE`).
* `jmp_to_forward_label_backpatches_correct_offset` — pinned
  `B target = 0xEA00_0000 | 0` for a forward jump to imm24=0.
* `jmp_to_backward_label_emits_negative_offset` — pinned
  `0xEAFFFFFD` for an imm24 of -3 (showing the two's-complement
  24-bit masking).
* `jmp_to_undefined_label_is_rejected` — confirms the
  `UndefinedLabel` error path.
* `jmp_if_true_emits_cmp_zero_then_bne` — pinned
  `0xE3A0_0001 / 0xE350_0000 / 0x1A00_0000 / 0xE3A0_1000 / BX_LR`.
* `jmp_if_false_emits_cmp_zero_then_beq` — same shape with
  `0x0A00_0000` (BEQ).
* `errors_for_branch_variants_display_without_panic`.

### What is NOT in v0.4.5 (deferred to v0.4.6 / A3++.6 / A3+++)

* `call <fn_name>` — `BL` instruction (B with link, opcode 1
  instead of 0) + return-value-from-r0 capture.
* `ret` for non-entry functions emits BX LR — that already works
  from v0.3.0.  Real RET via the AAPCS frame-pointer dance lands
  in A3++.6.
* Stack spilling once the 13-register pool exhausts.
* `lang-aot --emit=armv7` wiring — A3+++ (v0.5.0).

## [0.4.4] — 2026-06-02 (A3++.5.5 fourth slice — `cmp_ne`/`cmp_lt`/`cmp_gt`/`cmp_gte`/`cmp_lte` via condition prefixes)

### Added — full boolean comparison family

Completes the boolean-comparison surface with the remaining 5
variants.  All share v0.4.3's CMP + MOV + MOV<cond> capture
skeleton — only the trailing MOV's 4-bit condition prefix changes:

| IIR op   | Condition | MOV-cond base | Skip-true unless         |
|----------|-----------|---------------|--------------------------|
| `cmp`    | `EQ`      | `0x03A0_0000` | Z=1 (equal)              |
| `cmp_ne` | `NE`      | `0x13A0_0000` | Z=0 (not equal)          |
| `cmp_lt` | `CC`      | `0x33A0_0000` | C=0 (unsigned <)         |
| `cmp_gt` | `HI`      | `0x83A0_0000` | C=1 ∧ Z=0 (unsigned >)   |
| `cmp_gte`| `CS`      | `0x23A0_0000` | C=1 (unsigned >=)        |
| `cmp_lte`| `LS`      | `0x93A0_0000` | C=0 ∨ Z=1 (unsigned <=)  |

The condition prefix sits in bits 31..28 of every A32 instruction;
each new MOV<cond> base is just `(cond << 28) | 0x03A0_0000`.

#### Why ARMv7's HI condition is special

ARMv7 ships a dedicated condition for "unsigned greater than" (HI),
so `cmp_gt` doesn't need the operand-swap trick the 8008 and RV32I
backends use (`cmp_gt a, b ⇔ cmp_lt b, a`).  Same number of
instructions emitted, but conceptually cleaner.

#### Unified 6-way match arm

The v0.4.3 `"cmp"` arm widens to `"cmp" | "cmp_ne" | "cmp_lt" |
"cmp_gt" | "cmp_gte" | "cmp_lte"` with an inner match on op-name →
condition-MOV base.  The encode_mov_imm_cond helper takes the base
and emits the cond-prefixed word:

```rust
let cond_mov_base = match instr.op.as_str() {
    "cmp"     => MOV_IMM_EQ_BASE,
    "cmp_ne"  => MOV_IMM_NE_BASE,
    "cmp_lt"  => MOV_IMM_CC_BASE,
    "cmp_gt"  => MOV_IMM_HI_BASE,
    "cmp_gte" => MOV_IMM_CS_BASE,
    "cmp_lte" => MOV_IMM_LS_BASE,
    _ => unreachable!(),
};
words.push(encode_cmp_reg(rn, rm));
words.push(encode_mov_imm(rd, 0));
words.push(encode_mov_imm_cond(cond_mov_base, rd, 1));
```

#### New constants (5 of them)

* `pub const MOV_IMM_NE_BASE: u32 = 0x13A0_0000;`
* `pub const MOV_IMM_CC_BASE: u32 = 0x33A0_0000;`
* `pub const MOV_IMM_CS_BASE: u32 = 0x23A0_0000;`
* `pub const MOV_IMM_HI_BASE: u32 = 0x83A0_0000;`
* `pub const MOV_IMM_LS_BASE: u32 = 0x93A0_0000;`

#### New generic encoder helper

* `encode_mov_imm_cond(cond_base: u32, rd: u8, imm8: u8) -> u32` —
  ORs in `(rd << 12) | imm8` over any cond-MOV base.  Avoids 6
  nearly-identical named helpers; the v0.4.3 `encode_mov_imm_eq`
  is kept as a named convenience but now delegates conceptually
  to this generic shape.

#### Tests added (51 total, was 41)

* 5 constant-pinning tests guarding each new condition-MOV base.
* 5 lowering tests using a shared `assert_cmp_variant` helper that
  pins the full 7-word output for each variant against the
  canonical `v=10, w=20, r=v OP w; ret r` template — only the
  expected MOV<cond> word at index 4 changes per test.

### What is NOT in v0.4.4 (deferred to v0.4.5 / A3++.6 / A3+++)

* Conditional branches (`Bcond`) — `B` opcode with a non-AL cond
  prefix.  Same condition-field mechanism, different instruction
  family.
* Signed comparisons (`cmp_slt`/`cmp_sgt` via LT/GT conditions
  which read the V overflow flag together with N sign flag).
* Function calls via `bl` + stack spilling.
* `lang-aot --emit=armv7` wiring — A3+++ (v0.5.0).

## [0.4.3] — 2026-06-02 (A3++.5.5 third slice — `cmp` equality via the EQ condition prefix)

### Added — boolean equality with conditional MOV capture

Wires IIR's `cmp dest, a, b` (boolean equality) to ARMv7's CMP +
flag-to-bool capture using the **EQ condition prefix on MOV** — a
feature unique to ARMv7 among the three architecture backends.

#### Lowering shape (4 words after the const-loads — no
backpatching!)

```text
CMP   rn, rm             ; sets Z if rn == rm
MOV   dest, #0           ; default false (cond = AL)
MOVEQ dest, #1           ; if Z=1 (equal), overwrite to true
```

The cond-field is the 4 high bits of every A32 instruction.  Setting
it to **EQ = 0000** instead of the usual **AL = 1110** changes the
opcode word from `0xE3A0_X00N` to `0x03A0_X00N` — the silicon
checks the Z flag and either executes or no-ops the instruction.

#### Compare with other backends' flag-to-bool capture

| Backend | Words/bytes | Mechanism |
|---------|-------------|-----------|
| **ARMv7** | **4 words** | CMP + MOV + MOVEQ — no backpatching |
| RV32I | 1 word (`sltu`) | Set-less-than-unsigned reads the comparison and writes 0/1 directly |
| Intel 8008 | 8 bytes | CMP + MVI dest,0 + JFZ + 2 addr bytes + MVI dest,1 — inline-backpatched JFZ target |

ARMv7's `cond` field gives it middle ground: more verbose than
RV32I's purpose-built SLT but cleaner than the 8008's address-
backpatched skip-jump-over-MVI dance.

#### New constants

* `pub const CMP_REG_BASE: u32 = 0xE150_0000;` — CMP Rn, Rm base
  (S=1 forced, no Rd output).
* `pub const MOV_IMM_EQ_BASE: u32 = 0x03A0_0000;` — MOV imm under
  EQ condition prefix.  Identical to `MOV_IMM_R0_BASE` except the
  top nibble is `0` (EQ) instead of `E` (AL).

#### New encoder helpers

* `encode_cmp_reg(rn, rm) -> u32` — note: only two args because
  CMP has no Rd.
* `encode_mov_imm_eq(rd, imm8) -> u32` — MOV imm with EQ cond.

#### Tests added (41 total, was 37)

* `cmp_reg_base_pinned_to_0xe1500000` / `mov_imm_eq_base_pinned_to_0x03a00000`.
* `cmp_pins_full_capture_word_stream` — pinned full 7-word sequence
  `0xE3A0_0005 0xE3A0_1005 0xE150_0001 0xE3A0_2000 0x03A0_2001
  0xE1A0_0002 0xE12F_FF1E`.
* `cmp_with_same_register_emits_cmp_a_a_then_capture` — `cmp r v v`
  case: `CMP r0, r0 = 0xE150_0000`.

### What is NOT in v0.4.3 (deferred to v0.4.4 / A3++.6 / A3+++)

* `cmp_ne`/`cmp_lt`/`cmp_gt`/`cmp_gte`/`cmp_lte` — would use NE,
  CC, HI, CS, LS condition prefixes respectively.  The same capture
  shape applies; only the cond-field on the second MOV varies.
* Conditional branches (`Bcond`) — `B` opcode with a non-AL cond
  prefix.
* S-suffix flag-setting variants of all 7 ALU ops (ADDS, SUBS,
  ANDS, etc.) — needed for cmp_lt / cmp_gt which read the carry
  and sign flags.
* Function calls via `bl` + stack spilling.
* `lang-aot --emit=armv7` wiring — A3+++ (v0.5.0).

## [0.4.2] — 2026-06-02 (A3++.5.5 second slice — carry-chained `adc`/`sbb` on the DP-register family)

### Added — carry-chained DP-register ALU

Extends v0.4.1 with two more DP-register ops that consume the C flag
set by a PRIOR flag-affecting ALU op:

| IIR op | ARM mnemonic | Opcode | First word base |
|--------|--------------|--------|-----------------|
| `adc dest, a, b` | `ADC Rd, Rn, Rm` | `0101` | `0xE0A0_0000` |
| `sbb dest, a, b` | `SBC Rd, Rn, Rm` | `0110` | `0xE0C0_0000` |

The match arm widens from `add | sub | and | or | xor` to `add |
sub | and | or | xor | adc | sbb` — 7-way inner dispatch on
op-name → encoder.

#### Carry-flag contract

This crate emits the **non-S** form by default (no flag update).
Front-ends that need the carry chain must arrange for the producer
to use the S-suffix variant (`ADDS` / `SUBS` etc.) so the C flag
survives.  The S-suffix flag-setting variants of all seven ops land
alongside `cmp` in v0.4.3, paired with conditional branches.

In the canonical u16-add idiom:

```text
adds r_lo lo_a lo_b   ; sets C if overflow
adc  r_hi hi_a hi_b   ; consumes that C
```

#### New constants

* `pub const ADC_REG_BASE: u32 = 0xE0A0_0000;`
* `pub const SBC_REG_BASE: u32 = 0xE0C0_0000;`

#### New encoder helpers

* `encode_adc_reg(rd, rn, rm) -> u32`
* `encode_sbc_reg(rd, rn, rm) -> u32`

#### Tests added (37 total, was 33)

* `adc_reg_base_pinned_to_0xe0a00000` / `sbc_reg_base_pinned_to_0xe0c00000`.
* `adc_three_consts_emits_single_adc_instruction` — pinned
  `ADC r2, r0, r1 = 0xE0A0_2001`.
* `sbb_three_consts_emits_single_sbc_instruction` — `SBC r2, r0, r1
  = 0xE0C0_2001`.

### What is NOT in v0.4.2 (deferred to v0.4.3 / A3++.6 / A3+++)

* The S-suffix (flag-setting) variants of all 7 ALU ops (ADDS,
  SUBS, ANDS, etc.) — needed to make ADC/SBC actually chain.
* `cmp` (CMP opcode `1010`) + flag-to-bool capture using
  conditional jumps.
* Conditional branches via the `cond` field every A32 instruction
  carries (using bits 31..28 instead of the usual 0xE = always).
* Function calls via `bl` + stack spilling.
* `lang-aot --emit=armv7` wiring — A3+++ (v0.5.0).

## [0.4.1] — 2026-06-02 (A3++.5.5 first slice — bitwise `and`/`or`/`xor` on the DP-register family)

### Added — bitwise DP-register ALU

Extends v0.4.0 with three more accumulator-target — wait, NOT
accumulator-target.  ARMv7's DP-register family is true 3-register
(`Rd = Rn op Rm`), so unlike the 8008's bitwise ANA/ORA/XRA which
needed accumulator wrappers, ARMv7's AND/ORR/EOR slot in
identically to ADD/SUB.

| IIR op | ARM mnemonic | Opcode | First word base |
|--------|--------------|--------|-----------------|
| `and dest, a, b` | `AND Rd, Rn, Rm` | `0000` | `0xE000_0000` |
| `or  dest, a, b` | `ORR Rd, Rn, Rm` | `1100` | `0xE180_0000` |
| `xor dest, a, b` | `EOR Rd, Rn, Rm` | `0001` | `0xE020_0000` |

ARM uses the older mnemonics **ORR** (OR Register) and **EOR**
(Exclusive OR) rather than the universal `OR`/`XOR`.  We pin both
in the constant names so the bytes round-trip correctly through the
`arm-simulator` disassembler, but the IIR-facing op names keep the
universal `or`/`xor` spellings (matching iir-to-intel8008 and
iir-to-riscv).

### Unified DP-register match arm

The `add | sub` arm from v0.4.0 widens to `add | sub | and | or |
xor` with a 5-way inner match on op-name → encoder.  All five share
the same operand-extraction/register-allocation/word-push skeleton;
only the encoder function differs.

### New constants

* `pub const AND_REG_BASE: u32 = 0xE000_0000;`
* `pub const ORR_REG_BASE: u32 = 0xE180_0000;`
* `pub const EOR_REG_BASE: u32 = 0xE020_0000;`

### New encoder helpers

* `encode_and_reg(rd, rn, rm) -> u32`
* `encode_orr_reg(rd, rn, rm) -> u32`
* `encode_eor_reg(rd, rn, rm) -> u32`

All `debug_assert!` their three 4-bit register selectors.

### Tests added (33 total, was 27)

* `and_reg_base_pinned_to_0xe0000000` / `orr_reg_base_pinned_to_0xe1800000`
  / `eor_reg_base_pinned_to_0xe0200000`.
* `and_three_consts_emits_single_and_instruction` — pinned 5-word
  sequence with `AND r2, r0, r1 = 0xE000_2001`.
* `or_three_consts_emits_single_orr_instruction` — `ORR r2, r0, r1 =
  0xE180_2001`.
* `xor_three_consts_emits_single_eor_instruction` — `EOR r2, r0, r1 =
  0xE020_2001`.

### What is NOT in v0.4.1 (deferred to v0.4.2 / A3++.6 / A3+++)

* Carry-chained arithmetic (`adc`/`sbc` on the same DP family with
  opcodes `0101`/`0110`).
* Comparisons (`cmp` = `1010`, `cmn` = `1011`) — set flags only,
  need a paired flag-to-bool capture sequence to produce an IIR
  boolean.
* Conditional branches via the `cond` field every A32 instruction
  carries.
* Function calls via `bl` with PC-relative offsets.
* Stack spilling once the 13-register pool exhausts.
* `lang-aot --emit=armv7` wiring — A3+++ (v0.5.0).

## [0.4.0] — 2026-06-02 (A3++.5 — `add`/`sub` on the data-processing-register family)

### Added — 3-register ALU

Extends v0.3.0 with two ALU ops on the data-processing-register
encoding family.  Unlike the 8008's `ADD r` (accumulator-anchored —
`A = A + r`), ARMv7's `ADD Rd, Rn, Rm` is a true 3-register
operation: `Rd = Rn + Rm` in a single instruction with no staging
MOVs.  Same shape as RV32I's `add rd, rs1, rs2`.

| IIR op | A32 lowering |
|--------|--------------|
| `add dest, a, b` | `ADD Rd, Rn, Rm` (`0xE080_0000 \| (Rn << 16) \| (Rd << 12) \| Rm`) |
| `sub dest, a, b` | `SUB Rd, Rn, Rm` (`0xE040_0000 \| ...`) |

### New constants

* `pub const ADD_REG_BASE: u32 = 0xE080_0000;` — `ADD r0, r0, r0`
  base (cond=AL, opcode=0100, S=0, no shift).
* `pub const SUB_REG_BASE: u32 = 0xE040_0000;` — same shape, opcode
  `0010` (SUB).

The opcode field (bits 24..21) is the only differing nibble between
ADD and SUB.  Both fit inside the 7-bit `cond | 000 | opcode | S`
prefix.

### New encoder helpers

* `encode_add_reg(rd: u8, rn: u8, rm: u8) -> u32`
* `encode_sub_reg(rd: u8, rn: u8, rm: u8) -> u32`

Both `debug_assert!` their three 4-bit register selectors.

### Tests added (27 total, was 21)

* `add_reg_base_pinned_to_0xe0800000` / `sub_reg_base_pinned_to_0xe0400000`.
* `add_three_consts_emits_single_instruction_no_staging` — pinned
  5-word sequence `0xE3A0_0003 0xE3A0_1004 0xE080_2001 0xE1A0_0002
  0xE12F_FF1E` (MOV r0,#3 / MOV r1,#4 / ADD r2,r0,r1 / MOV r0,r2 /
  BX LR).
* `sub_three_consts_emits_sub_instruction_with_correct_opcode_field`
  — same shape, `SUB r2, r0, r1 = 0xE040_2001`.
* `add_with_same_register_uses_it_as_both_rn_and_rm` — `add r v v`
  case: `ADD r1, r0, r0 = 0xE080_1000`.
* `add_then_ret_into_non_r0_register_emits_staging_mov` — regression
  that the v0.3.0 ret-staging logic still fires for ALU dest
  registers.

### What is NOT in v0.4.0 (deferred to A3++.5.5 / A3++.6 / A3+++)

* Bitwise ops `and`/`or`/`xor` (opcodes `0000`, `1100`, `0001` in
  the same data-processing-register family) — straightforward
  extension once the test patterns are in place.
* Wider arithmetic (`mul`/`mla` on the multiply family).
* Comparisons + conditional branches via the cond-field every A32
  instruction carries.
* Function calls via `bl` with PC-relative offsets + stack
  spilling.
* `lang-aot --emit=armv7` wiring — A3+++ (v0.5.0).

## [0.3.0] — 2026-06-02 (A3++ — linear register allocator + `mov` + ret-value staging)

### Added — linear register allocator over r0..r12

Extends v0.2.0's accumulator-only `const` to a real allocator that
hands out the 13 ARMv7 general-purpose registers in order:
`r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12`.  `r0` comes
first so the trivial `const v; ret v` case stays at the 2-word shape
(`MOV r0, #imm; BX LR`) without an extra `MOV r0, X` round-trip.

`r13` (`sp`), `r14` (`lr`), and `r15` (`pc`) are NOT in the pool —
touching them as locals would break the calling convention's stack
discipline, the return address, or the instruction pointer.

| IIR op | A32 lowering |
|--------|--------------|
| `const dest, Int(n)` | `MOV rrr, #n` (`MOV_IMM_R0_BASE | (rrr << 12) | n`) |
| `mov dest, src` | `MOV Rd, Rm` (`MOV_REG_BASE | (Rd << 12) | Rm`) |
| `ret <var>` | stage `var` into `r0` via `MOV r0, var_reg` if needed, then `BX LR` |
| `ret_void` | `BX LR` |

### New constants

* `pub const MOV_REG_BASE: u32 = 0xE1A0_0000;` — `MOV r0, r0` base
  encoding for the register-to-register form.  OR in `(Rd << 12) |
  Rm` for arbitrary register pairs.

CAREFUL: bit-25 distinguishes this from `MOV_IMM_R0_BASE` (which is
`0xE3A0_0000`).  Data-processing-immediate has bit-25 set; register-
form doesn't.

### New encoder helper

* `encode_mov_reg(rd: u8, rm: u8) -> u32` — emits `MOV Rd, Rm` with
  `debug_assert!`s on both 4-bit register selectors.

### New error variants

* `IIRArmv7Error::UndefinedVariable` — `mov` or `ret` referenced a
  name that was never bound.
* `IIRArmv7Error::OutOfRegisters` — 14th local exhausted the 13-
  register pool.  Stack spilling lands in A3++.5 or later.

### Tests added (21 total, was 14)

* `mov_reg_base_pinned_to_0xe1a00000` — guards the new constant.
* `two_consts_use_r0_then_r1_then_mov_r0_r1_before_bx_lr` — pinned
  exact 4-word sequence including `MOV r1, #2 = 0xE3A0_1002` (the
  `1` in the `Rd` nibble) and `MOV r0, r1 = 0xE1A0_0001`.
* `ret_of_first_const_omits_the_redundant_mov` — regression for
  the v0.2.0 2-word trivial-case shape (r0-first allocator preserves
  it).
* `mov_lowers_to_canonical_mov_rd_rm` — `MOV r1, r0 = 0xE1A0_1000`
  and `MOV r0, r1 = 0xE1A0_0001` shown side-by-side.
* `allocator_exhaustion_yields_out_of_registers` — 14 consts → fails
  on `v13` with `OutOfRegisters`.
* `undefined_variable_in_mov_is_rejected`.
* `errors_for_new_variants_display_without_panic`.

### What is NOT in v0.3.0 (deferred to A3++.5 and beyond)

* Arithmetic (`add`/`sub`/`mul`) and the data-processing-register
  family those expand into.
* Function calls via `bl` with PC-relative offsets.
* Comparisons + conditional branches via the `cond` field every
  A32 instruction carries.
* Wider immediates via `movw`/`movt` (the ARMv7 16-bit-imm pair).
* `lang-aot --emit=armv7` wiring — A3+++.

## [0.2.0] — 2026-06-02 (A3+ — `const` → `MOV r0, #imm8`; `ret`/`ret_void` → `BX LR`)

### Added — first real instruction lowering

Extends v0.1.0's BKPT-only skeleton with three IIR ops:

| IIR op | A32 lowering |
|--------|--------------|
| `const dest, Int(n)` (8-bit imm) | `mov r0, #n` (`0xE3A0_00NN`) |
| `ret <var>` (int) | `bx lr` (`0xE12FFF1E`) — `var` is already in `r0` |
| `ret_void` | `bx lr` |

The accumulator-only first slice: every `const` goes into `r0`.
Multi-register allocation (`r1..r12`) and a real linear allocator
land in A3++ alongside arithmetic.

#### Why `BX LR` for both `ret` and `ret_void`?

In the ARM AAPCS calling convention, `r0` (a.k.a. `a1`) is the
return-value register.  Since every `const` already lands in `r0`,
`ret <var>` doesn't need any staging MOV — the value is in the
right place by construction.  `ret_void` simply doesn't care about
the value, so the same `bx lr` works.

(Compared to the 8008 backend's v0.2.0 which used `HLT` for `ret`
as a temporary stand-in until v0.3.9 wired up real `RET`, ARMv7's
`bx lr` is a proper return from the start.)

#### Why `BX LR` over `MOV PC, LR`?

`MOV PC, LR` would work on pure A32 cores but doesn't switch
instruction sets when the lr's low bit signals "return to Thumb
mode".  `BX LR` reads the low bit, switches mode accordingly, and
branches — the canonical AAPCS return.  On a pure A32 module
emitted by this crate, the low bit will always be 0 so both
behave identically, but emitting `BX LR` keeps the output
interop-correct with any caller that mixes A32 and Thumb code.

#### New constants in the public surface

* `pub const BX_LR: u32 = 0xE12FFF1E;` — branch-to-link-register
  (AAPCS return).
* `pub const MOV_IMM_R0_BASE: u32 = 0xE3A0_0000;` — `mov r0, #0`
  base; OR in the 8-bit immediate to form `MOV r0, #N`.

CAREFUL: `BX_LR = 0xE12FFF1E` vs `BKPT = 0xE12FFF7F` — bit-7
differs.  Both share the same `12F_FF` family bits, so the bit-7
nibble difference is the canonical confusion point.

#### Immediate byte range

`const` accepts integers in `[-128, 255]`:
* `[0, 255]` cast straight to `u8`.
* `[-128, -1]` reinterpreted as two's-complement (`-1 → 0xFF`).
* Anything outside → `InvalidOperand` with a message naming the
  8-bit limit and pointing forward to `movw`/`movt` (the ARMv7+
  wide-immediate idiom that lands in A3++).

#### New internal helper

* `encode_mov_imm(rd: u8, imm8: u8) -> u32` — encodes the full
  `MOV Rd, #imm8` word.  `debug_assert!`s on the 4-bit `rd` range.

#### Tests added (14 total, was 7)

* `bx_lr_constant_pinned_to_e12fff1e` — guards against the
  `BKPT ↔ BX_LR` (bit-7) confusion.
* `mov_imm_r0_base_pinned_to_0xe3a00000`.
* `const_42_then_ret_lowers_to_mov_r0_42_then_bx_lr` — pinned
  exact 2-word sequence `0xE3A0002A 0xE12FFF1E`.
* `const_negative_uses_twos_complement_byte` (`-1 → 0xFF`).
* `const_out_of_byte_range_is_rejected` (1000 overflows).
* `ret_void_alone_emits_just_bx_lr`.
* `unsupported_op_is_rejected_with_function_name` — `safepoint`
  rejected with function name preserved in the error.

### What is NOT in v0.2.0 (deferred to A3++ and beyond)

* Multi-register allocator over `r1..r12` + `mov rd, rs` register-
  to-register lowering.
* Arithmetic (`add`/`sub`/`mul`).
* Wider immediates via `movw`/`movt` (ARMv7's 16-bit-imm pair) or
  rotated 8-bit values (the A32 12-bit-imm field's full power).
* Function calls via `bl` and PC-relative offsets.
* Comparisons + conditional branches via the `cond` field every
  A32 instruction carries (a stronger version of the 8008's
  flag-based jumps).
* `lang-aot --emit=armv7` wiring — A3+++.

## [0.1.0] — 2026-06-02 (A3 — crate skeleton)

### Added — `BKPT`-only emission

First release.  Implements item A3 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single ARMv7-A `BKPT #0xFFFF`
instruction (encoding `0xE12FFF7F`).

#### Public surface

```rust
pub struct IIRArmv7Config { pub module_name: String }
impl IIRArmv7Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRArmv7Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_armv7(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_armv7(
    module: &IIRModule,
    cfg: &IIRArmv7Config,
) -> Result<Vec<u32>, IIRArmv7Error>;

pub const BKPT: u32 = 0xE12FFF7F;
```

#### Why an ARMv7 backend?

ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
the LANG VM backend lane.  It covers Cortex-A7/A8/A9-era SoCs and
many embedded boards (early Raspberry Pi, BeagleBone, Olimex
A20-OLinuXino) — vastly more deployed silicon than any single 8008
chip ever shipped, but architecturally a clean fixed-width 32-bit
RISC like RV32I with one big twist: every A32 instruction carries a
4-bit `cond` field that can predicate execution.

A3 establishes the third architecture backend alongside RV32I (A1)
and Intel 8008 (A2).  Together they exercise the IIR's neutrality
across genuinely different ISAs:

- **RV32I**: clean 32-bit RISC, load-store, no condition codes.
- **Intel 8008**: irregular 8-bit accumulator CISC, 14-bit address
  bus — historical fidelity for Oct.
- **ARMv7**: 32-bit RISC with cond prefixes on every instruction
  plus a barrel shifter on the second operand.

#### Why `Vec<u32>` output, not textual asm?

* **Round-trips with `arm-simulator`** — its decoder consumes raw
  little-endian 32-bit words directly.
* **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
  is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
  and ARMASM divergence we don't want to entangle with.
* **Trivial encoding shape** — every A32 instruction is exactly 4
  bytes (in stark contrast to the 8008's 1/2/3 byte variability).

#### The `BKPT #0xFFFF` encoding

`0xE12FFF7F`.  Bit layout (cond=AL):

```text
31..28  cond    = 0xE = 1110            (always — unconditional)
27..20          = 0001 0010 = 0x12      (BKPT opcode family)
19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
 7.. 4          = 0111 = 0x7            (BKPT opcode family)
 3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
```

Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
`0xE12FFF7F`.

#### Why BKPT and not WFI or `b .`?

| Candidate | Pros | Cons |
|-----------|------|------|
| `BKPT #imm16` | Semantically "stop"; every ARM debugger / emulator recognises it | None for skeleton purposes |
| `WFI`         | True halt | Requires kernel/hypervisor privilege; illegal in userspace |
| `B .`         | Pure userspace, no traps | Burns CPU; harder to detect without a host timeout |

BKPT wins on simplicity + emulator round-trip.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A3+) lowers `const` + `bx lr`.
* **No `lang-aot --emit=armv7`.**  Deferred to v0.4.0 (A3+++).
* **No external assembler / linker integration.**

#### Tests added (7 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_word`
* `lower_emits_the_canonical_bkpt_word` (exact `0xE12FFF7F`)
* `bkpt_constant_pinned_to_e12fff7f`
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
