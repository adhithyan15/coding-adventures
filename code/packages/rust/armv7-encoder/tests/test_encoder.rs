use armv7_encoder::*;

#[test]
fn canonical_word_constants_pinned() {
    assert_eq!(BX_LR, 0xE12F_FF1E);
    assert_eq!(BKPT, 0xE12F_FF7F);
    assert_eq!(MOV_IMM_R0_BASE, 0xE3A0_0000);
    assert_eq!(MOV_REG_BASE, 0xE1A0_0000);
}

#[test]
fn capacity_constants_pinned() {
    assert_eq!(GP_REGISTER_COUNT, 12);
    assert_eq!(MOV_IMM_MAX, 255);
}

#[test]
fn encode_mov_imm_zero() {
    assert_eq!(encode_mov_imm(0, 0), 0xE3A0_0000);
}

#[test]
fn encode_mov_imm_canonical_42() {
    // The bytes the existing armv7 e2e test pins.
    assert_eq!(encode_mov_imm(0, 42), 0xE3A0_002A);
}

#[test]
fn encode_mov_imm_max_imm_max_reg() {
    assert_eq!(encode_mov_imm(15, 255), 0xE3A0_F0FF);
}

#[test]
fn encode_mov_imm_masks_register() {
    // Anything above 4 bits gets masked.
    assert_eq!(encode_mov_imm(0xFF, 0), 0xE3A0_F000);
}

#[test]
fn encode_mov_reg_simple() {
    assert_eq!(encode_mov_reg(0, 0), 0xE1A0_0000);
    assert_eq!(encode_mov_reg(1, 0), 0xE1A0_1000);
    assert_eq!(encode_mov_reg(0, 1), 0xE1A0_0001);
    assert_eq!(encode_mov_reg(15, 15), 0xE1A0_F00F);
}

#[test]
fn bx_lr_versus_bkpt_distinct() {
    // These two share the `12F_FF` family bits but encode very
    // different operations.  The diff is in the low byte.
    assert_eq!(BX_LR & 0xFFFFFF00, BKPT & 0xFFFFFF00);
    assert_ne!(BX_LR & 0xFF, BKPT & 0xFF);
}
