//! Stimulus helpers: `exhaustive` and `random_stimulus`.
//!
//! ## Exhaustive stimulus
//!
//! For small combinational circuits (up to ~20 input bits total) you can drive
//! *every* possible input combination.  That's at most 2^20 = 1 048 576 steps,
//! which a combinational simulator handles in milliseconds.
//!
//! Truth-table for a 2-input AND gate (`a`, `b`, each 1 bit):
//!
//! ```text
//! combo  | a  b  | a & b
//! -------|--------|------
//!   0    | 0  0  |   0
//!   1    | 1  0  |   0   ← bit 0 of combo → a
//!   2    | 0  1  |   0   ← bit 1 of combo → b
//!   3    | 1  1  |   1
//! ```
//!
//! The bit-slicing formula: for signal *i* with bit-offset `off_i` and
//! width `w_i`, extract `(combo >> off_i) & ((1 << w_i) - 1)`.
//!
//! ## Random stimulus
//!
//! For wider inputs, exhaustive search is impractical (2^64 steps would
//! take millennia).  `random_stimulus` drives N randomly chosen vectors.
//! We use a deterministic xorshift64 PRNG seeded by the caller so runs
//! are reproducible without any external dependency.
//!
//! xorshift64 recurrence (Marsaglia 2003):
//!
//! ```text
//! x ^= x << 13
//! x ^= x >> 7
//! x ^= x << 17
//! ```
//!
//! Period = 2^64 - 1 (every non-zero 64-bit value is visited exactly once).
//! Statistical quality is sufficient for HDL stimulus; use a cryptographic
//! PRNG if you need security properties.

use crate::runner::DutHandle;

// ---------------------------------------------------------------------------
// Exhaustive
// ---------------------------------------------------------------------------

/// Drive every combination of `inputs` and call `on_step` after each.
///
/// `inputs` is a slice of `(signal_name, bit_width)` pairs.  Total input
/// bits must not exceed 20, or this returns `Err` immediately.
///
/// ## Example
///
/// ```rust
/// use testbench_framework::{DutHandle, exhaustive};
///
/// fn check_and(dut: &mut DutHandle) {
///     let mut check = |d: &mut DutHandle| {
///         let expected = d.get("a") & d.get("b");
///         let _ = expected; // suppressed for doc example
///     };
///     exhaustive(dut, &[("a", 1), ("b", 1)], Some(&mut check)).unwrap();
/// }
/// ```
pub fn exhaustive(
    dut: &mut DutHandle,
    inputs: &[(&str, u32)],
    on_step: Option<&mut dyn FnMut(&mut DutHandle)>,
) -> Result<(), String> {
    let total_bits: u32 = inputs.iter().map(|(_, w)| w).sum();
    if total_bits > 20 {
        return Err(format!(
            "exhaustive over {total_bits} bits would take 2^{total_bits} iterations"
        ));
    }

    let n_combos: u64 = 1u64 << total_bits;

    if let Some(step_fn) = on_step {
        for combo in 0..n_combos {
            apply_combo(dut, inputs, combo);
            step_fn(dut);
        }
    } else {
        for combo in 0..n_combos {
            apply_combo(dut, inputs, combo);
        }
    }
    Ok(())
}

/// Decompose `combo` into per-signal values and drive them onto `dut`.
fn apply_combo(dut: &mut DutHandle, inputs: &[(&str, u32)], combo: u64) {
    let mut offset = 0u32;
    for (name, width) in inputs {
        let mask = (1u64 << width) - 1;
        let val = ((combo >> offset) & mask) as i64;
        dut.set(name, val);
        offset += width;
    }
}

// ---------------------------------------------------------------------------
// Random stimulus
// ---------------------------------------------------------------------------

/// Drive `iterations` random input vectors and call `on_step` after each.
///
/// `seed` controls the PRNG — the same seed always produces the same sequence,
/// making failing test cases reproducible.  Uses xorshift64 internally.
///
/// ## Example
///
/// ```rust
/// use testbench_framework::{DutHandle, random_stimulus};
///
/// fn stress_adder(dut: &mut DutHandle) {
///     random_stimulus(dut, &[("a", 4), ("b", 4)], 1000, 42, Some(&mut |d: &mut DutHandle| {
///         // just check it doesn't crash
///         let _ = d.get("sum");
///     }));
/// }
/// ```
pub fn random_stimulus(
    dut: &mut DutHandle,
    inputs: &[(&str, u32)],
    iterations: usize,
    seed: u64,
    on_step: Option<&mut dyn FnMut(&mut DutHandle)>,
) {
    let mut state = if seed == 0 { 1 } else { seed }; // xorshift64 must not start at 0

    if let Some(step_fn) = on_step {
        for _ in 0..iterations {
            for (name, width) in inputs {
                let mask = (1u64 << width) - 1;
                let val = (xorshift64(&mut state) & mask) as i64;
                dut.set(name, val);
            }
            step_fn(dut);
        }
    } else {
        for _ in 0..iterations {
            for (name, width) in inputs {
                let mask = (1u64 << width) - 1;
                let val = (xorshift64(&mut state) & mask) as i64;
                dut.set(name, val);
            }
        }
    }
}

/// Marsaglia (2003) xorshift64 — period 2^64 - 1.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}
