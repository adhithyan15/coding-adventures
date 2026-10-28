use ibm_system370_gatelevel::gates;

#[test]
fn extension_decode_is_exactly_one_hot_only_for_five_opcodes() {
    for opcode in 0_u8..=u8::MAX {
        let lines = gates::decode_extension(opcode);
        let expected =
            [0x0e, 0x0f, 0xbd, 0xbe, 0xbf].map(|candidate| u8::from(opcode == candidate));
        assert_eq!(lines, expected, "opcode {opcode:#04x}");
        assert!(lines.into_iter().sum::<u8>() <= 1);
    }
}

#[test]
fn byte_comparator_exhausts_the_complete_truth_table() {
    for left in 0_u8..=u8::MAX {
        for right in 0_u8..=u8::MAX {
            let expected = if left < right {
                1
            } else if left > right {
                2
            } else {
                0
            };
            assert_eq!(gates::compare_byte(left, right), expected);
        }
    }
}

#[test]
fn byte_and_register_selection_mux_truth_tables_are_exhaustive() {
    for zero in 0_u8..=u8::MAX {
        for one in 0_u8..=u8::MAX {
            assert_eq!(gates::select_u8(zero, one, 0), zero);
            assert_eq!(gates::select_u8(zero, one, 1), one);
        }
    }
    for selected in 0_u8..=u8::MAX {
        for index in 0..4 {
            let mut bytes = [0x11, 0x55, 0xaa, 0xee];
            bytes[index] = selected;
            assert_eq!(
                gates::select_register_byte(u32::from_be_bytes(bytes), index),
                selected
            );
        }
    }
}

#[test]
fn twenty_four_bit_add_subtract_compare_and_overlap_boundaries_match() {
    let mut values = vec![
        0, 1, 2, 0x7f, 0xff, 0x100, 0x0f_ffff, 0x10_0000, 0x7f_ffff, 0xff_fffe, 0xff_ffff,
    ];
    let mut sample = 0x00c0_ffee_u32;
    for _ in 0..256 {
        sample = sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223) & 0x00ff_ffff;
        values.push(sample);
    }
    for &left in &values {
        for &right in &values {
            assert_eq!(
                gates::add_u24(left, right),
                left.wrapping_add(right) & 0x00ff_ffff
            );
            assert_eq!(
                gates::sub_u24(left, right),
                left.wrapping_sub(right) & 0x00ff_ffff
            );
            let expected = if left < right {
                1
            } else if left > right {
                2
            } else {
                0
            };
            assert_eq!(gates::compare_u24(left, right), expected);
        }
    }
    for source in 0_u32..32 {
        for destination in 0_u32..32 {
            for copied in 0_u32..16 {
                let expected = copied != 0
                    && destination > source
                    && destination.wrapping_sub(source) < copied;
                assert_eq!(
                    gates::destructive_overlap(destination, source, copied),
                    u8::from(expected)
                );
            }
        }
    }
}

#[test]
fn mask_prefix_merge_and_icm_condition_truth_tables_are_exact() {
    for mask in 0_u8..16 {
        let mut prefix = 0;
        for index in 0..4 {
            assert_eq!(gates::mask_prefix(mask, index), prefix);
            prefix += gates::mask_enabled(mask, index);
        }
        let source = [0x80, 0x00, 0x01, 0xff];
        let leftmost = (0..4)
            .find(|index| gates::mask_enabled(mask, *index) == 1)
            .map_or(0, |index| source[index]);
        assert_eq!(gates::leftmost_enabled_byte(mask, source), leftmost);
        let any = (0..4).any(|index| gates::mask_enabled(mask, index) == 1 && source[index] != 0);
        let expected_cc = if !any {
            0
        } else if leftmost & 0x80 != 0 {
            1
        } else {
            2
        };
        assert_eq!(gates::icm_condition_code(mask, source), expected_cc);
    }
    for original in [0, 1, u32::MAX, 0x1122_3344, 0x8000_0001, 0x55aa_c33c] {
        for byte in 0_u8..=u8::MAX {
            for index in 0..4 {
                assert_eq!(
                    gates::merge_register_byte(original, byte, index, 0),
                    original
                );
                assert_eq!(
                    gates::merge_register_byte(original, byte, index, 1),
                    (original & !(0xff << (24 - index * 8)))
                        | (u32::from(byte) << (24 - index * 8))
                );
            }
        }
    }
}
