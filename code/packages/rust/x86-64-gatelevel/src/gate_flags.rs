//! Gate-derived arithmetic flags for the x86-64 integer execution path.

use crate::bits::{add_64, and_64, invert_64, u64_to_bits, zero_64};
use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use x86_simulator::state::Flags;

pub(crate) fn add_with_flags(dst: u64, src: u64) -> (u64, Flags) {
    let (result, carry, overflow) = add_64(dst, src, 0);
    (
        result,
        flags(result, carry, overflow, auxiliary_add(dst, src)),
    )
}

pub(crate) fn sub_with_flags(dst: u64, src: u64) -> (u64, Flags) {
    let (result, no_borrow, overflow) = add_64(dst, invert_64(src), 1);
    (
        result,
        flags(
            result,
            not_gate(no_borrow),
            overflow,
            auxiliary_sub(dst, src),
        ),
    )
}

pub(crate) fn adc_with_flags(dst: u64, src: u64, carry: bool) -> (u64, Flags) {
    let carry_in = u8::from(carry);
    let (partial, carry_a, _) = add_64(dst, src, 0);
    let (result, carry_b, _) = add_64(partial, u64::from(carry_in), 0);
    let dst_bits = u64_to_bits(dst);
    let src_bits = u64_to_bits(src);
    let result_bits = u64_to_bits(result);
    let same_sign = not_gate(xor_gate(dst_bits[63], src_bits[63]));
    let changed_sign = xor_gate(dst_bits[63], result_bits[63]);
    let overflow = and_gate(same_sign, changed_sign);
    let low = ripple_carry_adder_with_carry(&dst_bits[..4], &src_bits[..4], carry_in);
    (
        result,
        flags(result, or_gate(carry_a, carry_b), overflow, low.carry_out),
    )
}

pub(crate) fn sbb_with_flags(dst: u64, src: u64, borrow: bool) -> (u64, Flags) {
    let borrow_in = u8::from(borrow);
    let (partial, no_borrow_a, _) = add_64(dst, invert_64(src), 1);
    let (result, no_borrow_b, _) = add_64(partial, invert_64(u64::from(borrow_in)), 1);
    let dst_bits = u64_to_bits(dst);
    let src_bits = u64_to_bits(src);
    let result_bits = u64_to_bits(result);
    let different_sign = xor_gate(dst_bits[63], src_bits[63]);
    let changed_sign = xor_gate(dst_bits[63], result_bits[63]);
    let overflow = and_gate(different_sign, changed_sign);
    let low = ripple_carry_adder_with_carry(
        &dst_bits[..4],
        &src_bits[..4]
            .iter()
            .copied()
            .map(not_gate)
            .collect::<Vec<_>>(),
        not_gate(borrow_in),
    );
    (
        result,
        flags(
            result,
            or_gate(not_gate(no_borrow_a), not_gate(no_borrow_b)),
            overflow,
            not_gate(low.carry_out),
        ),
    )
}

pub(crate) fn logic_flags(result: u64) -> Flags {
    flags(result, 0, 0, 0)
}

fn flags(result: u64, cf: u8, overflow: u8, auxiliary: u8) -> Flags {
    let bits = u64_to_bits(result);
    Flags {
        cf: cf != 0,
        zf: zero_64(result) != 0,
        sf: bits[63] != 0,
        of: overflow != 0,
        pf: parity_even(result),
        af: auxiliary != 0,
    }
}

fn auxiliary_add(dst: u64, src: u64) -> u8 {
    let dst = u64_to_bits(dst);
    let src = u64_to_bits(src);
    ripple_carry_adder_with_carry(&dst[..4], &src[..4], 0).carry_out
}

fn auxiliary_sub(dst: u64, src: u64) -> u8 {
    let dst = u64_to_bits(dst);
    let src = u64_to_bits(src);
    let inverted: Vec<u8> = src[..4].iter().copied().map(not_gate).collect();
    not_gate(ripple_carry_adder_with_carry(&dst[..4], &inverted, 1).carry_out)
}

fn parity_even(value: u64) -> bool {
    let bits = u64_to_bits(and_64(value, 0xff));
    let odd = bits[..8].iter().copied().fold(0, xor_gate);
    not_gate(odd) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_flags_match_reference_edges() {
        for (a, b) in [
            (0, 0),
            (u64::MAX, 1),
            (i64::MAX as u64, 1),
            (i64::MIN as u64, u64::MAX),
            (0x0f, 1),
        ] {
            assert_eq!(
                add_with_flags(a, b),
                x86_simulator::flags::add_with_flags(a, b)
            );
            assert_eq!(
                sub_with_flags(a, b),
                x86_simulator::flags::sub_with_flags(a, b)
            );
            for carry in [false, true] {
                assert_eq!(
                    adc_with_flags(a, b, carry),
                    x86_simulator::flags::adc_with_flags(a, b, carry)
                );
                assert_eq!(
                    sbb_with_flags(a, b, carry),
                    x86_simulator::flags::sbb_with_flags(a, b, carry)
                );
            }
        }
        assert_eq!(logic_flags(0), x86_simulator::flags::logic_flags(0));
        assert_eq!(
            logic_flags(0x8000_0000_0000_0001),
            x86_simulator::flags::logic_flags(0x8000_0000_0000_0001)
        );
        assert_eq!(crate::bits::xor_64(0xaaaa, 0x5555), 0xffff);
    }
}
