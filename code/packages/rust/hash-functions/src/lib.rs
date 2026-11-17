//! DT17 hash functions implemented from scratch in Rust.
//!
//! This crate mirrors the DT17 specification with a trait-based interface
//! plus free functions for direct use.  The statistical analysis helpers
//! (`avalanche_score`, `distribution_test`) live in `coding_adventures_hash_analysis`,
//! because they draw OS randomness and this crate is pure computation.

mod algorithms;

pub use algorithms::{
    djb2, fnv1a_32, fnv1a_64, hash_str_fnv1a_32, hash_str_siphash, murmur3_32,
    murmur3_32_with_seed, polynomial_rolling, polynomial_rolling_with_params,
    siphash_2_4, Djb2, Fnv1a32, Fnv1a64, HashFunction, Murmur3_32, PolynomialRolling,
    SipHash24, DJB2_OFFSET_BASIS, FNV32_OFFSET_BASIS, FNV32_PRIME, FNV64_OFFSET_BASIS,
    FNV64_PRIME, POLYNOMIAL_ROLLING_DEFAULT_BASE, POLYNOMIAL_ROLLING_DEFAULT_MODULUS,
};

// Compile and run the README's usage example as a doctest, so its asserted values
// cannot drift from the crate (`cargo test --doc`).  Only under `cfg(doctest)`:
// it adds nothing to the library or its public documentation.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
