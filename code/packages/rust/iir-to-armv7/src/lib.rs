//! # iir-to-armv7 — IIR → ARMv7 (A32) machine code backend (v0.5.0).
//!
//! ## ⚠ DEPRECATED — use `armv7-backend` instead
//!
//! As of Phase 5 of the historical-arch backend migration
//! ([`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)),
//! this crate is deprecated.  Use:
//!
//! - **`armv7-encoder`** — pure encoding tables.
//! - **`armv7-backend`** — implements `jit_core::backend::Backend`
//!   over monomorphised CIR.
//!
//! `lang-aot --emit=armv7` routes through the new pair as of
//! Phase 5; existing public API of this crate continues to work
//! for backward compatibility but emits deprecation warnings.
//!
//! Note: `armv7-backend` v0.1.0 is a **minimal-viable** port (just
//! `const_*` + `ret_*`).  Programs that need the full op set
//! (add/sub/and/or/xor/adc/sbb/cmp/branches/calls) currently
//! fall through to `Backend::compile` returning `None`.  Future
//! increments to `armv7-backend` can port more.
//!
//! ## Original module docs (still applicable to the lowering algorithm)
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u32>` of encoded
//! 32-bit ARMv7-A (A32) instructions, suitable to drop into the
//! in-tree `arm-simulator` or to write out as a flat `.bin` for
//! `qemu-arm` / `objcopy`.
//!
//! ## Why an ARMv7 backend?
//!
//! ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
//! the LANG VM architecture-backend lane.  It covers Cortex-A7/A8/A9-
//! era SoCs and many embedded boards (early Raspberry Pi, BeagleBone,
//! Olimex A20-OLinuXino) — vastly more deployed silicon than any
//! single 8008 chip ever shipped, but architecturally a clean
//! fixed-width 32-bit RISC like RV32I.
//!
//! Adding ARMv7 as a backend gives us:
//!
//! 1. **A third architecture backend** alongside RV32I (A1) and
//!    Intel 8008 (A2).  The three sit at meaningfully different
//!    points in the design space:
//!    - RV32I: clean 32-bit RISC, load-store, no condition codes.
//!    - Intel 8008: irregular 8-bit accumulator CISC, 14-bit address
//!      bus — historical fidelity for Oct.
//!    - **ARMv7 (A32)**: 32-bit RISC with a `cond` field on EVERY
//!      instruction plus a barrel shifter on the second operand.
//!      Same word width as RV32I, fundamentally different ISA.
//! 2. **Foundation for native phone-OS targets.**  Once the AOT
//!    wiring lands (A3+++), the same LANG VM source program can
//!    cross-compile to ARMv7 Linux executables.
//! 3. **Round-trip with the in-tree `arm-simulator`.**  The
//!    `Vec<u32>` output drops directly into the simulator for
//!    in-process tests.
//!
//! ## Scope of v0.1.0 (A3)
//!
//! This release is a **skeleton**: any IIR module lowers to a single
//! `BKPT #0xFFFF` instruction (encoding `0xE12FFF7F`).  No instruction
//! selection yet; that arrives in A3+ (`const` + `bx lr`) and beyond.
//!
//! ## Why `Vec<u32>` output, not textual asm?
//!
//! - **Round-trips with `arm-simulator`** — its decoder consumes raw
//!   little-endian 32-bit words.
//! - **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
//!   is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
//!   and ARMASM divergence we don't want to entangle with.
//! - **Trivial encoding shape** — every A32 instruction is exactly 4
//!   bytes (in stark contrast to the 8008's 1/2/3 byte variability).
//!
//! ## Quick start
//!
//! ```
//! #![allow(deprecated)]
//! use interpreter_ir::IIRModule;
//! use iir_to_armv7::{validate_for_armv7, lower_iir_to_armv7, IIRArmv7Config};
//!
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![],
//!     entry_point: None,
//!     language: "demo".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! assert!(validate_for_armv7(&module).is_empty());
//!
//! let words = lower_iir_to_armv7(&module, &IIRArmv7Config::default())
//!     .expect("lowering should succeed");
//! // 0xE12FFF7F == ARMv7-A BKPT #0xFFFF.
//! assert_eq!(words, vec![0xE12F_FF7F]);
//! ```
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule
//!   → validate_for_armv7()      pre-flight, returns Vec<String>
//!   → lower_iir_to_armv7()      returns Vec<u32> of A32 words
//!   → (optional)
//!       • arm-simulator: in-process testing
//!       • write to .bin + qemu-arm
//!       • objcopy + linker for an ELF on a phone-class Linux board
//! ```

use interpreter_ir::{IIRModule, Operand};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// ARMv7 (A32) opcode constants
// ===========================================================================
//
// Every A32 instruction is exactly 4 bytes, with a fixed encoding
// template `cond IIII OOOO ... ` where `cond` is the conditional-
// execution prefix (the 4-bit field bits 31..28 every A32 instruction
// carries) and `IIII OOOO` selects the instruction family.  Unlike
// RV32I and the 8008, ARMv7 has no "unconditional" sub-encoding for
// most ops — `cond = 0b1110 = 0xE` is the "always-execute" value used
// everywhere a conditional prefix isn't actively wanted.

/// ARMv7-A `BKPT #0xFFFF` opcode — `0xE12FFF7F`.  Triggers a
/// breakpoint exception; semantically "stop execution".
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond    = 0xE = 1110            (always — unconditional)
/// 27..20          = 0001 0010 = 0x12      (BKPT opcode family)
/// 19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
///  7.. 4          = 0111 = 0x7            (BKPT opcode family)
///  3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
/// ```
///
/// Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
/// `0xE12FFF7F`.
///
/// ## Why BKPT and not WFI or `b .`?
///
/// | Candidate | Pros | Cons |
/// |-----------|------|------|
/// | `BKPT #imm16` | Semantically "stop"; every ARM debugger / emulator recognises it | None for skeleton purposes |
/// | `WFI`         | True halt | Requires kernel/hypervisor privilege; illegal in userspace |
/// | `B .`         | Pure userspace, no traps | Burns CPU; harder to detect without a host timeout |
///
/// BKPT wins on simplicity + emulator round-trip.  The
/// `arm-simulator`'s decoder flags it as `bkpt` and stops single-
/// stepping.
pub const BKPT: u32 = 0xE12F_FF7F;

/// ARMv7-A `BX LR` opcode — `0xE12FFF1E`.  Branches to the address in
/// the link register (`r14`), exchanging instruction sets (which on
/// pure A32 code is a no-op — A32 → A32).  Semantically: "return from
/// this function" per the AAPCS calling convention.
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond  = 0xE = 1110            (always — unconditional)
/// 27..20        = 0001 0010 = 0x12      (BX opcode family)
/// 19.. 8        = 1111 1111 1111 = 0xFFF
///  7.. 4        = 0001 = 0x1            (BX opcode family)
///  3.. 0  Rm    = 1110 = 0xE            (Rm = lr = r14)
/// ```
///
/// Concatenated: `1110 0001 0010 1111_1111_1111 0001 1110` =
/// `0xE12FFF1E`.
///
/// CAREFUL: BX is `0xE12FFF1E`, NOT `0xE12FFF7F` (which is BKPT —
/// the bit-7 difference distinguishes "branch & exchange" from
/// "breakpoint").  Both share the same `12F_FF` family bits.
pub const BX_LR: u32 = 0xE12F_FF1E;

/// ARMv7-A `MOV Rd, #imm8` (data-processing immediate) base
/// encoding for `Rd = r0` — `0xE3A0_0000`.  OR in the 8-bit immediate
/// (bits 7..0) and the destination register (bits 15..12) to form the
/// full instruction word.
///
/// Bit layout (cond=AL, S=0, Rn=0):
///
/// ```text
/// 31..28  cond     = 0xE = 1110           (always — unconditional)
/// 27..25           = 001                   (data-processing immediate)
/// 24..21  opcode   = 1101                 (MOV)
/// 20      S        = 0                     (don't set flags)
/// 19..16  Rn       = 0000                  (unused for MOV)
/// 15..12  Rd       = (in this base, 0)    (target register)
/// 11.. 8  rotate   = 0000                  (no rotation on the imm)
///  7.. 0  imm8     = (in this base, 0)    (the 8-bit value)
/// ```
///
/// Concatenated for `MOV r0, #0`: `1110 0011 1010 0000 0000 0000 0000 0000`
/// = `0xE3A00000`.
///
/// For `MOV r0, #N`: OR in `N` (8 bits).
/// For `MOV Rd, #N`: OR in `(Rd << 12) | N`.
pub const MOV_IMM_R0_BASE: u32 = 0xE3A0_0000;

/// Encode an `ARMv7-A `MOV Rd, #imm8`` instruction.
///
/// `rd` must be in `[0, 15]` (4-bit ARM register selector).  `imm8`
/// is the immediate value, range `[0, 255]`.
///
/// Wider immediates (9-32 bits) require either a rotate (the 12-bit
/// immediate field encodes 8 value bits + 4 rotation bits, allowing
/// any rotated 8-bit value) or a `movw`/`movt` pair (ARMv7+).  Those
/// land in A3++ — v0.2.0's `const` only supports 8-bit values.
pub(crate) fn encode_mov_imm(rd: u8, imm8: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    MOV_IMM_R0_BASE | ((rd as u32) << 12) | (imm8 as u32)
}

/// ARMv7-A `MOV Rd, Rm` (data-processing register) base encoding —
/// `0xE1A0_0000`.  OR in the destination register (bits 15..12) and
/// the source register (bits 3..0) to form the full instruction word.
///
/// Bit layout (cond=AL, S=0, Rn=0, shift=0, type=00):
///
/// ```text
/// 31..28  cond    = 0xE = 1110            (always — unconditional)
/// 27..21          = 0001 101             (data-processing register, MOV opcode)
/// 20      S       = 0                     (don't set flags)
/// 19..16  Rn      = 0000                  (unused for MOV)
/// 15..12  Rd      = (in this base, 0)    (target register)
/// 11.. 7  shift_imm = 00000               (no shift)
///  6.. 5  type    = 00                    (LSL — but shift_imm=0 means no shift)
///  4              = 0                     (shift by immediate, not register)
///  3.. 0  Rm      = (in this base, 0)    (source register)
/// ```
///
/// For `MOV r0, r0`: `1110 0001 1010 0000 0000 0000 0000 0000` =
/// `0xE1A00000`.  For arbitrary `MOV Rd, Rm`: OR in
/// `(Rd << 12) | Rm`.
///
/// CAREFUL: This is the register-to-register MOV, distinct from
/// `MOV_IMM_R0_BASE = 0xE3A0_0000` (note the bit-25 difference —
/// data-processing-immediate has bit-25 set, register form doesn't).
pub const MOV_REG_BASE: u32 = 0xE1A0_0000;

/// Encode an ARMv7-A `MOV Rd, Rm` (register-to-register) instruction.
///
/// Both `rd` and `rm` must be in `[0, 15]` (4-bit ARM register
/// selectors).
pub(crate) fn encode_mov_reg(rd: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    MOV_REG_BASE | ((rd as u32) << 12) | (rm as u32)
}

/// ARMv7-A `ADD Rd, Rn, Rm` (data-processing register) base encoding —
/// `0xE080_0000`.  OR in `(Rn << 16) | (Rd << 12) | Rm` to form the
/// full instruction word.
///
/// Bit layout (cond=AL, S=0, no shift):
///
/// ```text
/// 31..28  cond      = 0xE = 1110          (always — unconditional)
/// 27..25            = 000                  (data-processing register)
/// 24..21  opcode    = 0100                 (ADD)
/// 20      S         = 0                    (don't set flags)
/// 19..16  Rn        = (in this base, 0)   (first source register)
/// 15..12  Rd        = (in this base, 0)   (destination register)
/// 11.. 7  shift_imm = 00000                (no shift on Rm)
///  6.. 5  type      = 00                   (LSL — but shift_imm=0 means no shift)
///  4                = 0                    (immediate shift, not register)
///  3.. 0  Rm        = (in this base, 0)   (second source register)
/// ```
///
/// For `ADD r0, r0, r0`: `1110 0000 1000 0000 0000 0000 0000 0000` =
/// `0xE0800000`.  For `ADD Rd, Rn, Rm`: OR in
/// `(Rn << 16) | (Rd << 12) | Rm`.
///
/// Unlike the 8008's `ADD r` (which forces A = A + r — accumulator-
/// anchored), ARMv7's `ADD` is a 3-register operation: `Rd = Rn + Rm`.
/// No staging MOVs needed.
pub const ADD_REG_BASE: u32 = 0xE080_0000;

/// ARMv7-A `SUB Rd, Rn, Rm` (data-processing register) base encoding —
/// `0xE040_0000`.  Same shape as `ADD_REG_BASE` but with opcode
/// `0010` (SUB) instead of `0100` (ADD).  OR in
/// `(Rn << 16) | (Rd << 12) | Rm` for arbitrary operand triples.
///
/// `Rd = Rn - Rm`.  Same 3-register no-staging shape as ADD.
pub const SUB_REG_BASE: u32 = 0xE040_0000;

/// Encode an ARMv7-A `ADD Rd, Rn, Rm` instruction.
///
/// All three register selectors must be in `[0, 15]`.
pub(crate) fn encode_add_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    ADD_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// Encode an ARMv7-A `SUB Rd, Rn, Rm` instruction.
pub(crate) fn encode_sub_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    SUB_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// ARMv7-A `AND Rd, Rn, Rm` (bitwise AND) base — `0xE000_0000`.
///
/// Same shape as `ADD_REG_BASE` but with opcode `0000` (AND).  In
/// IIR terms this is the lowering target for the `and` op.
pub const AND_REG_BASE: u32 = 0xE000_0000;

/// ARMv7-A `ORR Rd, Rn, Rm` (bitwise OR — "OR Register") base —
/// `0xE180_0000`.
///
/// Same shape as `ADD_REG_BASE` but with opcode `1100` (ORR).  In
/// IIR terms this is the lowering target for the `or` op.  ARM
/// spells it "ORR" rather than "OR" to free up the `OR` mnemonic
/// for variants like `ORRS` (set flags) and `ORR.W` (Thumb-2 wide).
pub const ORR_REG_BASE: u32 = 0xE180_0000;

/// ARMv7-A `EOR Rd, Rn, Rm` (bitwise XOR — "Exclusive OR") base —
/// `0xE020_0000`.
///
/// Same shape as `ADD_REG_BASE` but with opcode `0001` (EOR).  In
/// IIR terms this is the lowering target for the `xor` op.  ARM's
/// "EOR" is an unusually old-school mnemonic — most modern ISAs
/// spell this `XOR`.  The bit pattern is the same either way.
pub const EOR_REG_BASE: u32 = 0xE020_0000;

/// Encode an ARMv7-A `AND Rd, Rn, Rm` instruction.
pub(crate) fn encode_and_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    AND_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// Encode an ARMv7-A `ORR Rd, Rn, Rm` instruction.
pub(crate) fn encode_orr_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    ORR_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// Encode an ARMv7-A `EOR Rd, Rn, Rm` instruction.
pub(crate) fn encode_eor_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    EOR_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// ARMv7-A `ADC Rd, Rn, Rm` (add with carry-in) base — `0xE0A0_0000`.
///
/// Same shape as `ADD_REG_BASE` but with opcode `0101` (ADC).  The
/// carry-in comes from the C flag set by a PRIOR flag-affecting ALU
/// op (`ADDS`, `SUBS`, `ADCS`, etc.).  This crate emits the non-S
/// (no-flag-update) form by default — front-ends that need the
/// carry chain must arrange for the producer to use the S-suffix
/// variant.  The S-form constants land alongside `cmp` in v0.4.3.
pub const ADC_REG_BASE: u32 = 0xE0A0_0000;

/// ARMv7-A `SBC Rd, Rn, Rm` (subtract with borrow-in) base —
/// `0xE0C0_0000`.
///
/// Same shape as `ADD_REG_BASE` but with opcode `0110` (SBC).  Like
/// ADC, the borrow-in (inverted carry) comes from a prior flag-
/// affecting op.  IIR maps `sbb` → `SBC` mirroring the
/// iir-to-intel8008 and iir-to-riscv naming conventions.
pub const SBC_REG_BASE: u32 = 0xE0C0_0000;

/// Encode an ARMv7-A `ADC Rd, Rn, Rm` instruction.
pub(crate) fn encode_adc_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    ADC_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// Encode an ARMv7-A `SBC Rd, Rn, Rm` instruction.
pub(crate) fn encode_sbc_reg(rd: u8, rn: u8, rm: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    SBC_REG_BASE | ((rn as u32) << 16) | ((rd as u32) << 12) | (rm as u32)
}

/// ARMv7-A `CMP Rn, Rm` (compare register) base — `0xE150_0000`.
///
/// Same shape as `SUB_REG_BASE` (opcode `1010` for CMP vs `0010` for
/// SUB) but with **S=1 forced** (bit 20) and **Rd discarded** (bits
/// 15..12 are zero — CMP has no register output, it only sets flags).
///
/// Bit layout (cond=AL, S=1, Rd=0000):
///
/// ```text
/// 31..28  cond      = 0xE = 1110         (always — unconditional)
/// 27..25            = 000                 (data-processing register)
/// 24..21  opcode    = 1010                (CMP)
/// 20      S         = 1                   (CMP IS the flag-setting variant)
/// 19..16  Rn        = (in this base, 0)  (first compare operand)
/// 15..12            = 0000                (Rd field — unused by CMP)
/// 11.. 7  shift_imm = 00000                (no shift)
///  6.. 5  type      = 00                   (LSL — no-op when shift=0)
///  4                = 0                    (immediate shift)
///  3.. 0  Rm        = (in this base, 0)  (second compare operand)
/// ```
///
/// For `CMP r0, r0`: `1110 0001 0101 0000 0000 0000 0000 0000` =
/// `0xE150_0000`.  For arbitrary `CMP Rn, Rm`: OR in
/// `(Rn << 16) | Rm`.
///
/// CMP semantically computes `Rn - Rm` and updates Z/C/N/V — the
/// difference itself is discarded.  IIR-level `cmp dest, a, b`
/// produces a boolean dest; the capture sequence after CMP uses
/// `MOVEQ` (MOV under the EQ condition prefix) to set the dest to 1
/// only when Z=1 (i.e. equal).  See `MOV_IMM_EQ_BASE` below.
pub const CMP_REG_BASE: u32 = 0xE150_0000;

/// ARMv7-A `MOVEQ Rd, #imm8` base — `0x03A0_0000`.
///
/// Identical encoding to `MOV_IMM_R0_BASE` (`0xE3A0_0000`) EXCEPT the
/// 4-bit `cond` field at bits 31..28 is `0000` (EQ — execute only if
/// Z flag is set) instead of `1110` (AL — always execute).
///
/// This is the canonical ARMv7 "flag-to-bool capture" idiom:
///
/// ```text
/// CMP   Rn, Rm                ; sets Z if Rn == Rm
/// MOV   dest, #0              ; default false (cond=AL)
/// MOVEQ dest, #1              ; if Z=1 (equal), overwrite to true
/// ```
///
/// Compare with the 8008's much more verbose flag-to-bool capture
/// (CMP + MVI dest, 0 + JFZ + 2 addr bytes + MVI dest, 1 = 8 bytes
/// with address-backpatching), or RV32I's typical SLT-based pattern.
/// ARMv7's `cond` field on every instruction makes this naturally
/// 4 words with no backpatching.
pub const MOV_IMM_EQ_BASE: u32 = 0x03A0_0000;

/// Encode an ARMv7-A `CMP Rn, Rm` instruction.
///
/// Note: CMP has no Rd — it only sets the Z/C/N/V flags.  The Rd
/// nibble (bits 15..12) is the architecturally-defined "should-be-
/// zero" field.
pub(crate) fn encode_cmp_reg(rn: u8, rm: u8) -> u32 {
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    debug_assert!(rm <= 15, "rm out of 4-bit range: {rm}");
    CMP_REG_BASE | ((rn as u32) << 16) | (rm as u32)
}

/// Encode an ARMv7-A `MOVEQ Rd, #imm8` instruction (MOV immediate
/// under the EQ condition prefix — "move only if Z flag is set").
// Part of the complete conditional-MOV encoder family (EQ/NE/CC/…); kept for
// ISA-encoder completeness and symmetry even though the current lowering only
// wires the NE/CC variants.
#[allow(dead_code)]
pub(crate) fn encode_mov_imm_eq(rd: u8, imm8: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    MOV_IMM_EQ_BASE | ((rd as u32) << 12) | (imm8 as u32)
}

/// ARMv7-A `MOVNE Rd, #imm8` base — `0x13A0_0000`.  Cond = NE
/// (`0001`) — execute only if Z flag is CLEAR.  Used by `cmp_ne`.
pub const MOV_IMM_NE_BASE: u32 = 0x13A0_0000;

/// ARMv7-A `MOVCC Rd, #imm8` base — `0x33A0_0000`.  Cond = CC
/// (`0011`) — execute only if Carry flag is CLEAR.  After CMP this
/// fires when `A < r` (unsigned compare).  Used by `cmp_lt`.
pub const MOV_IMM_CC_BASE: u32 = 0x33A0_0000;

/// ARMv7-A `MOVCS Rd, #imm8` base — `0x23A0_0000`.  Cond = CS
/// (`0010`) — execute only if Carry flag is SET.  After CMP this
/// fires when `A >= r` (unsigned compare).  Used by `cmp_gte`.
pub const MOV_IMM_CS_BASE: u32 = 0x23A0_0000;

/// ARMv7-A `MOVHI Rd, #imm8` base — `0x83A0_0000`.  Cond = HI
/// (`1000`) — execute only if Carry SET AND Zero CLEAR.  After CMP
/// this fires when `A > r` (unsigned compare).  Used by `cmp_gt`.
///
/// Note: ARMv7's HI condition does the right thing for unsigned
/// "greater than" without the operand-swap trick the 8008 and RV32I
/// backends had to use.
pub const MOV_IMM_HI_BASE: u32 = 0x83A0_0000;

/// ARMv7-A `MOVLS Rd, #imm8` base — `0x93A0_0000`.  Cond = LS
/// (`1001`) — execute only if Carry CLEAR OR Zero SET.  After CMP
/// this fires when `A <= r` (unsigned compare).  Used by `cmp_lte`.
pub const MOV_IMM_LS_BASE: u32 = 0x93A0_0000;

/// Encode an ARMv7-A `MOV<cond> Rd, #imm8` instruction with an
/// arbitrary condition prefix.
///
/// `cond_base` must be one of `MOV_IMM_{EQ,NE,CC,CS,HI,LS}_BASE`
/// (or a future addition).  The function ORs in the destination
/// register (bits 15..12) and the 8-bit immediate (bits 7..0).
///
/// This generic helper avoids cluttering the public surface with
/// six nearly-identical `encode_mov_imm_{eq,ne,cc,cs,hi,ls}` fns.
/// `encode_mov_imm_eq` is kept as a named convenience for v0.4.3's
/// equality lowering which existed before this generalisation.
pub(crate) fn encode_mov_imm_cond(cond_base: u32, rd: u8, imm8: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    cond_base | ((rd as u32) << 12) | (imm8 as u32)
}

/// ARMv7-A `BL addr` (branch with link) base — `0xEB00_0000`.
///
/// Same shape as `B_BASE` (`0xEA00_0000`) but with bit 24 SET.  The
/// silicon writes `PC + 4` (the return address) into LR (`r14`)
/// before branching, so a subsequent `BX LR` in the callee returns
/// to the next instruction in the caller.
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond  = 0xE = 1110            (always — unconditional)
/// 27..25        = 101                    (branch family)
/// 24    = 1                              (BL; B = 0)
/// 23.. 0  imm24 = (signed PC-relative offset in WORDS)
/// ```
///
/// CAREFUL: `BL` is `0xEB00_0000`, NOT `0xEA00_0000`.  The bit-24
/// difference distinguishes "branch with link" (function call) from
/// "branch" (goto).  Same family of nibble-off-by-one hazard as the
/// 8008's `JMP ↔ JFC` and `CAL ↔ CFZ` confusions.
pub const BL_BASE: u32 = 0xEB00_0000;

/// ARMv7-A `B addr` (unconditional branch) base — `0xEA00_0000`.
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond     = 0xE = 1110            (always — unconditional)
/// 27..25           = 101                    (branch family)
/// 24               = 0                      (B; 1 = BL with link)
/// 23.. 0  imm24    = (signed PC-relative offset in WORDS)
/// ```
///
/// At execute time the PC reads `instruction_address + 8` (the
/// classic ARM 2-stage pipeline prefetch offset).  So for a branch
/// at byte offset `A` targeting byte offset `T`:
///
/// ```text
/// imm24 = (T - A - 8) / 4    ; sign-extended 24 bits
/// ```
///
/// OR this base with `imm24 & 0x00FF_FFFF` to form the full word.
pub const B_BASE: u32 = 0xEA00_0000;

/// ARMv7-A `BNE addr` (branch if Z flag CLEAR) base — `0x1A00_0000`.
///
/// Same shape as `B_BASE` but with cond = NE = `0001`.  After
/// CMP/TST that sets the Z flag, BNE branches when the comparison
/// was non-equal / non-zero — which `jmp_if_true cond_var, label`
/// lowers to after CMP cond_var, #0.
pub const B_NE_BASE: u32 = 0x1A00_0000;

/// ARMv7-A `BEQ addr` (branch if Z flag SET) base — `0x0A00_0000`.
///
/// Same shape as `B_BASE` but with cond = EQ = `0000`.  Pairs with
/// `jmp_if_false cond_var, label` after CMP cond_var, #0.
pub const B_EQ_BASE: u32 = 0x0A00_0000;

/// ARMv7-A `CMP Rn, #0` base — `0xE350_0000`.
///
/// Data-processing-immediate form of CMP.  Compares `Rn` against the
/// 8-bit immediate `0` and sets Z if equal (i.e. Rn == 0).  The
/// canonical "test whether a boolean register is zero" idiom.
///
/// Bit layout (cond=AL, S=1, Rn=0, imm=0):
///
/// ```text
/// 31..28  cond     = 0xE = 1110            (always — unconditional)
/// 27..25           = 001                    (data-processing immediate)
/// 24..21  opcode   = 1010                   (CMP)
/// 20      S        = 1                      (CMP always sets flags)
/// 19..16  Rn       = (in this base, 0)     (the register to compare)
/// 15..12           = 0000                   (Rd unused for CMP)
/// 11.. 8  rotate   = 0000
///  7.. 0  imm8     = 0                      (we compare against 0)
/// ```
///
/// For `CMP Rn, #0`: OR in `(Rn << 16)`.
pub const CMP_IMM_ZERO_BASE: u32 = 0xE350_0000;

/// Encode an ARMv7-A `CMP Rn, #0` instruction.
pub(crate) fn encode_cmp_imm_zero(rn: u8) -> u32 {
    debug_assert!(rn <= 15, "rn out of 4-bit range: {rn}");
    CMP_IMM_ZERO_BASE | ((rn as u32) << 16)
}

/// Encode an ARMv7-A branch instruction (`B`/`BEQ`/`BNE`/...).
///
/// `cond_base` is one of `B_BASE`/`B_EQ_BASE`/`B_NE_BASE`/...  The
/// `imm24` argument is the signed 24-bit word offset; the encoder
/// masks it to 24 bits and ORs it with the base.  Range check is
/// the caller's responsibility — `lower_iir_to_armv7` returns
/// `BranchOutOfRange` if the offset doesn't fit.
pub(crate) fn encode_branch(cond_base: u32, imm24: i32) -> u32 {
    cond_base | ((imm24 as u32) & 0x00FF_FFFF)
}

// ===========================================================================
// IIRArmv7Config
// ===========================================================================

/// Configuration for the IIR → ARMv7 lowering pass.
///
/// Currently only the module name is configurable, reserved for future
/// symbol-table / ELF-section emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRArmv7Config {
    /// Module name — reserved for future symbol-table / `.bin` header use.
    pub module_name: String,
}

impl IIRArmv7Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRArmv7Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRArmv7Error
// ===========================================================================

/// Errors that can occur during IIR → ARMv7 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRArmv7Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0
    /// doesn't lower any instructions, so a non-empty function body
    /// would surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any ARMv7 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
    /// A variable name was used (via `mov` or `ret`) before it was
    /// bound by `const` or `mov`.
    UndefinedVariable { function: String, name: String },
    /// The function tried to bind more locals than the 13 general-
    /// purpose ARMv7 registers (r0..r12) can hold.  Stack spilling
    /// lands in a future increment (A3++.5 or later).  r13 (sp),
    /// r14 (lr), and r15 (pc) are not part of the pool.
    OutOfRegisters { function: String, name: String },
    /// A `jmp`/`jmp_if_*` referenced a label name not defined in
    /// the same function.
    UndefinedLabel { function: String, label: String },
    /// A computed branch offset doesn't fit in ARM's signed 24-bit
    /// immediate (range ±32 MiB, more than enough for any practical
    /// function).
    BranchOutOfRange { function: String, target: usize, current: usize },
    /// A `call` referenced a function name not defined anywhere in
    /// the module.  Cross-module calls aren't yet supported.
    UndefinedFunction { caller: String, callee: String },
}

impl fmt::Display for IIRArmv7Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationFailed(errs) => {
                write!(f, "validation failed:\n  {}", errs.join("\n  "))
            }
            Self::UnsupportedOp { function, op } => {
                write!(f, "unsupported op in function {function:?}: {op}")
            }
            Self::UnsupportedType { function, type_hint } => {
                write!(f, "unsupported type in function {function:?}: {type_hint}")
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "invalid operand in function {function:?}: {detail}")
            }
            Self::UndefinedVariable { function, name } => {
                write!(f, "undefined variable {name:?} in function {function:?}")
            }
            Self::OutOfRegisters { function, name } => {
                write!(f, "out of ARMv7 registers (r0..r12) while binding {name:?} in function {function:?}; stack spilling not yet supported")
            }
            Self::UndefinedLabel { function, label } => {
                write!(f, "undefined label {label:?} referenced by jmp/jmp_if in function {function:?}")
            }
            Self::BranchOutOfRange { function, target, current } => {
                write!(f, "branch in function {function:?} from word offset {current} to word offset {target} doesn't fit in ARM's signed 24-bit imm")
            }
            Self::UndefinedFunction { caller, callee } => {
                write!(f, "undefined function {callee:?} called from {caller:?}")
            }
        }
    }
}

impl std::error::Error for IIRArmv7Error {}

// ===========================================================================
// validate_for_armv7
// ===========================================================================

/// Pre-flight validation for IIR → ARMv7 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A3).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv,intel8008}` so callers
/// can switch backends without changing their pre-flight logic.
pub fn validate_for_armv7(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_armv7
// ===========================================================================

/// Register `r0` — the AAPCS first-argument / return-value register.
/// `ret <var>` stages the value into `r0` (via `MOV r0, var_reg` if
/// the var lives elsewhere) before `BX LR`.
const REG_R0: u8 = 0;

/// Linear-allocator pool ordered to keep the trivial `const v; ret v`
/// case at one MVI byte: r0 is handed out first, so `ret v` finds the
/// value already in r0 and skips the redundant `MOV r0, X` round-trip.
///
/// `r13` (`sp`), `r14` (`lr`), and `r15` (`pc`) are NOT in the pool —
/// touching them as locals would break the calling convention's
/// stack discipline, the return address, or the instruction pointer
/// (respectively).
const REGISTER_POOL: [u8; 13] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/// Supported instruction opcodes in v0.3.0 (A3++).
///
/// * `const dest, Int(n)` lowers to `MOV rrr, #n` with `rrr` allocated
///   from `REGISTER_POOL`.
/// * `mov dest, src` lowers to `MOV Rd, Rm` (no-op when Rd == Rm).
/// * `ret <var>` stages the value into `r0` (if not already there) and
///   emits `BX LR`.  `ret_void` just emits `BX LR`.
const SUPPORTED_OPS: &[&str] = &[
    // A3 / A3+ / A3++
    "const", "mov", "ret", "ret_void",
    // A3++.5 — data-processing-register ALU
    "add", "sub",
    // A3++.5.5 first slice — bitwise data-processing-register ALU
    // (and = AND opcode 0000, or = ORR opcode 1100, xor = EOR opcode 0001)
    "and", "or", "xor",
    // A3++.5.5 second slice — carry-chained DP-register ALU
    // (adc = ADC opcode 0101, sbb = SBC opcode 0110)
    "adc", "sbb",
    // A3++.5.5 third slice — equality comparison with flag-to-bool
    // capture via the EQ condition prefix on every A32 instruction.
    "cmp",
    // A3++.5.5 fourth slice — remaining 5 comparison ops using
    // distinct condition prefixes on the trailing MOV.
    "cmp_ne", "cmp_lt", "cmp_gt", "cmp_gte", "cmp_lte",
    // A3++.5.5 fifth slice — labels + branches.  Two-pass per-function
    // backpatching for PC-relative offsets.
    "label", "jmp", "jmp_if_true", "jmp_if_false",
    // A3++.6 — function calls via BL with module-level backpatching.
    // `ret` already emits BX LR from v0.3.0 — that handles both
    // function returns (via the LR saved by BL) and module-entry
    // returns to the OS (BX LR on a fresh entry returns to whatever
    // address LR was passed in with).
    "call",
];

/// Lower an [`IIRModule`] to a `Vec<u32>` of ARMv7 (A32) opcode words.
///
/// **v0.2.0 scope** (A3+ — first real lowering):
///
/// | IIR op | A32 lowering |
/// |--------|--------------|
/// | `const dest, Int(n)` (8-bit imm) | `mov r0, #n` (`0xE3A0_00NN`) |
/// | `ret <var>` (int) | `bx lr` (`0xE12FFF1E`) — `var` is already in `r0` |
/// | `ret_void` | `bx lr` |
///
/// ### Accumulator-only first slice
///
/// Every `const` allocates to `r0` — the AAPCS return-value register.
/// A real linear allocator over `r0..r12` (and the v0.3.x ARM
/// equivalent of v0.3.0's RISC-V move) arrives in A3++.
///
/// ### Empty-module contract
///
/// Preserves v0.1.0's behaviour for the trivial "`fn main() {}`" case:
/// any module with no functions emits a single `BKPT #0xFFFF` so the
/// in-tree `arm-simulator` halts deterministically.  Once at least
/// one function is lowered, the BKPT is replaced by the function's
/// real instruction stream.
#[deprecated(
    since = "0.5.0",
    note = "use `armv7_backend::compile` over CIR — see code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md"
)]
pub fn lower_iir_to_armv7(
    module: &IIRModule,
    _cfg: &IIRArmv7Config,
) -> Result<Vec<u32>, IIRArmv7Error> {
    let errors = validate_for_armv7(module);
    if !errors.is_empty() {
        return Err(IIRArmv7Error::ValidationFailed(errors));
    }

    // Trivial empty-module contract — preserves v0.1.0 callable behaviour
    // for the canonical "fn main() {}" minimal case.
    if module.functions.is_empty() {
        return Ok(vec![BKPT]);
    }

    // ── Module-level call-site resolution state (v0.4.6) ────────────────
    //
    // Each function's start word index is recorded as we walk
    // `module.functions` in source order.  When a `call <fn_name>` is
    // emitted, we record (slot, fn_name, caller) into `pending_calls`;
    // after every function has been emitted, a final pass backpatches
    // each pending BL with the PC-relative offset of its target.
    //
    // Mirrors the 8008's v0.3.9 module-level call resolution; ARMv7's
    // BL just uses a 24-bit signed word offset like B/Bcond, instead
    // of the 8008's 14-bit absolute address.
    let mut function_addrs: HashMap<String, usize> = HashMap::new();
    let mut pending_calls: Vec<(usize, String, String /* caller */)> = Vec::new();

    let mut words = Vec::new();
    for f in &module.functions {
        // Record this function's start word index before emitting its body.
        function_addrs.insert(f.name.clone(), words.len());
        // ── Per-function allocator state ──────────────────────────────
        //
        // IIR var name → its assigned 4-bit register index.  Sequentially
        // hands out registers from REGISTER_POOL starting with r0 — this
        // keeps the trivial `const v; ret v` case at the same 2-word
        // shape as v0.2.0 (no redundant `MOV r0, X` round-trip).
        let mut env: HashMap<String, u8> = HashMap::new();
        let mut next_reg: usize = 0;

        // ── Per-function label-resolution state (v0.4.5) ──────────────
        //
        // ARMv7's branch instructions carry a 24-bit signed PC-relative
        // offset (in words; the silicon shifts left 2 to convert to
        // bytes).  Pass 1 emits each branch with a placeholder zero
        // offset and records `(word_index_of_branch, target_label,
        // cond_base)` in `pending_branches`.  After all instructions in
        // the function are emitted, pass 2 looks up each pending
        // branch's label in `labels`, computes the PC-relative offset
        // (accounting for ARM's +8 PC prefetch quirk), range-checks it
        // against signed 24-bit, and ORs it into the placeholder word.
        //
        // Labels are keyed by name → word index (not byte index — every
        // A32 instruction is a fixed 4 bytes, and the branch offset is
        // already in word units).
        let mut labels: HashMap<String, usize> = HashMap::new();
        let mut pending_branches: Vec<(usize, String, u32)> = Vec::new();

        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                return Err(IIRArmv7Error::UnsupportedOp {
                    function: f.name.clone(),
                    op: instr.op.clone(),
                });
            }
            match instr.op.as_str() {
                // ── const dest, Int(n) → MOV rrr, #n ────────────────────
                "const" => {
                    let dest = require_dest(instr, "const", &f.name)?;
                    let imm8 = encode_immediate_byte(instr.srcs.first(), &f.name)?;
                    let rrr = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    words.push(encode_mov_imm(rrr, imm8));
                }

                // ── mov dest, src → MOV Rd, Rm ──────────────────────────
                //
                // If the source and dest happen to be the same register
                // (unlikely under SSA but possible if upstream re-binds a
                // name), we emit no word — the move is a no-op.
                "mov" => {
                    let dest = require_dest(instr, "mov", &f.name)?;
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "mov srcs[0] must be Var".into(),
                        }),
                    };
                    let rm = lookup_register(&env, &src_name, &f.name)?;
                    let rd = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    if rd != rm {
                        words.push(encode_mov_reg(rd, rm));
                    }
                }

                // ── ret <var>: stage value in r0, then BX LR ────────────
                //
                // If `var`'s register is already r0, the MOV is omitted.
                // Per AAPCS, the return value lives in r0; we
                // unconditionally branch to lr after staging.
                "ret" => {
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "ret srcs[0] must be Var".into(),
                        }),
                    };
                    let rm = lookup_register(&env, &src_name, &f.name)?;
                    if rm != REG_R0 {
                        words.push(encode_mov_reg(REG_R0, rm));
                    }
                    words.push(BX_LR);
                }

                // ── ret_void → BX LR ───────────────────────────────
                "ret_void" => {
                    words.push(BX_LR);
                }

                // ── cmp / cmp_ne / cmp_lt / cmp_gt / cmp_gte / cmp_lte ─
                //
                // All six boolean comparisons share the same shape:
                //
                //   CMP    rn, rm           ; sets Z/C/N/V flags
                //   MOV    dest, #0         ; default false (cond=AL)
                //   MOV<C> dest, #1         ; flip to true under cond <C>
                //
                // The (operation → condition prefix) mapping:
                //
                //   cmp     → EQ (0x03A0..)  : Z=1 (equal)
                //   cmp_ne  → NE (0x13A0..)  : Z=0
                //   cmp_lt  → CC (0x33A0..)  : C=0 (unsigned A < r)
                //   cmp_gt  → HI (0x83A0..)  : C=1 & Z=0
                //   cmp_gte → CS (0x23A0..)  : C=1
                //   cmp_lte → LS (0x93A0..)  : C=0 | Z=1
                //
                // The four-bit `cond` field at bits 31..28 of every
                // A32 instruction is the only thing that varies
                // between the six lowerings — same CMP+MOV+MOV
                // skeleton, the trailing MOV's top nibble selects
                // the comparison.
                //
                // ARMv7's HI condition does the right thing for
                // unsigned "greater than" natively — no operand-swap
                // trick like the 8008 and RV32I backends needed.
                "cmp" | "cmp_ne" | "cmp_lt" | "cmp_gt" | "cmp_gte" | "cmp_lte" => {
                    let dest = require_dest(instr, &instr.op, &f.name)?;
                    let a_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[0] must be Var", instr.op),
                        }),
                    };
                    let b_name = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[1] must be Var", instr.op),
                        }),
                    };
                    let rn = lookup_register(&env, &a_name, &f.name)?;
                    let rm = lookup_register(&env, &b_name, &f.name)?;
                    let rd = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    let cond_mov_base = match instr.op.as_str() {
                        "cmp"     => MOV_IMM_EQ_BASE,
                        "cmp_ne"  => MOV_IMM_NE_BASE,
                        "cmp_lt"  => MOV_IMM_CC_BASE,
                        "cmp_gt"  => MOV_IMM_HI_BASE,
                        "cmp_gte" => MOV_IMM_CS_BASE,
                        "cmp_lte" => MOV_IMM_LS_BASE,
                        _ => unreachable!("outer arm restricts to these 6"),
                    };
                    words.push(encode_cmp_reg(rn, rm));
                    words.push(encode_mov_imm(rd, 0));
                    words.push(encode_mov_imm_cond(cond_mov_base, rd, 1));
                }

                // ── add/sub/and/or/xor/adc/sbb → DP-register family ─────
                //
                // All seven data-processing-register ALU ops share an
                // identical shape, differing only in the 4-bit
                // `opcode` field (bits 24..21):
                //
                //   ADD = 0100   AND = 0000
                //   SUB = 0010   ORR = 1100
                //   ADC = 0101   EOR = 0001
                //   SBC = 0110
                //
                // Unlike the 8008's accumulator-anchored ALU (which
                // needs MOV wrappers for non-accumulator operands),
                // ARMv7's DP-register family takes 3 register
                // selectors in a single instruction: `Rd = Rn op Rm`.
                // No staging MOVs.  Same shape as RISC-V's `add rd,
                // rs1, rs2`.
                //
                // ADC/SBC consume the C flag set by a PRIOR flag-
                // affecting ALU op.  This crate emits the non-S
                // (no-flag-update) form by default — front-ends
                // arrange for the producer to use the S-suffix
                // variant so the carry survives.
                "add" | "sub" | "and" | "or" | "xor" | "adc" | "sbb" => {
                    let dest = require_dest(instr, &instr.op, &f.name)?;
                    let a_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[0] must be Var", instr.op),
                        }),
                    };
                    let b_name = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[1] must be Var", instr.op),
                        }),
                    };
                    let rn = lookup_register(&env, &a_name, &f.name)?;
                    let rm = lookup_register(&env, &b_name, &f.name)?;
                    let rd = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    let word = match instr.op.as_str() {
                        "add" => encode_add_reg(rd, rn, rm),
                        "sub" => encode_sub_reg(rd, rn, rm),
                        "and" => encode_and_reg(rd, rn, rm),
                        "or"  => encode_orr_reg(rd, rn, rm),
                        "xor" => encode_eor_reg(rd, rn, rm),
                        "adc" => encode_adc_reg(rd, rn, rm),
                        "sbb" => encode_sbc_reg(rd, rn, rm),
                        _ => unreachable!("outer arm restricts to these 7"),
                    };
                    words.push(word);
                }

                // ── label "<name>": record current word index ──────────
                "label" => {
                    let name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "label requires srcs[0] = Operand::Var(name)".into(),
                        }),
                    };
                    labels.insert(name, words.len());
                }

                // ── jmp "<name>": B with cond=AL ───────────────────────
                "jmp" => {
                    let target = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "jmp requires srcs[0] = Operand::Var(target_label)".into(),
                        }),
                    };
                    pending_branches.push((words.len(), target, B_BASE));
                    words.push(0); // placeholder — pass 2 overwrites
                }

                // ── call dest, "<fn_name>" → BL + capture r0 into dest ─
                //
                // Operand layout: srcs = [Var(fn_name)], optional dest.
                //
                // Pass 1: emit `BL 0` (placeholder offset) and record
                // (slot, fn_name, caller) into the module-level
                // `pending_calls` for the final backpatching pass.
                //
                // ARMv7's BL has bit 24 set vs B's bit 24 clear — the
                // same `0xEA00_0000 vs 0xEB00_0000` confusion to avoid
                // as the 8008's `JMP 0x7C vs CAL 0x7E` family-bit
                // hazard.
                //
                // Argument passing isn't yet supported — calls in
                // v0.4.6 are zero-arg.  Mirrors the 8008's v0.3.9
                // staging where args came in a future slice.  For
                // ARMv7 the AAPCS argument-register convention
                // (r0..r3) would dictate the lowering shape.
                "call" => {
                    let fn_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "call requires srcs[0] = Operand::Var(fn_name)".into(),
                        }),
                    };
                    pending_calls.push((words.len(), fn_name, f.name.clone()));
                    words.push(0); // placeholder — pass 2 overwrites
                    // If the IIR site binds a dest, capture the return
                    // value from r0 into dest_reg.  A bare `call`
                    // without a dest discards the return value (void
                    // call).
                    if let Some(dest) = instr.dest.as_deref() {
                        let dest_reg = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                        if dest_reg != REG_R0 {
                            words.push(encode_mov_reg(dest_reg, REG_R0));
                        }
                    }
                }

                // ── jmp_if_true / jmp_if_false ─────────────────────────
                //
                // ARMv7 has no "branch on register"; we provoke the Z
                // flag via `CMP cond_reg, #0` and use BNE / BEQ:
                //
                //   CMP   cond_reg, #0    ; sets Z if cond == 0
                //   BNE   target          ; jmp_if_true  — branch if cond != 0
                //   BEQ   target          ; jmp_if_false — branch if cond == 0
                //
                // The CMP-imm-zero idiom is one word; the conditional
                // branch is another.  2 words total — neat parallel to
                // the 8008's MOV A, r; ANA A; JFZ/JTZ sequence.
                "jmp_if_true" | "jmp_if_false" => {
                    let cond_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} requires srcs[0] = Operand::Var(cond)", instr.op),
                        }),
                    };
                    let target = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} requires srcs[1] = Operand::Var(target_label)", instr.op),
                        }),
                    };
                    let cond_reg = lookup_register(&env, &cond_name, &f.name)?;
                    words.push(encode_cmp_imm_zero(cond_reg));
                    let branch_base = if instr.op == "jmp_if_true" {
                        B_NE_BASE
                    } else {
                        B_EQ_BASE
                    };
                    pending_branches.push((words.len(), target, branch_base));
                    words.push(0); // placeholder
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }

        // ── Pass 2: backpatch pending branches ─────────────────────────
        //
        // For each pending entry (slot, target_label, cond_base):
        //   1. Resolve target_label → target_word_index via `labels`.
        //   2. Compute the signed 24-bit word offset:
        //        imm24 = target - slot - 2
        //      (the `- 2` accounts for ARM's PC = current_instruction + 8
        //      = current_instruction + 2 words; the branch's "current"
        //      address is `slot * 4` bytes, and the silicon adds 8.)
        //   3. Range-check imm24 against signed 24-bit (±2^23 words).
        //   4. OR the encoded imm24 into the placeholder word.
        for (slot, label, cond_base) in &pending_branches {
            let target = *labels.get(label).ok_or_else(|| {
                IIRArmv7Error::UndefinedLabel {
                    function: f.name.clone(),
                    label: label.clone(),
                }
            })?;
            let imm24: i32 = (target as i32) - (*slot as i32) - 2;
            if !(-(1 << 23)..(1 << 23)).contains(&imm24) {
                return Err(IIRArmv7Error::BranchOutOfRange {
                    function: f.name.clone(),
                    target,
                    current: *slot,
                });
            }
            words[*slot] = encode_branch(*cond_base, imm24);
        }
    }

    // ── Module-level call-backpatching pass (v0.4.6) ──────────────
    //
    // All functions have been emitted, so `function_addrs` has every
    // valid call target.  Walk `pending_calls` and write the
    // BL-encoded word at each placeholder slot.
    //
    // BL uses the same PC-relative offset shape as B (with the +8
    // prefetch quirk): imm24 = target_word - slot - 2.
    for (slot, callee, caller) in &pending_calls {
        let target = *function_addrs.get(callee).ok_or_else(|| {
            IIRArmv7Error::UndefinedFunction {
                caller: caller.clone(),
                callee: callee.clone(),
            }
        })?;
        let imm24: i32 = (target as i32) - (*slot as i32) - 2;
        if !(-(1 << 23)..(1 << 23)).contains(&imm24) {
            return Err(IIRArmv7Error::BranchOutOfRange {
                function: caller.clone(),
                target,
                current: *slot,
            });
        }
        words[*slot] = encode_branch(BL_BASE, imm24);
    }

    // Defensive — if a function had no instructions at all, fall back
    // to BKPT so the output is still a valid halting program.
    if words.is_empty() {
        words.push(BKPT);
    }

    Ok(words)
}

// ---------------------------------------------------------------------------
// Per-instruction helpers
// ---------------------------------------------------------------------------

fn require_dest<'a>(
    instr: &'a interpreter_ir::IIRInstr,
    op: &str,
    fn_name: &str,
) -> Result<&'a str, IIRArmv7Error> {
    instr.dest.as_deref().ok_or_else(|| IIRArmv7Error::InvalidOperand {
        function: fn_name.to_string(),
        detail: format!("{op} requires a dest"),
    })
}

fn alloc_register(
    next_reg: &mut usize,
    dest: &str,
    env: &mut HashMap<String, u8>,
    fn_name: &str,
) -> Result<u8, IIRArmv7Error> {
    if *next_reg >= REGISTER_POOL.len() {
        return Err(IIRArmv7Error::OutOfRegisters {
            function: fn_name.to_string(),
            name: dest.to_string(),
        });
    }
    let rrr = REGISTER_POOL[*next_reg];
    *next_reg += 1;
    env.insert(dest.to_string(), rrr);
    Ok(rrr)
}

fn lookup_register(
    env: &HashMap<String, u8>,
    name: &str,
    fn_name: &str,
) -> Result<u8, IIRArmv7Error> {
    env.get(name).copied().ok_or_else(|| IIRArmv7Error::UndefinedVariable {
        function: fn_name.to_string(),
        name: name.to_string(),
    })
}

fn encode_immediate_byte(
    op: Option<&Operand>,
    fn_name: &str,
) -> Result<u8, IIRArmv7Error> {
    let n = match op {
        Some(Operand::Int(n)) => *n,
        Some(Operand::Bool(b)) => if *b { 1 } else { 0 },
        _ => return Err(IIRArmv7Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: "const srcs[0] must be Int or Bool".into(),
        }),
    };
    if (0..=255).contains(&n) {
        Ok(n as u8)
    } else if (-128..0).contains(&n) {
        Ok((n as i8) as u8)
    } else {
        Err(IIRArmv7Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "const {n} exceeds 8-bit byte range ([-128, 255]); A32's \
                 12-bit MOV immediate field supports rotated 8-bit values \
                 — wider raw immediates need a `movw`/`movt` pair, which \
                 lands in A3++"
            ),
        })
    }
}
