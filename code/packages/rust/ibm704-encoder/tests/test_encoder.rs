//! Byte-pinning tests for `ibm704-encoder`.
//!
//! Every constant and helper the LANG VM IBM 704 e2e tests rely
//! on is asserted here as a unit-level regression invariant.

use ibm704_encoder::*;

#[test]
fn opcode_constants_pinned() {
    assert_eq!(HTR, 0o420);
    assert_eq!(CLA, 0o500);
}

#[test]
fn word_geometry_constants_pinned() {
    assert_eq!(WORD_BITS, 36);
    assert_eq!(WORD_MASK, 0xF_FFFF_FFFF);
    assert_eq!(BYTES_PER_WORD, 5);
    assert_eq!(ADDR_BITS, 15);
    assert_eq!(ADDR_MASK, 0x7FFF);
    assert_eq!(OPCODE_SHIFT, 27);
}

#[test]
fn encode_htr_zero_is_canonical_halt_word() {
    // HTR opcode = 0o420 = 0b100_010_000 (9 bits)
    //   shifted into word bits 35..27:
    //     bit 8 of opcode (256) → word bit 35 (= 2^35 = 0x8_0000_0000)
    //     bit 4 of opcode  (16) → word bit 31 (= 2^31 = 0x8000_0000)
    //   sum = 0x8_8000_0000
    assert_eq!(encode_htr(0), 0x8_8000_0000);
}

#[test]
fn encode_htr_low_address_bits_land_in_word() {
    assert_eq!(encode_htr(0x7FFF), 0x8_8000_7FFF);
}

#[test]
fn encode_htr_masks_oversize_address() {
    // 0x8000 is outside the 15-bit address field — silently masked.
    assert_eq!(encode_htr(0x8000), 0x8_8000_0000);
}

#[test]
fn encode_cla_zero_picks_clean_opcode_bits() {
    // CLA opcode = 0o500 = 0b101_000_000 (9 bits)
    //   bit 8 of opcode (256) → word bit 35 = 0x8_0000_0000
    //   bit 6 of opcode  (64) → word bit 33 = 0x2_0000_0000
    //   sum = 0xA_0000_0000
    assert_eq!(encode_cla(0), 0xA_0000_0000);
}

#[test]
fn encode_cla_42_canonical_twig_42_program() {
    // Twig `42` lowers to one `const_i64 v=42` instruction — the
    // backend emits `CLA 42` to materialise it in the accumulator.
    assert_eq!(encode_cla(42), 0xA_0000_002A);
}

#[test]
fn pack_htr_zero_matches_canonical_halt_bytes() {
    let bytes = pack_word(encode_htr(0));
    assert_eq!(bytes, [0x00, 0x00, 0x00, 0x80, 0x08]);
    assert_eq!(bytes, HTR_HALT_BYTES);
}

#[test]
fn pack_cla_42_is_pinned_lsb_first() {
    let bytes = pack_word(encode_cla(42));
    assert_eq!(bytes, [0x2A, 0x00, 0x00, 0x00, 0x0A]);
}

#[test]
fn pack_word_top_byte_high_nibble_is_always_zero() {
    // Even with every word bit set, byte 4's high nibble must be 0.
    let bytes = pack_word(WORD_MASK);
    assert_eq!(bytes[4] & 0xF0, 0x00);
    assert_eq!(bytes, [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
}

#[test]
fn pack_word_drops_stray_high_bits_above_word() {
    // Bits above 35 must not leak into the packed bytes.
    let bytes = pack_word(0xFF_FFFF_FFFF_FFFF);
    assert_eq!(bytes, [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
}

#[test]
fn htr_halt_bytes_constant_matches_pack_of_encode() {
    // The convenience constant must agree byte-for-byte with the
    // dynamic encode + pack sequence — guards against silent drift.
    assert_eq!(HTR_HALT_BYTES, pack_word(encode_htr(0)));
}

#[test]
fn twig_42_full_byte_sequence_round_trips() {
    // CLA 42 followed by HTR 0 = the entire emitted program for
    // Twig `42` on the IBM 704.  10 bytes total.
    let mut out = Vec::with_capacity(10);
    out.extend_from_slice(&pack_word(encode_cla(42)));
    out.extend_from_slice(&HTR_HALT_BYTES);
    assert_eq!(
        out,
        vec![
            0x2A, 0x00, 0x00, 0x00, 0x0A, // CLA 42
            0x00, 0x00, 0x00, 0x80, 0x08, // HTR 0 (halt)
        ]
    );
}
