use intel8080_encoder::*;

#[test]
fn opcode_constants_pinned() {
    assert_eq!(HLT, 0x76);
    assert_eq!(RET, 0xC9);
}

#[test]
fn capacity_constants_pinned() {
    assert_eq!(MVI_MAX, 255);
}

#[test]
fn register_codes_pinned() {
    assert_eq!(REG_B, 0);
    assert_eq!(REG_C, 1);
    assert_eq!(REG_D, 2);
    assert_eq!(REG_E, 3);
    assert_eq!(REG_H, 4);
    assert_eq!(REG_L, 5);
    assert_eq!(REG_M, 6);
    assert_eq!(REG_A, 7);
}

#[test]
fn encode_mvi_a_canonical_42() {
    // The bytes the lang-aot Intel 8080 e2e smoke test pins.
    assert_eq!(encode_mvi_a(42), vec![0x3E, 0x2A]);
}

#[test]
fn encode_mvi_a_zero() {
    assert_eq!(encode_mvi_a(0), vec![0x3E, 0x00]);
}

#[test]
fn encode_mvi_a_max() {
    assert_eq!(encode_mvi_a(255), vec![0x3E, 0xFF]);
}

#[test]
fn assemble_mvi_a_then_hlt() {
    assert_eq!(assemble(&[encode_mvi_a(42), vec![HLT]]), vec![0x3E, 0x2A, 0x76]);
}

#[test]
fn hlt_versus_ret_distinct() {
    assert_ne!(HLT, RET);
}
