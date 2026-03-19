//! # IEEE 754 Floating-Point Arithmetic — from bits to fused multiply-add pipelines.
//!
//! This crate implements IEEE 754 floating-point formats and arithmetic operations,
//! built up from first principles. It is an educational implementation designed to
//! teach how floating-point numbers actually work at the hardware level.
//!
//! ## What is floating-point?
//!
//! Floating-point is how computers represent real numbers (like 3.14 or -0.001).
//! It works like scientific notation, but in binary:
//!
//! ```text
//! Scientific notation:   -6.022 x 10^23
//! IEEE 754 (binary):     (-1)^sign x 1.mantissa x 2^(exponent - bias)
//! ```
//!
//! ## Modules
//!
//! - **`formats`** — The bit-level anatomy of a float: `FloatFormat`, `FloatBits`,
//!   and the standard constants `FP32`, `FP16`, `BF16`.
//! - **`ieee754`** — Encoding and decoding: converting between Rust `f64` values
//!   and our explicit bit-level `FloatBits` representation.
//! - **`fp_adder`** — Floating-point addition, subtraction, negation, absolute
//!   value, and comparison.
//! - **`fp_multiplier`** — Floating-point multiplication using shift-and-add.
//! - **`fma`** — Fused multiply-add (single rounding!) and format conversion.
//! - **`pipeline`** — Pipelined arithmetic units that simulate GPU-style throughput.

pub mod formats;
pub mod ieee754;
pub mod fp_adder;
pub mod fp_multiplier;
pub mod fma;
pub mod pipeline;

// Re-export the most commonly used types and functions for convenience.
pub use formats::{FloatFormat, FloatBits, FP32, FP16, BF16};
pub use ieee754::{float_to_bits, bits_to_float, is_nan, is_inf, is_zero, is_denormalized};
pub use ieee754::{int_to_bits_msb, bits_msb_to_int};
pub use fp_adder::{fp_add, fp_sub, fp_neg, fp_abs, fp_compare};
pub use fp_multiplier::fp_mul;
pub use fma::{fp_fma, fp_convert};
pub use pipeline::{PipelinedFPAdder, PipelinedFPMultiplier, PipelinedFMA, FPUnit};
