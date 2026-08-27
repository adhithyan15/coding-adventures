use ibm704_encoder::{
    encode_cla, encode_hpr, encode_htr, encode_instruction, encode_type_a, encode_type_b,
    pack_word, unpack_word, unpack_words, DecodeError, ADDR_MASK, BYTES_PER_WORD, CLA,
    DECREMENT_SHIFT, HPR, HTR, HTR_HALT_BYTES, OPCODE_MASK, OPCODE_SHIFT, TAG_MASK, TAG_SHIFT,
    WORD_BITS, WORD_MASK,
};

#[test]
fn canonical_opcode_constants_distinguish_halt_variants() {
    assert_eq!(HTR, 0o000);
    assert_eq!(HPR, 0o420);
    assert_eq!(CLA, 0o500);
}

#[test]
fn word_geometry_matches_the_704_manual() {
    assert_eq!(WORD_BITS, 36);
    assert_eq!(WORD_MASK, 0xF_FFFF_FFFF);
    assert_eq!(BYTES_PER_WORD, 5);
    assert_eq!(ADDR_MASK, 0x7FFF);
    assert_eq!(OPCODE_MASK, 0o777);
    assert_eq!(TAG_MASK, 0b111);
    assert_eq!(OPCODE_SHIFT, 24);
    assert_eq!(DECREMENT_SHIFT, 18);
    assert_eq!(TAG_SHIFT, 15);
}

#[test]
fn type_b_places_positive_operation_tag_and_address() {
    let word = encode_type_b(false, CLA, 5, 0x1234);

    assert_eq!(word >> 35, 0);
    assert_eq!((word >> 33) & 0b11, 0);
    assert_eq!((word >> OPCODE_SHIFT) & OPCODE_MASK, CLA as u64);
    assert_eq!((word >> 18) & 0x3F, 0);
    assert_eq!((word >> TAG_SHIFT) & TAG_MASK, 5);
    assert_eq!(word & ADDR_MASK, 0x1234);
}

#[test]
fn type_b_sign_bit_represents_negative_operation_code() {
    let positive = encode_type_b(false, CLA, 0, 0);
    let negative = encode_type_b(true, CLA, 0, 0);

    assert_eq!(positive, 0x1_4000_0000);
    assert_eq!(negative, 0x9_4000_0000);
    assert_eq!(positive ^ negative, 1 << 35);
}

#[test]
fn type_a_places_prefix_decrement_tag_and_address() {
    let word = encode_type_a(0b110, 0x4567, 3, 0x2345);

    assert_eq!((word >> 33) & 0b111, 0b110);
    assert_eq!((word >> DECREMENT_SHIFT) & ADDR_MASK, 0x4567);
    assert_eq!((word >> TAG_SHIFT) & TAG_MASK, 3);
    assert_eq!(word & ADDR_MASK, 0x2345);
}

#[test]
fn field_builders_mask_oversized_inputs() {
    assert_eq!(
        encode_type_b(true, 0xFFFF, 0xFF, 0xFFFF),
        (1 << 35) | (OPCODE_MASK << OPCODE_SHIFT) | (TAG_MASK << TAG_SHIFT) | ADDR_MASK
    );
    assert_eq!(
        encode_type_a(0xFF, 0xFFFF, 0xFF, 0xFFFF),
        (0b111 << 33) | (ADDR_MASK << DECREMENT_SHIFT) | (TAG_MASK << TAG_SHIFT) | ADDR_MASK
    );
}

#[test]
fn convenience_encoders_use_canonical_type_b_words() {
    assert_eq!(
        encode_instruction(CLA, 42),
        encode_type_b(false, CLA, 0, 42)
    );
    assert_eq!(encode_cla(42), 0x1_4000_002A);
    assert_eq!(encode_htr(0), 0);
    assert_eq!(encode_hpr(0), 0x1_1000_0000);
}

#[test]
fn pack_word_is_five_byte_big_endian() {
    assert_eq!(pack_word(encode_cla(42)), [0x01, 0x40, 0x00, 0x00, 0x2A]);
    assert_eq!(pack_word(WORD_MASK), [0x0F, 0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(pack_word(u64::MAX), [0x0F, 0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn unpack_word_round_trips_and_rejects_reserved_nibble() {
    let word = encode_type_a(0b101, 0x4567, 6, 0x1234);
    assert_eq!(unpack_word(pack_word(word)).unwrap(), word);
    assert_eq!(
        unpack_word([0x10, 0, 0, 0, 0]),
        Err(DecodeError::ReservedNibble(0x10))
    );
}

#[test]
fn unpack_words_validates_length_and_each_word() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&pack_word(encode_cla(2)));
    bytes.extend_from_slice(&HTR_HALT_BYTES);
    bytes.extend_from_slice(&pack_word(42));

    assert_eq!(unpack_words(&bytes).unwrap(), vec![encode_cla(2), 0, 42]);
    assert_eq!(
        unpack_words(&bytes[..bytes.len() - 1]),
        Err(DecodeError::InvalidLength(14))
    );

    bytes[5] = 0x80;
    assert_eq!(unpack_words(&bytes), Err(DecodeError::ReservedNibble(0x80)));
}

#[test]
fn canonical_htr_bytes_are_all_zero() {
    assert_eq!(HTR_HALT_BYTES, [0; BYTES_PER_WORD]);
    assert_eq!(HTR_HALT_BYTES, pack_word(encode_htr(0)));
}
