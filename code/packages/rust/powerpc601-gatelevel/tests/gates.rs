use coding_adventures_powerpc601_gatelevel::{alu, bits, decoder, FLIP_FLOP_COUNT};

#[test]
fn exact_topology_and_gate_arithmetic_are_stable() {
    assert_eq!(FLIP_FLOP_COUNT, 525_473);
    assert_eq!(alu::add(u32::MAX, 1, 0).result, 0);
    assert_eq!(alu::add(u32::MAX, 1, 0).carry, 1);
    assert_eq!(alu::sub(3, 7).result, (-4_i32) as u32);
    assert_eq!(alu::multiply_low((-7_i32) as u32, 6), (-42_i32) as u32);
    assert_eq!(alu::divide_unsigned(100, 7), Some(14));
    assert_eq!(
        alu::divide_signed((-100_i32) as u32, 7),
        Some((-14_i32) as u32)
    );
    assert_eq!(
        alu::divide_signed(i32::MIN as u32, (-1_i32) as u32),
        Some(i32::MIN as u32)
    );
    assert_eq!(alu::divide_unsigned(1, 0), None);
}

#[test]
fn bit_networks_and_decoder_cover_edge_fields() {
    assert_eq!(bits::shl_32(1, 31), 0x8000_0000);
    assert_eq!(bits::shl_32(1, 32), 0);
    assert_eq!(bits::shr_32(0x8000_0000, 31), 1);
    assert_eq!(bits::sar_32(0x8000_0000, 31), u32::MAX);
    assert_eq!(alu::count_leading_zeros(0), 32);
    assert_eq!(alu::compare_signed(u32::MAX, 0), std::cmp::Ordering::Less);
    assert_eq!(
        alu::compare_unsigned(u32::MAX, 0),
        std::cmp::Ordering::Greater
    );
    let decoded = decoder::decode(0x7c64_1a14);
    assert_eq!(decoded.opcode, 31);
    assert_eq!((decoded.rd, decoded.ra, decoded.rb), (3, 4, 3));
}
