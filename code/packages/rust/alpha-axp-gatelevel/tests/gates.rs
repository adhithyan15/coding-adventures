use coding_adventures_alpha_axp_gatelevel::{alu, decoder};

#[test]
fn arithmetic_logic_compare_shift_and_multiply_use_gate_networks() {
    assert_eq!(alu::addq(u64::MAX, 1).result, 0);
    assert_eq!(alu::subq(3, 5).result, u64::MAX - 1);
    assert_eq!(alu::andq(0xaa, 0x0f).result, 0x0a);
    assert_eq!(alu::bis(0xa0, 0x0f).result, 0xaf);
    assert_eq!(alu::xorq(0xaa, 0x0f).result, 0xa5);
    assert_eq!(alu::bic(0xff, 0x55).result, 0xaa);
    assert_eq!(alu::cmpeq(9, 9), 1);
    assert_eq!(alu::cmplt((-2_i64) as u64, 1), 1);
    assert_eq!(alu::cmpult(u64::MAX, 1), 0);
    assert_eq!(alu::sll(1, 63).result, 1 << 63);
    assert_eq!(alu::sra(1 << 63, 63).result, u64::MAX);
    assert_eq!(alu::mulq(u64::MAX, 2), u64::MAX - 1);
    assert_eq!(alu::umulh(u64::MAX, u64::MAX), u64::MAX - 1);
    assert_eq!(alu::mull(0xffff_ffff, 2), u64::MAX - 1);
}

#[test]
fn decoder_extracts_every_instruction_format_field() {
    let decoded =
        decoder::decode((0x10 << 26) | (3 << 21) | (0xab << 13) | (1 << 12) | (0x69 << 5) | 7);
    assert_eq!(decoded.op, 0x10);
    assert_eq!(decoded.ra, 3);
    assert_eq!(decoded.rc, 7);
    assert_eq!(decoded.function, 0x69);
    assert_eq!(decoded.literal, Some(0xab));

    let branch = decoder::decode((0x3f << 26) | 0x10_0000);
    assert_eq!(branch.displacement21, -0x10_0000);
}
