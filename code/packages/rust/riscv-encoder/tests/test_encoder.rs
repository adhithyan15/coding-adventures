//! Byte-pinning tests for `riscv-encoder`.
//!
//! Every constant the LANG VM e2e tests rely on is asserted here so
//! that any silent drift in `riscv-simulator::encoding` (the upstream
//! source we re-export) trips a unit-level failure before reaching
//! the cross-crate e2e pipeline.

use riscv_encoder::*;

#[test]
fn register_constants_pinned() {
    assert_eq!(X0_ZERO, 0);
    assert_eq!(X1_RA, 1);
    assert_eq!(X2_SP, 2);
    assert_eq!(A0, 10);
}

#[test]
fn temp_register_pool_matches_iir_to_riscv_legacy() {
    // Must match `iir-to-riscv` v0.3.3's TEMP_REGISTERS exactly
    // — that's how byte-for-byte parity is preserved during the
    // Phase 7 migration.
    assert_eq!(TEMP_REGISTERS, [5, 6, 7, 28, 29, 30, 31]);
    assert_eq!(TEMP_REGISTERS.len(), 7);
}

#[test]
fn ret_word_is_canonical_jalr() {
    // The canonical "return": jalr x0, x1, 0
    // I-type: imm[11:0]=0 | rs1=1 | funct3=0 | rd=0 | opcode=1100111
    //       = 0 | (1 << 15) | 0 | 0 | 0x67
    //       = 0x0000_8067
    assert_eq!(RET_WORD, 0x0000_8067);

    // And the re-exported encode_jalr produces the same word.
    assert_eq!(encode_jalr(X0_ZERO, X1_RA, 0), RET_WORD);
}

#[test]
fn ret_word_little_endian_bytes_match_e2e_pin() {
    // The lang-aot RV32I e2e smoke test pins these exact 4 bytes
    // at the tail of the emitted `.bin`.
    assert_eq!(RET_WORD.to_le_bytes(), [0x67, 0x80, 0x00, 0x00]);
}

#[test]
fn encode_addi_canonical_42() {
    // `addi t0, x0, 42` — the first instruction Twig `42` emits.
    // rd=t0=5, rs1=x0=0, imm=42
    //   = (42 << 20) | (0 << 15) | (0 << 12) | (5 << 7) | 0x13
    //   = 0x02A0_0000 | 0x0000_0280 | 0x0000_0013
    //   = 0x02A0_0293
    assert_eq!(encode_addi(5, X0_ZERO, 42), 0x02A0_0293);
}

#[test]
fn encode_addi_mv_a0_t0_zero_imm() {
    // `addi a0, t0, 0` — the mv-to-return-register prologue.
    // rd=a0=10, rs1=t0=5, imm=0
    //   = (0 << 20) | (5 << 15) | (0 << 12) | (10 << 7) | 0x13
    //   = 0 | 0x0002_8000 | 0 | 0x0000_0500 | 0x13
    //   = 0x0002_8513
    assert_eq!(encode_addi(A0, 5, 0), 0x0002_8513);
}

#[test]
fn encode_jalr_ecall_distinct() {
    // Sanity-check that ecall (system call — RV32I env trap) and
    // jalr (return) don't collide on their canonical encodings.
    assert_ne!(encode_jalr(X0_ZERO, X1_RA, 0), encode_ecall());
}

#[test]
fn assemble_little_endian_byte_order() {
    // `assemble` is the re-exported convenience to flatten a
    // Vec<u32> of instruction words into a `Vec<u8>` of
    // little-endian bytes, the way the lang-aot RV32I .bin emitter
    // expects.
    let words = vec![RET_WORD];
    let bytes = assemble(&words);
    assert_eq!(bytes, vec![0x67, 0x80, 0x00, 0x00]);
}

#[test]
fn assemble_multi_word() {
    // const_i64 v=42 ; ret v == three words = 12 bytes:
    //   addi t0, x0, 42      ; 0x02A0_0293  → 0x93 0x02 0xA0 0x02
    //   addi a0, t0, 0       ; 0x0002_8513  → 0x13 0x85 0x02 0x00
    //   jalr x0, x1, 0       ; 0x0000_8067  → 0x67 0x80 0x00 0x00
    let words = vec![
        encode_addi(5, X0_ZERO, 42),
        encode_addi(A0, 5, 0),
        RET_WORD,
    ];
    let bytes = assemble(&words);
    assert_eq!(
        bytes,
        vec![
            0x93, 0x02, 0xA0, 0x02, // addi t0, x0, 42
            0x13, 0x85, 0x02, 0x00, // addi a0, t0, 0
            0x67, 0x80, 0x00, 0x00, // jalr x0, x1, 0 (ret)
        ]
    );
}
