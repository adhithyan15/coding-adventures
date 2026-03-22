//! Configurable Logic Block (CLB) — the core compute tile of an FPGA.
//!
//! # What is a CLB?
//!
//! A CLB is the primary logic resource in an FPGA. It is a tile on the FPGA
//! grid that contains multiple slices, each with LUTs, flip-flops, and carry
//! chains. CLBs are connected to each other through the routing fabric.
//!
//! # CLB Architecture
//!
//! Our CLB follows the Xilinx-style architecture with 2 slices:
//!
//! ```text
//! +----------------------------------------------+
//! |                     CLB                       |
//! |                                               |
//! |  +---------------------+                      |
//! |  |       Slice 0       |                      |
//! |  |  [LUT A] [LUT B]   |                      |
//! |  |  [FF A]  [FF B]    |                      |
//! |  |  [carry chain]      |                      |
//! |  +---------+-----------+                      |
//! |            | carry                             |
//! |  +---------v-----------+                      |
//! |  |       Slice 1       |                      |
//! |  |  [LUT A] [LUT B]   |                      |
//! |  |  [FF A]  [FF B]    |                      |
//! |  |  [carry chain]      |                      |
//! |  +---------------------+                      |
//! |                                               |
//! +----------------------------------------------+
//! ```
//!
//! The carry chain flows from slice 0 to slice 1, enabling fast multi-bit
//! arithmetic within a single CLB.
//!
//! # CLB Capacity
//!
//! One CLB with 2 slices x 2 LUTs per slice = 4 LUTs total.
//!
//! A 4-input LUT can implement any boolean function of 4 variables, so
//! one CLB provides 4 independent boolean functions.

use crate::slice::{Slice, SliceOutput};

/// Output from a CLB evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CLBOutput {
    /// Output from slice 0.
    pub slice0: SliceOutput,
    /// Output from slice 1.
    pub slice1: SliceOutput,
}

/// Configurable Logic Block — contains 2 slices.
///
/// The carry chain connects slice 0's carry_out to slice 1's carry_in,
/// enabling fast multi-bit arithmetic.
///
/// # Example — 2-bit adder using carry chain
///
/// ```
/// use fpga::clb::CLB;
/// let mut clb = CLB::new(4);
/// // Each slice computes one bit of the addition
/// // LUT A = XOR (sum bit), LUT B = AND (generate carry)
/// let mut xor_tt = vec![0u8; 16]; xor_tt[1] = 1; xor_tt[2] = 1;
/// let mut and_tt = vec![0u8; 16]; and_tt[3] = 1;
/// clb.slice0_mut().configure(&xor_tt, &and_tt, false, false, true);
/// clb.slice1_mut().configure(&xor_tt, &and_tt, false, false, true);
/// ```
#[derive(Debug, Clone)]
pub struct CLB {
    slice0: Slice,
    slice1: Slice,
    k: usize,
}

impl CLB {
    /// Create a new CLB with 2 slices.
    pub fn new(lut_inputs: usize) -> Self {
        Self {
            slice0: Slice::new(lut_inputs),
            slice1: Slice::new(lut_inputs),
            k: lut_inputs,
        }
    }

    /// First slice (immutable).
    pub fn slice0(&self) -> &Slice {
        &self.slice0
    }

    /// First slice (mutable, for configuration).
    pub fn slice0_mut(&mut self) -> &mut Slice {
        &mut self.slice0
    }

    /// Second slice (immutable).
    pub fn slice1(&self) -> &Slice {
        &self.slice1
    }

    /// Second slice (mutable, for configuration).
    pub fn slice1_mut(&mut self) -> &mut Slice {
        &mut self.slice1
    }

    /// Number of LUT inputs per slice.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Evaluate both slices in the CLB.
    ///
    /// The carry chain flows: carry_in -> slice0 -> slice1.
    ///
    /// # Parameters
    ///
    /// - `slice0_inputs_a/b`: Inputs for slice 0's LUTs
    /// - `slice1_inputs_a/b`: Inputs for slice 1's LUTs
    /// - `clock`: Clock signal (0 or 1)
    /// - `carry_in`: External carry input (default 0)
    pub fn evaluate(
        &mut self,
        slice0_inputs_a: &[u8],
        slice0_inputs_b: &[u8],
        slice1_inputs_a: &[u8],
        slice1_inputs_b: &[u8],
        clock: u8,
        carry_in: u8,
    ) -> CLBOutput {
        // Evaluate slice 0 first (carry chain starts here)
        let out0 = self.slice0.evaluate(slice0_inputs_a, slice0_inputs_b, clock, carry_in);

        // Slice 1 receives carry from slice 0
        let out1 =
            self.slice1
                .evaluate(slice1_inputs_a, slice1_inputs_b, clock, out0.carry_out);

        CLBOutput {
            slice0: out0,
            slice1: out1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clb_basic_evaluation() {
        let mut clb = CLB::new(4);
        let mut and_tt = vec![0u8; 16];
        and_tt[3] = 1;
        let zeros = vec![0u8; 16];

        clb.slice0_mut()
            .configure(&and_tt, &zeros, false, false, false);
        clb.slice1_mut()
            .configure(&and_tt, &zeros, false, false, false);

        let out = clb.evaluate(
            &[1, 1, 0, 0],
            &[0, 0, 0, 0],
            &[1, 0, 0, 0],
            &[0, 0, 0, 0],
            0,
            0,
        );

        assert_eq!(out.slice0.output_a, 1); // AND(1,1) = 1
        assert_eq!(out.slice1.output_a, 0); // AND(1,0) = 0
    }

    #[test]
    fn test_clb_carry_chain_propagation() {
        let mut clb = CLB::new(4);
        let mut and_tt = vec![0u8; 16];
        and_tt[3] = 1;
        let mut xor_tt = vec![0u8; 16];
        xor_tt[1] = 1;
        xor_tt[2] = 1;

        clb.slice0_mut()
            .configure(&and_tt, &xor_tt, false, false, true);
        clb.slice1_mut()
            .configure(&and_tt, &xor_tt, false, false, true);

        // Slice 0: LUT A(1,1)->1, LUT B(1,0)->1
        // carry_out = OR(AND(1,1), AND(0, XOR(1,1))) = 1
        let out = clb.evaluate(
            &[1, 1, 0, 0], // slice0 LUT A: AND(1,1)=1
            &[1, 0, 0, 0], // slice0 LUT B: XOR(1,0)=1
            &[0, 0, 0, 0],
            &[0, 0, 0, 0],
            0,
            0,
        );

        assert_eq!(out.slice0.carry_out, 1);
    }
}
