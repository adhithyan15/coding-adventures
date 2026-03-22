//! Slice — the building block of a Configurable Logic Block (CLB).
//!
//! # What is a Slice?
//!
//! A slice is one "lane" inside a CLB. It combines:
//! - 2 LUTs (A and B) for combinational logic
//! - 2 D flip-flops for registered (sequential) outputs
//! - 2 output MUXes that choose between combinational or registered output
//! - Carry chain logic for fast arithmetic
//!
//! # Slice Architecture
//!
//! ```text
//! inputs_a --> [LUT A] --> +---------+
//!                          | MUX_A   |--> output_a
//!               +-> [FF A]-|         |
//!               |          +---------+
//!               |
//! inputs_b --> [LUT B] --> +---------+
//!                          | MUX_B   |--> output_b
//!               +-> [FF B]-|         |
//!               |          +---------+
//!               |
//! carry_in --> [CARRY] ----------------> carry_out
//!
//! clock -------> [FF A] [FF B]
//! ```
//!
//! # Carry Chain
//!
//! For arithmetic operations, the carry chain connects adjacent slices
//! to propagate carry bits without going through the general routing
//! fabric. This is what makes FPGA arithmetic fast.
//!
//! Our carry chain computes:
//!     `carry_out = (LUT_A_out AND LUT_B_out) OR (carry_in AND (LUT_A_out XOR LUT_B_out))`
//!
//! This is the standard full-adder carry equation where LUT_A computes
//! the generate signal and LUT_B computes the propagate signal.

use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, or_gate, xor_gate};
use logic_gates::sequential::{d_flip_flop, FlipFlopState};

use crate::lut::LUT;

/// Output from a single slice evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceOutput {
    /// LUT A result (combinational or registered).
    pub output_a: u8,
    /// LUT B result (combinational or registered).
    pub output_b: u8,
    /// Carry chain output (0 if carry disabled).
    pub carry_out: u8,
}

/// One slice of a CLB: 2 LUTs + 2 flip-flops + output MUXes + carry chain.
///
/// # Example — combinational AND + XOR
///
/// ```
/// use fpga::slice::Slice;
/// let mut s = Slice::new(4);
/// let mut and_tt = vec![0u8; 16]; and_tt[3] = 1;
/// let mut xor_tt = vec![0u8; 16]; xor_tt[1] = 1; xor_tt[2] = 1;
/// s.configure(&and_tt, &xor_tt, false, false, false);
/// let out = s.evaluate(&[1, 1, 0, 0], &[1, 0, 0, 0], 0, 0);
/// assert_eq!(out.output_a, 1); // AND(1,1) = 1
/// assert_eq!(out.output_b, 1); // XOR(1,0) = 1
/// ```
#[derive(Debug, Clone)]
pub struct Slice {
    lut_a: LUT,
    lut_b: LUT,
    k: usize,
    ff_a_state: FlipFlopState,
    ff_b_state: FlipFlopState,
    ff_a_enabled: bool,
    ff_b_enabled: bool,
    carry_enabled: bool,
}

impl Slice {
    /// Create a new slice with the given number of LUT inputs.
    pub fn new(lut_inputs: usize) -> Self {
        Self {
            lut_a: LUT::new(lut_inputs, None),
            lut_b: LUT::new(lut_inputs, None),
            k: lut_inputs,
            ff_a_state: FlipFlopState::default(),
            ff_b_state: FlipFlopState::default(),
            ff_a_enabled: false,
            ff_b_enabled: false,
            carry_enabled: false,
        }
    }

    /// Configure the slice's LUTs, flip-flops, and carry chain.
    ///
    /// # Parameters
    ///
    /// - `lut_a_table`: Truth table for LUT A (2^k entries)
    /// - `lut_b_table`: Truth table for LUT B (2^k entries)
    /// - `ff_a_enabled`: Route LUT A output through flip-flop A
    /// - `ff_b_enabled`: Route LUT B output through flip-flop B
    /// - `carry_enabled`: Enable carry chain computation
    pub fn configure(
        &mut self,
        lut_a_table: &[u8],
        lut_b_table: &[u8],
        ff_a_enabled: bool,
        ff_b_enabled: bool,
        carry_enabled: bool,
    ) {
        self.lut_a.configure(lut_a_table);
        self.lut_b.configure(lut_b_table);
        self.ff_a_enabled = ff_a_enabled;
        self.ff_b_enabled = ff_b_enabled;
        self.carry_enabled = carry_enabled;

        // Reset flip-flop state on reconfiguration
        self.ff_a_state = FlipFlopState::default();
        self.ff_b_state = FlipFlopState::default();
    }

    /// Evaluate the slice for one half-cycle.
    ///
    /// # Parameters
    ///
    /// - `inputs_a`: Input bits for LUT A (length k)
    /// - `inputs_b`: Input bits for LUT B (length k)
    /// - `clock`: Clock signal (0 or 1)
    /// - `carry_in`: Carry input from previous slice (default 0)
    ///
    /// # Returns
    ///
    /// [`SliceOutput`] with output_a, output_b, and carry_out.
    pub fn evaluate(
        &mut self,
        inputs_a: &[u8],
        inputs_b: &[u8],
        clock: u8,
        carry_in: u8,
    ) -> SliceOutput {
        // Evaluate LUTs (combinational — always computed)
        let lut_a_out = self.lut_a.evaluate(inputs_a);
        let lut_b_out = self.lut_b.evaluate(inputs_b);

        // Flip-flop A: route through if enabled
        let output_a = if self.ff_a_enabled {
            let (q_a, _q_bar_a) = d_flip_flop(lut_a_out, clock, &mut self.ff_a_state);
            // MUX: select registered (1) or combinational (0)
            mux2(lut_a_out, q_a, 1)
        } else {
            lut_a_out
        };

        // Flip-flop B: route through if enabled
        let output_b = if self.ff_b_enabled {
            let (q_b, _q_bar_b) = d_flip_flop(lut_b_out, clock, &mut self.ff_b_state);
            mux2(lut_b_out, q_b, 1)
        } else {
            lut_b_out
        };

        // Carry chain: standard full-adder carry equation
        //   carry_out = (A AND B) OR (carry_in AND (A XOR B))
        let carry_out = if self.carry_enabled {
            or_gate(
                and_gate(lut_a_out, lut_b_out),
                and_gate(carry_in, xor_gate(lut_a_out, lut_b_out)),
            )
        } else {
            0
        };

        SliceOutput {
            output_a,
            output_b,
            carry_out,
        }
    }

    /// LUT A (for inspection).
    pub fn lut_a(&self) -> &LUT {
        &self.lut_a
    }

    /// LUT B (for inspection).
    pub fn lut_b(&self) -> &LUT {
        &self.lut_b
    }

    /// Number of LUT inputs.
    pub fn k(&self) -> usize {
        self.k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_combinational() {
        let mut s = Slice::new(4);
        let mut and_tt = vec![0u8; 16];
        and_tt[3] = 1;
        let mut xor_tt = vec![0u8; 16];
        xor_tt[1] = 1;
        xor_tt[2] = 1;
        s.configure(&and_tt, &xor_tt, false, false, false);

        let out = s.evaluate(&[1, 1, 0, 0], &[1, 0, 0, 0], 0, 0);
        assert_eq!(out.output_a, 1); // AND(1,1)
        assert_eq!(out.output_b, 1); // XOR(1,0)
        assert_eq!(out.carry_out, 0); // carry disabled
    }

    #[test]
    fn test_slice_carry_chain() {
        let mut s = Slice::new(4);
        // LUT A = AND (generate), LUT B = XOR (propagate)
        let mut and_tt = vec![0u8; 16];
        and_tt[3] = 1;
        let mut xor_tt = vec![0u8; 16];
        xor_tt[1] = 1;
        xor_tt[2] = 1;
        s.configure(&and_tt, &xor_tt, false, false, true);

        // LUT A inputs [1,1] -> AND=1, LUT B inputs [1,1] -> XOR=0
        // carry_out = OR(AND(1,0), AND(cin=0, XOR(1,0))) = OR(0,0) = 0
        let out = s.evaluate(&[1, 1, 0, 0], &[1, 1, 0, 0], 0, 0);
        assert_eq!(out.carry_out, 0);

        // LUT A inputs [1,1] -> AND=1, LUT B inputs [1,0] -> XOR=1
        // carry_out = OR(AND(1,1), AND(cin=0, XOR(1,1))) = OR(1,0) = 1
        let out = s.evaluate(&[1, 1, 0, 0], &[1, 0, 0, 0], 0, 0);
        assert_eq!(out.carry_out, 1);

        // LUT A inputs [1,0] -> AND=0, LUT B inputs [1,0] -> XOR=1
        // carry_out = OR(AND(0,1), AND(cin=1, XOR(0,1))) = OR(0,1) = 1
        let out = s.evaluate(&[1, 0, 0, 0], &[1, 0, 0, 0], 0, 1);
        assert_eq!(out.carry_out, 1);
    }
}
