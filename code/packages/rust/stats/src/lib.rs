// # Stats — Statistics, Frequency Analysis, and Cryptanalysis
//
// This crate provides three categories of functions:
//
// 1. **Descriptive statistics** — mean, median, mode, variance,
//    standard deviation, min, max, range.
//
// 2. **Frequency analysis** — letter frequency counting, frequency
//    distributions, chi-squared tests.
//
// 3. **Cryptanalysis helpers** — index of coincidence, Shannon entropy,
//    and standard English letter frequency tables.
//
// ## Module Organization
//
// Each category lives in its own module for tree-shaking (users can
// import only what they need). The top-level `lib.rs` re-exports
// everything for convenience.

pub mod descriptive;
pub mod frequency;
pub mod cryptanalysis;

// Re-export all public items for convenience.
pub use descriptive::*;
pub use frequency::*;
pub use cryptanalysis::*;
