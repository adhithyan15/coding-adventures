// Tests for intel4004-encoder.
//
// Pins every opcode high nibble and every `encode_*` output byte.

use intel4004_encoder::*;

#[test]
fn opcode_high_nibbles_pinned() {
    assert_eq!(LDM_OPCODE, 0xD0);
    assert_eq!(LD_OPCODE, 0xA0);
    assert_eq!(XCH_OPCODE, 0xB0);
    assert_eq!(JUN_OPCODE, 0x40);
}

#[test]
fn halt_loop_pinned() {
    assert_eq!(HALT_LOOP, [0x40, 0x00]);
}

#[test]
fn capacity_constants_pinned() {
    assert_eq!(GP_REGISTER_COUNT, 16);
    assert_eq!(LDM_MAX, 15);
    assert_eq!(LDM_MIN_SIGNED, -8);
}

#[test]
fn encode_ldm_zero_keeps_opcode_visible() {
    assert_eq!(encode_ldm(0), 0xD0);
}

#[test]
fn encode_ldm_small_values() {
    assert_eq!(encode_ldm(5), 0xD5);
    assert_eq!(encode_ldm(7), 0xD7);
}

#[test]
fn encode_ldm_max_4bit() {
    assert_eq!(encode_ldm(15), 0xDF);
}

#[test]
fn encode_ldm_masks_overflow() {
    // Anything above 4 bits gets masked.
    assert_eq!(encode_ldm(0xFF), 0xDF);
    assert_eq!(encode_ldm(0x10), 0xD0);
}

#[test]
fn encode_ld_and_xch() {
    assert_eq!(encode_ld(0), 0xA0);
    assert_eq!(encode_ld(7), 0xA7);
    assert_eq!(encode_ld(15), 0xAF);
    assert_eq!(encode_xch(0), 0xB0);
    assert_eq!(encode_xch(15), 0xBF);
    // Mask high bits.
    assert_eq!(encode_ld(0xFF), 0xAF);
    assert_eq!(encode_xch(0xFF), 0xBF);
}

#[test]
fn encode_jun_addresses() {
    assert_eq!(encode_jun(0x000), [0x40, 0x00]);
    assert_eq!(encode_jun(0x001), [0x40, 0x01]);
    assert_eq!(encode_jun(0x0FF), [0x40, 0xFF]);
    assert_eq!(encode_jun(0x100), [0x41, 0x00]);
    assert_eq!(encode_jun(0xFFF), [0x4F, 0xFF]);
    // Mask bits 12+.
    assert_eq!(encode_jun(0xF000), [0x40, 0x00]);
}

#[test]
fn jun_round_trips_via_halt_loop() {
    assert_eq!(encode_jun(0), HALT_LOOP);
}
