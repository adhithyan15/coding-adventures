// Tests for ge225-encoder.
//
// These pin every opcode nibble and every `encode_*` helper's
// output bytes.  Because `iir-to-ge225` re-exports through this
// crate (Phase 1 of the historical-arch backend migration), any
// drift here automatically breaks the iir-to-ge225 tests too —
// which is exactly the invariant we want.

use ge225_encoder::*;

// ===========================================================================
// §1. Opcode nibble constants
// ===========================================================================

#[test]
fn opcode_nibbles_pinned() {
    assert_eq!(LDA_OPCODE_NIBBLE, 0x1);
    assert_eq!(STA_OPCODE_NIBBLE, 0x2);
    assert_eq!(LD_OPCODE_NIBBLE, 0x3);
    assert_eq!(ADD_OPCODE_NIBBLE, 0x4);
    assert_eq!(SUB_OPCODE_NIBBLE, 0x5);
    assert_eq!(BR_OPCODE_NIBBLE, 0x6);
    assert_eq!(BNZ_OPCODE_NIBBLE, 0x7);
    assert_eq!(BZ_OPCODE_NIBBLE, 0x8);
    assert_eq!(JSR_OPCODE_NIBBLE, 0x9);
    assert_eq!(RTS_OPCODE_NIBBLE, 0xA);
    assert_eq!(BMI_OPCODE_NIBBLE, 0xB);
}

// ===========================================================================
// §2. Canonical word constants
// ===========================================================================

#[test]
fn halt_word_pinned_to_zeros() {
    assert_eq!(HALT_WORD, [0x00, 0x00, 0x00]);
}

#[test]
fn rts_word_pinned() {
    assert_eq!(RTS_WORD, [0x0A, 0x00, 0x00]);
}

// ===========================================================================
// §3. Capacity constants
// ===========================================================================

#[test]
fn capacity_constants_pinned() {
    assert_eq!(GP_REGISTER_COUNT, 16);
    assert_eq!(LDA_MAX_SIGNED, 32_767);
    assert_eq!(LDA_MIN_SIGNED, -32_768);
    assert_eq!(LDA_MAX_UNSIGNED, 65_535);
}

// ===========================================================================
// §4. encode_lda
// ===========================================================================

#[test]
fn encode_lda_zero() {
    // LDA 0 must still surface the LDA opcode nibble.
    assert_eq!(encode_lda(0), [0x01, 0x00, 0x00]);
}

#[test]
fn encode_lda_small_positive() {
    assert_eq!(encode_lda(5), [0x01, 0x00, 0x05]);
    assert_eq!(encode_lda(42), [0x01, 0x00, 0x2A]);
}

#[test]
fn encode_lda_byte_boundary() {
    assert_eq!(encode_lda(0x00FF), [0x01, 0x00, 0xFF]);
    assert_eq!(encode_lda(0x0100), [0x01, 0x01, 0x00]);
}

#[test]
fn encode_lda_max_positive_i16() {
    // i16::MAX = 32767 = 0x7FFF
    assert_eq!(encode_lda(32_767), [0x01, 0x7F, 0xFF]);
}

#[test]
fn encode_lda_min_negative_twos_complement() {
    // i16::MIN = -32768 = 0x8000 via two's complement
    assert_eq!(encode_lda((-32_768i16) as u16), [0x01, 0x80, 0x00]);
}

#[test]
fn encode_lda_negative_one() {
    assert_eq!(encode_lda((-1i16) as u16), [0x01, 0xFF, 0xFF]);
}

#[test]
fn encode_lda_max_unsigned() {
    assert_eq!(encode_lda(65_535), [0x01, 0xFF, 0xFF]);
}

// ===========================================================================
// §5. encode_sta / encode_ld / encode_add / encode_sub
// ===========================================================================

#[test]
fn encode_register_ops_simple() {
    assert_eq!(encode_sta(0), [0x02, 0x00, 0x00]);
    assert_eq!(encode_sta(5), [0x02, 0x00, 0x05]);
    assert_eq!(encode_sta(15), [0x02, 0x00, 0x0F]);

    assert_eq!(encode_ld(0), [0x03, 0x00, 0x00]);
    assert_eq!(encode_ld(15), [0x03, 0x00, 0x0F]);

    assert_eq!(encode_add(0), [0x04, 0x00, 0x00]);
    assert_eq!(encode_add(15), [0x04, 0x00, 0x0F]);

    assert_eq!(encode_sub(0), [0x05, 0x00, 0x00]);
    assert_eq!(encode_sub(15), [0x05, 0x00, 0x0F]);
}

#[test]
fn encode_register_ops_mask_high_bits() {
    // Registers are 4 bits; the encoders mask anything above to
    // guarantee a well-formed 3-byte word even on caller bugs.
    assert_eq!(encode_sta(0xFF), [0x02, 0x00, 0x0F]);
    assert_eq!(encode_ld(0xFF), [0x03, 0x00, 0x0F]);
    assert_eq!(encode_add(0xFF), [0x04, 0x00, 0x0F]);
    assert_eq!(encode_sub(0xFF), [0x05, 0x00, 0x0F]);
}

// ===========================================================================
// §6. Branch encoders (BR / BNZ / BZ / BMI / JSR)
// ===========================================================================

#[test]
fn encode_branches_at_zero_address() {
    assert_eq!(encode_br(0), [0x06, 0x00, 0x00]);
    assert_eq!(encode_bnz(0), [0x07, 0x00, 0x00]);
    assert_eq!(encode_bz(0), [0x08, 0x00, 0x00]);
    assert_eq!(encode_bmi(0), [0x0B, 0x00, 0x00]);
    assert_eq!(encode_jsr(0), [0x09, 0x00, 0x00]);
}

#[test]
fn encode_branches_at_small_address() {
    // Address 3 (just past a single 3-byte instruction).
    assert_eq!(encode_br(3), [0x06, 0x00, 0x03]);
    assert_eq!(encode_bnz(3), [0x07, 0x00, 0x03]);
    assert_eq!(encode_bz(3), [0x08, 0x00, 0x03]);
}

#[test]
fn encode_branches_at_max_address() {
    // u16::MAX = 0xFFFF
    assert_eq!(encode_br(0xFFFF), [0x06, 0xFF, 0xFF]);
    assert_eq!(encode_bnz(0xFFFF), [0x07, 0xFF, 0xFF]);
    assert_eq!(encode_bz(0xFFFF), [0x08, 0xFF, 0xFF]);
    assert_eq!(encode_bmi(0xFFFF), [0x0B, 0xFF, 0xFF]);
    assert_eq!(encode_jsr(0xFFFF), [0x09, 0xFF, 0xFF]);
}

#[test]
fn encode_branches_byte_boundary() {
    // Address 0x0100 — boundary between low and high address bytes.
    assert_eq!(encode_br(0x0100), [0x06, 0x01, 0x00]);
    assert_eq!(encode_bnz(0x0100), [0x07, 0x01, 0x00]);
}

// ===========================================================================
// §7. decode_word round-trip
// ===========================================================================

#[test]
fn decode_word_round_trips_every_encode_helper() {
    // LDA
    let (op, val) = decode_word(encode_lda(0x1234));
    assert_eq!(op, LDA_OPCODE_NIBBLE);
    assert_eq!(val, 0x1234);

    // STA r5
    let (op, val) = decode_word(encode_sta(5));
    assert_eq!(op, STA_OPCODE_NIBBLE);
    assert_eq!(val, 0x0005);

    // BR 0x7FFF
    let (op, val) = decode_word(encode_br(0x7FFF));
    assert_eq!(op, BR_OPCODE_NIBBLE);
    assert_eq!(val, 0x7FFF);

    // RTS
    let (op, val) = decode_word(RTS_WORD);
    assert_eq!(op, RTS_OPCODE_NIBBLE);
    assert_eq!(val, 0);

    // HLT
    let (op, val) = decode_word(HALT_WORD);
    assert_eq!(op, 0x0);
    assert_eq!(val, 0);
}

#[test]
fn decode_word_strips_top_4_bits_of_byte_0() {
    // The decoder should ignore the top 4 bits of byte 0 (always
    // zero on real GE-225 words but defensive coding still applies).
    let (op, _) = decode_word([0xF1, 0x00, 0x00]);
    assert_eq!(op, 0x1, "top 4 bits of byte 0 must be ignored");
}
