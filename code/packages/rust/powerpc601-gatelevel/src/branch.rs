//! Conditional-branch control built from repository gates.
//!
//! PowerPC's five-bit branch-options field independently controls the count
//! register and condition-register halves of a branch decision:
//!
//! ```text
//! BO[0]  0: decrement CTR and test it   1: ignore CTR
//! BO[1]  polarity expected from the decremented CTR nonzero wire
//! BO[2]  0: test the selected CR bit    1: ignore CR
//! BO[3]  polarity expected from the selected CR bit
//! ```
//!
//! The instruction encoding numbers those controls from the most-significant
//! end of the five-bit field. In the LSB-first array used below, they therefore
//! arrive at indices 4, 3, 2, and 1 respectively. The network computes both
//! halves even when a bypass is selected; muxes choose the architectural
//! inputs, and a final AND produces the one-bit `taken` wire.

use logic_gates::{
    combinational::{mux2, mux_n},
    gates::{and_gate, not_gate, xnor_gate, xor_gate},
};

use crate::{
    alu,
    bits::{bits_to_u32, u32_to_bits, zero_32},
};

/// Stable outputs of the conditional-branch combinational network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchDecision {
    /// CTR after the BO-selected decrement/bypass mux.
    pub next_ctr: u32,
    /// The final one-bit branch wire at the public execution boundary.
    pub taken: bool,
}

/// Evaluate the PowerPC conditional-branch predicate through gate networks.
///
/// `bo` and `bi` are decoded five-bit instruction fields. Values outside that
/// width are safely masked at this public helper's selection boundary.
pub fn evaluate(ctr: u32, cr: u32, bo: u8, bi: u8) -> BranchDecision {
    let bo_bits = u32_to_bits(u32::from(bo & 0x1f));

    // BO[0] (encoded bit 4) bypasses both the decrement and its predicate.
    let decrement_enable = not_gate(bo_bits[4]);
    let decremented = alu::sub(ctr, 1).result;
    let ctr_bits = u32_to_bits(ctr);
    let decremented_bits = u32_to_bits(decremented);
    let next_ctr_bits: [u8; 32] =
        std::array::from_fn(|bit| mux2(ctr_bits[bit], decremented_bits[bit], decrement_enable));
    let next_ctr = bits_to_u32(&next_ctr_bits);
    let ctr_nonzero = not_gate(zero_32(next_ctr));
    let ctr_matches = xor_gate(ctr_nonzero, bo_bits[3]);
    let ctr_ok = mux2(ctr_matches, 1, bo_bits[4]);

    // CR bits are numbered from the architectural MSB; BI selects exactly one
    // stable-Q CR wire before BO[2] chooses it or the unconditional high wire.
    let cr_bits = u32_to_bits(cr);
    let cr_architectural_order: [u8; 32] = std::array::from_fn(|bit| cr_bits[31 - bit]);
    let bi_bits = u32_to_bits(u32::from(bi & 0x1f));
    let cr_bit = mux_n(&cr_architectural_order, &bi_bits[..5]);
    let condition_matches = xnor_gate(cr_bit, bo_bits[1]);
    let condition_ok = mux2(condition_matches, 1, bo_bits[2]);

    BranchDecision {
        next_ctr,
        taken: and_gate(ctr_ok, condition_ok) != 0,
    }
}
