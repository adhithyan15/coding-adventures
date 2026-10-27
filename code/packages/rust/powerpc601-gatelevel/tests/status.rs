use coding_adventures_powerpc601_gatelevel::{
    decoder,
    status::{self, RelationWires},
};
use powerpc601_simulator::encoding::{d_form, x_form, xo_form};

fn relation_nibble(relation: RelationWires, so: u8) -> u32 {
    (u32::from(relation.lt) << 3)
        | (u32::from(relation.gt) << 2)
        | (u32::from(relation.eq) << 1)
        | u32::from(so)
}

#[test]
fn signed_and_unsigned_control_wire_truth_tables_are_exhaustive() {
    for zero in 0..=1 {
        for negative in 0..=1 {
            for overflow in 0..=1 {
                let actual = status::signed_relation(zero, negative, overflow);
                let expected = if zero == 1 {
                    RelationWires {
                        lt: 0,
                        gt: 0,
                        eq: 1,
                    }
                } else if negative ^ overflow == 1 {
                    RelationWires {
                        lt: 1,
                        gt: 0,
                        eq: 0,
                    }
                } else {
                    RelationWires {
                        lt: 0,
                        gt: 1,
                        eq: 0,
                    }
                };
                assert_eq!(actual, expected, "Z={zero} N={negative} V={overflow}");
                assert_eq!(actual.lt + actual.gt + actual.eq, 1);
            }
        }
        for carry in 0..=1 {
            let actual = status::unsigned_relation(zero, carry);
            let expected = if zero == 1 {
                RelationWires {
                    lt: 0,
                    gt: 0,
                    eq: 1,
                }
            } else if carry == 0 {
                RelationWires {
                    lt: 1,
                    gt: 0,
                    eq: 0,
                }
            } else {
                RelationWires {
                    lt: 0,
                    gt: 1,
                    eq: 0,
                }
            };
            assert_eq!(actual, expected, "Z={zero} C={carry}");
            assert_eq!(actual.lt + actual.gt + actual.eq, 1);
        }
    }
}

#[test]
fn architectural_relation_witnesses_cross_signed_and_unsigned_boundaries() {
    let values = [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX];
    for left in values {
        for right in values {
            let signed = status::compare_signed(left, right);
            let expected_signed = match (left as i32).cmp(&(right as i32)) {
                std::cmp::Ordering::Less => [1, 0, 0],
                std::cmp::Ordering::Greater => [0, 1, 0],
                std::cmp::Ordering::Equal => [0, 0, 1],
            };
            assert_eq!([signed.lt, signed.gt, signed.eq], expected_signed);

            let unsigned = status::compare_unsigned(left, right);
            let expected_unsigned = match left.cmp(&right) {
                std::cmp::Ordering::Less => [1, 0, 0],
                std::cmp::Ordering::Greater => [0, 1, 0],
                std::cmp::Ordering::Equal => [0, 0, 1],
            };
            assert_eq!([unsigned.lt, unsigned.gt, unsigned.eq], expected_unsigned);

            assert_eq!(status::select_relation(signed, unsigned, 1, 0), signed);
            assert_eq!(status::select_relation(signed, unsigned, 0, 1), unsigned);
            assert_eq!(
                status::select_relation(signed, unsigned, 0, 0),
                RelationWires {
                    lt: 0,
                    gt: 0,
                    eq: 0,
                }
            );
        }
    }
}

#[test]
fn cr_field_decoder_and_cr0_write_mux_preserve_every_unselected_bit() {
    let old = 0x1234_5678;
    let relations = [
        RelationWires {
            lt: 1,
            gt: 0,
            eq: 0,
        },
        RelationWires {
            lt: 0,
            gt: 1,
            eq: 0,
        },
        RelationWires {
            lt: 0,
            gt: 0,
            eq: 1,
        },
    ];
    for field in 0_u32..8 {
        let selector = std::array::from_fn(|bit| ((field >> bit) & 1) as u8);
        for relation in relations {
            for so in 0..=1 {
                let actual = status::write_cr_field(old, selector, relation, so, 1);
                let shift = 28 - field * 4;
                let expected = (old & !(0xf << shift)) | (relation_nibble(relation, so) << shift);
                assert_eq!(
                    actual, expected,
                    "field={field} relation={relation:?} SO={so}"
                );
                assert_eq!(status::write_cr_field(old, selector, relation, so, 0), old);
            }
        }
    }

    for value in [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        for flags in 0_u32..8 {
            let xer = 0x15a5_5a5a | ((flags & 4) << 29) | ((flags & 2) << 29) | ((flags & 1) << 29);
            for enable in 0..=1 {
                let actual = status::write_cr0(old, value, status::so(xer), enable);
                let candidate = status::write_cr_field(
                    old,
                    [0, 0, 0],
                    status::compare_signed(value, 0),
                    status::so(xer),
                    1,
                );
                assert_eq!(actual, if enable == 0 { old } else { candidate });
            }
        }
    }
}

#[test]
fn ca_mux_truth_table_preserves_so_ov_and_opaque_xer_bits() {
    let sentinel = 0xd5a5_5a5a;
    for old_ca in 0..=1 {
        for new_ca in 0..=1 {
            for enable in 0..=1 {
                let old = (sentinel & !(1 << 29)) | (u32::from(old_ca) << 29);
                let actual = status::write_ca(old, new_ca, enable);
                let expected_ca = if enable == 0 { old_ca } else { new_ca };
                assert_eq!((actual >> 29) & 1, u32::from(expected_ca));
                assert_eq!(actual & !(1 << 29), old & !(1 << 29));
            }
        }
    }
}

#[test]
fn mtcrf_uses_field_mask_wires_for_exact_isolation() {
    let old = 0x0123_4567;
    let source = 0xfedc_ba98;
    for mask in [0x00_u8, 0x01, 0x80, 0x55, 0xaa, 0xff] {
        let wires = std::array::from_fn(|bit| (mask >> bit) & 1);
        let actual = status::mtcrf(old, source, wires, 1);
        let mut expected = old;
        for field in 0..8 {
            let field_mask = 0xf_u32 << (28 - field * 4);
            if mask & (0x80 >> field) != 0 {
                expected = (expected & !field_mask) | (source & field_mask);
            }
        }
        assert_eq!(actual, expected, "mask={mask:02x}");
        assert_eq!(status::mtcrf(old, source, wires, 0), old);
    }
}

#[test]
fn sraw_ca_network_uses_the_full_six_bit_count() {
    fn expected(source: u32, count: u32) -> u8 {
        let negative = source & 0x8000_0000 != 0;
        let discarded = match count {
            0 => false,
            1..=31 => source & ((1_u32 << count) - 1) != 0,
            32..=63 => source != 0,
            _ => unreachable!(),
        };
        u8::from(negative && discarded)
    }

    for source in [0, 1, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, u32::MAX] {
        for count in 0..64 {
            assert_eq!(
                status::arithmetic_right_shift_ca(source, status::count_wires(count)),
                expected(source, count),
                "source={source:08x} count={count}"
            );
        }
    }
    assert_eq!(
        status::arithmetic_right_shift_ca(0x8000_0000, status::count_wires(31)),
        0
    );
    assert_eq!(
        status::arithmetic_right_shift_ca(0x8000_0000, status::count_wires(32)),
        1
    );
    assert_eq!(
        status::arithmetic_right_shift_ca(0x8000_0000, status::count_wires(63)),
        1
    );
}

#[test]
fn decoder_exposes_gate_lines_for_every_status_producer() {
    let subfic = decoder::decode(d_form(8, 4, 1, 0));
    assert_eq!(subfic.status.subfic, 1);
    assert_eq!(subfic.status.addc, 0);

    let andi = decoder::decode(d_form(28, 1, 2, 0));
    assert_eq!((andi.status.andi, andi.status.andis), (1, 0));
    let andis = decoder::decode(d_form(29, 1, 2, 0));
    assert_eq!((andis.status.andi, andis.status.andis), (0, 1));

    let cmpli = decoder::decode(d_form(10, 0, 1, 0));
    assert_eq!(
        (
            cmpli.status.compare_immediate,
            cmpli.status.compare_immediate_signed,
            cmpli.status.compare_immediate_unsigned,
        ),
        (1, 0, 1)
    );
    let cmpi = decoder::decode(d_form(11, 0, 1, 0));
    assert_eq!(
        (
            cmpi.status.compare_immediate,
            cmpi.status.compare_immediate_signed,
            cmpi.status.compare_immediate_unsigned,
        ),
        (1, 1, 0)
    );

    let cmp = decoder::decode(x_form(31, 0, 1, 2, 0, false));
    assert_eq!(
        (
            cmp.status.compare_register,
            cmp.status.compare_register_signed,
            cmp.status.compare_register_unsigned,
        ),
        (1, 1, 0)
    );
    let cmpl = decoder::decode(x_form(31, 0, 1, 2, 32, false));
    assert_eq!(
        (
            cmpl.status.compare_register,
            cmpl.status.compare_register_signed,
            cmpl.status.compare_register_unsigned,
        ),
        (1, 0, 1)
    );
    assert_eq!(
        decoder::decode(x_form(31, 1, 0, 0, 144, false))
            .status
            .mtcrf,
        1
    );
    for xo in [24, 26, 28, 124, 316, 444, 476, 536, 792, 824] {
        assert_eq!(
            decoder::decode(x_form(31, 1, 4, 2, xo, true))
                .status
                .recordable_x,
            1,
            "XO={xo}"
        );
    }
    assert_eq!(
        decoder::decode(xo_form(31, 4, 1, 2, false, 10, false))
            .status
            .addc,
        1
    );
    assert_eq!(
        decoder::decode(xo_form(31, 4, 1, 2, false, 138, false))
            .status
            .adde,
        1
    );
    assert_eq!(
        decoder::decode(x_form(31, 1, 4, 2, 792, false)).status.sraw,
        1
    );
    assert_eq!(
        decoder::decode(x_form(31, 1, 4, 31, 824, false))
            .status
            .srawi,
        1
    );
}
