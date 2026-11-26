//! # Sweeping a circuit over a whole domain, 64 cases at a time
//!
//! `Aig::eval` simulates the whole graph for ONE input assignment.  The cross-checks need every
//! case of a domain (65,536 at N = 8, 262,144 for the widest function probe), so this module
//! uses `Aig::simulate`, which evaluates 64 assignments at once: bit `p` of every input word is
//! the value of that input in the `p`-th case of the batch.
//!
//! ```text
//!   cases 64..127 of a 3-input circuit        inputs[j] is a u64 whose bit p is
//!   case 64 = 0b000001000000 ...              bit j of (first + p)
//! ```
//!
//! A **case** is a number `c` and input `j` is bit `j` of `c` (the numbering of
//! [`crate::lemmas`]).  A domain of `k` inputs has `2^k` cases, so `k` is bounded
//! ([`MAX_SWEEP_INPUTS`]): a sweep is exhaustive or it is refused, never sampled.

use bit_blaster::aig::{edge_value, Aig, Edge};

/// The most inputs a sweep accepts: `2^24` cases is far above every domain here (the widest is
/// 18 inputs) and bounds the work of a malformed request.
pub const MAX_SWEEP_INPUTS: usize = 24;

/// Why a sweep was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TooWide(pub usize);

/// Call `f(case, value)` for every case (`inputs` must equal the AIG's input count) `0 .. 2^inputs`, in order; `value(e)` is edge `e`'s truth
/// on that case.
pub fn sweep(
    aig: &Aig,
    inputs: usize,
    mut f: impl FnMut(u64, &dyn Fn(Edge) -> bool),
) -> Result<(), TooWide> {
    if inputs > MAX_SWEEP_INPUTS || inputs != aig.num_inputs() {
        return Err(TooWide(inputs));
    }
    let total: u64 = 1 << inputs;
    let mut words = vec![0u64; aig.num_inputs()];
    let mut first = 0u64;
    while first < total {
        let batch = (total - first).min(64);
        for (j, w) in words.iter_mut().enumerate() {
            *w = 0;
            for p in 0..batch {
                *w |= (((first + p) >> j) & 1) << p;
            }
        }
        let values = aig.simulate(&words);
        for p in 0..batch {
            let value = |e: Edge| (edge_value(&values, e) >> p) & 1 == 1;
            f(first + p, &value);
        }
        first += batch;
    }
    Ok(())
}
