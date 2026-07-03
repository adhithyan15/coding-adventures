//! # `riscv-encoder` — pure RV32I instruction encoder.
//!
//! Mirror of [`ge225-encoder`] / [`intel4004-encoder`] /
//! [`armv7-encoder`] / [`intel8008-encoder`] for the RISC-V RV32I
//! ISA.  Phase 7 (the FINAL lane) of the historical-arch backend
//! migration.
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — the canonical `encode_*` helpers
//!    (e.g. `encode_addi`, `encode_jalr`) come from
//!    [`riscv_simulator::encoding`].  We re-export them here so
//!    that `riscv-backend` (Backend trait over CIR) can depend on a
//!    small, IR-agnostic surface without pulling the full
//!    simulator into every consumer.  Future RV32I-spec updates
//!    land in `riscv-simulator::encoding` and propagate
//!    automatically.
//! 2. **Register-index constants** — the small subset of
//!    architectural registers `riscv-backend` actually touches:
//!    `X0_ZERO`, `X1_RA`, `A0`, plus the temporary-register pool
//!    [`TEMP_REGISTERS`].  These mirror the RISC-V calling
//!    convention (psABI).
//! 3. **Canonical word constants** — pre-computed instruction
//!    words for the sequences the e2e tests pin:
//!    [`RET_WORD`] for `jalr x0, x1, 0` (the universal RV32I
//!    return-from-function).
//!
//! No IR knowledge lives here.  Consumers map their IR onto
//! encoder calls + the register table.
//!
//! ## ISA quick reference (subset used by the backend)
//!
//! | Mnemonic | Encoding | Effect |
//! |----------|----------|--------|
//! | `addi rd, rs1, imm` (I-type) | `imm[11:0] | rs1 | 000 | rd | 0010011` | `rd ← rs1 + sign_extend(imm)` |
//! | `add  rd, rs1, rs2` (R-type) | `0000000 | rs2 | rs1 | 000 | rd | 0110011` | `rd ← rs1 + rs2` |
//! | `jalr rd, rs1, imm` (I-type) | `imm[11:0] | rs1 | 000 | rd | 1100111` | `pc ← (rs1 + imm) & ~1; rd ← pc + 4` |
//!
//! `jalr x0, x1, 0` (i.e. `0x0000_8067`) is the canonical "return"
//! used at every function epilogue.
//!
//! ## Quick start
//!
//! ```
//! use riscv_encoder::{encode_addi, encode_jalr, RET_WORD, X0_ZERO, X1_RA, A0};
//!
//! // const_i64 v=42 lowered to `addi t0, x0, 42` — the bytes
//! // `riscv-backend` emits for the canonical Twig `42` program.
//! let const_word = encode_addi(/*rd=*/5, /*rs1=*/X0_ZERO, /*imm=*/42);
//! assert_eq!(const_word, 0x02A0_0293);
//!
//! // ret  lowered to `jalr x0, x1, 0`.
//! let ret_word = encode_jalr(X0_ZERO, X1_RA, 0);
//! assert_eq!(ret_word, RET_WORD);
//! assert_eq!(RET_WORD, 0x0000_8067);
//!
//! // a0 (x10) is the i32/i64 return-value register per psABI.
//! assert_eq!(A0, 10);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `riscv-simulator::encoding` is the in-tree source of truth for the
// RV32I bit layout — it's the only place the funct3/funct7 constants
// and the imm-bit-fiddling lives.  We re-export the subset of
// `encode_*` helpers that `riscv-backend` actually uses.

pub use riscv_simulator::encoding::{
    assemble, encode_add, encode_addi, encode_and, encode_andi, encode_auipc,
    encode_beq, encode_bge, encode_bgeu, encode_blt, encode_bltu, encode_bne,
    encode_ecall, encode_jal, encode_jalr, encode_lb, encode_lbu, encode_lh,
    encode_lhu, encode_lui, encode_lw, encode_or, encode_ori, encode_sb,
    encode_sh, encode_sll, encode_slli, encode_slt, encode_slti, encode_sltiu,
    encode_sltu, encode_sra, encode_srai, encode_srl, encode_srli, encode_sub,
    encode_sw, encode_xor, encode_xori,
};

// ===========================================================================
// Register layout (the small subset riscv-backend touches by index)
// ===========================================================================
//
// RV32I has 32 integer registers: x0..x31.  The psABI assigns
// canonical roles to several of them.  `riscv-backend` only needs to
// name a few directly — the rest come from the temporary pool below.

/// `x0` — hardwired to zero.  Writes are silently discarded; reads
/// always yield zero.  Used as `rs1` for `addi rd, x0, n` to
/// materialise immediates.
pub const X0_ZERO: u32 = 0;

/// `x1` — return address (`ra`).  Per psABI, `jalr` rd writes to
/// `ra` on a call and `jalr x0, x1, 0` pops it as the canonical
/// return.
pub const X1_RA: u32 = 1;

/// `x2` — stack pointer (`sp`).  16-byte-aligned per psABI.
/// Reserved here for future call-prologue support.
pub const X2_SP: u32 = 2;

/// `a0` = `x10` — first argument / primary return-value register
/// per psABI.  After a `ret`, this holds the integer return value
/// the caller reads.
pub const A0: u32 = 10;

// ---------------------------------------------------------------------------
// Temporary registers — `t0..t6` per psABI
// ---------------------------------------------------------------------------
//
// `riscv-backend` uses a simple linear allocator that hands out
// temps from this pool, one per `dest`, in declaration order.
// When the pool is exhausted, the backend reports
// `OutOfRegisters` — stack-spilling lands in a future increment.

/// `t0..t6` = `[x5, x6, x7, x28, x29, x30, x31]` — caller-saved
/// general-purpose registers the linear allocator hands out.
///
/// Note the non-contiguous indices: `t0..t2` are `x5..x7` (the
/// original RV32I temps) and `t3..t6` are `x28..x31` (added in the
/// RV32E spec — fine on RV32I too).  The ordering matches what
/// `iir-to-riscv` v0.3.3 used so byte-for-byte output is preserved.
pub const TEMP_REGISTERS: [u32; 7] = [5, 6, 7, 28, 29, 30, 31];

// ===========================================================================
// Canonical instruction-word constants
// ===========================================================================
//
// Pre-computed words for the sequences the e2e smoke tests pin.

/// `jalr x0, x1, 0` — the canonical RV32I "return from function".
/// Encoded value: `0x0000_8067`.  Stored little-endian on disk as
/// `[0x67, 0x80, 0x00, 0x00]`.
pub const RET_WORD: u32 = 0x0000_8067;
