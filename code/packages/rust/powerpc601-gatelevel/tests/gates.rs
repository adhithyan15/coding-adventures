use coding_adventures_powerpc601_gatelevel::{alu, bits, branch, decoder, FLIP_FLOP_COUNT};

fn architectural_branch(ctr: u32, cr: u32, bo: u8, bi: u8) -> (u32, bool) {
    let next_ctr = if bo & 0x10 == 0 {
        ctr.wrapping_sub(1)
    } else {
        ctr
    };
    let ctr_ok = bo & 0x10 != 0 || ((next_ctr != 0) ^ (bo & 8 != 0));
    let cr_bit = cr & (1 << (31 - u32::from(bi))) != 0;
    let condition_ok = bo & 4 != 0 || cr_bit == (bo & 2 != 0);
    (next_ctr, ctr_ok && condition_ok)
}

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

#[test]
fn branch_network_covers_every_bo_cr_selector_and_boundary() {
    let ctr_boundaries = [0, 1, 2, u32::MAX, 0x8000_0000];
    for bo in 0_u8..32 {
        for bi in 0_u8..32 {
            for cr_high in [false, true] {
                let cr = if cr_high {
                    1_u32 << (31 - u32::from(bi))
                } else {
                    0
                };
                for ctr in ctr_boundaries {
                    let expected = architectural_branch(ctr, cr, bo, bi);
                    let actual = branch::evaluate(ctr, cr, bo, bi);
                    assert_eq!(
                        (actual.next_ctr, actual.taken),
                        expected,
                        "BO={bo:05b} BI={bi} CR={cr:08x} CTR={ctr:08x}"
                    );
                }
            }
        }
    }
}
