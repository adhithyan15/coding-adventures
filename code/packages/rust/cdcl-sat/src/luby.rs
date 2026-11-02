//! # The Luby restart sequence
//!
//! A CDCL solver **restarts**: every so often it throws away all its current
//! decisions (backtracks to level 0) while *keeping* everything it learned.
//! Early decisions made before the solver knew much are often bad, and a
//! restart lets VSIDS, now better informed, choose again.
//!
//! How often to restart? Luby, Sinclair and Zuckerman (1993) showed that the
//! sequence
//!
//! ```text
//!   1, 1, 2, 1, 1, 2, 4, 1, 1, 2, 1, 1, 2, 4, 8, 1, 1, 2, …
//! ```
//!
//! is within a logarithmic factor of the best possible restart strategy for a
//! randomised algorithm whose run-time distribution is unknown. The solver
//! runs `UNIT · luby(i)` conflicts before restart `i`.
//!
//! ## How the sequence is built
//!
//! The sequence is self-similar. Write it in blocks whose lengths are
//! `2^k - 1`:
//!
//! ```text
//!   block of length 1:   1
//!   block of length 3:   1 1 2
//!   block of length 7:   1 1 2 1 1 2 4
//!   block of length 15:  1 1 2 1 1 2 4 1 1 2 1 1 2 4 8
//! ```
//!
//! A block of length `2^k - 1` is two copies of the previous block followed
//! by `2^(k-1)`. To find term `i`: find the smallest block that contains
//! `i`; if `i` is its last position the answer is that block's final power
//! of two; otherwise reduce `i` into the shorter block and repeat.

/// Term `i` (0-based) of the Luby sequence.
pub fn luby(mut i: u64) -> u64 {
    // Find the smallest complete block (length 2^k - 1) that contains
    // position i, tracking k in `exponent`.
    let mut size: u64 = 1;
    let mut exponent: u32 = 0;
    while size < i + 1 {
        exponent += 1;
        size = 2 * size + 1;
    }
    // Walk down: if i is not the block's final slot, it lies in one of the
    // two copies of the previous block, at position i mod (that block's size).
    while size - 1 != i {
        size = (size - 1) >> 1;
        exponent -= 1;
        i %= size;
    }
    1u64 << exponent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_terms_match_the_published_sequence() {
        let expected = [1, 1, 2, 1, 1, 2, 4, 1, 1, 2, 1, 1, 2, 4, 8, 1, 1, 2];
        let got: Vec<u64> = (0..expected.len() as u64).map(luby).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn block_ends_are_powers_of_two() {
        // Position 2^k - 2 is the end of the block of length 2^k - 1.
        for k in 1..20u32 {
            assert_eq!(luby((1u64 << k) - 2), 1u64 << (k - 1));
        }
    }
}
