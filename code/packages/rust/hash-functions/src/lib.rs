//! DT17 hash functions implemented from scratch in Rust.
//!
//! This crate mirrors the DT17 specification with a trait-based interface
//! plus free functions for direct use.

mod algorithms;
mod analysis;

pub use algorithms::{
    djb2, fnv1a_32, fnv1a_64, hash_str_fnv1a_32, hash_str_siphash, murmur3_32,
    murmur3_32_with_seed, polynomial_rolling, polynomial_rolling_with_params,
    siphash_2_4, Djb2, Fnv1a32, Fnv1a64, HashFunction, Murmur3_32, PolynomialRolling,
    SipHash24, DJB2_OFFSET_BASIS, FNV32_OFFSET_BASIS, FNV32_PRIME, FNV64_OFFSET_BASIS,
    FNV64_PRIME, POLYNOMIAL_ROLLING_DEFAULT_BASE, POLYNOMIAL_ROLLING_DEFAULT_MODULUS,
};
pub use analysis::{avalanche_score, distribution_test};
