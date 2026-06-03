use intel8008_encoder::*;

#[test]
fn opcode_constants_pinned() {
    assert_eq!(HLT, 0x76);
    assert_eq!(RET, 0x07);
    assert_eq!(MVI_A, 0x3E);
    assert_eq!(JMP, 0x7C);
    assert_eq!(CAL, 0x7E);
}

#[test]
fn capacity_constants_pinned() {
    assert_eq!(GP_REGISTER_COUNT, 7);
    assert_eq!(MVI_MAX, 255);
}

#[test]
fn encode_mvi_a_canonical_42() {
    // The bytes the existing intel8008 e2e test pins.
    assert_eq!(encode_mvi_a(42), [0x3E, 0x2A]);
}

#[test]
fn encode_mvi_a_zero() {
    assert_eq!(encode_mvi_a(0), [0x3E, 0x00]);
}

#[test]
fn encode_mvi_a_max() {
    assert_eq!(encode_mvi_a(255), [0x3E, 0xFF]);
}

#[test]
fn encode_jmp_simple() {
    assert_eq!(encode_jmp(0x000), [0x7C, 0x00, 0x00]);
    assert_eq!(encode_jmp(0x001), [0x7C, 0x01, 0x00]);
    assert_eq!(encode_jmp(0x100), [0x7C, 0x00, 0x01]);
    assert_eq!(encode_jmp(0x3FFF), [0x7C, 0xFF, 0x3F]);
}

#[test]
fn encode_jmp_masks_high_bits() {
    // 14-bit address; anything above bit 13 should mask off.
    assert_eq!(encode_jmp(0xC000), [0x7C, 0x00, 0x00]);
}

#[test]
fn encode_cal_simple() {
    assert_eq!(encode_cal(0x123), [0x7E, 0x23, 0x01]);
}

#[test]
fn hlt_versus_jmp_versus_cal_distinct() {
    // Group-01 jump/call/return family-bit hazards documented
    // in iir-to-intel8008's comments.
    assert_ne!(HLT, JMP);
    assert_ne!(JMP, CAL);
    assert_eq!(JMP & 0x80, 0x00, "JMP is in group 01");
    assert_eq!(CAL & 0x80, 0x00, "CAL is in group 01");
    assert_eq!(HLT & 0x80, 0x00, "HLT is in group 01");
}
