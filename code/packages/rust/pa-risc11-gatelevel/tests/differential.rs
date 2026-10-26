mod common;

use common::*;

#[test]
fn all_primary_and_extension_classifications_match() {
    for primary in 0..64_u32 {
        for len in [1_usize, 2] {
            let word = if primary == 0x34 {
                shd(0, 0, 0, 0)
            } else {
                primary << 26
            };
            let image = vec![word; len];
            let (mut gate, mut oracle) = pair(&image, 0);
            assert_step_equal(&mut gate, &mut oracle);
        }
    }
    for extension in 0..64 {
        let (mut gate, mut oracle) = pair(&[alu(extension, 2, 1, 3)], 0);
        mutate_pair(&mut gate, &mut oracle, |state| {
            state.gpr[1] = 0x8000_0001;
            state.gpr[2] = 0x7fff_ffff;
        });
        assert_step_equal(&mut gate, &mut oracle);
    }
    for extension in 0..8 {
        let word = (0x3a << 26) | (2 << 21) | (3 << 16) | (extension << 13);
        let (mut gate, mut oracle) = pair(&[word], 0);
        assert_step_equal(&mut gate, &mut oracle);
    }
}

#[test]
fn alu_overlap_gr0_and_nibble_carries_match() {
    let values = [
        0,
        1,
        15,
        16,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX,
        0xaaaa_5555,
    ];
    for extension in [0, 8, 9, 10, 16, 24] {
        for right in [0, 1, 2, 31] {
            for left in [0, 1, 2, 31] {
                for target in [0, 1, 2, 31] {
                    let (mut gate, mut oracle) = pair(&[alu(extension, right, left, target)], 0);
                    mutate_pair(&mut gate, &mut oracle, |state| {
                        for register in 1..32 {
                            state.gpr[register] = values[register % values.len()];
                        }
                        state.carry_borrow = [true, false, true, false, true, false, true, false];
                    });
                    assert_step_equal(&mut gate, &mut oracle);
                }
            }
        }
    }
}

#[test]
fn immediates_permutations_and_all_shd_counts_match() {
    for value in [-8192, -1, 0, 1, 8191] {
        let (mut gate, mut oracle) = pair(&[immediate(0x0d, 1, 2, value)], 0);
        mutate_pair(&mut gate, &mut oracle, |state| state.gpr[1] = 0x1234_5678);
        assert_step_equal(&mut gate, &mut oracle);
    }
    for bit in 0..21 {
        let field = assemble_21_inverse(1 << bit);
        for primary in [0x08, 0x0a] {
            let (mut gate, mut oracle) = pair(&[(primary << 26) | (3 << 21) | field], 0);
            mutate_pair(&mut gate, &mut oracle, |state| state.gpr[3] = 17);
            assert_step_equal(&mut gate, &mut oracle);
        }
    }
    for count in 0..32 {
        for (left, right) in [
            (0, 0),
            (1, 0x8000_0000),
            (0xaaaa_aaaa, 0x5555_5555),
            (u32::MAX, u32::MAX),
        ] {
            let (mut gate, mut oracle) = pair(&[shd(2, 1, count, 3)], 0);
            mutate_pair(&mut gate, &mut oracle, |state| {
                state.gpr[1] = left;
                state.gpr[2] = right;
            });
            assert_step_equal(&mut gate, &mut oracle);
        }
    }
}

#[test]
fn every_condition_sense_form_direction_and_nullification_matches() {
    let operands = [
        (0, 0),
        (0, 1),
        (1, 0),
        (0x7fff_ffff, 1),
        (0x8000_0000, u32::MAX),
        (u32::MAX, 1),
    ];
    for primary in [0x20, 0x21, 0x22, 0x23, 0x28, 0x29, 0x2a, 0x2b] {
        for condition in 0..8 {
            for displacement in [-8, 8] {
                for nullify in [false, true] {
                    for (left, right) in operands {
                        let immediate = primary & 1 == 1;
                        let left_field = if immediate {
                            low_sign(left as i32, 5)
                        } else {
                            1
                        };
                        let word =
                            conditional(primary, 2, left_field, condition, displacement, nullify);
                        let (mut gate, mut oracle) = pair(&[word, 0, 0, 0], 0x100);
                        mutate_pair(&mut gate, &mut oracle, |state| {
                            state.gpr[1] = left;
                            state.gpr[2] = right;
                        });
                        assert_step_equal(&mut gate, &mut oracle);
                    }
                }
            }
        }
    }
}

#[test]
fn memory_width_lane_alignment_and_boundaries_match_atomically() {
    for (primary, width) in [
        (0x10, 1),
        (0x11, 2),
        (0x12, 4),
        (0x18, 1),
        (0x19, 2),
        (0x1a, 4),
    ] {
        for address in [0, 1, 2, 3, 0x100, 65_532, 65_535, 65_536] {
            let (mut gate, mut oracle) = pair(&[immediate(primary, 1, 2, 0)], 0);
            mutate_pair(&mut gate, &mut oracle, |state| {
                state.gpr[1] = address;
                state.gpr[2] = 0x89ab_cdef;
                if address < 65_536 {
                    let end = usize::min(65_536, address as usize + width);
                    for (offset, byte) in state.memory[address as usize..end].iter_mut().enumerate()
                    {
                        *byte = 0x80 + offset as u8;
                    }
                }
            });
            assert_step_equal(&mut gate, &mut oracle);
        }
    }
}

#[test]
fn branch_calls_returns_delay_and_deferred_fetch_match() {
    for word in [
        bl(2, 8, false),
        bl(2, -8, true),
        blr(2, 1, true),
        bv(2, 1, false),
    ] {
        let (mut gate, mut oracle) = pair(&[word, immediate(0x0d, 0, 3, 7), 0, 0], 0x100);
        mutate_pair(&mut gate, &mut oracle, |state| {
            state.gpr[1] = 1;
            state.gpr[2] = 0x100;
        });
        assert_step_equal(&mut gate, &mut oracle);
        assert_step_equal(&mut gate, &mut oracle);
    }

    let (mut gate, mut oracle) = pair(&[bl(2, 8, false), bl(3, 8, false), 0, 0], 0);
    assert_step_equal(&mut gate, &mut oracle);
    assert_step_equal(&mut gate, &mut oracle);
}

#[test]
fn every_branch_displacement_source_bit_matches() {
    for bit in 0..12 {
        let displacement = if bit == 11 { -8192 } else { (1_i32 << bit) * 4 };
        let word = conditional(0x20, 1, 1, 0, displacement, false);
        let (mut gate, mut oracle) = pair(&[word], 0x8000);
        assert_step_equal(&mut gate, &mut oracle);
    }
    for bit in 0..17 {
        let displacement = if bit == 16 {
            -262_144
        } else {
            (1_i32 << bit) * 4
        };
        let (mut gate, mut oracle) = pair(&[bl(2, displacement, false)], 0x8000);
        assert_step_equal(&mut gate, &mut oracle);
    }
}
