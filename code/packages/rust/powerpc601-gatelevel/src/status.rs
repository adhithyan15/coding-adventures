//! Gate-authoritative PowerPC condition-register and XER status networks.
//!
//! PowerPC numbers architectural bits from the most-significant end, while
//! the repository gate bridges use LSB-first arrays.  Consequently XER SO,
//! OV, and CA are array wires 31, 30, and 29.  Every public helper in this
//! module accepts or returns one-bit wires and preserves unselected stable-Q
//! state through muxes.

use logic_gates::{
    combinational::mux2,
    gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate},
};

use crate::{
    alu,
    bits::{bits_to_u32, u32_to_bits},
};

/// One-hot signed or unsigned comparison result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationWires {
    pub lt: u8,
    pub gt: u8,
    pub eq: u8,
}

fn relation_from_subtract(zero: u8, less: u8) -> RelationWires {
    let unequal = not_gate(zero);
    RelationWires {
        lt: and_gate(unequal, less),
        gt: and_gate(unequal, not_gate(less)),
        eq: zero,
    }
}

/// Signed relation truth table at the subtractor control-wire boundary.
pub fn signed_relation(zero: u8, negative: u8, overflow: u8) -> RelationWires {
    relation_from_subtract(zero, xor_gate(negative, overflow))
}

/// Unsigned relation truth table at the subtractor control-wire boundary.
pub fn unsigned_relation(zero: u8, carry: u8) -> RelationWires {
    relation_from_subtract(zero, not_gate(carry))
}

/// Signed relation wires derived from subtractor Z, N, and V outputs.
pub fn compare_signed(left: u32, right: u32) -> RelationWires {
    let difference = alu::sub(left, right);
    signed_relation(difference.zero, difference.negative, difference.overflow)
}

/// Unsigned relation wires derived from subtractor Z and carry/no-borrow.
pub fn compare_unsigned(left: u32, right: u32) -> RelationWires {
    let difference = alu::sub(left, right);
    unsigned_relation(difference.zero, difference.carry)
}

fn decode_lines<const INPUTS: usize, const OUTPUTS: usize>(inputs: &[u8; INPUTS]) -> [u8; OUTPUTS] {
    std::array::from_fn(|line| {
        inputs.iter().enumerate().fold(1, |selected, (bit, input)| {
            let expected = ((line >> bit) & 1) as u8;
            and_gate(selected, xnor_gate(*input, expected))
        })
    })
}

fn relation_bits(relation: RelationWires, so: u8) -> [u8; 4] {
    [so, relation.eq, relation.gt, relation.lt]
}

/// Replace exactly one CR field selected by three decoder wires.
pub fn write_cr_field(
    old_cr: u32,
    field_selector: [u8; 3],
    relation: RelationWires,
    so: u8,
) -> u32 {
    let selected = decode_lines::<3, 8>(&field_selector);
    let old = u32_to_bits(old_cr);
    let nibble = relation_bits(relation, so);
    let updated: [u8; 32] = std::array::from_fn(|bit| {
        let architectural_field = 7 - bit / 4;
        mux2(old[bit], nibble[bit % 4], selected[architectural_field])
    });
    bits_to_u32(&updated)
}

/// Mux an optional signed relation-against-zero result into CR0.
pub fn write_cr0(old_cr: u32, value: u32, so: u8, write_enable: u8) -> u32 {
    let candidate = write_cr_field(old_cr, [0, 0, 0], compare_signed(value, 0), so);
    let old = u32_to_bits(old_cr);
    let candidate = u32_to_bits(candidate);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        mux2(old[bit], candidate[bit], write_enable)
    }))
}

/// Replace only `XER[CA]` through an old/new/write-enable mux.
pub fn write_ca(old_xer: u32, new_ca: u8, write_enable: u8) -> u32 {
    let mut updated = u32_to_bits(old_xer);
    updated[29] = mux2(updated[29], new_ca, write_enable);
    bits_to_u32(&updated)
}

/// Stable-Q `XER[SO]` wire.
pub fn so(xer: u32) -> u8 {
    u32_to_bits(xer)[31]
}

/// Stable-Q `XER[CA]` wire.
pub fn ca(xer: u32) -> u8 {
    u32_to_bits(xer)[29]
}

/// Eight field-mask wires and 32 muxes implement `mtcrf` field isolation.
pub fn mtcrf(old_cr: u32, source: u32, field_mask: [u8; 8]) -> u32 {
    let old = u32_to_bits(old_cr);
    let source = u32_to_bits(source);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        let architectural_field = 7 - bit / 4;
        mux2(old[bit], source[bit], field_mask[7 - architectural_field])
    }))
}

/// The six low count wires used by register-form word shifts.
pub fn count_wires(value: u32) -> [u8; 6] {
    let bits = u32_to_bits(value);
    std::array::from_fn(|bit| bits[bit])
}

/// `XER[CA]` for `sraw`/`srawi`, including the architectural 32..63 edge.
///
/// `discarded_by_count[n]` is a fixed OR tree over the lowest `n` source
/// wires.  Counts 32..63 reuse the complete 32-wire reduction, so a negative
/// source necessarily selects its sign wire without changing the shift
/// datapath owned by the separate repair cell.
pub fn arithmetic_right_shift_ca(source: u32, count: [u8; 6]) -> u8 {
    let source = u32_to_bits(source);
    let sign = source[31];
    let mut discarded_by_count = [0_u8; 64];
    let mut discarded = 0;
    for (bit, source_wire) in source.into_iter().enumerate() {
        discarded = or_gate(discarded, source_wire);
        discarded_by_count[bit + 1] = and_gate(sign, discarded);
    }
    discarded_by_count[33..].fill(and_gate(sign, discarded));

    let selected = decode_lines::<6, 64>(&count);
    (0..64).fold(0, |ca, line| {
        or_gate(ca, and_gate(selected[line], discarded_by_count[line]))
    })
}
